# Move `idxd` bindgen ownership into `idxd-bindings`

## Summary

Moved the `linux/idxd.h` bindgen step out of `hw-eval` and into the sibling
bindings crate, then switched `hw-eval` IAX support to consume those generated
types from `idxd-bindings`.

## What changed

- Renamed the bindings package in `dsa-bindings/Cargo.toml` from
  `dsa-bindings` to `idxd-bindings`.
- Added `dsa-bindings/build.rs` to generate the IAX subset of
  `/usr/include/linux/idxd.h` with bindgen.
- Exported the generated kernel UAPI as `idxd_bindings::idxd` while keeping the
  existing handwritten DSA submission API in the same crate.
- Updated `hw-eval/Cargo.toml` to depend on `idxd-bindings` and removed its
  local bindgen/build-script setup.
- Updated `hw-eval/src/iax.rs` and `hw-eval/README.md` to reflect the new
  ownership model.

## Why

This makes one crate the single owner of the kernel `idxd` UAPI generation
logic. `hw-eval` now consumes generated IAX types instead of maintaining its
own build-time bindgen step.

## Important note

The filesystem path remains `dsa-bindings/`; this change only renames the
Cargo package/crate identity to `idxd-bindings` / `idxd_bindings`.

This work was not followed by a build or hardware rerun in this session, so the
change remains unvalidated until an explicit build/test step is requested.
