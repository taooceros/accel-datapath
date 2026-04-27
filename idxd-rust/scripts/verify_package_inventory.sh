#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/../.." && pwd)
cd "${REPO_ROOT}"

STALE_PATTERN='dsa-ffi|idxd-bindings|dsa-bindings'

failures=0

check_no_matches() {
  local label=$1
  shift
  local output
  if output=$(rg -n "${STALE_PATTERN}" "$@" 2>/dev/null); then
    printf '[verify_package_inventory] stale %s references found:\n%s\n' "${label}" "${output}" >&2
    failures=1
  fi
}

check_no_matches "active manifest" Cargo.toml accel-rpc/Cargo.toml accel-rpc/*/Cargo.toml
check_no_matches "active M003 roadmap" .gsd/milestones/M003/M003-ROADMAP.md

project_matches=$(rg -n "${STALE_PATTERN}" .gsd/PROJECT.md 2>/dev/null || true)
if [[ -n "${project_matches}" ]]; then
  disallowed_project_matches=$(printf '%s\n' "${project_matches}" | rg -v 'then-current|✅ complete' || true)
  if [[ -n "${disallowed_project_matches}" ]]; then
    printf '[verify_package_inventory] stale active PROJECT references found:\n%s\n' "${disallowed_project_matches}" >&2
    failures=1
  fi
fi

if [[ "${failures}" -ne 0 ]]; then
  printf '[verify_package_inventory] verdict=fail canonical_stack=idxd-sys+idxd-rust\n' >&2
  exit 1
fi

printf '[verify_package_inventory] verdict=pass canonical_stack=idxd-sys+idxd-rust manifests=clean roadmap=clean project=historical-only\n'
