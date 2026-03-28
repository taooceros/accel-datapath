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
tmp_terms="$(mktemp)"
trap 'rm -f "$tmp_query" "$tmp_sql" "$tmp_terms"' EXIT

if [ -z "$query_text" ]; then
  printf 'usage: search-kb-fts "query text" [limit]\n' >&2
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

term_ctes=""
term_joins=""
term_match_expr="0"
fts_score_sum="0.0"
term_index=0
snippet_term_expr=""

if [ -s "$tmp_terms" ]; then
  while IFS= read -r term; do
    [ -n "$term" ] || continue
    term_sql="$(printf '%s' "$term" | sed "s/'/''/g")"
    term_index=$((term_index + 1))
    term_name="term_${term_index}"
    term_ctes="$term_ctes,
$term_name AS (
  SELECT rowid AS doc_id, fts_score(title, body, '$term_sql') AS score
  FROM source_documents
  WHERE fts_match(title, body, '$term_sql')
)"
    term_joins="$term_joins
  LEFT JOIN $term_name ON $term_name.doc_id = base.doc_id"
    if [ "$term_match_expr" = "0" ]; then
      term_match_expr="$term_name.doc_id IS NOT NULL"
    else
      term_match_expr="$term_match_expr AND $term_name.doc_id IS NOT NULL"
    fi
    fts_score_sum="$fts_score_sum + COALESCE($term_name.score, 0.0)"
    term_snippet_expr="CASE WHEN instr(base.body_lc, '$term_sql') > 0 THEN '$term_sql' END"
    if [ -z "$snippet_term_expr" ]; then
      snippet_term_expr="$term_snippet_expr"
    else
      snippet_term_expr="COALESCE($snippet_term_expr, $term_snippet_expr)"
    fi
  done < "$tmp_terms"
fi

phrase_bonus="0.0"
if [ -n "$phrase_query" ]; then
  phrase_sql="$(printf '%s' "$phrase_query" | sed "s/'/''/g")"
  phrase_bonus="CASE WHEN instr(base.title_lc, '$phrase_sql') > 0 THEN 1.5 ELSE 0.0 END + CASE WHEN instr(base.body_lc, '$phrase_sql') > 0 THEN 0.75 ELSE 0.0 END"
  if [ -z "$snippet_term_expr" ]; then
    snippet_term_expr="CASE WHEN instr(base.body_lc, '$phrase_sql') > 0 THEN '$phrase_sql' END"
  else
    snippet_term_expr="COALESCE(CASE WHEN instr(base.body_lc, '$phrase_sql') > 0 THEN '$phrase_sql' END, $snippet_term_expr)"
  fi
fi

if [ -z "$snippet_term_expr" ]; then
  snippet_term_expr="NULL"
fi

cat > "$tmp_sql" <<EOF
.open $db_path
WITH base AS (
  SELECT
    rowid AS doc_id,
    source_kind,
    source_path,
    title,
    body,
    lower(title) AS title_lc,
    lower(body) AS body_lc
  FROM source_documents
)$term_ctes,
scored AS (
  SELECT
    'fts' AS retrieval_mode,
    base.source_kind,
    base.source_path,
    base.title,
    base.body,
    base.body_lc,
    ($fts_score_sum) + ($phrase_bonus) AS lexical_score,
    $snippet_term_expr AS snippet_match_term
  FROM base$term_joins
  WHERE $term_match_expr
)
SELECT
  retrieval_mode,
  source_kind,
  source_path,
  title,
  CASE
    WHEN snippet_match_term IS NULL THEN replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
    WHEN instr(body_lc, snippet_match_term) > 120 THEN replace(replace(substr(body, instr(body_lc, snippet_match_term) - 120, 240), char(10), ' '), char(13), ' ')
    ELSE replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
  END AS snippet,
  lexical_score
FROM scored
ORDER BY lexical_score DESC, source_path ASC
LIMIT $limit;
.quit
EOF

tursodb --quiet --experimental-index-method < "$tmp_sql"
