# Rework `hw-eval` IAX support to use bindgen-backed `idxd` UAPI

## Goal

Replace the handwritten IAX descriptor/completion definitions in `hw-eval`
with bindgen-generated bindings from the local kernel `linux/idxd.h`, then
rebuild the IAX helpers around those generated types so `hw-eval` uses the
same ABI the `idxd` driver exposes.

## Why

- The repository currently hand-defines IAX descriptor and completion layouts.
- The local kernel already exposes `enum iax_opcode`, `struct iax_hw_desc`,
  and `struct iax_completion_record` in `/usr/include/linux/idxd.h`.
- The user requested that `idxd` driver definitions be consumed via bindgen.

## Planned steps

1. Add a `build.rs` in `hw-eval` that runs bindgen against
   `/usr/include/linux/idxd.h` and emits only the `idxd` items needed by
   `hw-eval`.
2. Update `hw-eval/Cargo.toml` with the required build dependencies.
3. Replace `hw-eval/src/iax.rs` handwritten layout/constants with helpers that
   wrap the generated bindgen types and enums.
4. Keep the existing benchmark flow in `hw-eval/src/main.rs`, but make it call
   the bindgen-backed IAX helpers instead of handwritten structs/constants.
5. Update `hw-eval/README.md` to note that IAX uses bindgen-backed `idxd`
   driver definitions.
6. Record the implementation change and test status in a report.

## Expected outcome

`hw-eval` stops duplicating the `idxd` IAX ABI by hand and instead derives it
from the kernel header that the active system provides.
