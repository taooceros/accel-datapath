CREATE TABLE IF NOT EXISTS source_documents (
  id INTEGER PRIMARY KEY,
  source_path TEXT NOT NULL UNIQUE,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('plan', 'report', 'specs')),
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  embedding_kind TEXT NOT NULL DEFAULT 'token_sketch_v2',
  binary_embedding BLOB NOT NULL,
  sha256 TEXT NOT NULL,
  loaded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS source_documents_kind_path_idx
  ON source_documents (source_kind, source_path);

CREATE INDEX IF NOT EXISTS source_documents_fts
  ON source_documents USING fts (title, body)
  WITH (weights = 'title=2.0,body=1.0');
