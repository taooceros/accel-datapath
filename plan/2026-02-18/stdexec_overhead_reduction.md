# Plan: Reduce stdexec Per-Op Overhead

## Problem

The current hot path builds a 3-layer sender chain per operation:
```
scope.nest( dsa_data_move(dsa, src, dst, size) | stdexec::then(record) )
```
This costs ~20 ns/op in stdexec machinery (connect, nest, then, set_value propagation),
which is 60% of the total ~35 ns budget. None of these layers are needed per-op:
- `scope.nest()`: cancellation/lifetime tracking — we manage lifetimes via SlotArena
- `stdexec::then(record)`: wraps CompletionRecord — can be folded into the receiver
- Per-op `connect()`: constructs ~448-byte nested op state — could be pre-built and reused

## Level 1: Direct Connect (eliminate nest + then)

**Target**: ~35 ns → ~13 ns/op (77 Mpps)

### Changes

1. **Create `DirectBenchReceiver`** in `benchmark/dsa/helpers.hpp`:
   ```cpp
   template <size_t SlotSize>
   struct DirectBenchReceiver {
     using receiver_concept = stdexec::receiver_t;
     SlotArena<SlotSize> *arena;
     OperationSlot<SlotSize> *slot;
     std::atomic<size_t> *remaining;  // counts down to 0
     LatencyCollector *latency;
     std::chrono::high_resolution_clock::time_point start_time;

     void set_value(auto&&...) && noexcept {
       if (latency->enabled()) {
         auto end = std::chrono::high_resolution_clock::now();
         latency->record(std::chrono::duration<double, std::nano>(end - start_time).count());
       }
       remaining->fetch_sub(1, std::memory_order_release);
       arena->release(slot);
     }
     void set_error(auto&&) && noexcept {
       remaining->fetch_sub(1, std::memory_order_release);
       arena->release(slot);
     }
     void set_stopped() && noexcept {
       remaining->fetch_sub(1, std::memory_order_release);
       arena->release(slot);
     }
     auto get_env() const noexcept { return stdexec::empty_env{}; }
   };
   ```

2. **New strategy `sliding_window_direct_impl_inline`**:
   ```cpp
   // No async_scope, no scope.nest(), no stdexec::then()
   size_t remaining = num_ops;
   while (next_op < num_ops) {
     while (next_op < num_ops) {
       auto *slot = arena.acquire();
       if (!slot) break;
       size_t offset = next_op * msg_size;
       auto recv = DirectBenchReceiver<SlotSize>{
         &arena, slot, &remaining, &latency,
         latency.enabled() ? clock::now() : clock::time_point{}
       };
       slot->start_op_with(make_sender(offset), std::move(recv));
       ++next_op;
     }
     dsa.poll();
   }
   // Drain: poll until all ops complete
   while (remaining.load(std::memory_order_acquire) > 0) {
     dsa.poll();
   }
   ```

3. **Add `SchedulingPattern::SlidingWindowDirect`** in config.

4. **Compute slot size** for direct connect (no NestSender wrapper):
   ```cpp
   template <class MakeSender, size_t SlotSize>
   constexpr size_t direct_slot_size() {
     using Sender = decltype(std::declval<MakeSender>()(size_t{0}));
     using Receiver = DirectBenchReceiver<SlotSize>;
     return sizeof(stdexec::connect_result_t<Sender, Receiver>);
   }
   ```
   Note: circular dependency (receiver needs SlotSize, SlotSize needs receiver).
   Bootstrap: use a fixed-size receiver proxy or two-pass computation.

## Level 2: Reusable Operation States (eliminate connect)

**Target**: ~13 ns → ~8 ns/op (125 Mpps)

### Changes

1. **`ReusableSlot`** that pre-constructs `DataMoveOperation` once per slot:
   - On reuse: update `src_`, `dst_` fields, memset desc/comp, fill_descriptor, submit
   - Bypasses `stdexec::connect()` and `stdexec::start()` — calls internal methods directly
   - Still uses stdexec types (`DataMoveOperation`, `DsaOperationMixin`)

2. **New strategy `sliding_window_reusable_impl_inline`**:
   - Pre-allocate ReusableSlots
   - Hot loop: `slot->relaunch(offset)` — no connect, no start, just fill + submit

3. This is more invasive — requires either:
   - Making `DataMoveOperation` fields mutable after construction
   - Or creating a thin wrapper that holds the operation state and provides `reset()`

## Execution

- **Level 1** and **Level 2** can be implemented in parallel as separate scheduling patterns
- Both can be benchmarked independently against the baseline
- Level 2 depends on understanding from Level 1 (same receiver design)

## Success Criteria

| Metric | Current | Level 1 Target | Level 2 Target |
|--------|---------|---------------|---------------|
| Mock c=2048 msg=8 | 27 Mpps | 60-77 Mpps | 100-125 Mpps |
| Per-op latency | ~35 ns | ~13-17 ns | ~8-10 ns |
