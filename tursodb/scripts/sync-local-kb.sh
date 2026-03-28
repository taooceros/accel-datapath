#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/kb-common.sh"

repo_root="$kb_repo_root"
db_path="$kb_db_path"
schema_path="$kb_schema_path"
tmp_sql="$(mktemp)"
tmp_paths="$(mktemp)"
shell_output="$(mktemp)"
trap 'rm -f "$tmp_sql" "$tmp_paths" "$shell_output"' EXIT
tracked_rel_paths=()

mkdir -p "$(dirname "$db_path")"

cat "$schema_path" > "$tmp_sql"
cat >> "$tmp_sql" <<'EOF'
BEGIN IMMEDIATE;
EOF

append_seen_path() {
  local rel_path="$1"
  tracked_rel_paths+=("$rel_path")
}

if [ "$#" -gt 0 ]; then
  for arg in "$@"; do
    abs_path="$(kb_abs_from_arg "$arg")"
    if [ -d "$abs_path" ]; then
      find "$abs_path" -type f -name '*.md' | LC_ALL=C sort >> "$tmp_paths"
      continue
    fi
    printf '%s\n' "$abs_path" >> "$tmp_paths"
  done
else
  kb_tracked_files > "$tmp_paths"
fi

sort -u -o "$tmp_paths" "$tmp_paths"

while IFS= read -r abs_path; do
  [ -n "$abs_path" ] || continue
  if ! rel_path="$(kb_rel_from_abs "$abs_path" 2>/dev/null)"; then
    continue
  fi
  if ! kb_is_tracked_rel "$rel_path"; then
    continue
  fi
  append_seen_path "$rel_path"
  if [ -f "$abs_path" ]; then
    kb_append_upsert_sql "$abs_path" "$tmp_sql"
  else
    kb_append_delete_sql_for_rel "$rel_path" "$tmp_sql"
  fi
done < "$tmp_paths"

if [ "$#" -eq 0 ]; then
  if [ "${#tracked_rel_paths[@]}" -eq 0 ]; then
    cat >> "$tmp_sql" <<'EOF'
DELETE FROM source_documents
WHERE source_path GLOB 'docs/plan/*' OR source_path GLOB 'docs/report/*' OR source_path GLOB 'docs/specs/*';
EOF
  else
    printf "DELETE FROM source_documents\n" >> "$tmp_sql"
    printf "WHERE (source_path GLOB 'docs/plan/*' OR source_path GLOB 'docs/report/*' OR source_path GLOB 'docs/specs/*')\n" >> "$tmp_sql"
    printf "  AND source_path NOT IN (" >> "$tmp_sql"
    for i in "${!tracked_rel_paths[@]}"; do
      rel_path_sql="$(printf '%s' "${tracked_rel_paths[$i]}" | sed "s/'/''/g")"
      if [ "$i" -gt 0 ]; then
        printf ", " >> "$tmp_sql"
      fi
      printf "'%s'" "$rel_path_sql" >> "$tmp_sql"
    done
    printf ");\n" >> "$tmp_sql"
  fi
fi

cat >> "$tmp_sql" <<'EOF'
COMMIT;
.quit
EOF

{
  printf ".open %s\n" "$db_path"
  cat "$tmp_sql"
} | tursodb --quiet --experimental-index-method >"$shell_output" 2>&1

cat "$shell_output"

if grep -q '^  × ' "$shell_output"; then
  printf 'sync-kb failed; see parser output above\n' >&2
  exit 1
fi

if [ "$#" -gt 0 ]; then
  printf "Synced selected sources into %s\n" "$db_path"
else
  printf "Synced tracked sources into %s\n" "$db_path"
fi
