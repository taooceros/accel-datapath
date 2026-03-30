# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## DEFAULT MODE: SPEED FIRST
- Prefer response time over exhaustive verification unless the user asks for high confidence.
- Answer from local context first; escalate only when the risk or uncertainty is meaningful.
- If external lookup may help but is not blocking, answer first and enrich later.

## SOURCE ORDER
1. Current conversation
2. This file, nearest child `AGENTS.md`, nearby `README.md`
3. Repo docs in `docs/` and `remark/`
4. Local indexes/tools: `codemogger`, Turso KB, `read`, `grep`, `glob`, `lsp_*`
5. External docs and web search

## TOOL ROUTING
- Known path: `read`
- Narrow local lookup: `grep` / `glob`
- Code discovery: `codemogger search`
- Semantic navigation/refactor safety: `lsp_*`
- Plans/reports/spec history: `tursodb-kb` when the path is not already known
- External docs (`@librarian`, web) only when local sources are insufficient, version-specific behavior matters, or the user asks for verification
- Reuse prior external findings instead of refetching; store reusable notes under `docs/cache/external/<topic>.md` and reindex after adding them

## PARALLELISM
- Prefer concurrent tool calls for independent reads/searches.
- Run sequentially only when later steps depend on earlier results.
- For broad codebase discovery, prefer parallel search or `@explorer`.
- When delegating to `@explorer`, batch independent searches in one request and ask it to run them concurrently.
- Do not use `@explorer` for a single known-path read or a narrow symbol/file lookup; use direct local tools first.

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
- C++ framework: `dsa-stdexec/AGENTS.md`
- Rust workspace: `accel-rpc/AGENTS.md`
- Hardware benchmarking: `hw-eval/AGENTS.md`
- Docs placement rules: `docs/AGENTS.md`
- Launcher behavior: `tools/README.md`
