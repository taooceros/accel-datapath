---
description: Generate static OMP SYSTEM.md from SYSTEM2.md
---

Generate `.omp/SYSTEM.md` from `.omp/SYSTEM2.md`.

Requirements:
- Treat `SYSTEM2.md` as the human-authored source of truth for the customized system prompt.
- Do not copy raw Handlebars/internal template directives such as `{{#if ...}}`, `{{#each ...}}`, `{{#has ...}}`, or `{{toolRefs.*}}` into `SYSTEM.md`.
- Use `.omp/SYSTEM_DUMP.md` as the concrete rendered reference for environment-dependent sections that prompt engineers cannot render directly.
- Expand the available environmental switches into plain Markdown text in `SYSTEM.md`, including:
  - available internal URL schemes,
  - skills,
  - active tool inventory,
  - tool input conventions,
  - discovery instructions,
  - AST tool instructions,
  - exploration/tool-priority rules.
- Omit unavailable conditional sections rather than leaving template syntax behind.
- Preserve the customized tone and policy changes from `SYSTEM2.md`.
- Preserve concrete resolved values from `SYSTEM_DUMP.md` where `SYSTEM2.md` contains internal placeholders.
- After generation, verify that `.omp/SYSTEM.md` contains no `{{` or `}}`.
- Keep the result as a static prompt file suitable for OMP to load directly.

Additional user request:
$ARGUMENTS
