# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## DEFAULT MODE: SPEED FIRST
- Prefer response time over exhaustive verification unless the user asks for high confidence.
- Answer from local context first; escalate only when the risk or uncertainty is meaningful.
- If external lookup may help but is not blocking, answer first and enrich later.
- For code discovery, use codemogger first (it can search semantically or by keyword); use other search tools only as fallback.

## SOURCE ORDER
1. Current conversation
2. Repo docs in `docs/` and `remark/`
3. Local indexes/tools: `codemogger`, Turso KB, `read`, `grep`, `glob`, `lsp_*`
4. External docs and web search

## EXPLORER WORKFLOW
- Explorer workflow details live in `.agents/workflows/explorer.md`.
- Use `@explorer` for bounded candidate discovery, not final synthesis.
- When delegating, follow the separate workflow doc and cite the exact explorer template when one exists.

## AGENT TEMPLATES
- Agent-specific task templates live under `.agents/templates/`.
- Use `.agents/templates/<agent_name>_*.md` naming.
- When delegating, cite the exact matching template file when one exists; do not reference `.agents/templates/` generically.
- Restate critical task budgets and stop conditions inline even when a template is provided.

## CONVENTIONS
- Read the latest relevant plan/report before acting.
- On resume, read the latest relevant plan/report and any linked durable artifacts before continuing.
- Keep durable detail in plans, reports, and remarks rather than transient dashboard files.
- Keep commit headlines short and consistent with current style; use the repo-local `.gitmessage` template for the body (`Summary` / `Why` / `Details` / `Verification`).
- Write a commit when you finish a small job so progress lands in focused, reviewable increments.
- Write a plan in `docs/plan/YYYY-MM-DD/NN.<topic>.<state>.md` before non-trivial changes.
- Write plans for humans first: state the goal, scope, intended changes, verification, and completion notes in plain language so a reader can review the work without needing agent-only workflow context.
- Do not write plans as agent-private shorthand, terse scratchpads, or control notes that only make sense to the executing agent.
- Write findings to `docs/report/<topic>/NNN.<descriptor>.<ext>`; write single-point insights to `remark/NNN_<topic>.md`.
- Read the nearest README before modifying a module.
- Match code to specs, not specs to code, unless explicitly told otherwise.
- Keep child `AGENTS.md` files lean and local; do not repeat parent guidance.

## DO NOT
- Guess DSA/IAX behavior if `docs/specs/*.md` or `docs/report/architecture/001.design_decisions.md` already cover it.
- Treat raw PDFs as KB-ingested content; searchable paper content belongs in tracked markdown.
- Run hardware-facing binaries directly when the documented flow requires `launch` / `dsa_launcher`.

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
