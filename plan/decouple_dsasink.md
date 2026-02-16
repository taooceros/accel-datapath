# Plan: Decouple DsaSink from Hardware Submission

## Problem Statement

The `DsaSink` concept currently lives in `include/dsa_stdexec/dsa_sink.hpp` but is
**never actually used** — no template parameter is constrained by it, and nothing
includes it. More importantly, the interface it describes couples two orthogonal
concerns:

1. **Hardware submission** — writing descriptors to the DSA work queue portal
   (`_movdir64b` / `_enqcmd`) or staging them for batching
2. **Completion tracking** — pushing operations onto a task queue and polling for
   completion

These are separate concerns that are currently fused in `DsaBase::submit()`:

```cpp
void DsaBase::submit(OperationBase *op, dsa_hw_desc *desc) {
    // Concern 1: hardware submission
    _mm_sfence();
    _movdir64b(wq_portal_, desc);
    // Concern 2: completion tracking
    task_queue_.push(op);
}
```

The batch variants (`DsaBatchBase`, `DsaRingBatchBase`, `DsaFixedRingBatchBase`) only
differ in Concern 1 (how descriptors reach hardware) while sharing identical Concern 2
logic. Yet from the stdexec layer's perspective, both concerns are accessed through the
same `dsa_.submit(op, desc)` call inside `DsaOperationMixin::start()`.

## Current Architecture

```
                     ┌──────────────────┐
                     │  DataMoveOp etc. │  (stdexec operation senders)
                     │  self.dsa_.submit│
                     └────────┬─────────┘
                              │  calls submit(op, desc)
                              ▼
              ┌───────────────────────────────┐
              │  DsaBase / DsaBatchBase / ... │  (fused: HW + queue)
              │  submit(op, desc) {           │
              │    hw_submit(desc);           │
              │    task_queue_.push(op);       │
              │  }                            │
              └───────────────────────────────┘
```

### What the stdexec layer actually needs

The operation senders (`DsaOperationMixin::start()` and `notify()`) need exactly two
things from their `dsa_` reference:

1. `dsa_.submit(this, desc)` — enqueue a descriptor for hardware execution AND track
   the operation for completion notification
2. `dsa_.submit(this)` — track an operation for completion notification only (used by
   `DsaScheduler`)

The operations don't care *how* descriptors reach hardware (direct MMIO, staged batch,
ring buffer batch). They also don't care *how* completion polling works (mutex queue,
spinlock queue, lock-free queue, ring buffer queue).

### What the hardware layer provides

`DsaBase` is actually doing three things:
1. Device discovery + work queue mapping (constructor)
2. Descriptor submission to hardware portal (`submit_raw`, or inline in `submit`)
3. Completion queue management (`task_queue_.push`, `task_queue_.poll`)

The batch variants wrap #2 with staging logic while delegating #3 to `inner_.task_queue()`.

## Proposed Decoupling

Split the `DsaSink` concept into two independent concepts:

### `DescriptorSink` — how descriptors reach hardware

```cpp
template <typename T>
concept DescriptorSink = requires(T &sink, dsa_hw_desc *desc) {
    { sink.submit_descriptor(desc) } -> std::same_as<void>;
};
```

Implementations:
- **`DirectSink`** — wraps `_movdir64b` / `_enqcmd` (what DsaBase does today)
- **`StagingSink`** — double-buffered staging array (what DsaBatchBase does)
- **`RingSink`** — ring buffer staging (what DsaRingBatchBase does)
- **`FixedRingSink`** — fixed-capacity ring staging (what DsaFixedRingBatchBase does)

### `CompletionQueue` — how operations are tracked

```cpp
template <typename T>
concept CompletionQueue = requires(T &q, OperationBase *op) {
    { q.push(op) } -> std::same_as<void>;
    { q.poll() };
};
```

This is essentially what `TaskQueue` already is, but exposed at the `dsa_stdexec`
abstraction level. The existing `TaskQueue` concept in `task_queue.hpp` already captures
this. The insight is that `CompletionQueue` doesn't need to be inside `DsaBase` — it
could be a separate object.

### Composed `DsaSink` — the unified interface operations use

```cpp
template <typename T>
concept DsaSink = requires(T &dsa, OperationBase *op, dsa_hw_desc *desc) {
    { dsa.submit(op, desc) } -> std::same_as<void>;  // descriptor + track
    { dsa.submit(op) } -> std::same_as<void>;         // track only
    { dsa.poll() };
};
```

This stays the same from the operation sender's perspective. But now the *implementations*
compose a `DescriptorSink` and a `CompletionQueue` rather than implementing everything
monolithically.

## Design Options

### Option A: Extract DescriptorSink as a strategy object (composition)

`DsaBase` gains a `DescriptorSink` template parameter:

```cpp
template <DescriptorSink Sink, template<typename> class QueueTemplate>
class DsaEngine {
public:
    void submit(OperationBase *op, dsa_hw_desc *desc) {
        if (desc) sink_.submit_descriptor(desc);
        task_queue_.push(op);
    }
    void submit(OperationBase *op) { task_queue_.push(op); }
    void poll() { flush(); task_queue_.poll(); }
    void flush() { if constexpr (requires { sink_.flush(); }) sink_.flush(); }

private:
    Sink sink_;
    QueueTemplate<DsaHwContext> task_queue_;
};
```

**Pros:** Clean separation. Each sink is a simple, testable unit. New batching
strategies just implement `DescriptorSink`. Task queue logic is shared.

**Cons:** Requires reworking `DsaBase`, `DsaBatchBase`, etc. into a unified template.
The batch variants currently use composition (`has-a DsaBase`), which would need to
change. Significant refactor of the hardware layer.

### Option B: Keep DsaBase hierarchy, extract and constrain DsaSink at the stdexec boundary (minimal)

Leave `DsaBase` / `DsaBatchBase` / etc. as they are internally. The change is purely
at the stdexec layer:

1. Actually **use** the `DsaSink` concept as a constraint on operation templates
2. Move `DsaSink` definition next to where it's used (or keep it standalone)
3. Remove `#include <dsa/dsa.hpp>` from `operation_base_mixin.hpp` — operations should
   only depend on the concept, not the concrete hardware types

```cpp
// operation_base_mixin.hpp - BEFORE:
#include <dsa/dsa.hpp>  // pulls in hardware layer, x86 intrinsics, accel-config

// operation_base_mixin.hpp - AFTER:
#include <dsa_stdexec/dsa_sink.hpp>  // just the concept
```

And in the `DsaOperation` concept:

```cpp
// BEFORE (unconstrained):
template <typename T>
concept DsaOperation = ... && requires(T &op) { op.dsa_; };

// AFTER (DsaSink-constrained):
template <typename T>
concept DsaOperation = ... && DsaSink<decltype(op.dsa_)>;
```

**Pros:** Minimal change. Establishes the right dependency direction
(stdexec → concept ← hardware). Operations become truly generic over any DsaSink.
Include graph becomes cleaner.

**Cons:** Doesn't actually decouple the *implementations*. `DsaBatchBase` still has
all the batching + queue logic fused together. But that's an implementation detail
hidden behind the concept.

### Option C: Hybrid — extract DescriptorSink, keep DsaBase as a facade (recommended)

Separate the descriptor submission strategy from `DsaBase` without completely
rearchitecting:

1. Define `DescriptorSink` concept
2. Create concrete sinks: `DirectDescriptorSink`, `StagingDescriptorSink`, etc.
3. `DsaBase` becomes `DsaEngine<DescriptorSink, QueueTemplate>` internally
4. Keep existing type aliases (`Dsa`, `DsaBatch`, etc.) working via alias changes
5. stdexec layer uses `DsaSink` concept constraint (same as Option B)

**Step 1:** Extract `DirectDescriptorSink` from `DsaBase`:

```cpp
class DirectDescriptorSink {
public:
    DirectDescriptorSink(void *portal, accfg_wq_mode mode)
        : portal_(portal), mode_(mode) {}

    void submit_descriptor(dsa_hw_desc *desc) {
        _mm_sfence();
        if (mode_ == ACCFG_WQ_DEDICATED)
            _movdir64b(portal_, desc);
        else
            while (_enqcmd(portal_, desc) != 0) _mm_pause();
    }

    void flush() {}  // no-op

private:
    void *portal_;
    accfg_wq_mode mode_;
};
```

**Step 2:** Extract `StagingDescriptorSink` from `DsaBatchBase`:

```cpp
class StagingDescriptorSink {
public:
    StagingDescriptorSink(DirectDescriptorSink &inner, size_t max_batch)
        : inner_(inner), max_batch_size_(max_batch) {}

    void submit_descriptor(dsa_hw_desc *desc) {
        memcpy(&staged_[active_buf_][staged_count_++], desc, sizeof(dsa_hw_desc));
        if (staged_count_ >= max_batch_size_) flush();
    }

    void flush() {
        if (staged_count_ == 0) return;
        // ... build batch descriptor, submit via inner_ ...
    }

private:
    DirectDescriptorSink &inner_;
    // ... staging buffers, counts ...
};
```

**Step 3:** Compose in `DsaEngine`:

```cpp
template <class DescSink, template<typename> class QueueTemplate>
class DsaEngine {
public:
    void submit(OperationBase *op, dsa_hw_desc *desc) {
        if (desc) desc_sink_.submit_descriptor(desc);
        queue_.push(op);
    }
    // ...
};

// Type aliases
using Dsa = DsaEngine<DirectDescriptorSink, MutexTaskQueue>;
using DsaBatch = DsaEngine<StagingDescriptorSink, MutexTaskQueue>;
```

**Pros:** Clean internal architecture. Each piece is independently testable.
New submission strategies are easy to add. The stdexec layer only sees `DsaSink`.

**Cons:** Largest refactor. Requires careful handling of initialization (device
discovery creates the portal, which the sink needs).

## Recommendation

**Start with Option B** (minimal decoupling at the concept boundary), then evolve
toward Option C if needed.

### Rationale

Option B delivers the key insight — *the stdexec layer should not depend on hardware
implementation details* — with minimal disruption:

1. **Actually use `DsaSink`** as a constraint, making it the formal contract
2. **Remove `#include <dsa/dsa.hpp>`** from `operation_base_mixin.hpp` — this is the
   single most impactful change, breaking the compile-time dependency on hardware types
3. **Remove `#include <dsa/dsa.hpp>`** from `scheduler.hpp` and `batch.hpp` if possible
4. **Verify** the concept is satisfied by all 4 DSA type families

Option C is a follow-up refactor of the *hardware layer itself* — extracting
`DescriptorSink` from `DsaBase` internals. It's a good direction but independent
of the stdexec/concept boundary work.

## Concrete Steps (Option B)

### Step 1: Update `dsa_sink.hpp`

The current definition is correct but needs `dsa_hw_desc` without pulling in
`<dsa/dsa.hpp>`. Currently it includes `<dsa_stdexec/operation_base.hpp>` which
transitively has `<linux/idxd.h>` (provides `dsa_hw_desc`). This is fine — the
concept only depends on the kernel header, not our hardware layer.

### Step 2: Update `operation_base_mixin.hpp`

Replace:
```cpp
#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
```

With:
```cpp
#include <dsa_stdexec/dsa_sink.hpp>
#include <dsa/dsa_operation_base.hpp>  // still needed for DsaOperationBase
```

The `#include <dsa/dsa.hpp>` was only needed because operations accessed `dsa_`
as a concrete `DsaBase<Q>`. With the concept constraint, they only need the interface.

**BUT** — `operation_base_mixin.hpp` also includes `<dsa_stdexec/batch.hpp>` for the
`dsa::fill_*` functions. `batch.hpp` includes `<dsa/dsa.hpp>`. So we need to:

### Step 3: Extract `fill_*` functions from `batch.hpp`

Move the `dsa::fill_*` functions from `batch.hpp` to a new header
`include/dsa_stdexec/descriptor_fill.hpp` (or `include/dsa/fill_descriptor.hpp`).
These functions only depend on `<linux/idxd.h>` — they don't need `DsaBase`,
`OperationBase`, or stdexec.

```
batch.hpp currently contains:
  1. dsa::fill_* functions (pure descriptor helpers, no deps)
  2. BatchOperation (stdexec sender, needs DsaBase)
  3. BatchSender (stdexec sender)
  4. dsa_batch() factory

After split:
  descriptor_fill.hpp → just the fill_* functions
  batch.hpp → imports descriptor_fill.hpp, keeps BatchOperation etc.
  operation_base_mixin.hpp → imports descriptor_fill.hpp instead of batch.hpp
```

### Step 4: Constrain `DsaOperation` concept with `DsaSink`

```cpp
template <typename T>
concept DsaOperation =
    std::derived_from<T, dsa::DsaOperationBase> &&
    DsaSink<std::remove_reference_t<decltype(std::declval<T&>().dsa_)>> &&
    requires(T &op, dsa_hw_desc &desc) {
      typename T::result_type;
      { T::op_name } -> std::convertible_to<std::string_view>;
      { op.fill_descriptor(desc) } -> std::same_as<void>;
      op.r_;
    };
```

Note: `op.dsa_` is replaced by the `DsaSink` constraint. The bare `op.dsa_`
requirement is removed since the concept now constrains its type.

### Step 5: Update `scheduler.hpp`

The scheduler uses `dsa_.submit(this)` (single-param). It includes `<dsa/dsa.hpp>`
for `DsaBase`. If we want to decouple, the scheduler should also use a concept
constraint instead of depending on concrete types. However, `ScheduleOperation`
doesn't go through `DsaOperationMixin` — it has its own `start()`/`notify()`.
This is acceptable; the scheduler is a simpler, independent path.

For now, leave `scheduler.hpp` as-is — it directly constructs a `ScheduleOperation`
that doesn't use the mixin. Decoupling it is a separate concern.

### Step 6: Verify include graph

After changes, the dependency should be:

```
operation_base_mixin.hpp
  └── dsa_stdexec/dsa_sink.hpp        (concept only)
  └── dsa/dsa_operation_base.hpp      (aligned storage)
  └── dsa_stdexec/descriptor_fill.hpp (fill_* functions)
  └── dsa_stdexec/operation_base.hpp  (OperationBase, OperationFacade)
  └── dsa_stdexec/error.hpp
  └── stdexec/execution.hpp
  // NO dependency on dsa/dsa.hpp!
```

### Step 7: Build + test

All examples and benchmarks must continue to work. The concrete DSA types are only
needed at the call site (examples, main, benchmark) — not inside the operation
sender headers.

## Impact

| Aspect | Before | After |
|--------|--------|-------|
| `operation_base_mixin.hpp` includes `<dsa/dsa.hpp>` | Yes | No |
| `DsaSink` concept used as constraint | No | Yes |
| `fill_*` functions location | `batch.hpp` | `descriptor_fill.hpp` |
| Operation senders depend on hardware impl | Yes (transitively) | No |
| Compile time for operation headers | Slower (pulls in x86 intrinsics, accel-config) | Faster |
| Can write a mock DsaSink for testing | Difficult (need full DsaBase) | Easy (satisfy concept) |

## Non-Goals

- **Not** rearchitecting `DsaBase` / `DsaBatchBase` internals (that's Option C, future work)
- **Not** changing the `DsaSink` interface (submit/poll is the right abstraction)
- **Not** removing `DsaProxy` / `DsaFacade` — they serve a different purpose (runtime polymorphism)
