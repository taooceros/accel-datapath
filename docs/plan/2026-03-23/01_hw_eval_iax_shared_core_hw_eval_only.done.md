# hw-eval IAX support with shared submit/poll core (hw-eval only)

## Goal

Implement Intel IAX support in `hw-eval` without modifying `dsa-bindings`, and share submission/polling mechanics between DSA and IAX.

## Scope

1. Implement local low-level Rust hardware bindings inside `hw-eval/src/dsa.rs`:
   - shared `WqPortal` open/submit path (`movdir64b` for dedicated, `enqcmd` for shared)
   - shared status polling primitive
   - DSA descriptor/completion types and helpers (compat with existing benchmark code)
   - IAX descriptor/completion types and helpers
2. Add accelerator selector in `hw-eval/src/main.rs`:
   - `--accel dsa|iax` (default `dsa`)
3. Keep full existing DSA benchmark suite unchanged in behavior.
4. Add IAX benchmark path in `hw-eval/src/main.rs`:
   - noop latency
   - memmove single-op latency
   - burst throughput (memmove)
   - sliding-window throughput (memmove)
5. Update `hw-eval/README.md` for accelerator selector and IAX coverage.
6. Write implementation report in `docs/report/`.

## Explicit non-goals

- No changes in `dsa-bindings`.
- No IAX deflate compress/decompress implementation in this pass (requires AECS/flag policy).
- No batch benchmarks for IAX (unsupported).

## Design choice

- Submission mechanism and raw completion polling are shared.
- Completion status interpretation and operation builders remain accelerator-specific.
