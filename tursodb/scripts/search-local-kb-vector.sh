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
tmp_terms="$(mktemp)"
tmp_sql="$(mktemp)"
trap 'rm -f "$tmp_query" "$tmp_terms" "$tmp_sql"' EXIT

if [ -z "$query_text" ]; then
  printf 'usage: search-kb-vector "query text" [limit]\n' >&2
  exit 1
fi

printf '%s\n' "$query_text" > "$tmp_query"

tr '[:upper:]' '[:lower:]' < "$tmp_query" \
  | awk '
      {
        for (i = 1; i <= NF; ++i) {
          if ($i != "" && !seen[$i]++) {
            print $i;
          }
        }
      }
    ' > "$tmp_terms"

phrase_query="$(
  tr '[:upper:]' '[:lower:]' < "$tmp_query" \
    | awk '
      {
        line = "";
        for (i = 1; i <= NF; ++i) {
          if (line == "") {
            line = $i;
          } else {
            line = line " " $i;
          }
        }
        print line;
      }
    '
)"

snippet_term_expr=""
if [ -n "$phrase_query" ]; then
  phrase_sql="$(printf '%s' "$phrase_query" | sed "s/'/''/g")"
  snippet_term_expr="CASE WHEN instr(base.body_lc, '$phrase_sql') > 0 THEN '$phrase_sql' END"
fi

if [ -s "$tmp_terms" ]; then
  while IFS= read -r term; do
    [ -n "$term" ] || continue
    term_sql="$(printf '%s' "$term" | sed "s/'/''/g")"
    term_snippet_expr="CASE WHEN instr(base.body_lc, '$term_sql') > 0 THEN '$term_sql' END"
    if [ -z "$snippet_term_expr" ]; then
      snippet_term_expr="$term_snippet_expr"
    else
      snippet_term_expr="COALESCE($snippet_term_expr, $term_snippet_expr)"
    fi
  done < "$tmp_terms"
fi

if [ -z "$snippet_term_expr" ]; then
  snippet_term_expr="NULL"
fi

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
WITH base AS (
  SELECT
    source_kind,
    source_path,
    title,
    body,
    lower(body) AS body_lc,
    vector_distance_cos(binary_embedding, $vector_sql) AS vector_distance
  FROM source_documents
)
SELECT
  'vector' AS retrieval_mode,
  source_kind,
  source_path,
  title,
  CASE
    WHEN $snippet_term_expr IS NULL THEN replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
    WHEN instr(body_lc, $snippet_term_expr) > 120 THEN replace(replace(substr(body, instr(body_lc, $snippet_term_expr) - 120, 240), char(10), ' '), char(13), ' ')
    ELSE replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
  END AS snippet,
  vector_distance
FROM base
ORDER BY vector_distance ASC, source_path ASC
LIMIT $limit;
.quit
EOF

tursodb --quiet --experimental-index-method < "$tmp_sql"
