# presentation AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Typst slide decks and presentation-only artifacts live here. This subtree is for concise, audience-facing outputs derived from repo-grounded plans, reports, specs, and code.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Current deck | `YYYY-MM-DD/*.typ` | Keep each presentation in a dated directory. |
| Supporting evidence | `../docs/plan/`, `../docs/report/`, `../docs/specs/` | Slides should summarize these sources, not replace them. |
| Typst workflow | `../presentation/<date>/` | Keep deck-local helpers close to the deck unless reused broadly. |

## CONVENTIONS
- Use dated directories: `presentation/YYYY-MM-DD/`.
- Keep one primary `.typ` entry file per deck, named by topic or meeting.
- Prefer plain Typst with lightweight local helpers over introducing a large slide framework unless the deck clearly needs it.
- Keep claims in slides traceable to repo sources; if a finding matters for later reuse, write it in `docs/report/` or `remark/` as well.
- When editing a deck, preserve readability on a 16:9 presentation page and favor concise speaker-facing structure over dense prose.
- Compile the deck after non-trivial changes to catch Typst errors.

## ANTI-PATTERNS
- Do not store the only copy of important analysis in slides.
- Do not create undated or ambiguously named presentation directories.
- Do not turn presentation files into a general knowledge dump; keep reusable knowledge in tracked docs.
