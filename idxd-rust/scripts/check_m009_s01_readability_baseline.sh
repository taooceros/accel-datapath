#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${M009_S01_BASELINE_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/012.hardware_rust_readability_baseline.md}"

fail() {
  printf '[check_m009_s01_readability_baseline] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[check_m009_s01_readability_baseline] %s\n' "$*"
}

require_tool() {
  local tool=$1
  command -v "${tool}" >/dev/null 2>&1 || fail "missing_tool=${tool} install_or_enter_repo_devenv_before_running_this_guard"
}

require_file() {
  local path=$1
  [[ -f "${path}" ]] || fail "missing_report=${path}"
  [[ -s "${path}" ]] || fail "empty_report=${path}"
}

require_heading() {
  local heading=$1
  rg --fixed-strings --line-regexp --quiet -- "${heading}" "${REPORT_PATH}" \
    || fail "missing_section=${heading} report=${REPORT_PATH}"
}

require_literal() {
  local literal=$1
  local description=$2
  rg --fixed-strings --ignore-case --quiet -- "${literal}" "${REPORT_PATH}" \
    || fail "missing_contract_reference=${description} literal=${literal} report=${REPORT_PATH}"
}

reject_literal() {
  local literal=$1
  local description=$2
  if rg --fixed-strings --ignore-case --quiet -- "${literal}" "${REPORT_PATH}"; then
    fail "forbidden_text=${description} literal=${literal} report=${REPORT_PATH}"
  fi
}

require_tool rg
require_file "${REPORT_PATH}"

required_headings=(
  '# Hardware Rust readability baseline'
  '## Purpose'
  '## Source inputs'
  '## Responsibility map'
  '## Direct async S02 inventory'
  '## Downstream slice contract baselines'
  '## Non-change boundaries'
  '## Verification matrix'
  '## R018 coverage'
  '## Ordinary-host and prepared-host limits'
)

for heading in "${required_headings[@]}"; do
  require_heading "${heading}"
done

for slice in S02 S03 S04 S05 S06; do
  require_literal "${slice}" "downstream_${slice}_baseline"
done

require_literal 'R018' 'requirement_R018'
require_literal 'idxd-rust' 'idxd_rust_responsibility'
require_literal 'idxd-sys' 'idxd_sys_responsibility'
require_literal 'hw-eval' 'hw_eval_responsibility'
require_literal 'documentation and verification baselines only' 'documentation_only_scope'
require_literal 'runtime behavior is intentionally unchanged' 'runtime_unchanged_scope'
require_literal 'ordinary-host' 'ordinary_host_limits'
require_literal 'prepared-host' 'prepared_host_limits'
require_literal 'host-free' 'host_free_guard'
require_literal 'tracked source' 'tracked_source_only'
require_literal 'no-payload' 'no_payload_contract'
require_literal 'must not include copied source bytes' 'forbid_copied_source_bytes'
require_literal 'destination bytes' 'forbid_destination_bytes'
require_literal 'idxd-rust/src/async_direct.rs' 'direct_async_runtime_file'
require_literal 'idxd-rust/src/async_session.rs' 'direct_async_session_file'
require_literal 'idxd-rust/src/direct_memmove.rs' 'direct_memmove_state_file'
require_literal 'idxd-rust/tests/async_memmove_contract.rs' 'async_memmove_contract_test'
require_literal 'idxd-rust/tests/tokio_handle_contract.rs' 'tokio_handle_contract_test'
require_literal 'idxd-rust/tests/direct_async_contract_guard.rs' 'direct_async_contract_guard_test'
require_literal 'AsyncDsaSession' 'direct_async_public_session'
require_literal 'AsyncDsaHandle' 'direct_async_public_handle'
require_literal 'AsyncMemmoveRequest' 'direct_async_public_request'
require_literal 'AsyncMemmoveResult' 'direct_async_public_result'
require_literal 'AsyncDirectFailure' 'direct_failure_accessor_contract'
require_literal 'direct_failure_kind' 'direct_failure_kind_accessor'
require_literal 'direct_failure' 'direct_failure_accessor'
require_literal 'into_request' 'request_recovery_semantics'
require_literal 'zero-length' 'zero_length_rejection_contract'
require_literal 'destination-size' 'destination_size_rejection_contract'
require_literal 'owner shutdown' 'owner_shutdown_contract'
require_literal 'backpressure' 'direct_backpressure_contract'
require_literal 'retry' 'direct_retry_contract'
require_literal 'completion snapshot' 'completion_snapshot_contract'
require_literal 'prepared-host hardware success' 'ordinary_host_must_not_claim_hardware_success'

reject_literal 'source_bytes' 'payload_field_source_bytes'
reject_literal 'destination_bytes' 'payload_field_destination_bytes'
reject_literal 'payload_dump' 'payload_dump_field'

log "verdict=pass report=${REPORT_PATH}"
