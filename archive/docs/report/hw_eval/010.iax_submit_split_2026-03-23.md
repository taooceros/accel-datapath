# hw-eval: IAX support with shared submit core

## Summary

Implemented IAX support in `hw-eval` with the requested module split:

- `src/submit.rs`: shared WQ submission (`movdir64b`/`enqcmd`), generic poll primitive, timing, NUMA, cache helpers.
- `src/dsa.rs`: DSA-specific descriptors/completions/opcodes and completion handling.
- `src/iax.rs`: IAX-specific descriptors/completions/opcodes and completion handling.
- `src/sw.rs`: software memcpy/CRC baselines.

## Runtime changes

- Added CLI selector: `--accel dsa|iax` (default `dsa`).
- `dsa` mode keeps the prior benchmark suite.
- `iax` mode runs:
  - noop latency
  - memmove single-op latency
  - burst memmove throughput
  - sliding-window memmove throughput

## Notes

- `submit.rs` intentionally contains no accelerator opcode definitions.
- Op/status definitions remain in hardware-specific modules only.
- IAX compress/decompress is not included in this pass (AECS and flag policy not wired yet).
