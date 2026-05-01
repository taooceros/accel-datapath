# idxd-sys raw-boundary worker report

## Summary

Implemented the `idxd-sys` raw-boundary slice. `idxd-sys` now exposes only the generated bindgen UAPI module/alias and raw `WqPortal` MMIO submission primitives. Non-raw helper modules were removed from the compiled source tree and deleted.

## Changes

- `idxd-sys/src/lib.rs`
  - Reduced public surface to `idxd_uapi`, `idxd`, and `WqPortal`.
  - Removed helper module declarations/re-exports for descriptors, completions, CRC, timing, topology, cache flushing, typed submission results, and lifecycle helpers.
- `idxd-sys/src/portal.rs`
  - Kept only raw portal mapping and unsafe raw `submit_movdir64b_desc64` / `submit_enqcmd_desc64` methods.
  - Removed queue-mode detection, typed DSA/IAX descriptor submission helpers, and typed backpressure enum.
- Deleted non-raw helper modules:
  - `cache.rs`, `completion.rs`, `descriptor.rs`, `iax.rs`, `timing.rs`, `topology.rs`.
- Tests now assert only:
  - generated DSA/IAX bindgen layout and generated values remain available through `idxd_uapi`;
  - raw `WqPortal` open/error behavior and raw submission API shape.

## Validation

Passed:

```bash
cargo fmt --package idxd-sys -- --check
cargo test -p idxd-sys --lib --tests
```

Result: `cargo test` reported 31 passed across 4 suites.

## Notes

The requested root `context.md` and `plan.md` files were absent, so implementation used the delegated task text and live tree state as source of truth.
