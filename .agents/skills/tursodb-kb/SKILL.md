---
name: tursodb-kb
description: Repo-local knowledge-base workflow for querying and maintaining the Turso-backed markdown index. Use when you need prior plans, reports, specs, or other tracked KB content before broader manual repo scans.
---

# TursoDB Local KB Workflow

Use this skill whenever the task depends on repository memory: prior plans, reports, specs, or other KB-tracked markdown. This skill makes the local KB the first retrieval step before broader document scans or code search.

## When to Use

- Looking for prior design decisions, reports, plans, or spec fragments already captured in tracked markdown.
- Checking whether a topic is already documented before starting a broader literature or implementation search.
- Refreshing the KB after adding or editing tracked markdown that should become searchable.
- Choosing between exact keyword lookup and semantic recall for repo-grounded document retrieval.

## When Not to Use

- Searching source code. Use the repo's code-search workflow instead.
- Searching raw PDFs directly. Raw PDFs are intentionally not KB inputs.
- Treating KB hits as authoritative when the underlying source file should still be read directly for details.

## Indexed Content Boundary

The local KB is built from tracked markdown, currently centered on paths such as:

- `docs/plan/**/*.md`
- `docs/report/*.md`
- `docs/specs/*.md`

If a paper, note, or extracted result should become searchable, convert or summarize it into tracked markdown first, then sync that markdown into the KB. Do not claim that `tursodb` ingests PDFs directly.

## Query Modes

Choose the narrowest command that matches the retrieval need.

| Need | Command | Why |
|------|---------|-----|
| General repo-memory lookup with both wording and concept recall | `devenv shell -- search-kb "query text"` | Hybrid retrieval combines lexical and vector signals. |
| Exact terms, known phrases, spec wording, file titles | `devenv shell -- search-kb-fts "query text"` | Best when the wording itself matters. |
| Approximate semantic recall, paraphrases, concept lookup | `devenv shell -- search-kb-vector "query text"` | Best when the right wording is unclear. |

## Maintenance Commands

Use maintenance commands only when the indexed markdown set has changed or when the KB is missing.

1. **Full rebuild** — `devenv shell -- rebuild-kb`
   - Recreates `.turso/knowledge.db` from the tracked markdown sources.
2. **Incremental update** — `devenv shell -- sync-kb [path ...]`
   - Upserts tracked markdown changes and removes tracked rows for deleted paths covered by the sync flow.

## Workflow

1. Start with KB lookup before broad manual scans of `docs/`.
2. Pick the query mode that best matches the task:
   - hybrid first for general exploration
   - FTS when exact wording matters
   - vector when concepts matter more than keywords
3. Read the referenced markdown file directly before making any non-trivial claim.
4. If your new markdown should be discoverable later, run `sync-kb` for the touched file or `rebuild-kb` when a full refresh is needed.
5. Keep conclusions grounded in the underlying tracked markdown, not just the KB snippet.

## Output Contract

When using this skill, return structured retrieval notes with:

- command and query used
- top matching file paths
- the key claim or reason each hit matters
- whether direct file reading is still required
- whether KB maintenance is needed after edits

## Stop Conditions

Stop and report instead of guessing when:

- the KB returns no relevant results and the task depends on prior repo docs
- the result snippets conflict and the source markdown needs adjudication
- the KB appears stale and the required markdown has not been synced yet

The correct fallback is to read the relevant tracked markdown directly and, if needed, refresh the KB.
