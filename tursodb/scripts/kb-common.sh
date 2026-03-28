#!/usr/bin/env bash
set -euo pipefail

kb_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
kb_db_path="${TURSODB_DB_PATH:-$kb_repo_root/.turso/knowledge.db}"
kb_schema_path="$kb_repo_root/tursodb/kb/schema.sql"

kb_tracked_files() {
  {
    find "$kb_repo_root/docs/plan" -type f -name '*.md'
    find "$kb_repo_root/docs/report" -maxdepth 1 -type f -name '*.md'
    find "$kb_repo_root/docs/specs" -maxdepth 1 -type f -name '*.md'
  } | LC_ALL=C sort
}

kb_is_tracked_rel() {
  case "$1" in
    docs/plan/*.md|docs/report/*.md|docs/specs/*.md) return 0 ;;
    *) return 1 ;;
  esac
}

kb_abs_from_arg() {
  case "$1" in
    /*) printf '%s\n' "$1" ;;
    plan/*|report/*|specs/*) printf '%s/docs/%s\n' "$kb_repo_root" "$1" ;;
    *) printf '%s/%s\n' "$kb_repo_root" "$1" ;;
  esac
}

kb_rel_from_abs() {
  case "$1" in
    "$kb_repo_root"/*) printf '%s\n' "${1#$kb_repo_root/}" ;;
    *) return 1 ;;
  esac
}

kb_source_kind_from_rel() {
  case "$1" in
    docs/plan/*) printf 'plan\n' ;;
    docs/report/*) printf 'report\n' ;;
    docs/specs/*) printf 'specs\n' ;;
    *) printf '%s\n' "${1%%/*}" ;;
  esac
}

kb_make_binary_vector_expr() {
  local abs_path="$1"
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
  ' "$abs_path"
}

kb_append_upsert_sql() {
  local abs_path="$1"
  local sql_file="$2"
  local rel_path
  local source_kind
  local title
  local rel_path_sql
  local source_kind_sql
  local title_sql
  local body_sql
  local vector_sql
  local sha256

  rel_path="$(kb_rel_from_abs "$abs_path")"
  source_kind="$(kb_source_kind_from_rel "$rel_path")"
  title="$(sed -n '/^# /{s/^# \+//;p;q;}' "$abs_path")"
  if [ -z "$title" ]; then
    title="$(basename "$rel_path")"
  fi

  rel_path_sql="$(printf '%s' "$rel_path" | sed "s/'/''/g")"
  source_kind_sql="$(printf '%s' "$source_kind" | sed "s/'/''/g")"
  title_sql="$(printf '%s' "$title" | sed "s/'/''/g")"
  body_sql="CAST(X'$(od -An -tx1 -v "$abs_path" | tr -d ' \n')' AS TEXT)"
  vector_sql="$(kb_make_binary_vector_expr "$abs_path")"
  sha256="$(sha256sum "$abs_path" | awk '{print $1}')"

  printf "INSERT INTO source_documents (source_path, source_kind, title, body, embedding_kind, binary_embedding, sha256) VALUES ('%s', '%s', '%s', %s, 'token_sketch_v2', %s, '%s') ON CONFLICT(source_path) DO UPDATE SET source_kind = excluded.source_kind, title = excluded.title, body = excluded.body, embedding_kind = excluded.embedding_kind, binary_embedding = excluded.binary_embedding, sha256 = excluded.sha256, loaded_at = CURRENT_TIMESTAMP;\n" \
    "$rel_path_sql" "$source_kind_sql" "$title_sql" "$body_sql" "$vector_sql" "$sha256" >> "$sql_file"
}

kb_append_delete_sql_for_rel() {
  local rel_path="$1"
  local sql_file="$2"
  local rel_path_sql

  rel_path_sql="$(printf '%s' "$rel_path" | sed "s/'/''/g")"
  printf "DELETE FROM source_documents WHERE source_path = '%s';\n" "$rel_path_sql" >> "$sql_file"
}
