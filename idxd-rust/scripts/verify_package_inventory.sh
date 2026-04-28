#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/../.." && pwd)
cd "${REPO_ROOT}"

STALE_PACKAGE_PATTERN='dsa[-]ffi|iax[-]ffi|idxd[-]bindings|dsa[-]bindings'
OLD_ASYNC_API_PATTERN='copy_exact|copy_into|source_bytes\(|destination_bytes\(|with_destination_len|result\.bytes|AsyncMemmoveResult[[:space:]]*\{[^}]*bytes|AsyncMemmoveRequest::new[[:space:]]*\([^,)]*\)|AsyncDsaHandle::memmove_into|\.memmove_into[[:space:]]*\('
SOURCE_ONLY_PATTERN='source[- ]only|source:[[:space:]]*Vec<u8>|src:[[:space:]]*Vec<u8>'
DST_LEN_PATTERN='dst_len'
HANDWRITTEN_DSA_ABI_PATTERN='pub[[:space:]]+struct[[:space:]]+(DsaHwDesc|DsaCompletionRecord)\b|struct[[:space:]]+(DsaHwDesc|DsaCompletionRecord)\b'

failures=0
checked_categories=()

existing_paths() {
  local path
  for path in "$@"; do
    if [[ -e "${path}" ]]; then
      printf '%s\n' "${path}"
    fi
  done
}

is_self_guard_line() {
  local line=$1
  [[ "${line}" == idxd-rust/scripts/verify_package_inventory.sh:* ]]
}

is_historical_or_guard_context() {
  local line=$1
  [[ "${line}" =~ historical|Historical|previous|Previous|prior|Prior|completed|complete|then-current|old|stale|Stale|replaced|removed|no[[:space:]-]longer|do[[:space:]-]not|without[[:space:]]reintroducing|reject|Reject|fail|fails|guard|allowlist|supersede|migrat|consolidat|override|before[[:space:]]further[[:space:]]integration ]]
}

is_allowed_dst_len_context() {
  local line=$1

  # The verifier itself names the pattern it enforces.
  if is_self_guard_line "${line}"; then
    return 0
  fi

  # Historical plans/summaries and current guard language may discuss the old shape.
  if [[ "${line}" == .gsd/* ]] && is_historical_or_guard_context "${line}"; then
    return 0
  fi

  # Project state may preserve the exact old one-off stale-reference command as
  # completed verification evidence; it is not an active API recommendation.
  if [[ "${line}" == .gsd/* ]] && [[ "${line}" =~ rg[[:space:]]+-n ]]; then
    return 0
  fi

  # dst_len remains a valid validation/report field; it is stale only when it
  # represents an async request side channel or source-only request shape.
  [[ "${line}" =~ DestinationTooSmall|for_buffers|dst_len[[:space:]]*\<|src_len|validation|Validation|report|Report|error|Error|too[[:space:]-]small|async_session.rs|custom_codec.rs|memmove_contract.rs ]]
}

is_allowed_package_context() {
  local line=$1

  if is_self_guard_line "${line}"; then
    return 0
  fi

  # Append-only decisions and completed milestone records are intentionally
  # historical. Active guidance must say idxd-sys/idxd-rust instead.
  if [[ "${line}" == .gsd/DECISIONS.md:* ]]; then
    return 0
  fi

  is_historical_or_guard_context "${line}"
}

is_allowed_async_context() {
  local line=$1

  if is_self_guard_line "${line}"; then
    return 0
  fi

  # Project state may preserve the exact old one-off stale-reference command as
  # completed verification evidence; it is not an active API recommendation.
  if [[ "${line}" == .gsd/* ]] && [[ "${line}" =~ rg[[:space:]]+-n ]]; then
    return 0
  fi

  # Exact removed-helper names may appear in guard definitions or contract-test
  # negative fixtures when the line clearly belongs to stale-reference checking.
  if [[ "${line}" =~ ^idxd-rust/scripts/|^idxd-rust/tests/|^accel-rpc/tonic-profile/tests/|^accel-rpc/tonic-profile/scripts/ ]] && is_historical_or_guard_context "${line}"; then
    return 0
  fi

  # `dst_len: usize` is still valid inside validation/report error shapes. It
  # is stale only in async request structs or active docs that describe a hidden
  # destination-length side channel.
  [[ "${line}" =~ DestinationTooSmall|for_buffers|validation|Validation|report|Report|error|Error|too[[:space:]-]small|custom_codec.rs|memmove_contract.rs ]]
}

record_match_failures() {
  local category=$1
  local pattern=$2
  local allow_function=$3
  shift 3

  mapfile -t paths < <(existing_paths "$@")
  if [[ "${#paths[@]}" -eq 0 ]]; then
    printf '[verify_package_inventory] category=%s no_paths=1\n' "${category}" >&2
    return 0
  fi

  checked_categories+=("${category}")

  local matches disallowed
  matches=$(rg -n --pcre2 "${pattern}" "${paths[@]}" 2>/dev/null || true)
  if [[ -z "${matches}" ]]; then
    return 0
  fi

  disallowed=""
  local line
  while IFS= read -r line; do
    if "${allow_function}" "${line}"; then
      continue
    fi
    disallowed+="${line}"$'\n'
  done <<< "${matches}"

  if [[ -n "${disallowed}" ]]; then
    printf '[verify_package_inventory] stale category=%s references found:\n%s' "${category}" "${disallowed}" >&2
    failures=1
  fi
}

package_paths=(
  Cargo.toml
  README.md
  idxd-sys/Cargo.toml
  idxd-rust/Cargo.toml
  idxd-rust/README.md
  hw-eval/Cargo.toml
  hw-eval/README.md
  accel-rpc/Cargo.toml
  accel-rpc/tonic-profile/Cargo.toml
  accel-rpc/tonic-profile/README.md
  .gsd/PROJECT.md
  .gsd/DECISIONS.md
  .gsd/milestones/M004/M004-ROADMAP.md
  .gsd/milestones/M004/slices/S03/S03-PLAN.md
)

api_paths=(
  idxd-rust/src
  idxd-rust/tests
  idxd-rust/scripts
  idxd-rust/README.md
  accel-rpc/tonic-profile/src
  accel-rpc/tonic-profile/tests
  accel-rpc/tonic-profile/scripts
  accel-rpc/tonic-profile/README.md
)

abi_paths=(
  idxd-sys/src
  idxd-rust/src
  idxd-rust/tests
  hw-eval/src
  accel-rpc/tonic-profile/src
)

record_match_failures "package-owner" "${STALE_PACKAGE_PATTERN}" is_allowed_package_context "${package_paths[@]}"
record_match_failures "old-async-api" "${OLD_ASYNC_API_PATTERN}" is_allowed_async_context "${api_paths[@]}"
record_match_failures "source-only-request-shape" "${SOURCE_ONLY_PATTERN}" is_allowed_async_context "${api_paths[@]}"
record_match_failures "dst-len-side-channel" "${DST_LEN_PATTERN}" is_allowed_dst_len_context "${api_paths[@]}"
record_match_failures "handwritten-dsa-abi" "${HANDWRITTEN_DSA_ABI_PATTERN}" is_self_guard_line "${abi_paths[@]}"

if [[ "${failures}" -ne 0 ]]; then
  printf '[verify_package_inventory] verdict=fail canonical_stack=idxd-sys+idxd-rust categories=%s\n' "$(IFS=,; printf '%s' "${checked_categories[*]}")" >&2
  exit 1
fi

printf '[verify_package_inventory] verdict=pass canonical_stack=idxd-sys+idxd-rust categories=%s\n' "$(IFS=,; printf '%s' "${checked_categories[*]}")"
