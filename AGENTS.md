# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-28 UTC  
**Commit:** `8a18b76`  
**Branch:** `literature-review`

## OVERVIEW
Research monorepo for Intel DSA/IAX data-path work. Main code lives in `dsa-stdexec/` (C++/xmake), `accel-rpc/` (Rust/Cargo workspace), and `hw-eval/` (Rust/Cargo hardware floor benchmarks).

## STRUCTURE
```text
./
├── dsa-stdexec/        C++ stdexec sender/receiver framework for DSA
├── accel-rpc/          Rust workspace for accelerator-aware RPC components
├── hw-eval/            Raw DSA/IAX benchmark harnesses
├── dsa-bindings/       Rust `idxd-bindings` crate used by `hw-eval`
├── docs/               Plans, reports, specs, related-work notes
├── remark/             Standalone research insights linked from reports
├── tools/              `dsa_launcher` capability wrapper
├── dsa-config/         accel-config JSONs for work queues
├── tursodb/            local KB tooling and ingestion rules
└── codemogger/         repo-local code-search wrapper
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Repo workflow | `AGENTS.md` | This file is the root policy layer. |
| C++ DSA framework work | `dsa-stdexec/AGENTS.md` | Read nearest child under `dsa-stdexec/` for local invariants. |
| Rust RPC workspace work | `accel-rpc/AGENTS.md` | Workspace-level guidance only; crates are still small. |
| Raw hardware benchmarking | `hw-eval/AGENTS.md` | Hardware vs `--sw-only` split lives there. |
| Rust IDXD bindings crate | `dsa-bindings/Cargo.toml` | Path is `dsa-bindings/`, package name is `idxd-bindings`. |
| Plans / reports / specs / related work | `docs/AGENTS.md` | Placement rules differ inside `docs/`. |
| Standalone insights | `remark/` | One insight per file, linked back to source work. |
| Low-level DSA engine internals | `dsa-stdexec/src/dsa/AGENTS.md` | Queue, submitter, alignment, backpressure rules. |
| DSA benchmark framework | `dsa-stdexec/benchmark/dsa/AGENTS.md` | Config/dispatch/CSV discipline. |
| Adding a DSA operation sender | `dsa-stdexec/include/dsa_stdexec/operations/AGENTS.md` | Local checklist spans headers, examples, and build registration. |
| Tool launcher behavior | `tools/README.md` | `dsa_launcher` is the source of truth. |
| Work-queue config assets | `dsa-config/README.md` | Config meanings and apply flow. |

## CONVENTIONS
- Write a plan in `docs/plan/YYYY-MM-DD/NN.<topic>.<state>.md` before non-trivial changes.
- Write findings to `docs/report/NNN.<descriptive_name>.md`; write single-point insights to `remark/NNN_<topic>.md`.
- For plans, reports, and specs, start with the repo-local `tursodb-kb` skill; its underlying commands are `devenv shell -- search-kb`, `search-kb-fts`, and `search-kb-vector`. For code search, prefer `devenv shell -- codemogger search "query"`.
- Read the co-located README before modifying a module. `dsa-stdexec/` has the richest nested README map.
- Match code to specs, not specs to code, unless explicitly told otherwise.

## ANTI-PATTERNS (THIS PROJECT)
- Do not guess DSA/IAX behavior when `docs/specs/*.md` or `docs/report/design_decisions.md` already cover it.
- Do not treat raw PDFs as KB-ingested content; searchable paper content belongs in tracked markdown.
- Do not run hardware-facing binaries directly when the documented flow requires `launch` / `dsa_launcher`.
- Do not duplicate parent guidance in child `AGENTS.md` files; child files should contain only local deltas.

## UNIQUE STYLES
- The repo favors short, repo-grounded workflow notes over generic advice.
- `dsa-stdexec/` treats inline polling as the primary optimization path; threaded polling is comparative, not default.
- `docs/related_work/` is organized by the repo thesis, not by paper title or venue.

## COMMANDS
```bash
devenv shell
devenv shell -- search-kb "query text"
devenv shell -- codemogger search "query text"
launch <command> [args...]

# dsa-stdexec
cd dsa-stdexec && xmake
cd dsa-stdexec && run -- --help

# accel-rpc
cd accel-rpc && cargo build
cd accel-rpc && cargo check

# hw-eval
cd hw-eval && cargo build --release
cd hw-eval && cargo run --release -- --sw-only
```

## NOTES
- `README.md` at repo root is still `dsa-stdexec`-centric; use this file for the true repo map.
- `CLAUDE.md` should remain a thin pointer to this hierarchy rather than a second full copy.
- Not every directory gets its own `AGENTS.md`; only real workflow or invariant boundaries do.
