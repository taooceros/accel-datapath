# agents AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW

This subtree is for agent-facing operating material: workflow reports, task-shaping notes, prompt/template rationale, and other durable guidance whose primary reader is a coding agent.

## PLACEMENT

- Use `plan/YYYY-MM-DD/NN.<topic>.<state>.md` for agent execution plans before non-trivial changes to agent operation, delegation, tooling, prompt templates, or live-thread workflow.
- Use `report/workflow/` for durable reports about agent orchestration, retrieval strategy, context budgeting, live-thread state, and process-level agent behavior.
- Keep domain research, architecture, benchmarking, hardware validation, literature synthesis, and specs in `../docs/`.
- Keep active executable agent assets in their established runtime locations unless a migration explicitly updates those consumers.

## CONVENTIONS

- Plans in `plan/` follow the same date/topic/state shape as `docs/plan/`, but their subject is agent execution rather than domain project work.
- Reports in `report/workflow/` use topic-local numeric prefixes such as `001.<descriptor>.md`.
- Prefer links to domain docs over copying project facts into this subtree.
- Do not use this subtree as an agent scratchpad; keep transient notes out of git or turn them into a plan/report with a clear reader and purpose.
