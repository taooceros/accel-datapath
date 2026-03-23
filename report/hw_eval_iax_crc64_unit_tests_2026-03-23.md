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

## Command run

```bash
devenv shell -- bash -lc 'cd hw-eval && cargo test crc64 -- --nocapture'
```

## Result

```text
running 3 tests
test iax::tests::fill_crc64_populates_expected_descriptor_fields ... ok
test iax::tests::completion_crc64_reads_64_bit_field_at_offset_32 ... ok
test iax::tests::crc64_t10dif_field_packs_crc_in_msb_bits ... ok

test result: ok. 3 passed; 0 failed
```

## Why

The hardware smoke test validates acceptance and completion of the IAX
descriptor, but unit tests are still needed for deterministic correctness
checks that do not depend on an IAX work queue being present.
