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
```

## Hardware Requirements

- Intel 4th Gen Xeon Scalable (Sapphire Rapids) or later
- DSA device configured with work queue enabled (via `accel-config`)
- `CAP_SYS_RAWIO` capability for user-space DSA access

## Git commit hygiene

This repo ships a tracked `commit-msg` hook in `.githooks/commit-msg`.

It blocks:
- truncated subjects ending in `...` or `…`
- subjects longer than 72 chars
- commit bodies that skip the `.gitmessage` sections

Enable it for your local checkout:

```bash
git config core.hooksPath .githooks
```

## Dependencies

All dependencies are managed via the Nix flake (`flake.nix` / `devenv.nix`):

- **stdexec** -- NVIDIA's P2300 reference implementation
- **libaccel-config** -- Intel accelerator configuration library
- **proxy** -- Microsoft's polymorphic proxy library
- **fmt**, **tomlplusplus**
