# Ingest top-tier paper PDFs into markdown and KB

## Goal

Download accessible PDFs for the selected top-tier related-work papers, transform them into markdown, and record the transformed markdown in the local knowledge database.

## Constraints discovered

- the current local KB tracks only `docs/plan/**/*.md`, `docs/report/*.md`, and `docs/specs/*.md`
- `docs/related_work/` is useful for curated topical notes, but it is not the primary tracked ingestion target
- there is no existing paper-corpus pipeline in this checkout, so the ingestion path needs to be explicit and minimal
- final decision: raw PDFs remain in `papers/top_tier_pdfs/`, while searchable paper content is stored as markdown under `docs/report/`; `sync-kb` remains markdown-only

## Planned layout

- raw PDFs under `papers/top_tier_pdfs/`
- transformed markdown as tracked files under `docs/report/`
- one index report that maps titles to local PDF paths, source URLs, and markdown files

## Planned execution

1. resolve direct PDF URLs or canonical landing pages for the selected papers
2. create a local PDF directory for downloaded assets
3. download only openly accessible PDFs
4. convert each downloaded PDF into markdown with source metadata and provenance headers
5. sync the markdown files into the local KB
6. record inaccessible or missing PDFs in an index report

## Result

- downloaded the accessible paper PDFs into `papers/top_tier_pdfs/`
- wrote the searchable extracted paper corpus into `docs/report/004` through `docs/report/007`
- kept KB sync markdown-only and documented that policy in the repo KB docs

## Acceptance criteria

- every successfully acquired paper has a stable local PDF path
- every acquired paper has a corresponding markdown file in `docs/report/`
- every markdown file records title, venue/year, and source URL
- `sync-kb` completes successfully for the new markdown files
- any inaccessible paper is explicitly listed with its canonical URL and status
