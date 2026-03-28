#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
db_path="${TURSODB_DB_PATH:-$repo_root/.turso/knowledge.db}"
query_text="${1:-}"
limit="${2:-10}"
if ! [[ "$limit" =~ ^[0-9]+$ ]]; then
  printf 'error: limit must be a non-negative integer\n' >&2
  exit 1
fi
tmp_query="$(mktemp)"
tmp_sql="$(mktemp)"
trap 'rm -f "$tmp_query" "$tmp_sql"' EXIT

if [ -z "$query_text" ]; then
  printf 'usage: search-kb-vector "query text" [limit]\n' >&2
  exit 1
fi

printf '%s\n' "$query_text" > "$tmp_query"

vector_sql="$(
  awk -v dims=256 -v fanout=8 '
    function token_hash(tok,    h, j, ch, pos) {
      h = 17;
      for (j = 1; j <= length(tok); ++j) {
        ch = substr(tok, j, 1);
        pos = index(chars, ch);
        if (pos == 0) {
          continue;
        }
        h = (h * 131 + pos + j) % 2147483647;
      }
      return h;
    }
    BEGIN {
      chars = "abcdefghijklmnopqrstuvwxyz0123456789";
      for (i = 0; i < dims; ++i) {
        scores[i] = 0;
      }
    }
    {
      raw = $0;
      weight = (raw ~ /^#/) ? 4 : 1;
      line = tolower(raw);
      n = split(line, tokens, /[[:space:]]+/);
      for (i = 1; i <= n; ++i) {
        tok = tokens[i];
        if (tok == "") {
          continue;
        }
        h = token_hash(tok);
        for (j = 0; j < fanout; ++j) {
          idx = (h + j * 104729 + length(tok) * 7919) % dims;
          sign = (((int(h / (j + 1)) + j + length(tok)) % 2) == 0) ? -weight : weight;
          scores[idx] += sign;
        }
      }
    }
    END {
      printf "vector1bit('\''[";
      for (i = 0; i < dims; ++i) {
        if (i > 0) {
          printf ",";
        }
        printf "%d", (scores[i] >= 0) ? 1 : -1;
      }
      printf "]'\'')";
    }
  ' "$tmp_query"
)"

cat > "$tmp_sql" <<EOF
.open $db_path
SELECT
  'vector' AS retrieval_mode,
  source_kind,
  source_path,
  title,
  vector_distance_cos(binary_embedding, $vector_sql) AS vector_distance
FROM source_documents
ORDER BY vector_distance ASC, source_path ASC
LIMIT $limit;
.quit
EOF

tursodb --quiet --experimental-index-method < "$tmp_sql"
