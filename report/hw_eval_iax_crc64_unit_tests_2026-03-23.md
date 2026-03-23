# `hw-eval` IAX CRC64 unit tests

## Summary

Added pure unit tests for the IAX CRC64 helper path in `hw-eval`.

## Coverage

- Verified the software T10-DIF CRC reference against a known vector.
- Verified packing of that CRC into the 64-bit IAX CRC field format used by the
  benchmark path.
- Verified extraction of the CRC field from a completion-record image.
- Verified the `fill_crc64()` descriptor builder populates the expected opcode,
  transfer size, source address, and CRC polynomial field.

## Why

The hardware smoke test validates acceptance and completion of the IAX
descriptor, but unit tests are still needed for deterministic correctness
checks that do not depend on an IAX work queue being present.
