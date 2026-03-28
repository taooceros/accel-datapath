# docs AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Documentation and knowledge-base inputs. This subtree is about document placement and retrieval semantics, not code execution.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Plans | `plan/` | Pre-change work plans. KB-tracked. |
| Reports | `report/` | Findings and validation writeups. KB-tracked. |
| Specs | `specs/README.md` | Authoritative local DSA/IAX specs. KB-tracked. |
| Related work | `related_work/README.md` | Thesis-driven note organization. |

## CONVENTIONS
- Keep KB-searchable project knowledge in tracked markdown under `docs/plan/`, `docs/report/`, or `docs/specs/`.
- Use `docs/related_work/` for curated topic notes tied to the repo thesis, not as the primary dump for searchable paper extraction.
- When a paper or PDF should become KB-searchable, extract or curate it into markdown under a tracked path, usually `docs/report/`.
- Keep single-point observations in `../remark/`; this file only narrows rules inside `docs/`.

## ANTI-PATTERNS
- Do not treat raw PDFs as KB inputs.
- Do not put spec facts into ad hoc notes when `docs/specs/` is the authoritative source.
- Do not duplicate root workflow rules for naming plans, reports, or remarks here.
