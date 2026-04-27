# accel-rpc

Accelerator-driven gRPC using Tonic (Rust). Offloads gRPC data path (memcpy, CRC, compression) to DSA/IAX via the Rust async framework.

## Build

```bash
cd accel-rpc
cargo build                                     # Build all crates
cargo check                                     # Type-check workspace
```

## Structure

```
tonic/                            Submodule: taooceros/tonic fork
accel-codec/                      Custom Tonic Codec with pooled buffers
accel-middleware/                  Tower CRC/compression middleware
idxd-rust/                        Canonical safe Rust/Tokio IDXD binding crate
async-bench/                      Async framework overhead characterization
tonic-profile/                    Tonic profiling harness
Cargo.toml                        Workspace root
```

## Dependencies

Managed via Cargo: tonic, tokio, prost, bytes, tower, libc, clap, criterion.
