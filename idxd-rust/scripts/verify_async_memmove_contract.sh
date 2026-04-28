#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${REPO_ROOT}/docs/report/architecture/004.bytes_async_memmove_contract.md"
PLAN_PATH="${REPO_ROOT}/docs/plan/2026-04-27/001.bytes_async_memmove_api_contract.planned.md"

fail() {
  printf '[verify_async_memmove_contract] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[verify_async_memmove_contract] %s\n' "$*"
}

require_file() {
  local path=$1
  [[ -s "${path}" ]] || fail "missing_or_empty=${path}"
}

require_literal() {
  local literal=$1
  local description=$2
  grep -Fqi -- "${literal}" "${REPORT_PATH}" || fail "missing=${description} literal=${literal}"
}

require_regex() {
  local regex=$1
  local description=$2
  grep -Eq -- "${regex}" "${REPORT_PATH}" || fail "missing=${description} regex=${regex}"
}

require_file "${PLAN_PATH}"
require_file "${REPORT_PATH}"

require_literal 'AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)' 'canonical_constructor'
require_literal 'AsyncMemmoveResult {' 'result_struct'
require_literal 'destination: BytesMut' 'result_destination'
require_literal 'report: MemmoveValidationReport' 'result_report'
require_literal 'BytesMut' 'bytesmut_contract'
require_literal 'spare capacity is not readable until it has been initialized' 'spare_capacity_readability'
require_literal 'Pointer validity' 'unsafe_pointer_validity'
require_literal 'Aliasing' 'unsafe_aliasing'
require_literal 'Cancellation safety' 'unsafe_cancellation_safety'
require_literal 'Completion safety' 'unsafe_completion_safety'

require_literal 'copy_exact' 'stale_copy_exact_rejection'
require_literal 'copy_into' 'stale_copy_into_rejection'
require_literal 'with_destination_len' 'stale_with_destination_len_rejection'
require_literal 'public `memmove_into`' 'stale_memmove_into_rejection'
require_literal 'result.bytes' 'stale_result_bytes_rejection'
require_literal 'source_bytes()' 'stale_source_bytes_rejection'
require_literal 'destination_bytes()' 'stale_destination_bytes_rejection'
require_literal 'source-only constructors' 'stale_source_only_constructor_rejection'

require_literal 'zero-length source' 'negative_zero_length_source'
require_literal 'Destination too small' 'negative_destination_too_small'
require_literal 'Oversized destination tail' 'oversized_tail_test'
require_literal 'Validation-before-enqueue' 'validation_before_enqueue_test'
require_literal 'Hardware `MemmoveError` propagation' 'hardware_memmove_error_test'
require_literal 'Cancellation after submission' 'cancellation_test'
require_literal 'FIFO/ordering behavior' 'ordering_test'
require_literal 'Owner shutdown' 'owner_shutdown_test'
require_literal 'Inline submission failure classification' 'inline_submission_failure_test'
require_literal 'Compatibility worker/channel failure classification' 'worker_channel_test'

require_literal 'owner_shutdown' 'owner_shutdown_diagnostic'
require_literal 'inline `enqcmd` submission failure' 'inline_submission_diagnostic'
require_literal 'worker_init_closed' 'worker_init_closed_diagnostic'
require_literal 'request_channel_closed' 'request_channel_closed_diagnostic'
require_literal 'response_channel_closed' 'response_channel_closed_diagnostic'
require_literal 'worker_panicked' 'worker_panicked_diagnostic'
require_literal 'invalid_length' 'wrapped_invalid_length'
require_literal 'destination_too_small' 'wrapped_destination_too_small'
require_literal 'completion_timeout' 'wrapped_completion_timeout'
require_literal 'malformed_completion' 'wrapped_malformed_completion'
require_literal 'page_fault_retry_exhausted' 'wrapped_page_fault_retry_exhausted'
require_literal 'completion_status' 'wrapped_completion_status'
require_literal 'byte_mismatch' 'wrapped_byte_mismatch'

require_literal 'inline `enqcmd` submission as the first implementation strategy' 'inline_enqcmd_first_version'
require_literal 'multiple submitting threads under the `enqcmd` model' 'enqcmd_multi_producer'
require_literal 'software aggregation' 'software_aggregation_deferral'
require_literal 'batching' 'batching_deferral'
require_literal 'movdir64' 'movdir64_deferral'
require_regex 'not required for M006|must not require a software aggregation thread' 'aggregation_not_required'

require_literal 'idxd-rust/tests/async_memmove_contract.rs' 'async_contract_test_target'
require_literal 'idxd-rust/tests/tokio_handle_contract.rs' 'tokio_contract_test_target'

log "verdict=pass report=${REPORT_PATH} plan=${PLAN_PATH}"
