# Update workflow to prefer codemogger for code search

## Goal

Make the agent workflow prefer `codemogger` when searching code, while keeping the local Turso KB as the first retrieval path for plans, reports, and specs.

## Scope

- update agent workflow guidance
- document the intended split between code search and document search

## Result

- updated `AGENTS.md` to prefer `codemogger` for code search
- updated `CLAUDE.md` to keep the same guidance aligned
- kept the local Turso KB as the first retrieval path for plans, reports, and specs
