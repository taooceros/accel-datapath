# CLAUDE.md

Guidance for working in this repository.

## Workflow Rules

- **Plans**: Write to `docs/plan/YYYY-MM-DD/NN.<topic>.<state>.md` before non-trivial changes.
- **Reports**: Write findings to `docs/report/NNN.<descriptive_name>.md`.
- **Remarks**: Write concise, standalone insights to `remark/NNN_<topic>.md`. Each remark captures one interesting finding with data, explanation, and implication. Number sequentially. Reference source report.
- **Early Hypotheses**: Deliver a preliminary analysis within the first 30 seconds. State what you've found so far if you need more time.
- **Code search first via codemogger**: When searching source code definitions, implementations, symbols, or behavior across the repo, use `devenv shell -- codemogger search "query"` before broad manual scans.
- **Local KB first**: Use the repo-local Turso knowledge base before broad manual scans when looking for prior plans, reports, and hardware specs.
- **Keep KB fresh**: After adding, moving, deleting, or editing tracked KB sources, run `devenv shell -- sync-kb` or `devenv shell -- sync-kb <path>` before relying on retrieval results.
- **Use the right retrieval mode**: Use `devenv shell -- search-kb "query"` for hybrid retrieval, `devenv shell -- search-kb-fts "query"` for exact keyword/spec lookup, and `devenv shell -- search-kb-vector "query"` for semantic recall when wording may differ.
- **Read before modify**: Read the module's co-located README before changing any module. They document patterns, conventions, and extension steps.
- **Check prior work**: Scan `docs/report/*.md` before starting analysis — previous sessions may have covered the topic.
- **Design decisions first**: Check `docs/report/design_decisions.md` before proposing architectural changes to understand existing rationale.
- **Spec over guessing**: For hardware behavior questions (opcodes, completion records, page faults), consult `docs/specs/dsa_architecture_spec.md` or `docs/specs/iax(iaa)_architecture_spec.md` rather than guessing.
- **Strategy taxonomy**: Check `dsa-stdexec/benchmark/dsa/strategies/README.md` before adding or modifying benchmark strategies.

## Project Overview

Research project for Intel hardware accelerator (DSA + IAX) integration into data path systems:

1. **dsa-stdexec/** — C++ stdexec sender/receiver framework for DSA. Maximize message rate (ops/sec) for small transfers using inline polling. xmake build; TOML-based benchmark config.
2. **accel-rpc/** — Accelerator-driven gRPC using Tonic (Rust). Offload gRPC data path (memcpy, CRC, compression) to DSA/IAX via the Rust async framework.
3. **hw-eval/** — Raw hardware performance evaluation (Rust). Zero-framework-overhead DSA/IAX benchmarks. Calls hardware directly via inline asm (MOVDIR64B/ENQCMD). Establishes true hardware floor.

## Subprojects

Each has its own README with build instructions, structure, and dependencies:

| Subfolder | Description |
|-----------|-------------|
| [`dsa-stdexec/`](dsa-stdexec/README.md) | C++ stdexec sender/receiver framework for DSA (xmake) |
| [`accel-rpc/`](accel-rpc/README.md) | Accelerator-driven gRPC using Tonic (Rust/Cargo) |
| [`hw-eval/`](hw-eval/README.md) | Raw hardware performance evaluation (Rust/Cargo) |

Shared root resources: `tools/` (dsa_launcher), `docs/specs/dsa_architecture_spec.md` (dsa hardware spec), `docs/specs/iax(iaa)_architecture_spec.md` (iax/iaa hardware spec), `dsa-config/` (accel-config), `docs/report/`, `docs/plan/`, `remark/`, `docs/`, `tursodb/`, `devenv.nix`.

## Code Rules

- Match CODE to SPEC, not spec to code, unless told otherwise.
- In multi-agent setups, minimize idle messages. Only send updates on meaningful progress or when blocked.

## Execution

Run everything in a devenv shell;
Run hardware related code with launcher;
Use `devenv shell -- codemogger search "query"` first when searching code;
Use the local KB helpers through `devenv shell -- sync-kb`, `search-kb`, `search-kb-fts`, and `search-kb-vector`;

## Remark

Don't check for user for every steps; Only check for user if you have thought too long or you are blocked or completed.
