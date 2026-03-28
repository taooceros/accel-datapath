# src/dsa AGENTS

Inherits `../../AGENTS.md`.

## OVERVIEW
Low-level DSA hardware interface: queues, submitters, descriptor/completion storage, polling, and work-queue backpressure.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Engine behavior | `dsa.hpp`, `dsa.ipp` | `DsaEngine` device discovery, submit, poll. |
| Submitter variants | `descriptor_submitter.hpp` | Direct and batch submitters. |
| Queue concepts | `task_queue.hpp` | Queue interface and implementations. |
| Descriptor/completion storage | `dsa_operation_base.hpp` | Runtime over-alignment rules. |
| Batch ring internals | `mirrored_ring.hpp` | Double-mapped ring semantics. |
| Explicit instantiations | `dsa_*_instantiate.cpp` | Keep template build costs bounded. |

## CONVENTIONS
- Keep hot-path logic monomorphic: concepts and static dispatch are deliberate.
- Maintain explicit instantiation files when adding new queue or submitter combinations.
- Preserve WQ backpressure behavior for dedicated queues.
- When in doubt, cross-check `docs/report/design_decisions.md` and the local README before changing core semantics.

## ANTI-PATTERNS
- Do not use `alignas()` for coroutine-related descriptor/completion storage; use the runtime over-allocation pattern.
- Do not replace concept-based interfaces with virtual inheritance in hot paths.
- Do not remove or bypass inflight / WQ-depth tracking for dedicated queues.
- Do not change mirrored-ring behavior without understanding the double-mapping contract.
