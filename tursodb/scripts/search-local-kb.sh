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
  printf 'usage: search-kb "query text" [limit]\n' >&2
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
    rowid AS doc_id,
    source_kind,
    source_path,
    title,
    body,
    lower(title) AS title_lc,
    lower(body) AS body_lc,
    vector_distance_cos(binary_embedding, $vector_sql) AS vector_distance
  FROM source_documents
)$term_ctes,
scored AS (
  SELECT
    base.source_kind,
    base.source_path,
    base.title,
    base.body,
    base.body_lc,
    CASE WHEN $term_match_expr THEN 1 ELSE 0 END AS is_fts_match,
    ($fts_score_sum) + ($phrase_bonus) AS lexical_score,
    base.vector_distance,
    $snippet_term_expr AS snippet_match_term
  FROM base$term_joins
),
ranked AS (
  SELECT
    s1.source_kind,
    s1.source_path,
    s1.title,
    s1.body,
    s1.body_lc,
    s1.is_fts_match,
    s1.lexical_score,
    s1.vector_distance,
    s1.snippet_match_term,
    (
      SELECT COUNT(*)
      FROM scored s2
      WHERE
        s2.lexical_score > s1.lexical_score
        OR (
          s2.lexical_score = s1.lexical_score
          AND s2.is_fts_match > s1.is_fts_match
        )
        OR (
          s2.lexical_score = s1.lexical_score
          AND s2.is_fts_match = s1.is_fts_match
          AND s2.vector_distance < s1.vector_distance
        )
    ) + 1 AS lexical_rank,
    (
      SELECT COUNT(*)
      FROM scored s2
      WHERE
        s2.vector_distance < s1.vector_distance
        OR (
          s2.vector_distance = s1.vector_distance
          AND s2.lexical_score > s1.lexical_score
        )
    ) + 1 AS vector_rank
  FROM scored s1
)
SELECT
  source_kind,
  source_path,
  title,
  CASE
    WHEN snippet_match_term IS NULL THEN replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
    WHEN instr(body_lc, snippet_match_term) > 120 THEN replace(replace(substr(body, instr(body_lc, snippet_match_term) - 120, 240), char(10), ' '), char(13), ' ')
    ELSE replace(replace(substr(body, 1, 240), char(10), ' '), char(13), ' ')
  END AS snippet,
  is_fts_match,
  lexical_score,
  vector_distance,
  lexical_rank,
  vector_rank,
  ((1.0 / (20 + lexical_rank)) + (1.0 / (20 + vector_rank)) + CASE WHEN is_fts_match = 1 THEN 0.05 ELSE 0.0 END) AS hybrid_score
FROM ranked
ORDER BY
  hybrid_score DESC,
  is_fts_match DESC,
  lexical_score DESC,
  vector_distance ASC
LIMIT $limit;
.quit
EOF

tursodb --quiet --experimental-index-method < "$tmp_sql"
