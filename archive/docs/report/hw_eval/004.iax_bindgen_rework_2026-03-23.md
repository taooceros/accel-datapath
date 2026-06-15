# `hw-eval` IAX rework to bindgen-backed `idxd` UAPI

## Summary

Reworked `hw-eval` IAX descriptor/completion handling to consume the local
kernel `linux/idxd.h` via bindgen instead of maintaining handwritten Rust
copies of the IAX ABI.

## What changed

- Added `hw-eval/build.rs` to generate IAX bindings from
  `/usr/include/linux/idxd.h` at build time.
- Added a `bindgen` build dependency in `hw-eval/Cargo.toml`.
- Replaced the handwritten `hw-eval/src/iax.rs` layout/constants with wrapper
  types over bindgen-generated `iax_hw_desc` and `iax_completion_record`.
- Kept the existing `IaxHwDesc`, `IaxCompletionRecord`, `poll_completion()`,
  `reset_completion()`, `touch_fault_page()`, and `submit_iax()` API so
  `hw-eval/src/main.rs` does not need to change.
- Stopped writing `max_dst_size` in the IAX memmove builder, leaving the rest
  of the operation-specific tail zeroed for the memmove path.
- Updated `hw-eval/README.md` to document the bindgen-based source of truth and
  the `libclang`/`IDXD_HEADER` build inputs.

## Why

The active system already exposes the IAX ABI in `/usr/include/linux/idxd.h`,
including:

- `enum iax_opcode`
- `enum iax_completion_status`
- `struct iax_hw_desc`
- `struct iax_completion_record`

Using bindgen removes duplicated ABI definitions from `hw-eval` and aligns the
benchmark with the kernel driver's own UAPI.

## Important note

This change was not followed by a rebuild or hardware rerun in this session.
Per current workflow constraints, the rework is recorded here but remains
untested until an explicit build/test step is requested.
