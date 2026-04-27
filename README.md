# dsa-stdexec

C++ sender/receiver (stdexec) bindings for Intel Data Streaming Accelerator (DSA), focused on maximizing message rate for small transfers using inline polling.

See [AGENTS.md](AGENTS.md) for the repo-wide map, workflow, and subdirectory guidance.

## Quick Start

```bash
# Enter development shell (requires Nix with flakes)
devenv shell

# Build all targets
xmake

# Run benchmarks (handles CAP_SYS_RAWIO via dsa_launcher)
run
```

## Project Structure

```
src/dsa/                     Low-level DSA hardware interface
include/dsa_stdexec/         stdexec sender/receiver integration
benchmark/dsa/               Multi-dimensional benchmark suite
examples/                    Per-operation examples (data_move, crc_gen, etc.)
tools/                       dsa_launcher capability helper
docs/specs/                  Local DSA / IAX hardware specification copies
test/                        Unit and integration tests
dsa-config/                  accel-config device configurations
idxd-sys/          Canonical low-level Rust IDXD/UAPI/MMIO binding crate
idxd-rust/         Canonical safe Rust and Tokio-facing IDXD binding crate
```

## Canonical Rust IDXD binding stack

M003/S04 consolidates the active Rust IDXD package surface to two crates:

- `idxd-sys` owns raw C/UAPI/MMIO integration.
- `idxd-rust` owns the safe Rust memmove API, async owner/handle API, proof binaries, and verifier scripts.

Run package checks from the `accel-rpc` workspace root when validating this stack:

```bash
cd accel-rpc
cargo metadata --no-deps >/tmp/m003-s04-cargo-metadata.json
cargo test -p idxd-rust -- --nocapture
bash idxd-rust/scripts/verify_package_inventory.sh
```

The historical `dsa-ffi` wrapper paths are compatibility shims only. New integration and S05 downstream proof work should consume `idxd-rust`, not `dsa-ffi` or `idxd-bindings`.

## Hardware Requirements

- Intel 4th Gen Xeon Scalable (Sapphire Rapids) or later
- DSA device configured with work queue enabled (via `accel-config`)
- `CAP_SYS_RAWIO` capability for user-space DSA access

## Dependencies

All dependencies are managed via the Nix flake (`flake.nix` / `devenv.nix`):

- **stdexec** -- NVIDIA's P2300 reference implementation
- **libaccel-config** -- Intel accelerator configuration library
- **proxy** -- Microsoft's polymorphic proxy library
- **fmt**, **tomlplusplus**
