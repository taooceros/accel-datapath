# Layer-Removal Experiments Measure Abstraction Cost Directly

**Date**: 2026-02-22
**Source**: `report/progress_post_alignment_debug.md`, Section 4 "What the Measured Deltas Tell Us"

## Finding

Building three benchmark strategies that progressively bypass stdexec layers
gives us measured per-layer costs without needing per-phase instrumentation.

## Key data

| Transition | What's removed | Measured delta |
|---|---|---|
| `noalloc` -> `direct` | scope.nest() + then() | 14 ns/op |
| `direct` -> `reusable` | connect() + start() | 7 ns/op |
| Total stdexec overhead | All of the above | 21 ns/op |

## Why it matters

- Previous attempts to decompose per-op cost into phases (9 ns for connect,
  5 ns for poll, etc.) were analytical guesses that proved inaccurate --
  they over-predicted savings by 4x.
- Layer-removal gives reliable numbers because it measures *actual* end-to-end
  throughput, not estimated sub-component costs.
- The approach generalizes: for any layered abstraction, building
  progressively-stripped variants measures layer cost as a delta.

## Methodological lesson

Measure whole-system deltas, not sub-component estimates. The interaction
effects between phases (instruction cache, branch prediction state, register
pressure) are invisible to analytical decomposition but captured by
end-to-end measurement.
