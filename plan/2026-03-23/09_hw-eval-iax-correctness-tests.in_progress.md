# Add unit tests for `hw-eval` IAX CRC64 correctness helpers

## Goal

Add unit tests for the new spec-aligned `hw-eval` IAX CRC64 path so the code
has a hardware-independent correctness check in addition to the smoke test.

## Why

- The current hardware smoke test proves that the IAX descriptor is accepted.
- It does not by itself give a pure unit-test check for CRC result handling.
- The new IAX path uses a fixed T10-DIF CRC configuration, which is small
  enough to validate with deterministic software-side tests.

## Files to change

- `hw-eval/src/iax.rs`
- `report/...`

## Planned steps

1. Add software reference helpers for the fixed T10-DIF CRC configuration used
   by the benchmark path.
2. Add unit tests for:
   - the reference CRC value on a known input;
   - the completion-record CRC field extraction helper;
   - the IAX CRC64 descriptor builder.
3. Run targeted unit tests inside `devenv`.
