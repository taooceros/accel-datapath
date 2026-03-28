# Move hardware specs and scaffold the local KB rebuild

## Goal

Move the hardware specifications into a dedicated `docs/specs/` folder and add a reproducible local Turso rebuild path for `docs/plan/`, `docs/report/*.md`, and `docs/specs/*.md`.

## Plan

1. Move the root-level hardware spec markdown into `docs/specs/` and update the repo docs that reference them.
2. Add a minimal local KB schema for tracked markdown sources.
3. Add a rebuild script and shell entrypoint that recreates `.turso/knowledge.db` from the tracked text sources.
