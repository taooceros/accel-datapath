# Thread State: Tonic characterization execution and next measurement pass

```yaml
thread_id: thr-20260414-tonic-characterization
title: Tonic characterization execution and next measurement pass
status: active
owner_agent: legacy-active-owner
owner_session_id: legacy-current-md-unrecorded-active
previous_owner_session_id: null
lease_acquired_at: 2026-04-14T12:00:00Z
lease_expires_at: 2026-04-14T16:00:00Z
last_updated: 2026-04-14T12:00:00Z
handoff_to: null
handoff_reason: null
resume_allowed: true
match_hints:
  - tonic characterization
  - unary RPC regime map
  - matched comparison
  - split endpoint-local timers
  - advisor-ready taxonomy
  - offload-readiness ranking
superseded_by: null
source_of_truth_scope: .agents/state/threads/ canonical mutable thread state for this thread
index_label: Tonic characterization execution and next measurement pass
summary: Tighten bounded-matrix attribution into a regime-based unary Tonic characterization pass, reduce instrumentation distortion, connect higher-level buckets to lower-level CPU evidence, and turn the result into an offload-readiness ranking.
next_actions:
  - Execute the high-priority pre-advisor characterization priorities plan so the next meeting is framed around taxonomy, workload dimensions, evidence thresholds, and the controller-model question.
  - Tighten matched-comparison claims across size, concurrency, and runtime regimes instead of relying on isolated points.
  - Reduce timer overhead and split client and server snapshots before using timer data as evidence in larger or higher-concurrency regimes.
  - Implement remaining software variants and async microbenchmark expansion where they still sharpen the regime map.
  - Defer streaming expansion until unary refinement is stable.
blocked_by:
  - Current timer instrumentation is diagnostic-only outside the tiny single-thread point, so stronger claims need lower-overhead endpoint-local timers.
related_artifacts:
  - docs/plan/2026-04-13/05.pre_advisor_tonic_characterization_priorities.in_progress.md
  - docs/report/benchmarking/012.tonic_characterization_refinement_results.md
  - docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md
  - results/tonic/2026-04-12-characterization/
  - results/tonic/2026-04-01-loop2/
  - results/tonic/2026-04-08-frameptr/
  - results/tonic/2026-04-08-frameptr-debuginfo/
  - accel-rpc/tonic-profile/src/main.rs
  - accel-rpc/async-bench/benches/async_overhead.rs
```

## Detailed state

- The execution checklist now stays centered on the surviving pre-advisor characterization priorities plan, with current readouts in reports `012` and `013`.
- The first Phase A characterization subset is complete in `results/tonic/2026-04-12-characterization/`, with the current readout in `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`.
- Current measurements already separate fixed tiny-RPC codec work, 4 KiB buffer-policy sensitivity, large-message body and encode or decode movement, and compression transform cost.
- FleetBench RPC intake is complete in `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`.
- The thread now uses a two-level characterization model: FleetBench-style CPU and code-path characterization for realistic gRPC and protobuf behavior, plus Tonic stage decomposition for higher-level attribution across encode and decode, copy and buffer lifecycle, compression, framing, runtime, and tails.
- Earlier characterization planning passes are preserved as historical cancelled plans and are no longer part of this live thread surface.

## Key findings carried over from the old ledger

- Runtime crossover is workload-dependent. Tiny RPCs prefer single-thread runs, medium points often prefer multi-thread runs, and large payloads fall back to movement-dominated behavior.
- Medium and large uncompressed runs are dominated by `memmove`, allocator paths, and `BytesMut` or `RawVec` growth instead of protobuf or scheduler work.
- Compression is highly regime-sensitive. Incompressible payloads get worse, while structured payloads trade throughput for latency gains.
- The strongest next software lane is still buffer lifecycle and copy behavior.
- Report `012` shows the current timer instrumentation is useful for diagnosis, but too expensive for larger or higher-concurrency attribution claims.
