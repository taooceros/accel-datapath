# hw-eval AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Raw DSA/IAX benchmark crate. Measures hardware submission/completion costs with minimal framework overhead.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI and benchmark matrix | `README.md`, `src/main.rs` | Entry point for modes and output. |
| Shared submission helpers | `src/submit.rs` | Portal, polling, timing, topology. |
| DSA path | `src/dsa.rs` | DSA descriptors and helpers. |
| IAX path | `src/iax.rs` | IAX descriptors, completions, CRC64 flow. |
| Software baselines | `src/sw.rs` | Non-hardware fallback path. |
| Criterion bench | `benches/dsa_raw.rs` | Software-only bench target. |
| Bindings dependency | `../dsa-bindings/Cargo.toml` | Path is `dsa-bindings/`; crate name is `idxd-bindings`. |

## CONVENTIONS
- Use `launch` for hardware-facing runs; use `--sw-only` when hardware is not required.
- Keep DSA and IAX benchmark matrices distinct; the two paths are intentionally different.
- Preserve JSON output and graphing compatibility when changing benchmark result shapes.

## ANTI-PATTERNS
- Do not run the hardware binary directly when the documented flow requires `launch`.

## COMMANDS
```bash
cd hw-eval && cargo build --release
cd hw-eval && cargo run --release -- --sw-only
cd hw-eval && cargo bench
launch ./hw-eval/target/release/hw-eval --json --iterations 3000
```
