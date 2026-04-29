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
  '## hw-eval S06 inventory'
  '## idxd-sys S05 inventory'
  '## Downstream slice contract baselines'
  '## Non-change boundaries'
  '## Verification matrix'
  '## R018 coverage'
  '## Ordinary-host and prepared-host limits'
  '## Fresh S01 verification evidence'
)

for heading in "${required_headings[@]}"; do
  require_heading "${heading}"
done

for slice in S02 S03 S04 S05 S06; do
  require_literal "${slice}" "downstream_${slice}_baseline"
done

require_literal '| S02 | `idxd-rust/src/async_session.rs`' 'downstream_S02_row_sources'
require_literal 'owner/session wiring, request validation/recovery, direct registration/submission' 'downstream_S02_readability_seams'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test direct_async_contract_guard -- --nocapture' 'downstream_S02_verification_command'
require_literal '| S03 | `idxd-rust/src/bin/tokio_memmove_bench.rs`' 'downstream_S03_row_sources'
require_literal 'benchmark CLI/config, backend/suite/mode dispatch' 'downstream_S03_readability_seams'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test async_benchmark_cli_contract --test async_benchmark_verifier_contract -- --nocapture' 'downstream_S03_verification_command'
require_literal '| S04 | `idxd-rust` proof-binary and verifier surfaces' 'downstream_S04_row_sources'
require_literal 'proof/verifier readability for `idxd-rust` artifacts' 'downstream_S04_readability_seams'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test async_benchmark_verifier_contract -- --nocapture' 'downstream_S04_verification_command'
require_literal '| S05 | `idxd-sys/src/lib.rs`' 'downstream_S05_row_sources'
require_literal 'raw `std::io::Result` boundaries without hiding hardware semantics' 'downstream_S05_readability_seams'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-sys --test raw_boundary_contract --test dsa_descriptor_layout -- --nocapture' 'downstream_S05_verification_command'
require_literal '| S06 | `hw-eval/src/main.rs`' 'downstream_S06_row_sources'
require_literal 'WQ submission/timing/topology helpers, and benchmark ordering' 'downstream_S06_readability_seams'
require_literal 'cargo test --manifest-path ./Cargo.toml -p hw-eval --test cli_contract --bin hw-eval -- --nocapture' 'downstream_S06_verification_command'
require_literal 'Fresh ordinary-host evidence was recorded by T06' 'fresh_s01_evidence_section'

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
require_literal 'hw-eval/src/main.rs' 'hw_eval_main_source_file'
require_literal 'hw-eval/src/dsa.rs' 'hw_eval_dsa_source_file'
require_literal 'hw-eval/src/iax.rs' 'hw_eval_iax_source_file'
require_literal 'hw-eval/src/submit.rs' 'hw_eval_submit_source_file'
require_literal 'hw-eval/src/sw.rs' 'hw_eval_sw_source_file'
require_literal 'hw-eval/tests/cli_contract.rs' 'hw_eval_cli_contract_test'
require_literal 'cli_contract' 'hw_eval_cli_contract_name'
require_literal 'metadata' 'hw_eval_metadata_json_field'
require_literal 'latency' 'hw_eval_latency_json_field'
require_literal 'throughput' 'hw_eval_throughput_json_field'
require_literal 'malformed `--sizes`' 'hw_eval_malformed_sizes_negative_test'
require_literal 'invalid pin-core' 'hw_eval_invalid_pin_core_negative_test'
require_literal 'missing hardware WQ' 'hw_eval_missing_wq_negative_test'
require_literal 'open_wq' 'hw_eval_open_wq_diagnostic'
require_literal 'CAP_SYS_RAWIO' 'hw_eval_cap_sys_rawio_hint'
require_literal 'dsa_launcher' 'hw_eval_launcher_hint'
require_literal 'software-only JSON' 'hw_eval_software_only_json_contract'
require_literal 'JSON report schema is a non-change boundary' 'hw_eval_json_schema_non_change_boundary'
require_literal 'benchmark matrix/hot-loop semantics' 'hw_eval_hot_loop_non_redesign_boundary'
require_literal 'No-payload for hw-eval' 'hw_eval_no_payload_constraint'
require_literal 'idxd-sys/src/lib.rs' 'idxd_sys_source_file'
require_literal 'idxd-sys/src/descriptor.rs' 'idxd_sys_descriptor_module_file'
require_literal 'idxd-sys/src/portal.rs' 'idxd_sys_portal_module_file'
require_literal 'idxd-sys/src/completion.rs' 'idxd_sys_completion_module_file'
require_literal 'idxd-sys/src/timing.rs' 'idxd_sys_timing_module_file'
require_literal 'idxd-sys/src/topology.rs' 'idxd_sys_topology_module_file'
require_literal 'idxd-sys/src/cache.rs' 'idxd_sys_cache_module_file'
require_literal 'facade/UAPI' 'idxd_sys_final_facade_uapi_responsibility'
require_literal 'descriptor/completion ABI wrappers and constants' 'idxd_sys_final_descriptor_completion_responsibility'
require_literal 'portal submission primitives' 'idxd_sys_final_portal_responsibility'
require_literal 'completion polling/reset/drain/fault helpers' 'idxd_sys_final_completion_responsibility'
require_literal 'timing/cache helpers' 'idxd_sys_final_timing_cache_responsibility'
require_literal 'topology helpers' 'idxd_sys_final_topology_responsibility'
require_literal 'root-level re-exports' 'idxd_sys_root_reexport_contract'
require_literal 'mmap/munmap lifetime' 'idxd_sys_mmap_munmap_lifetime_boundary'
require_literal 'raw assembly' 'idxd_sys_raw_assembly_boundary'
require_literal 'caller-owned context' 'idxd_sys_caller_owned_context_boundary'
require_literal 'idxd-sys/tests/dsa_descriptor_layout.rs' 'idxd_sys_layout_contract_test'
require_literal 'idxd-sys/tests/raw_boundary_contract.rs' 'idxd_sys_raw_boundary_contract_test'
require_literal 'idxd_uapi' 'idxd_sys_bindgen_uapi_owner'
require_literal 'linux/idxd.h' 'idxd_sys_kernel_uapi_source'
require_literal 'BindgenDsaHwDesc' 'idxd_sys_descriptor_wrapper'
require_literal 'DsaCompletionRecord' 'idxd_sys_completion_wrapper'
require_literal 'dsa_descriptor_layout' 'idxd_sys_layout_test_name'
require_literal 'raw_boundary_contract' 'idxd_sys_raw_boundary_test_name'
require_literal 'descriptor alignment/layout' 'idxd_sys_descriptor_alignment_negative_test'
require_literal 'helper-field writes' 'idxd_sys_helper_field_writes_negative_test'
require_literal 'missing WQ `std::io::ErrorKind` preservation' 'idxd_sys_missing_wq_error_kind_negative_test'
require_literal 'std::io::ErrorKind::NotFound' 'idxd_sys_missing_wq_not_found_contract'
require_literal 'raw ENQCMD accepted/rejected typing' 'idxd_sys_enqcmd_negative_test'
require_literal 'EnqcmdSubmission::Accepted' 'idxd_sys_enqcmd_accepted_contract'
require_literal 'EnqcmdSubmission::Rejected' 'idxd_sys_enqcmd_rejected_contract'
require_literal 'submit_movdir64b' 'idxd_sys_movdir64b_submission'
require_literal 'submit_enqcmd_once' 'idxd_sys_enqcmd_once_submission'
require_literal 'poll_completion' 'idxd_sys_completion_polling'
require_literal 'reset_completion' 'idxd_sys_completion_reset'
require_literal 'drain_completions' 'idxd_sys_completion_drain'
require_literal 'touch_fault_page' 'idxd_sys_fault_touch'
require_literal 'rdtscp' 'idxd_sys_timing_rdtscp'
require_literal 'lfence' 'idxd_sys_timing_lfence'
require_literal 'tsc_frequency_hz' 'idxd_sys_tsc_frequency'
require_literal 'flush_range' 'idxd_sys_cache_flush'
require_literal 'pin_to_core' 'idxd_sys_pin_to_core'
require_literal 'cpu_numa_node' 'idxd_sys_cpu_topology'
require_literal 'device_numa_node' 'idxd_sys_device_topology'
require_literal 'unsafe submission preconditions' 'idxd_sys_unsafe_submission_boundary'
require_literal 'volatile completion-status reads' 'idxd_sys_volatile_completion_boundary'
require_literal 'unaligned packed-field access' 'idxd_sys_unaligned_access_boundary'
require_literal 'raw/std ABI boundary' 'idxd_sys_raw_std_boundary'
require_literal 'context belongs at callers' 'idxd_sys_caller_context_guardrail'
require_literal 'Do not force `bon` builders' 'idxd_sys_no_forced_bon_builder_language'
require_literal 'broad `snafu` error enum' 'idxd_sys_no_broad_snafu_language'
require_literal 'must not recommend hiding unsafe/volatile semantics' 'idxd_sys_no_vague_wrapper_contract'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-sys --test raw_boundary_contract --test dsa_descriptor_layout -- --nocapture' 'idxd_sys_combined_verification_command'

reject_literal 'source_bytes' 'payload_field_source_bytes'
reject_literal 'destination_bytes' 'payload_field_destination_bytes'
reject_literal 'payload_dump' 'payload_dump_field'
reject_literal 'idxd-sys should use bon' 'forced_idxd_sys_bon_builder'
reject_literal 'idxd-sys should use snafu' 'forced_idxd_sys_snafu_error_layer'
reject_literal 'hide unsafe semantics' 'hidden_idxd_sys_unsafe_semantics'
reject_literal 'hide volatile semantics' 'hidden_idxd_sys_volatile_semantics'
reject_literal 'safe wrapper around ENQCMD' 'vague_idxd_sys_safe_enqcmd_wrapper'

log "verdict=pass report=${REPORT_PATH}"
