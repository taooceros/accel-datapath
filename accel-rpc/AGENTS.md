# accel-rpc AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Rust workspace for accelerator-aware RPC components around Tonic, plus small benchmarking and profiling crates.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Workspace membership | `Cargo.toml` | `tonic` is excluded even though it is present as a submodule. |
| Codec work | `accel-codec/` | Pooled-buffer codec crate. |
| Middleware work | `accel-middleware/` | Tower CRC/compression middleware. |
| Rust IDXD binding stack | `idxd-rust/` (safe) with repo-root `idxd-sys/` (raw) | Canonical DSA/IDXD bridge crates. |
| Async overhead bench | `async-bench/` | `cargo bench` is the real measurement path. |
| Profiling harness | `tonic-profile/` | Still marked TODO. |

## CONVENTIONS
- Treat this subtree as a workspace first: update the member crate and the workspace manifest together when boundaries change.
- Do not assume the vendored `tonic/` subtree is part of normal workspace edits; it is intentionally excluded.
- Keep commands Cargo-native unless a crate README says otherwise.

## ANTI-PATTERNS
- Do not present `tonic-profile` as production-ready; the current harness is a placeholder.
- Do not add crate-specific guidance here if the crate is still only a manifest plus stub source; keep the parent file thin until complexity grows.

## COMMANDS
```bash
cd accel-rpc && cargo build
cd accel-rpc && cargo check
cd accel-rpc && cargo bench -p async-bench
```
