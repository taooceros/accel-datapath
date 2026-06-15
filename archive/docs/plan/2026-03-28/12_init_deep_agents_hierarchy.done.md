# Initialize deep AGENTS hierarchy

## Goal

Replace the current single-file root guidance with a hierarchical `AGENTS.md` layout that stays repo-grounded, avoids parent/child duplication, and scopes local instructions to real responsibility boundaries.

## Mode

- update existing files where they already exist
- create new child `AGENTS.md` files only where local complexity or workflow divergence justifies them
- limit hierarchy decisions to directories at depth 3 or shallower

## Discovery summary

- root guidance currently lives in both `AGENTS.md` and `CLAUDE.md`
- repo contains three primary code subprojects: `dsa-stdexec/`, `accel-rpc/`, and `hw-eval/`
- `dsa-stdexec/` has rich nested module READMEs and the strongest case for deeper child instructions
- `docs/` and `tursodb/` have distinct knowledge-base and research-document workflows
- support directories like `tools/`, `dsa-config/`, `codemogger/`, and `dsa-bindings/` appear important but may be better referenced from parent files unless scoring says otherwise

## Planned scoring directions

Create `AGENTS.md` at:

- `.` (mandatory root)
- likely `dsa-stdexec/`, `accel-rpc/`, `hw-eval/`, `docs/`, and `tursodb/`
- likely selected deeper `dsa-stdexec/` subtrees where the local README documents a distinct extension pattern or decision taxonomy

Skip directories that are primarily:

- artifact storage
- hidden tool state
- thin wrappers already adequately covered by parent guidance

## Generation constraints

- child files inherit root guidance instead of repeating it
- commands must come from repo files, not assumptions
- local anti-patterns should be explicit where documented, such as avoiding `alignas()` in coroutine-related DSA storage and preserving unique benchmark output files
- root file should mention KB-first retrieval, codemogger-first code search, and launcher/spec workflows

## Validation plan

- review each generated file for duplication with its parent
- keep root concise but useful; keep child files shorter and delta-focused
- sync the local KB for tracked markdown changes after edits

## Result

- rewrote root `AGENTS.md` as the canonical repo map and converted `CLAUDE.md` into a thin pointer
- added child `AGENTS.md` files at `dsa-stdexec/`, `dsa-stdexec/src/dsa/`, `dsa-stdexec/benchmark/dsa/`, `dsa-stdexec/include/dsa_stdexec/operations/`, `accel-rpc/`, `hw-eval/`, and `docs/`
- kept support directories like `tools/`, `dsa-config/`, `codemogger/`, and `tursodb/` README-driven rather than adding redundant child instruction files
- updated stale README backlinks that still pointed to the old rich `CLAUDE.md`
- synced the new plan file into the local KB with `devenv shell -- sync-kb`
