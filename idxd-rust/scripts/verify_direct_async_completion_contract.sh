#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${IDXD_RUST_DIRECT_ASYNC_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/006.direct_tokio_completion_record_contract.md}"

fail() {
  printf '[verify_direct_async_completion_contract] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[verify_direct_async_completion_contract] %s\n' "$*"
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
  grep -Eqi -- "${regex}" "${REPORT_PATH}" || fail "missing=${description} regex=${regex}"
}

reject_regex() {
  local regex=$1
  local description=$2
  if grep -Eqi -- "${regex}" "${REPORT_PATH}"; then
    fail "forbidden=${description} regex=${regex}"
  fi
}

require_file "${REPORT_PATH}"

require_literal 'direct completion-driven async memmove' 'architecture_name'
require_literal 'submit-now / complete-later ENQCMD model' 'submit_now_complete_later'
require_literal 'AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)' 'canonical_constructor'
require_regex 'ENQCMD(-oriented)? (direct )?submission|Direct Tokio v1 is ENQCMD-oriented|Attempt ENQCMD submission' 'enqcmd_direct_submission'
require_literal 'dynamically creates/registers one completion record for each submitted request' 'dynamic_per_request_completion_record'
require_literal 'PendingMemmoveOp' 'pending_operation_state'
require_literal 'an aligned DSA descriptor' 'pending_descriptor_ownership'
require_literal 'an aligned DSA completion record' 'pending_completion_ownership'
require_literal 'the source `Bytes`' 'pending_source_ownership'
require_literal 'the destination `BytesMut`' 'pending_destination_ownership'
require_literal 'CompletionMonitor' 'tokio_completion_monitor'
require_literal 'scan pending completion records' 'monitor_scans_completion_records'
require_literal 'future resolves only when the monitor observes terminal completion state' 'future_signaling_from_completion_record'
require_regex 'yield_now|adaptive backoff|yields/backoffs|bounded or adaptive policy' 'adaptive_yield_backoff'
require_literal 'Page-fault retry behavior from the synchronous path remains part of the direct async contract' 'page_fault_retry_preservation'
require_literal 'keep the same logical `PendingMemmoveOp` pending until terminal success or failure' 'page_fault_retry_continuation'
require_literal 'dropped-receiver lifecycle classification' 'dropped_receiver_cleanup'
require_literal 'Shutdown has two distinct phases' 'shutdown_classification'
require_literal 'R002' 'requirement_r002'
require_literal 'R003' 'requirement_r003'
require_literal 'R008' 'requirement_r008'
require_literal 'Expected host-free proof obligations' 's02_host_free_proof_obligations'
require_literal 'fake completion-record transitions' 'host_free_completion_transitions'
require_literal 'fake ENQCMD accept/reject behavior' 'host_free_enqcmd_accept_reject'
require_literal 'fake retry completion snapshots' 'host_free_retry_snapshots'
require_literal 'dropped-receiver cleanup' 'host_free_dropped_receiver_cleanup'
require_literal 'shutdown classification' 'host_free_shutdown_classification'
require_literal 'destination-length assertions' 'host_free_destination_length_assertions'

reject_regex 'Direct Tokio v1 uses MOVDIR64(/MOVDIR64B)? fallback|MOVDIR64(/MOVDIR64B)? fallback (is )?(allowed|required|the direct|for direct Tokio v1)' 'movdir64_fallback_direct_v1'
reject_regex 'Tonic is (required|a prerequisite)|required Tonic|must require Tonic' 'tonic_required_for_proof'
reject_regex 'preallocated (completion-record )?(registry|pool).* is the v1 design|v1 design .* preallocated (completion-record )?(registry|pool)' 'preallocated_registry_v1_design'
reject_regex 'blocking `?memmove_uninit`? is the future-resolution mechanism|direct async futures (are )?resolved by synchronous|futures resolve because synchronous `?memmove_uninit`? returned' 'blocking_memmove_uninit_future_resolution'
reject_regex 'public (borrowed )?copy-back helpers? (are|remain|is) (required|canonical|public|allowed)' 'public_copy_back_helpers'
reject_regex 'allocation convenience constructors? (are|remain|is) (required|canonical|allowed)' 'allocation_convenience_constructors'
reject_regex 'unbounded async(-context)? spin loops? (are|is) (acceptable|allowed|required)' 'unbounded_async_spin_loops'
reject_regex 'payload byte logging is acceptable|payload bytes? (are|is) logged|logs source payload bytes|logs destination payload bytes' 'payload_byte_logging'

log "verdict=pass report=${REPORT_PATH}"
