---
name: literature-review
description: Paper-specific acquisition and processing pipeline — discover, process, clarify, synthesize. Use when conducting a structured literature review on a topic, processing a batch of papers, or when the repository's knowledge-grounding workflow requires discovering and acquiring academic papers. Also triggers for structured synthesis when papers are already local or the repo's literature notes are already present.
---

# Literature Review Workflow

Structured pipeline for acquiring and processing academic papers. This is the paper-specific workflow within the broader repo grounding process described by `AGENTS.md`.

## Repository Notes

- Existing literature history lives under `archive/docs/related_work/` and `archive/docs/report/literature/`.
- Active literature conclusions should be curated into `docs/evidence/` only when a current thread needs a stable answer page.
- Old local PDF staging material lives under `archive/root/papers/`.
- Before broad manual scans, follow `AGENTS.md`: read the relevant tracked docs first, then use focused repo/code search.

## Input Modes

- **Specific papers** — local PDFs, web links, or paper titles → Start at Phase 1.
- **A topic or research question** → Start at Phase 0.

## Phase 0: Discover Resources

1. **Check existing knowledge** — Read archived related-work notes and literature reports, then search active evidence maps for current coverage. If sufficient, skip to Phase 3.
2. **Search** — Use the search tools below. Try multiple query formulations. Prioritize strong venues and authoritative sources. Apply search saturation criteria: stop only after multiple query reformulations converge on the same core set of papers or when marginal results stop changing the reading list.
3. **Curate a reading list** — Select the most relevant resources. Breadth first, then depth.
4. **Acquire** — Download PDFs locally when useful. Use an explicitly chosen active path for new corpus work; treat `archive/root/papers/` as historical staging material.

## Phase 1: Process Papers

For each paper:

1. **Check for existing coverage** — Search archived literature notes/reports and active evidence maps to see whether the paper or its claims are already captured.
2. **Store locally** — Save the PDF or canonical URL in a repo-appropriate location chosen for the current review.
3. **Deep comprehension** — Build a structured understanding of:
   - key claims, mechanisms, and quantitative results
   - system model, threat model, baselines, and evaluation methodology
   - limitations, assumptions, and future work
4. **Integrate findings** — Merge durable findings into the relevant active evidence map or explicitly requested report. Add cross-references instead of duplicating claims.

## Phase 2: Clarify and Expand

1. Re-read the local PDF or source for unclear figures, tables, or methodological details.
2. Query the repo KB for related design decisions, plans, reports, and specs that change the interpretation.
3. Search the web for cited papers, background, and follow-up work.
4. Process newly discovered papers via Phase 1.

## Phase 3: Synthesize and Report

1. **Cross-paper analysis** — Compare papers by mechanism, assumptions, metrics, and limitations.
2. **Update topic files** — Merge current conclusions into `docs/evidence/` or a requested report without creating paper-by-paper duplication.
3. **Update tracking docs** — Record next steps or open questions in `notes/now.md`, `docs/evidence/`, or another explicitly chosen repo document.
4. **Ground conclusions** — Keep every non-trivial claim attributable to a paper, spec, repo note, or directly inspected artifact.

## Search Tool Selection

| Need | Tool | Why |
|------|------|-----|
| Internal prior work, plans, reports, specs | direct reads under `archive/docs/`, `docs/evidence/`, and `docs/remark/` | Repo-local tracked docs should be first. |
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
2. Read the relevant archived topic file or active evidence map before editing so claims land in the right conceptual section.
3. If a claim does not fit an existing section, add a new section where it fits the file's conceptual flow.
4. Use comparison tables for cross-paper results when they reduce repetition.
5. Use cross-references instead of duplicating claims across files.
6. Every non-trivial claim should stay attributable to a paper, repo doc, or directly inspected artifact.
7. If a new paper confirms an existing claim, strengthen the existing claim with another citation instead of restating it.

## Context Management for Large Reviews

1. Plan subtopics before starting.
2. Work one subtopic at a time.
3. Write findings to disk immediately instead of keeping them only in context.
4. Re-scan the relevant tracked docs after updating sources when later synthesis depends on the new material.
