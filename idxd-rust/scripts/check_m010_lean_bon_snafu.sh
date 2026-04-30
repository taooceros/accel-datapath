#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${M010_LEAN_BON_SNAFU_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/016.lean_bon_snafu_refactor.md}"

fail() {
  printf '[check_m010_lean_bon_snafu] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[check_m010_lean_bon_snafu] %s\n' "$*"
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

require_regex_in_file() {
  local file=$1
  local regex=$2
  local description=$3
  rg --ignore-case --quiet -- "${regex}" "${file}" \
    || fail "missing=${description} regex=${regex} file=${file}"
}

reject_regex_in_file() {
  local file=$1
  local regex=$2
  local description=$3
  if rg --ignore-case --quiet -- "${regex}" "${file}"; then
    fail "forbidden=${description} regex=${regex} file=${file}"
  fi
}

reject_literal_in_tree() {
  local root=$1
  local literal=$2
  local description=$3
  if rg --fixed-strings --ignore-case --quiet -- "${literal}" "${root}"; then
    fail "forbidden=${description} literal=${literal} root=${root}"
  fi
}

require_report_literal() {
  local literal=$1
  local description=$2
  require_literal_in_file "${REPORT_PATH}" "${literal}" "${description}"
}

require_tool rg
require_file "${REPORT_PATH}" report

source_paths=(
  "${CRATE_DIR}/src/validation.rs"
  "${CRATE_DIR}/src/lib.rs"
  "${CRATE_DIR}/src/async_session.rs"
  "${CRATE_DIR}/src/async_direct.rs"
  "${REPO_ROOT}/hw-eval/src/config.rs"
  "${REPO_ROOT}/hw-eval/src/main.rs"
  "${REPO_ROOT}/hw-eval/src/report.rs"
  "${REPO_ROOT}/idxd-sys/Cargo.toml"
  "${REPO_ROOT}/idxd-sys/src/lib.rs"
)

for path in "${source_paths[@]}"; do
  require_file "${path}" source_input
done

# Report-level evidence: keep the cold-reader convention, claim limits, and
# no-payload boundary guardable without running Cargo or touching hardware.
require_report_literal '# Lean bon/snafu refactor evidence' report_title
require_report_literal 'M010' milestone_m010
require_report_literal 'R019' requirement_r019
require_report_literal 'R008' requirement_r008
require_report_literal 'Claim boundaries' claim_boundaries_section
require_report_literal 'does **not** claim' negative_claim_boundary
require_report_literal 'prepared-host DSA/IAX hardware success from this documentation task' no_prepared_host_claim
require_report_literal 'no-payload diagnostic boundaries remain part of the documented contract' no_payload_claim_boundary
require_report_literal 'must not log or serialize payload bytes' payload_redaction_contract
require_report_literal 'source or destination payload bytes' validation_no_payload_detail
require_report_literal 'Payload bytes are not diagnostic material' idxd_sys_no_payload_detail
require_report_literal 'Guardable convention' guardable_convention_section
require_report_literal 'keep, replace, or avoid `bon` and `snafu`' post_read_action

# Report-level module coverage: future edits should not remove the concrete
# examples that make the convention actionable.
require_report_literal 'idxd-rust/src/validation.rs' report_validation_path
require_report_literal 'idxd-rust/src/lib.rs' report_sync_session_path
require_report_literal 'idxd-rust/src/async_session.rs' report_async_session_path
require_report_literal 'idxd-rust/src/async_direct.rs' report_async_direct_path
require_report_literal 'hw-eval/src/config.rs' report_hw_eval_config_path
require_report_literal 'hw-eval/src/main.rs' report_hw_eval_main_path
require_report_literal 'idxd-sys' report_idxd_sys_boundary
require_report_literal 'Struct-level `Builder` derive' struct_level_builder_warning
require_report_literal 'Method-level `#[bon::builder]` on the constructor boundary' method_level_builder_rule
require_report_literal 'BenchmarkConfigError' benchmark_config_error_evidence
require_report_literal 'OpenWqSnafu' open_wq_snafu_evidence
require_report_literal 'Direct enum construction' direct_enum_construction_rule

# Source-level markers: these catch drift where the report remains but the
# module-scoped evidence no longer matches the code shape.
require_literal_in_file "${CRATE_DIR}/src/validation.rs" 'pub struct DsaConfig' dsa_config_struct
require_literal_in_file "${CRATE_DIR}/src/validation.rs" '#[bon::bon]' validation_method_bon_block
require_literal_in_file "${CRATE_DIR}/src/validation.rs" '#[builder(finish_fn = build)]' validation_method_builder_boundary
require_literal_in_file "${CRATE_DIR}/src/validation.rs" 'device_path: normalize_device_path(device_path.as_ref())?' validation_builder_normalization_path
require_literal_in_file "${CRATE_DIR}/src/validation.rs" 'fn context(&self) -> MemmoveErrorContext' validation_error_context_helper
reject_regex_in_file "${CRATE_DIR}/src/validation.rs" 'derive\([^)]*Builder' validation_struct_level_builder_derive

require_literal_in_file "${CRATE_DIR}/src/lib.rs" 'DsaSession::builder().open()' sync_session_builder_comment
require_literal_in_file "${CRATE_DIR}/src/lib.rs" '#[builder(start_fn = builder, finish_fn = open)]' sync_session_method_builder_boundary
require_literal_in_file "${CRATE_DIR}/src/lib.rs" 'MemmoveError::QueueOpen' sync_session_queue_open_diagnostics

require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'pub struct AsyncMemmoveRequest' async_owned_request
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'pub struct AsyncMemmoveRequestError' async_rejected_buffers_error
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'pub enum AsyncMemmoveError' async_typed_error_surface
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'pub fn into_parts(self) -> (MemmoveError, Bytes, BytesMut)' async_rejected_buffer_recovery
require_literal_in_file "${CRATE_DIR}/src/async_session.rs" 'AsyncDsaSession::builder().open()' async_session_builder_comment

require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'pub enum AsyncDirectFailure' async_direct_failure_surface
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'CompletionSnapshot' async_direct_completion_snapshot_metadata
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'retry_count' async_direct_retry_metadata
require_literal_in_file "${CRATE_DIR}/src/async_direct.rs" 'retry_budget' async_direct_retry_budget_metadata

require_literal_in_file "${REPO_ROOT}/hw-eval/src/config.rs" 'pub(crate) struct BenchmarkConfig' hw_eval_plain_config_struct
require_literal_in_file "${REPO_ROOT}/hw-eval/src/config.rs" '#[builder(start_fn = builder, finish_fn = build)]' hw_eval_method_builder_boundary
require_literal_in_file "${REPO_ROOT}/hw-eval/src/config.rs" 'pub(crate) fn from_args(args: Args)' hw_eval_from_args_adapter
require_literal_in_file "${REPO_ROOT}/hw-eval/src/config.rs" 'pub(crate) enum BenchmarkConfigError' hw_eval_config_error_enum
require_literal_in_file "${REPO_ROOT}/hw-eval/src/config.rs" 'source: ParseIntError' hw_eval_parse_source_chain
reject_regex_in_file "${REPO_ROOT}/hw-eval/src/config.rs" 'derive\([^)]*Builder' hw_eval_struct_level_builder_derive

require_literal_in_file "${REPO_ROOT}/hw-eval/src/main.rs" 'Config { source: BenchmarkConfigError }' hw_eval_config_snafu_source
require_literal_in_file "${REPO_ROOT}/hw-eval/src/main.rs" 'OpenWq {' hw_eval_open_wq_error_variant
require_literal_in_file "${REPO_ROOT}/hw-eval/src/main.rs" 'fn open_work_queue(config: &BenchmarkConfig)' hw_eval_open_wq_helper
require_literal_in_file "${REPO_ROOT}/hw-eval/src/main.rs" 'OpenWqSnafu' hw_eval_open_wq_snafu_selector
require_literal_in_file "${REPO_ROOT}/hw-eval/src/report.rs" 'crate::HwEvalError::SerializeReport { source }' hw_eval_direct_serialize_error

# `idxd-sys` intentionally stays a raw/std boundary, not a consistency target
# for higher-level bon/snafu usage.
reject_literal_in_tree "${REPO_ROOT}/idxd-sys" 'bon' idxd_sys_bon_dependency_or_usage
reject_literal_in_tree "${REPO_ROOT}/idxd-sys" 'snafu' idxd_sys_snafu_dependency_or_usage

log "verdict=pass report=${REPORT_PATH}"
