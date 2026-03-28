# Move bindgen-backed `idxd` bindings into a dedicated bindings crate

## Goal

Make the bindings crate the single owner of the kernel `idxd` UAPI bindings and
rename it from `dsa-bindings` to `idxd-bindings`. `hw-eval` should consume that
crate instead of generating `idxd` bindings locally.

## Files expected to change

- `dsa-bindings/Cargo.toml` (before rename)
- `dsa-bindings/src/lib.rs` (before rename)
- `dsa-bindings/build.rs` (new, before rename)
- `hw-eval/Cargo.toml`
- `hw-eval/src/iax.rs`
- `hw-eval/README.md`
- `hw-eval/build.rs` (remove)
- `docs/report/...` for the implementation note

## Planned steps

1. Rename the bindings crate to `idxd-bindings`.
2. Move bindgen generation for `linux/idxd.h` into that crate.
3. Export the generated `idxd` IAX types/constants from the bindings crate.
4. Switch `hw-eval` to depend on the renamed crate and delete its local bindgen
   step.
5. Record the new ownership model in a report.

## Result

Implemented on 2026-03-23:

- The bindings package is now named `idxd-bindings`.
- `linux/idxd.h` bindgen generation lives in `dsa-bindings/build.rs`.
- `hw-eval` consumes `idxd_bindings::idxd` for IAX types/constants and no
  longer owns a local `build.rs`.
- Recorded in `docs/report/hw_eval_idxd_bindings_crate_2026-03-23.md`.

The filesystem directory remains `dsa-bindings/`, and this change has not been
validated by a build or hardware run in this session.
