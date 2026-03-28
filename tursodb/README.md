# Local `tursodb` wrapper

This directory is a self-contained `devenv` setup for the preview `tursodb` CLI.
The database path is derived at shell startup as `repo/.turso/knowledge.db`.

The first local knowledge-base source set is:

- `docs/plan/**/*.md`
- `docs/report/*.md`
- `docs/specs/*.md`

Raw PDFs are intentionally **not** ingested directly into the local KB. If you want a paper or PDF to become searchable, extract or summarize it into markdown under a tracked path such as `docs/report/*.md`, then run `sync-kb` on that markdown.

The local KB uses:

- Turso FTS on `title` and `body`
- deterministic `vector1bit` embeddings for exact vector search
- no approximate vector index

## Layout

- `devenv.nix`: local shell definition and pinned preview package
- `devenv.yaml`: local devenv input definition

## Usage

```bash
cd /home/hongtao/accel-datapath/agent-env-wt/tursodb
devenv shell
rebuild-kb
sync-kb docs/plan/2026-03-27/18_add_incremental_kb_sync.done.md
search-kb "page fault retry"
search-kb-fts "page fault retry"
search-kb-vector "page fault retry"
tursodb
```

Inside the shell, open the persistent database with:

```sql
.open $TURSODB_DB_PATH
```

The `.turso/` directory is created automatically when the shell starts.
`rebuild-kb` recreates `.turso/knowledge.db` from the tracked markdown sources.
`sync-kb` incrementally upserts tracked markdown files into `.turso/knowledge.db`, and removes tracked rows for deleted paths that you pass in or for missing tracked files during a no-arg sync.
`sync-kb` is intentionally markdown-only; it does not parse PDFs.
`search-kb` runs a true hybrid search: it sums per-term `fts_score(...)` contributions for the lexical side, computes vector distance for the semantic side, derives lexical/vector ranks, and fuses them into one hybrid score.
`search-kb-fts` returns only FTS-ranked results.
`search-kb-vector` returns only vector-ranked results.

## Updating the preview version

Edit the pinned `version`, asset, and hash in `devenv.nix`.
The binary is fetched by Nix rather than by a repo-local script.
