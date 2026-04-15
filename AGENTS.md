# PROJECT KNOWLEDGE BASE

Research monorepo for Intel DSA/IAX data-path work.

## DEFAULT MODE: SPEED FIRST
- Prefer response time over exhaustive verification unless the user asks for high confidence.
- Answer from local context first; escalate only when the risk or uncertainty is meaningful.
- If external lookup may help but is not blocking, answer first and enrich later.
- For code discovery, use codemogger first (it can search semantically or by keyword); use other search tools only as fallback.

## SOURCE ORDER
1. Current conversation
2. Root status file: `current.md`
3. Repo docs in `docs/` and `remark/`
4. Local indexes/tools: `codemogger`, Turso KB, `read`, `grep`, `glob`, `lsp_*`
5. External docs and web search


## CURRENT MEMORY FILE
- Treat `current.md` as a dashboard and index for live work across sessions.
- Always consult `current.md` before acting, not only on resume.
- Multiple live threads are allowed. Every thread listed under active or paused in `current.md` must point to exactly one canonical state file under `.agents/state/threads/`.
- Canonical mutable thread state lives in the thread file, not in `current.md`. Keep durable detail in plans, reports, and remarks, then link those artifacts from the thread file and dashboard.
- Fixed resume order: `current.md` dashboard first, then the canonical thread file, then any linked plan or report artifacts.
- Dashboard updates must follow canonical thread file updates. Update the thread file first, then refresh the `current.md` entry.
- Remove a thread from the live dashboard once its thread file is marked `completed` or `archived` and the outcome is captured in durable artifacts.

### Dashboard and canonical thread authority
- `current.md` is an index and dashboard view for live work. It may mirror dashboard-facing metadata that helps agents route and inspect a live thread quickly, including whether the thread is listed as active or paused, its `index_label`, a short `summary`, brief `match_hints`, `related_artifacts` links, `owner_agent`, `owner_session_id`, `status`, lease timing, the canonical thread-file path, and `next_action`. Those mirrored fields stay non-authoritative in `current.md`.
- `.agents/state/threads/<thread-id>.md` is authoritative for all mutable thread state, including `thread_id`, `title`, `status`, `owner_agent`, `owner_session_id`, `previous_owner_session_id`, lease timestamps, handoff fields, `resume_allowed`, blockers, next actions, and any other per-thread detail from `.agents/templates/thread_state.md`, even when some of that metadata is also mirrored in `current.md` for dashboard visibility.
- If `current.md` and the thread file disagree, the canonical thread file wins. Refresh the dashboard instead of treating `current.md` as the source of truth.

### Ownership, sessions, and lease takeover
- A live thread may be owned by only one agent at a time. An agent may hold at most one live thread at a time.
- `owner_session_id` is the session that currently owns the thread lease. `previous_owner_session_id` stores the most recent prior owner session when ownership changes.
- Keep `owner_agent` stable for the current owner. On takeover or resume, move the old session id into `previous_owner_session_id`, write the new `owner_session_id`, and refresh `lease_acquired_at`, `last_updated`, and `lease_expires_at`.
- The default lease expires 4 hours after `last_updated`. Takeover is allowed only after that stale lease expires or when the current owner has recorded an explicit handoff with `handoff_to` and `handoff_reason`.
- If an older session resumes after losing ownership, it must read `current.md`, then the canonical thread file. If the file shows a different active `owner_session_id`, the older session no longer owns the thread. It must not overwrite the state, and should either continue only after a valid stale-lease takeover or treat the thread as unavailable and self-claim another matching thread.

### Thread lifecycle and self-claim rules
- Valid thread lifecycle states are `active`, `paused`, `blocked`, `handoff_pending`, `completed`, and `archived`.
- Main-agent sessions must determine their thread by matching the incoming request against dashboard metadata in `current.md`: `index_label`, `summary`, `match_hints`, and `related_artifacts`.
- If there is exactly one plausible live match and resume is allowed, resume that thread by claiming the canonical thread file.
- If there is no plausible live match, create a new canonical thread file under `.agents/state/threads/`, then add or refresh the dashboard entry in `current.md`.
- Ask one disambiguation question only when multiple plausible live matches exist.
- Never rely on agent name alone for self-claim. The request-to-dashboard match decides whether to resume or create a new thread.

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
- Read `current.md` first for dashboard routing metadata, then the latest relevant plan/report before acting.
- On resume, read `current.md` for dashboard routing, then the matched canonical thread file under `.agents/state/threads/`, then the linked plan/report artifacts named by that thread.
- Keep `current.md` focused on active and paused dashboard entries. Keep thread detail in canonical thread files, and use reports and completed plan notes as the durable record for finished work.
- Keep commit headlines short and consistent with current style; use the repo-local `.gitmessage` template for the body (`Summary` / `Why` / `Details` / `Verification`).
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
- Agent workflows: `.agents/workflows/`
- Agent prompt templates: `.agents/templates/`
- C++ framework: `dsa-stdexec/AGENTS.md`
- Rust workspace: `accel-rpc/AGENTS.md`
- Hardware benchmarking: `hw-eval/AGENTS.md`
- Docs placement rules: `docs/AGENTS.md`
- Launcher behavior: `tools/README.md`
