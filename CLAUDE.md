# CLAUDE.md

Guidance for Claude Code working in this repository.

## Workflow Rules

- **Plans**: Write to `plan/YYYY-MM-DD/<topic>.md` before non-trivial changes.
- **Reports**: Write findings to `report/<descriptive_name>.md`.
- **Remarks**: Write concise, standalone insights to `remark/NNN_<topic>.md`. Each remark captures one interesting finding with data, explanation, and implication. Number sequentially. Reference source report.
- **Early Hypotheses**: Deliver a preliminary analysis within the first 30 seconds. State what you've found so far if you need more time.
- **Read before modify**: Read the module's co-located README before changing any module. They document patterns, conventions, and extension steps.
- **Check prior work**: Scan `report/*.md` before starting analysis — previous sessions may have covered the topic.
- **Design decisions first**: Check `report/design_decisions.md` before proposing architectural changes to understand existing rationale.
- **Spec over guessing**: For hardware behavior questions (opcodes, completion records, page faults), consult `dsa_architecture_spec.md` rather than guessing.
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

Shared root resources: `tools/` (dsa_launcher), `dsa_architecture_spec.md` (hardware spec), `dsa-config/` (accel-config), `report/`, `plan/`, `remark/`, `docs/`, `devenv.nix`.

## Code Rules

- Match CODE to SPEC, not spec to code, unless told otherwise.
- In multi-agent setups, minimize idle messages. Only send updates on meaningful progress or when blocked.
