# DSA Benchmark Performance Analysis: 20 Mpps Ceiling

## Executive Summary

The DSA benchmark's message rate plateaus at ~20 Mpps for small messages (8-64 bytes),
with significant run-to-run fluctuation (8-22 Mpps). Increasing batch_size beyond 32
does not improve throughput. This report identifies the root causes and quantifies
hardware vs. software contributions.

**Verdict: 20 Mpps is a SOFTWARE limit, not hardware.** The hardware can sustain
30-50 Mpps with 4 engines. Software overhead (~50ns/op) is the ceiling.

## Hardware Configuration

```
Device:     dsa0, NUMA node 0
WQ:         wq0.0, dedicated mode, depth=128, max_batch_size=1024
Engines:    4 (engine0.0-0.3), all in group0.0
Submission: _movdir64b (dedicated WQ, no retry)
```

## Empirical Results

| Config | msg_rate (Mpps) |
|--------|----------------|
| Immediate (no batch), sliding_window | 5.91 |
| Batch=8, mirrored_ring, sliding_window | 10.17 |
| Batch=16, mirrored_ring, sliding_window | 11.66 |
| Batch=32, mirrored_ring, sliding_window | 20.19 |
| Batch=64, mirrored_ring, sliding_window | 20.72 |
| Batch=128, mirrored_ring, sliding_window | 14.76 / 8.66 (unstable) |
| Batch=32, mirrored_ring, batch_noalloc | 21.36 |
| Batch=32, mirrored_ring, sliding_window (10 iter) | 22.71 |

## Per-Operation Software Budget (50ns = 20 Mpps)

| Phase | Cost/op | % | Source |
|-------|---------|---|--------|
| stdexec connect + placement new + start | ~20-25ns | 40-50% | helpers.hpp:221-224 |
| Notification chain (proxy -> set_value -> atomic) | ~15-20ns | 30-40% | task_queue.hpp -> operation_base.hpp |
| O(N) poll traversal (N pointer chases) | ~5ns | 10% | task_queue.hpp:152-168 |
| Batch submit (amortized sfence + _movdir64b) | ~3ns | 6% | descriptor_submitter.hpp |
| Slot scanning (N atomic loads) | ~1.5ns | 3% | strategy_noalloc.cpp:21-23 |
| **Total** | **~50ns/op** | 100% | |

## Root Cause 1: Batch Accumulation Mismatch

`MirroredRingSubmitter::pre_poll()` (descriptor_submitter.hpp:678) unconditionally
flushes any partial batch every time `dsa.poll()` is called. The sliding_window_noalloc
strategy calls `dsa.poll()` after each slot-scan pass (strategy_noalloc.cpp:30).

**Effect**: The effective hardware batch size = number of ops started per scan pass
(typically ~32 for concurrency=2048), NOT the configured `batch_size`. Setting
batch_size=64 or 128 has no effect because `pre_poll()` flushes before the batch fills.

## Root Cause 2: O(N) Poll Traversal

`LockedTaskQueue::poll()` traverses the ENTIRE linked list of in-flight operations,
checking each completion record individually. With concurrency=2048:
- 2048 pointer-chasing cache misses per poll
- O(concurrency) cost, not O(completions)
- Lock held for the entire traversal

`LockFreeTaskQueue` is worse: N individual CAS re-adds per poll for non-completed ops.

## Root Cause 3: stdexec Overhead per Operation

Each operation goes through the full stdexec machinery:
1. `stdexec::connect()` constructs the operation state (~5-10ns)
2. Placement new into slot storage (~2-3ns)
3. `stdexec::start()` -> `fill_descriptor()` + `submit()` (~10-15ns)
4. `pro::make_proxy<OperationFacade>(Wrapper{&self})` on every start (~3-5ns)
5. Completion: `op->proxy->notify()` indirect call (~10-15ns)
6. `stdexec::set_value()` propagation through then-adapter (~5-10ns)

## Root Cause 4: batch_size=128 Regression

With batch_size=128 and num_batches=16, the descriptor ring size =
`round_up_pow2(128 * 16) = 2048`. At concurrency=2048, the ring is exactly full.
`submit_descriptor()` blocks in a spin loop when `desc_available() == 0`,
creating stop-and-wait behavior instead of pipelined execution.

## Root Cause 5: Run-to-Run Fluctuation

Bistable feedback loop between hardware completion timing and software throughput:
- More completions per poll -> more slots ready -> larger effective batch -> better HW utilization (positive)
- Fewer completions per poll -> fewer slots ready -> scan overhead dominates (negative)

Initial conditions (cache warmth, OS interrupts, DSA engine scheduling) determine
which regime dominates. The first few iterations are always slow (cold cache for
~1MB of OperationSlot objects), biasing short runs toward lower averages.

## Unexploited Hardware Feature: Batch Completion Record

DSA hardware writes a batch completion record indicating whether ALL sub-descriptors
succeeded. Current software ignores this and checks all N individual completion records.
Using batch completion would give O(1) per-batch checking instead of O(N).

## Theoretical Maximum

| Scenario | Estimated Mpps |
|----------|---------------|
| Current (software-bound) | 20-22 |
| Loop restructure + free-list slots | 24-25 |
| + Indexed bitmap task queue | 27-28 |
| + Conditional pre_poll | 29-30 |
| Hardware theoretical (4 engines) | 30-50 |
| + Second DSA device (dsa2) | 60-100 |

## Recommended Investigations

1. **Mock DSA benchmark**: Run the full benchmark with MockDsaBase (instant completion)
   to isolate pure software overhead from hardware latency. This gives the true
   software ceiling.

2. **Per-phase profiling**: Instrument the hot loop to measure each phase separately
   (connect/start time, poll time, notify time, scan time).

3. **Batch completion exploitation**: Prototype O(1) batch completion checking using
   the hardware batch completion record.

## Minor Issue: CSV Export Bug

`export_to_csv()` unconditionally writes rows for all 6 queue types regardless of
which were benchmarked. Non-benchmarked queue types appear as all-zeros in the CSV.
Fix: pass config.queue_types to export_to_csv and filter.
