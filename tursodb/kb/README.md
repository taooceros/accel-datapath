# Local KB sources

The local Turso database is rebuilt from git-tracked markdown sources:

- `docs/plan/**/*.md`
- `docs/report/*.md`
- `docs/specs/*.md`

Use `rebuild-kb` from a `devenv shell` to recreate `.turso/knowledge.db` from those files.
Use:

- `sync-kb [path ...]` for incremental upserts and deletes
- `search-kb "query text"` for hybrid search
- `search-kb-fts "query text"` for FTS-only search
- `search-kb-vector "query text"` for vector-only search

The current schema stores whole documents, a Turso FTS index on `title` and `body`, and a deterministic `vector1bit` embedding generated from normalized document tokens.

This is intentionally simple:

- FTS uses Turso's `USING fts` index support.
- vector search uses exact distance functions over `binary_embedding`.
- no approximate vector index is assumed.
- multi-word `search-kb` queries sum per-term `fts_score(...)` contributions into the lexical signal.
- results are fused with a simple reciprocal-rank hybrid score over lexical and vector ranks, with a small bonus for lexical matches.
- explicit retrieval-mode utilities make it clear whether a result set came from FTS, vector search, or hybrid fusion.
- `sync-kb` with no arguments reconciles all tracked KB sources; `sync-kb path/to/file.md` upserts just that path, and deleting a tracked path plus syncing it removes it from the DB.
