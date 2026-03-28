#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/kb-common.sh"

repo_root="$kb_repo_root"
db_path="$kb_db_path"
schema_path="$kb_schema_path"
tmp_sql="$(mktemp)"
shell_output="$(mktemp)"
trap 'rm -f "$tmp_sql" "$shell_output"' EXIT

mkdir -p "$(dirname "$db_path")"
rm -f "$db_path" "$db_path-wal" "$db_path-shm"

cat "$schema_path" > "$tmp_sql"
cat >> "$tmp_sql" <<'EOF'
BEGIN IMMEDIATE;
EOF

while IFS= read -r abs_path; do
  kb_append_upsert_sql "$abs_path" "$tmp_sql"
done < <(kb_tracked_files)

printf "COMMIT;\n" >> "$tmp_sql"
printf "OPTIMIZE INDEX source_documents_fts;\n" >> "$tmp_sql"

{
  printf ".open %s\n" "$db_path"
  cat "$tmp_sql"
  printf ".quit\n"
} | tursodb --quiet --experimental-index-method >"$shell_output" 2>&1

cat "$shell_output"

if grep -q '^  × ' "$shell_output"; then
  printf 'rebuild-kb failed; see parser output above\n' >&2
  exit 1
fi

printf "Rebuilt %s from docs/plan/, docs/report/*.md, and docs/specs/*.md\n" "$db_path"
