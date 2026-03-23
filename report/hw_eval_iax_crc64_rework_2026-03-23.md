# `hw-eval` IAX rework: replace non-spec memmove with CRC64

## Summary

Re-implemented the `hw-eval` IAX backend to benchmark a spec-aligned IAX
operation set instead of the previous `memmove` path.

## What changed

- Kept the user-visible backend name as `iax`.
- Updated `hw-eval/src/iax.rs`:
  - completion records are now 64-byte aligned;
  - added a spec-aligned `CRC64` descriptor builder;
  - added helpers to read IAX error details from the completion record.
- Updated `hw-eval/src/main.rs`:
  - replaced IAX `memmove` latency/burst/sliding-window benchmarks with IAX
    `crc64` latency/burst/sliding-window benchmarks;
  - failure paths now include IAX completion `error_code` and
    `invalid_flags`.
- Updated `hw-eval/README.md` to describe the new IAX benchmark coverage.

## Why

The local IAA architecture specification lists these relevant IAX operations:
`No-op`, `Drain`, `Translation Fetch`, `Decrypt`, `Encrypt`, `Decompress`,
`Compress`, `CRC64`, `Scan`, `Extract`, `Select`, and `Expand`.

It does not list `memmove` as an IAX operation, so the previous hardware path
was not a defensible target for `hw-eval`.

## Notes

- The new IAX data-path benchmark uses `CRC64` with a fixed T10-DIF
  polynomial.
- This change has not been followed by a build or hardware run in this
  session.
