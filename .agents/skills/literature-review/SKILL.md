---
name: literature-review
description: Paper-specific acquisition and processing pipeline — discover, process, clarify, synthesize. Use when conducting a structured literature review on a topic, processing a batch of papers, or when the repository's knowledge-grounding workflow requires discovering and acquiring academic papers. Also triggers for structured synthesis when papers are already local or the repo's literature notes are already present.
---

# Literature Review Workflow

Structured pipeline for acquiring and processing academic papers. This is the paper-specific workflow within the broader repo grounding process described by `AGENTS.md`, `tursodb/README.md`, and `tursodb/kb/README.md`.

## Repository Notes

- This repository's existing literature hub is `docs/related work/`, not `docs/literature-review/`.
- The repo currently contains `docs/research_plan.md`; it does not currently contain `docs/research_log.md`.
- The repo currently does not include `rules/knowledge_grounding.md`, `papers/`, or `local-corpus/bin/corpus`. If those are added later, treat them as the canonical paper-processing path. Until then, use this workflow with the repo's current docs and KB helpers.
- Before broad manual scans, follow `AGENTS.md`: start with the repo-local `tursodb-kb` skill for KB retrieval, then use `devenv shell -- codemogger search "query"` for code search.

## Input Modes

- **Specific papers** — local PDFs, web links, or paper titles → Start at Phase 1.
- **A topic or research question** → Start at Phase 0.

## Phase 0: Discover Resources

1. **Check existing knowledge** — Read `docs/related work/` and use the repo-local `tursodb-kb` skill to query the local KB. If sufficient, skip to Phase 3.
2. **Search** — Use the search tools below. Try multiple query formulations. Prioritize strong venues and authoritative sources. Apply search saturation criteria: stop only after multiple query reformulations converge on the same core set of papers or when marginal results stop changing the reading list.
3. **Curate a reading list** — Select the most relevant resources. Breadth first, then depth.
4. **Acquire** — Download PDFs locally when possible. If the repo later adds `papers/todo/`, use it as the staging area. For non-downloadable resources, note the URL and clearly flag that the paper was not acquired.

## Phase 1: Process Papers

For each paper:

1. **Check for existing coverage** — Search the repo KB and `docs/related work/` to see whether the paper or its claims are already captured.
2. **Store locally** — Save the PDF or canonical URL in a repo-appropriate location. If a future `papers/` or local corpus directory is added, use that canonical location instead.
3. **Deep comprehension** — Build a structured understanding of:
   - key claims, mechanisms, and quantitative results
   - system model, threat model, baselines, and evaluation methodology
   - limitations, assumptions, and future work
4. **Integrate findings** — Merge findings into the appropriate file under `docs/related work/`. Add cross-references instead of duplicating claims.

## Phase 2: Clarify and Expand

1. Re-read the local PDF or source for unclear figures, tables, or methodological details.
2. Query the repo KB for related design decisions, plans, reports, and specs that change the interpretation.
3. Search the web for cited papers, background, and follow-up work.
4. Process newly discovered papers via Phase 1.

## Phase 3: Synthesize and Report

1. **Cross-paper analysis** — Compare papers by mechanism, assumptions, metrics, and limitations.
2. **Update topic files** — Merge claims into `docs/related work/` without creating paper-by-paper duplication.
3. **Update planning or tracking docs** — Record next steps or open questions in `docs/research_plan.md` or another explicitly chosen repo document.
4. **Ground conclusions** — Keep every non-trivial claim attributable to a paper, spec, repo note, or directly inspected artifact.

## Search Tool Selection

| Need | Tool | Why |
|------|------|-----|
| Internal prior work, plans, reports, specs | `tursodb-kb` skill | Repo-local retrieval should be first. |
| Exact keyword/spec lookup in indexed docs | `devenv shell -- search-kb-fts` | Best when wording is known. |
| Semantic recall from indexed docs | `devenv shell -- search-kb-vector` | Best when wording may differ. |
| Repo code and implementation search | `devenv shell -- codemogger search "query"` | Required repo-first code search path. |
| Academic paper search | Scholar or web search tooling | Use to discover papers and PDF URLs. |
| Known URL | Direct fetch/download tools | Best for canonical landing pages or PDF downloads. |

## Paper and Resource Acquisition

1. **PDFs:** Prefer direct PDF downloads and keep a stable local copy when licensing permits.
2. **Open-access resources:** Search for a direct PDF before giving up. Queries like `"<title>" filetype:pdf` often help.
3. **Non-downloadable resources:** Record metadata plus URL and clearly mark the paper as not locally acquired.
4. **Complete the pipeline:** Every acquired resource should either be stored locally or recorded with a stable URL and enough metadata to revisit later.
5. **Self-evolve carefully:** If no existing tool can access a resource, first try the repo's current fetch/search options. Only introduce a new tool or skill if the existing ones are genuinely insufficient.

## Integration Rules

When merging findings into topic files:

1. Each file covers one coherent topic. Sections should flow from foundational ideas to more specific implications.
2. Read `docs/related work/README.md` and the target topic file before editing so claims land in the right conceptual section.
3. If a claim does not fit an existing section, add a new section where it fits the file's conceptual flow.
4. Use comparison tables for cross-paper results when they reduce repetition.
5. Use cross-references instead of duplicating claims across files.
6. Every non-trivial claim should stay attributable to a paper, repo doc, or directly inspected artifact.
7. If a new paper confirms an existing claim, strengthen the existing claim with another citation instead of restating it.

## Context Management for Large Reviews

1. Plan subtopics before starting.
2. Work one subtopic at a time.
3. Write findings to disk immediately instead of keeping them only in context.
4. Re-query the local KB after updating tracked sources if the repo's KB scope includes those sources.
