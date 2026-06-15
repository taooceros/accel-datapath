# Add paper-specific summary and correlation sections to the literature review

## Goal

Strengthen the repo's literature review by ensuring every ingested paper has a paper-specific summary section and an explicit correlation to the repository's research questions and thesis.

## Constraints discovered

- `docs/related_work/` is intentionally topic-first, not paper-first
- KB-tracked searchable paper content belongs under `docs/report/`
- `docs/report/005` through `docs/report/007` already contain one section per paper, but those sections are still extraction-oriented rather than curated summary-oriented
- the review needs stronger paper-to-repo correlation without duplicating the topic synthesis already present in `docs/related_work/`

## Planned layout

- keep `docs/related_work/` as the thematic synthesis layer
- upgrade `docs/report/005.paper_ingestion_async_runtime_2026-03-28.md` through `docs/report/007.paper_ingestion_accelerator_hostpath_2026-03-28.md` into first-pass curated per-paper summary documents
- add a correlation matrix report under `docs/report/` that maps papers to the main research questions in `docs/research_plan.md`
- update the paper index and related-work overview so readers can navigate between topic notes, per-paper summaries, and the cross-paper matrix

## Planned execution

1. add a uniform per-paper structure to the existing grouped paper reports
2. add explicit correlation fields grounded in the repo's research questions and topic notes
3. add a paper-to-question correlation matrix report
4. update the ingestion index and related-work overview to reflect the new summary layer

## Acceptance criteria

- every indexed paper has a consistent summary block beyond raw extracted text
- every indexed paper has an explicit repo-correlation section
- a single matrix exists that links papers to the repo's major research questions
- the related-work overview explains where topic synthesis ends and paper-level summaries begin

## Result

- upgraded `docs/report/005` through `docs/report/007` from extraction-oriented notes into first-pass curated paper-summary documents
- added `docs/report/008.paper_question_correlation_matrix_2026-03-28.md` to map papers onto the repo's current research questions
- updated `docs/report/004.top_tier_paper_ingestion_index_2026-03-28.md` and `docs/related_work/README.md` so readers can navigate between topic-first and paper-first literature views
