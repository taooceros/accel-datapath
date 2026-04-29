#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${M009_S02_DIRECT_ASYNC_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/013.idxd_rust_direct_async_readability.md}"

fail() {
  printf '[check_m009_s02_direct_async_readability] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[check_m009_s02_direct_async_readability] %s\n' "$*"
}

require_tool() {
  local tool=$1
  command -v "${tool}" >/dev/null 2>&1 || fail "missing_tool=${tool} install_or_enter_repo_devenv_before_running_this_guard"
}

require_file() {
  local path=$1
  local description=$2
  [[ -f "${path}" ]] || fail "missing_${description}=${path}"
  [[ -s "${path}" ]] || fail "empty_${description}=${path}"
}

require_literal_in_file() {
  local file=$1
  local literal=$2
  local description=$3
  rg --fixed-strings --ignore-case --quiet -- "${literal}" "${file}" \
    || fail "missing=${description} literal=${literal} file=${file}"
}

require_literal() {
  local literal=$1
  local description=$2
  require_literal_in_file "${REPORT_PATH}" "${literal}" "${description}"
}

require_regex() {
  local regex=$1
  local description=$2
  rg --ignore-case --quiet -- "${regex}" "${REPORT_PATH}" \
    || fail "missing=${description} regex=${regex} file=${REPORT_PATH}"
}

reject_regex() {
  local regex=$1
  local description=$2
  if rg --ignore-case --quiet -- "${regex}" "${REPORT_PATH}"; then
    fail "forbidden=${description} regex=${regex} file=${REPORT_PATH}"
  fi
}

require_tool rg
require_file "${REPORT_PATH}" report

source_paths=(
  "${REPO_ROOT}/docs/report/architecture/012.hardware_rust_readability_baseline.md"
  "${REPO_ROOT}/docs/report/architecture/006.direct_tokio_completion_record_contract.md"
  "${CRATE_DIR}/src/async_session.rs"
  "${CRATE_DIR}/src/async_direct.rs"
  "${CRATE_DIR}/src/direct_memmove.rs"
  "${CRATE_DIR}/tests/async_memmove_contract.rs"
  "${CRATE_DIR}/tests/tokio_handle_contract.rs"
  "${CRATE_DIR}/tests/direct_async_contract_guard.rs"
)

for path in "${source_paths[@]}"; do
  require_file "${path}" source_input
done

require_literal '# idxd-rust direct async readability map' 'report_title'
require_literal 'R018' 'requirement_r018'
require_literal 'S02' 'slice_s02'
require_literal 'ordinary-host' 'ordinary_host_limit'
require_literal 'prepared-host' 'prepared_host_limit'
require_literal 'host-free' 'host_free_guard_scope'
require_literal 'submit-now / complete-later ENQCMD' 'submit_now_complete_later_trace'
require_literal 'operation-owned descriptor/completion/source/destination lifetime' 'operation_owned_lifetime_phrase'
require_literal 'terminal completion or runtime cleanup' 'terminal_completion_or_cleanup'
require_literal 'Recovery of owned request buffers is allowed only before hardware acceptance' 'safe_recovery_limit'
require_literal 'public owner/session wiring' 'phase_public_owner_session'
require_literal 'request validation and safe recovery' 'phase_request_validation_recovery'
require_literal 'registration and initial ENQCMD submission' 'phase_registration_initial_submission'
require_literal 'operation-owned lifetime' 'phase_operation_owned_lifetime'
require_literal 'monitor polling and future resolution' 'phase_monitor_polling_future_resolution'
require_literal 'completion, retry, and recovery' 'phase_completion_retry_recovery'
require_literal 'fake/backend seams' 'phase_fake_backend_seams'
require_literal 'AsyncDsaSession' 'public_session'
require_literal 'AsyncDsaHandle' 'public_handle'
require_literal 'AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut)' 'canonical_request_constructor'
require_literal 'AsyncMemmoveResult' 'public_result'
require_literal 'AsyncMemmoveError::kind' 'async_error_kind_accessor'
require_literal 'direct_failure_kind' 'direct_failure_kind_accessor'
require_literal 'direct_failure' 'direct_failure_accessor'
require_literal 'memmove_error' 'memmove_error_accessor'
require_literal 'into_request' 'request_recovery_accessor'
require_literal 'DirectAsyncMemmoveRuntime' 'direct_runtime_owner'
require_literal 'PendingOperation' 'pending_operation_owner'
require_literal 'DirectMemmoveState' 'direct_memmove_state_owner'
require_literal 'DirectMemmoveBackend' 'direct_backend_trait'
require_literal 'DirectPortalBackend' 'direct_portal_backend'
require_literal 'ScriptedDirectBackend' 'scripted_direct_backend'
require_literal 'submit_enqcmd_once' 'direct_portal_enqcmd_once'
require_literal 'CompletionSnapshot' 'completion_snapshot_contract'
require_literal 'BackpressureExceeded' 'backpressure_failure_kind'
require_literal 'RuntimeUnavailable' 'runtime_unavailable_failure_kind'
require_literal 'RegistrationClosed' 'registration_closed_failure_kind'
require_literal 'MonitorClosed' 'monitor_closed_failure_kind'
require_literal 'zero-length' 'zero_length_rejection'
require_literal 'destination-too-small' 'destination_too_small_rejection'
require_literal 'destination readable length advances only after terminal success observation and post-copy verification' 'destination_length_safety'
require_literal 'Direct futures resolve from completion snapshots' 'completion_snapshot_future_resolution'
require_literal 'not from blocking `DsaSession::memmove_uninit` completion' 'no_blocking_future_resolution'
require_literal 'No public async allocation convenience constructor' 'no_public_allocation_constructor'
require_literal 'No public borrowed copy-back helper' 'no_public_borrowed_copy_back'
require_literal 'No public payload logging' 'no_public_payload_logging'
require_literal 'No prepared-host hardware success claim from ordinary-host' 'ordinary_host_no_hardware_claim'
require_literal 'idxd-rust/src/async_session.rs' 'source_path_async_session'
require_literal 'idxd-rust/src/async_direct.rs' 'source_path_async_direct'
require_literal 'idxd-rust/src/direct_memmove.rs' 'source_path_direct_memmove'
require_literal 'idxd-rust/tests/async_memmove_contract.rs' 'source_path_async_memmove_contract'
require_literal 'idxd-rust/tests/tokio_handle_contract.rs' 'source_path_tokio_handle_contract'
require_literal 'idxd-rust/tests/direct_async_contract_guard.rs' 'source_path_direct_async_contract_guard'
require_literal 'idxd-rust/tests/direct_async_readability_contract.rs' 'source_path_direct_async_readability_contract'
require_literal 'idxd-rust/scripts/check_m009_s02_direct_async_readability.sh' 'guard_script_path'
require_literal 'bash idxd-rust/scripts/check_m009_s02_direct_async_readability.sh' 'guard_command'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test direct_async_readability_contract -- --nocapture' 'cargo_guard_command'
require_literal 'bash idxd-rust/scripts/check_m009_s01_readability_baseline.sh' 's01_baseline_command'

require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'AsyncDsaSession' 'source_async_session_public_owner'
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'AsyncDsaHandle' 'source_async_session_public_handle'
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'AsyncMemmoveRequest' 'source_async_session_request'
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'AsyncMemmoveRequest::new(source, destination)' 'source_async_session_explicit_request_constructor'
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'open_default' 'source_async_session_open_default'
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'spawn_with_direct_backend' 'source_async_session_direct_fixture_seam'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'DirectAsyncMemmoveRuntime' 'source_async_direct_runtime'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'monitor_completion_records' 'source_async_direct_monitor'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'PendingOperation' 'source_async_direct_pending_operation'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'EnqcmdSubmission::Accepted' 'source_async_direct_enqcmd_accept'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'EnqcmdSubmission::Rejected' 'source_async_direct_enqcmd_reject'
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'tokio::task::yield_now' 'source_async_direct_yield_backoff'
require_literal_in_file "${CRATE_DIR}/src/direct_memmove.rs" 'Operation-local memmove state that can be submitted now and completed later.' 'source_direct_memmove_operation_local_comment'
require_literal_in_file "${CRATE_DIR}/src/direct_memmove.rs" 'The caller must keep `src..src + request.len()` allocated and immutable' 'source_direct_memmove_safety_source_lifetime'
require_literal_in_file "${CRATE_DIR}/src/direct_memmove.rs" 'verify_initialized_destination' 'source_direct_memmove_no_payload_verify_helper'

reject_regex 'Direct Tokio v1 uses MOVDIR64(/MOVDIR64B)? fallback|MOVDIR64(/MOVDIR64B)? fallback (is )?(allowed|required|the direct|for direct Tokio v1)' 'movdir64_fallback_direct_v1'
reject_regex 'blocking `?memmove_uninit`? is the future-resolution mechanism|direct async futures (are )?resolved by synchronous|futures resolve because synchronous `?memmove_uninit`? returned' 'blocking_memmove_uninit_future_resolution'
reject_regex 'public payload logging is (acceptable|allowed|required)|logs source payload bytes|logs destination payload bytes|payload bytes? (are|is) logged' 'public_payload_logging'
reject_regex 'public allocation convenience constructors? (are|remain|is) (required|canonical|allowed)' 'public_allocation_convenience_constructors'
reject_regex 'public (borrowed )?copy-back helpers? (are|remain|is) (required|canonical|allowed)' 'public_copy_back_helpers'
reject_regex 'operation-owned descriptor/completion/source/destination lifetime (can|may|should) (end|be released|be dropped) before terminal completion|descriptor/completion/source/destination lifetime is not operation-owned' 'weakened_operation_owned_lifetime'
reject_regex 'Tonic is (required|a prerequisite)|required Tonic|must require Tonic' 'tonic_required_for_direct_proof'
reject_regex 'preallocated (completion-record )?(registry|pool).* is the v1 design|v1 design .* preallocated (completion-record )?(registry|pool)' 'preallocated_registry_v1_design'
reject_regex 'unbounded async(-context)? spin loops? (are|is) (acceptable|allowed|required)' 'unbounded_async_spin_loops'

log "verdict=pass report=${REPORT_PATH}"
