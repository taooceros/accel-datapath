# Key Design Decisions

Architectural choices that shape this codebase. See the module READMEs for
implementation-level details.

## Type Erasure via Proxy

`pro::proxy<OperationFacade>` instead of virtual dispatch. This enables
heterogeneous operation types in intrusive linked lists without vtable
overhead. The hot poll path stays free of indirect calls while still
allowing mixed operation types in the same queue.

## Page Fault Handling

`DSA_COMP_PAGE_FAULT_NOBOF` triggers automatic page touch + re-submit with
adjusted byte offsets. Operations transparently retry after touching the
faulting page, so callers don't need to handle page faults explicitly.

## Static Dispatch for Completion

`HwContext::check_completion()` avoids virtual calls in the hot poll loop.
Queue types satisfy the `TaskQueue` concept rather than inheriting from a
virtual base, enabling compile-time dispatch.

## Runtime Over-Alignment

`DsaOperationBase` computes 64-byte descriptor alignment and 32-byte
completion record alignment at runtime via over-allocation, instead of using
`alignas()`. This is required for coroutine frame compatibility -- `alignas`
on coroutine frame members is not reliably honored by compilers.

## WQ Backpressure for Dedicated Queues

`DsaEngine::submit()` spins on `poll()` when inflight descriptors reach WQ
depth. Dedicated work queues accept `_movdir64b` unconditionally; without
backpressure, submissions silently drop when the WQ is full.

## Concept-Based Extensibility

Both `TaskQueue` and `DescriptorSubmitter` are concept-based (not inheritance-
based). This keeps the hot path monomorphic while allowing new implementations
to be added by satisfying the concept constraints.

## PollingRunLoop as Primary Execution Model

The calling thread drives both submission and completion in a tight loop with
no cross-thread coordination. This eliminates lock contention and context-switch
overhead, critical for maximizing message rate on small transfers.
