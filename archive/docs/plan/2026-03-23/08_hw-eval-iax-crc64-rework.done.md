# Re-implement `hw-eval` IAX path around spec-aligned operations

## Goal

Replace the current `hw-eval` IAX `memmove` benchmark path with a
spec-aligned IAX implementation that only exercises operations defined by the
IAA architecture spec, while keeping `iax` as the code and CLI name.

## Why

- The local IAA spec does not list `memmove` as a supported IAX operation.
- The current `hw-eval` IAX path fails in hardware on the first `memmove`
  submission.
- A minimal spec-aligned IAX benchmark path is needed before any deeper IAX
  hardware evaluation is meaningful.

## Files to change

- `hw-eval/src/iax.rs`
- `hw-eval/src/main.rs`
- `hw-eval/README.md`
- `docs/report/...`

## Planned steps

1. Fix the IAX descriptor/completion helpers to match spec expectations where
   possible, including 64-byte completion alignment.
2. Replace the IAX `memmove` benchmark path with a `CRC64` benchmark path.
3. Keep `noop` as the pure submission/completion benchmark for IAX.
4. Update the README and record the implementation change in a report.

## Outcome

- The IAX backend now benchmarks spec-aligned `CRC64` instead of non-spec `memmove`.
- The new path keeps `noop` for pure submission/completion measurement.
- Targeted CRC64 unit tests passed in `devenv`.
- The hardware-backed smoke test passed on `/dev/iax/wq1.0`.

## Conclusion

The CRC64 rework is implemented and validated for the scope of this plan.
