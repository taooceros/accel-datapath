---
name: codemogger
description: Search an indexed codebase for relevant code. Use semantic mode for natural-language discovery and keyword mode for identifier lookup. Results include file path, symbol name, kind, signature, and line numbers, with optional snippets.
---

# Codemogger Code Search Workflow

Use this tool FIRST when exploring or navigating code, before falling back to Glob or Grep.

## When to Use

Use this when searching source code and you do not already know the exact file to open.

It is useful for:
- finding where a function, class, type, or variable is defined
- finding code related to a concept or subsystem
- narrowing broad code exploration to a few candidate files

## When Not to Use

- searching plans, reports, specs, or other docs; use `tursodb-kb` or direct doc reads instead
- reading file details once the target path is already known
- using search hits as proof without reading the underlying file

## How to Use

`codemogger` is configured in this repository as a CLI workflow. Run it from the repo root through `devenv`:

- `devenv shell -- codemogger --help`
- `devenv shell -- codemogger index .`
- `devenv shell -- codemogger search "query text"`

Local state lives under `.codemogger/` at the repository root.

Common commands:

- `devenv shell -- codemogger index .` — build or refresh the local code index
- `devenv shell -- codemogger search "query text"` — search indexed code
- `devenv shell -- codemogger search --snippet "query text"` — include code snippets in results
- `devenv shell -- codemogger search --mode keyword "Name"` — exact-ish symbol/name lookup
- `devenv shell -- codemogger search --mode semantic "concept"` — concept or feature search

Check help when needed:

- `devenv shell -- codemogger search --help`
- `devenv shell -- codemogger index --help`

## Fallbacks

If codemogger is not returning useful results:

- use `glob` when you mainly need filenames or path discovery
- use `grep` for exact text or regex matching
- read the strongest candidate files directly once you have them

## Maintenance

Refresh the index whenever a codechange happen.

Primary maintenance command:

1. **Re-index code** — `devenv shell -- codemogger index .`

## Workflow

1. Confirm the task is code search, not document retrieval.
2. If the code index may be stale, run `devenv shell -- codemogger index .`.
3. Run `devenv shell -- codemogger search "query text"` with the narrowest useful query.
4. If results are weak, fall back to `glob` or `grep`.
5. Read the top candidate files directly before making any non-trivial claim.

## Output Contract

When using this skill, return structured search notes with:

- command and query used
- whether indexing was run or assumed current
- top candidate file paths
- why each candidate is relevant
- whether direct file reading is still required
