# Async memmove inline submission contract

Status: gate-facing companion report for M006/S01
Date: 2026-04-28

## Reader and post-read action

Reader: a future S02-S04 executor or reviewer checking whether the bytes-based async memmove API migration still follows the S01 submission policy.

After reading this report, the reader should be able to confirm that M006 permits first-version inline hardware submission and explicitly defers software aggregation, batching, and MOVDIR64 work.

## Inline v1 policy

The canonical contract remains the architecture report for `AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)` and `AsyncMemmoveResult { destination: BytesMut, report }`.

For the first implementation version, inline `enqcmd` hardware submission is allowed. The implementation does not need a dedicated software aggregation thread, hidden batching layer, or MOVDIR64 alternate submission path before the bytes-based API migration can proceed.

## Deferred work

Software aggregation, batching, and MOVDIR64 support are future optimization or alternate-submission topics. They must not become prerequisites for M006, and any later implementation must preserve per-request failure provenance rather than hiding validation, lifecycle, inline submission, worker/channel compatibility, or hardware failures behind a batch-level error.

## Mechanical verification

Run the contract verifier from the repository root before implementing S02, migrating downstream callers, or packaging final M006 evidence:

```sh
idxd-rust/scripts/verify_async_memmove_contract.sh
```

The verifier checks the canonical bytes-based API contract in the architecture report and fails with a named diagnostic when a required clause is missing or stale public API wording is reintroduced as an endorsed surface.
