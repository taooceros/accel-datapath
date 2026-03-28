# dsa-stdexec

C++ sender/receiver (stdexec) bindings for Intel Data Streaming Accelerator (DSA), focused on maximizing message rate for small transfers using inline polling.

See [CLAUDE.md](CLAUDE.md) for full architecture details, build options, and benchmark documentation.

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
docs/specs/                       Local DSA / IAX hardware specification copies
test/                        Unit and integration tests
dsa-config/                  accel-config device configurations
```

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
