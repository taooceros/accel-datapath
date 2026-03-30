# docs/report AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
`docs/report/` stores findings, validation notes, profiling analyses, and research-ingestion writeups. Reports here are grouped by topic first, then named with a topic-local numeric prefix plus a concise descriptor.

## TOPIC GROUPS
- `architecture/` — architecture analysis, design decisions, component breakdowns, and repo-level technical interpretation.
- `benchmarking/` — performance analysis, optimization results, profiling output, and benchmark-derived findings.
- `hw_eval/` — hardware enablement, validation, smoke tests, bindings status, launcher issues, and device-specific investigations.
- `literature/` — KB-searchable literature reviews, paper-ingestion summaries, and cross-paper synthesis.
- `artifacts/` — rendered `.pdf` / `.typ` report companions that support a markdown report but are not the primary KB input.
- `incidents/` — one-off environment or workflow failure reports that do not belong to a technical subsystem thread.

## CONVENTIONS
- Put new report files in the narrowest topic directory that matches the report's primary purpose.
- Keep the markdown report as the canonical KB-searchable source; place companion PDFs/Typst sources in `artifacts/`.
- Do not repeat the topic directory name in the filename when the directory already provides that context (for example, avoid `hw_eval/.../hw_eval_*` and `literature/.../*literature*`).
- Name report files as `NNN.<descriptor>.<ext>` within each topic directory, where `NNN` is a topic-local sequence number.
- Keep descriptors short, descriptive, and stable; dates may remain in the descriptor when they matter for historical traceability.
- Prefer a single primary topic per report; use links inside the document instead of duplicating the same report across topics.

## ANTI-PATTERNS
- Do not place new markdown reports directly under `docs/report/`; choose a topic directory.
- Do not treat `artifacts/` as the primary home for KB-searchable knowledge.
- Do not mix hardware debug notes into `benchmarking/` when the main value is device bring-up or validation.
