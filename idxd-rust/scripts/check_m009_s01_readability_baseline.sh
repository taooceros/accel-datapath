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
  '## Tokio benchmark S03 inventory'
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
require_literal 'tokio_memmove_bench' 'tokio_benchmark_binary_contract'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench.rs' 'tokio_benchmark_source_file'
require_literal 'idxd-rust/tests/async_benchmark_cli_contract.rs' 'tokio_benchmark_cli_contract_test'
require_literal 'idxd-rust/tests/async_benchmark_verifier_contract.rs' 'tokio_benchmark_verifier_contract_test'
require_literal 'idxd-rust/scripts/verify_tokio_memmove_bench.sh' 'tokio_benchmark_verifier_script'
require_literal 'schema_version' 'tokio_benchmark_schema_version_field'
require_literal 'claim_eligible' 'tokio_benchmark_claim_eligible_field'
require_literal 'software_direct_async_diagnostic' 'tokio_software_diagnostic_target'
require_literal 'direct_async' 'tokio_direct_async_target'
require_literal 'direct_sync' 'tokio_direct_sync_comparison_target'
require_literal 'stdout/artifact equality' 'tokio_stdout_artifact_equality_contract'
require_literal 'expected_failure' 'tokio_expected_failure_classification'
require_literal 'artifact-validation failures' 'tokio_artifact_validation_hard_failures'
require_literal 'invalid numeric CLI inputs' 'tokio_invalid_numeric_negative_tests'
require_literal 'invalid enum values' 'tokio_invalid_enum_negative_tests'
require_literal 'missing artifact values' 'tokio_missing_artifact_negative_test'
require_literal 'forbidden payload dump fields' 'tokio_payload_dump_negative_test'
require_literal 'No-payload for benchmark artifacts' 'tokio_no_payload_artifact_constraint'
require_literal 'Software diagnostic mode' 'tokio_software_diagnostic_claim_boundary'
require_literal 'Prepared-host hardware claims' 'tokio_prepared_host_claim_boundary'

reject_literal 'source_bytes' 'payload_field_source_bytes'
reject_literal 'destination_bytes' 'payload_field_destination_bytes'
reject_literal 'payload_dump' 'payload_dump_field'

log "verdict=pass report=${REPORT_PATH}"
