# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## DEFAULT MODE: SPEED FIRST
- Prefer response time over exhaustive verification unless the user asks for high confidence.
- Thinking is allowed and encouraged, but periodically report back with current best guess and confidence level.
- Answer from local context first; escalate only when the risk or uncertainty is meaningful.
- If external lookup may help but is not blocking, answer first and enrich later.

## SOURCE ORDER
1. Current conversation
3. Repo docs in `docs/` and `remark/`
4. Local indexes/tools: `codemogger`, Turso KB, `read`, `grep`, `glob`, `lsp_*`
5. External docs and web search

## EXPLORER WORKFLOW
- Use `@explorer` for bounded candidate discovery by default, not final synthesis.
- Do not combine repo-wide discovery, broad reading, and final recommendation in one explorer task.
- Every explorer task must specify: objective, scope boundary, allowed sources, max candidates, and stop condition.
- For resumed explorer work, the orchestrator must pass a session-scoped explorer memory summary with task id, objective, scope, rounds used, candidates found, rejected leads, unknowns, files already read, stop conditions seen, and compaction count.
- Explorer memory is execution state, not repo knowledge; keep it ephemeral and session-scoped.
- Explorer should checkpoint findings into session memory aggressively after each meaningful search step, especially new candidates, rejected leads, file reads, round boundaries, and before broadening scope.
- Default limits: max 2 search rounds, max 2 domains, max 8 candidates, max 1-2 reads per candidate.
- Stop early and hand back when: 3 plausible candidates are found; the candidate set is not converging; the task splits into multiple subquestions; deeper cross-domain synthesis is required; or prior compaction / resumed-with-missing-context is detected.
- If explorer resumes after prior compaction, or if required prior state is missing relative to explorer memory, it must stop immediately, return current findings, and suggest parallel sub-searches instead of broadening discovery to rebuild context.
- Explorer returns shortlists, rejected leads, unknowns, status, and next step; the orchestrator owns pruning, splitting, merging, and final synthesis.

## AGENT TEMPLATES
- Agent-specific task templates live under `.agents/templates/`.
- Naming convention: `.agents/templates/<agent_name>_*.md`.
- When delegating to a specialist, cite the exact matching template file when one exists.
- Do not reference `.agents/templates/` generically; reference a specific template path.
- Restate critical task budgets and stop conditions inline even when a template is provided.

## CONVENTIONS
- Write a plan in `docs/plan/YYYY-MM-DD/NN.<topic>.<state>.md` before non-trivial changes.
- Write findings to `docs/report/<topic>/NNN.<descriptor>.<ext>`; write single-point insights to `remark/NNN_<topic>.md`.
- Read the nearest README before modifying a module.
- Match code to specs, not specs to code, unless explicitly told otherwise.
- Keep child `AGENTS.md` files lean and local; do not repeat parent guidance.

## DO NOT
- Guess DSA/IAX behavior if `docs/specs/*.md` or `docs/report/architecture/001.design_decisions.md` already cover it.
- Treat raw PDFs as KB-ingested content; searchable paper content belongs in tracked markdown.
- Run hardware-facing binaries directly when the documented flow requires `launch` / `dsa_launcher`.
- Do not combine repo-wide discovery, broad file reading, and final synthesis in one explorer task unless the scope is already tightly bounded.

## REPO MAP
```text
dsa-stdexec/  C++ stdexec sender/receiver framework
accel-rpc/    Rust accelerator-aware RPC workspace
hw-eval/      Benchmark harnesses
docs/         Plans, reports, specs, related work
tools/        Launcher behavior
```

## KEY PATHS
- Root policy: `AGENTS.md`
- Agent workflows: `.agents/workflows/`
- Agent prompt templates: `.agents/templates/`
- C++ framework: `dsa-stdexec/AGENTS.md`
- Rust workspace: `accel-rpc/AGENTS.md`
- Hardware benchmarking: `hw-eval/AGENTS.md`
- Docs placement rules: `docs/AGENTS.md`
- Launcher behavior: `tools/README.md`
