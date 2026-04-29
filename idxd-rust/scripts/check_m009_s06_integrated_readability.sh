#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${M009_S06_INTEGRATED_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/015.hardware_rust_integrated_readability_evidence.md}"

fail() {
  printf '[check_m009_s06_integrated_readability] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[check_m009_s06_integrated_readability] %s\n' "$*"
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
    fail "stale_claim=${description} regex=${regex} file=${REPORT_PATH}"
  fi
}

require_tool rg
require_file "${REPORT_PATH}" integrated_report

owner_files=(
  "${CRATE_DIR}/src/async_session.rs"
  "${CRATE_DIR}/src/async_direct.rs"
  "${CRATE_DIR}/src/async_direct/operation.rs"
  "${CRATE_DIR}/src/async_direct/monitor.rs"
  "${CRATE_DIR}/src/async_direct/test_support.rs"
  "${CRATE_DIR}/src/direct_memmove.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/cli.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/failure.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/hardware.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/modes.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/runner.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/software.rs"
  "${REPO_ROOT}/hw-eval/src/main.rs"
  "${REPO_ROOT}/hw-eval/src/config.rs"
  "${REPO_ROOT}/hw-eval/src/report.rs"
  "${REPO_ROOT}/hw-eval/src/methodology/mod.rs"
  "${REPO_ROOT}/hw-eval/src/methodology/software.rs"
  "${REPO_ROOT}/hw-eval/src/methodology/dsa.rs"
  "${REPO_ROOT}/hw-eval/src/methodology/iax.rs"
  "${REPO_ROOT}/hw-eval/src/dsa.rs"
  "${REPO_ROOT}/hw-eval/src/iax.rs"
  "${REPO_ROOT}/hw-eval/src/submit.rs"
  "${REPO_ROOT}/hw-eval/src/sw.rs"
  "${REPO_ROOT}/idxd-sys/src/lib.rs"
  "${REPO_ROOT}/idxd-sys/src/descriptor.rs"
  "${REPO_ROOT}/idxd-sys/src/portal.rs"
  "${REPO_ROOT}/idxd-sys/src/completion.rs"
  "${REPO_ROOT}/idxd-sys/src/timing.rs"
  "${REPO_ROOT}/idxd-sys/src/topology.rs"
  "${REPO_ROOT}/idxd-sys/src/cache.rs"
  "${SCRIPT_DIR}/check_m009_s01_readability_baseline.sh"
  "${SCRIPT_DIR}/check_m009_s02_direct_async_readability.sh"
  "${SCRIPT_DIR}/check_m009_s03_tokio_benchmark_readability.sh"
  "${REPO_ROOT}/hw-eval/tests/cli_contract.rs"
  "${REPO_ROOT}/idxd-sys/tests/raw_boundary_contract.rs"
  "${REPO_ROOT}/idxd-sys/tests/dsa_descriptor_layout.rs"
)

for path in "${owner_files[@]}"; do
  require_file "${path}" owner_file
done

require_literal '# Hardware Rust integrated readability evidence' 'report_title'
require_literal 'R018' 'requirement_r018'
require_literal 'S06' 'slice_s06'
require_literal 'S02 provides the direct async owner map and guard' 's02_evidence_mapping'
require_literal 'S03 provides the Tokio benchmark/artifact/verifier owner map and guard' 's03_evidence_mapping'
require_literal 'S04 provides the `hw-eval` config/report/methodology/helper split' 's04_evidence_mapping'
require_literal 'S05 provides the `idxd-sys` raw facade/private-module split' 's05_evidence_mapping'
require_literal 'docs/report/architecture/012.hardware_rust_readability_baseline.md' 's01_baseline_report_reference'
require_literal 'docs/report/architecture/013.idxd_rust_direct_async_readability.md' 's02_direct_async_report_reference'
require_literal 'docs/report/architecture/014.idxd_rust_tokio_benchmark_readability.md' 's03_tokio_benchmark_report_reference'
require_literal 'idxd-rust' 'idxd_rust_owner_family'
require_literal 'tokio_memmove_bench' 'tokio_benchmark_owner_family'
require_literal 'hw-eval' 'hw_eval_owner_family'
require_literal 'idxd-sys' 'idxd_sys_owner_family'

require_literal 'idxd-rust/src/async_session.rs' 'direct_async_session_owner'
require_literal 'idxd-rust/src/async_direct.rs' 'direct_async_runtime_owner'
require_literal 'idxd-rust/src/async_direct/operation.rs' 'direct_async_operation_owner'
require_literal 'idxd-rust/src/async_direct/monitor.rs' 'direct_async_monitor_owner'
require_literal 'idxd-rust/src/async_direct/test_support.rs' 'direct_async_test_support_owner'
require_literal 'idxd-rust/src/direct_memmove.rs' 'direct_memmove_owner'
require_literal 'AsyncDsaSession' 'direct_async_public_session'
require_literal 'AsyncDsaHandle' 'direct_async_public_handle'
require_literal 'AsyncMemmoveRequest' 'direct_async_request_surface'
require_literal 'AsyncMemmoveResult' 'direct_async_result_surface'
require_literal 'submit-now / complete-later' 'direct_async_submit_complete_later_boundary'
require_literal 'completion snapshots' 'direct_async_completion_snapshot_boundary'
require_literal 'operation-owned descriptor/completion/source/destination lifetime' 'direct_async_operation_owned_lifetime'
require_literal 'owner_shutdown' 'direct_async_owner_shutdown_contract'
require_literal 'BackpressureExceeded' 'direct_async_backpressure_contract'
require_literal 'MonitorClosed' 'direct_async_monitor_closed_contract'

require_literal 'idxd-rust/src/bin/tokio_memmove_bench.rs' 'tokio_benchmark_entrypoint_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/cli.rs' 'tokio_benchmark_cli_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/artifact.rs' 'tokio_benchmark_artifact_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/failure.rs' 'tokio_benchmark_failure_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/hardware.rs' 'tokio_benchmark_hardware_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/modes.rs' 'tokio_benchmark_modes_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/runner.rs' 'tokio_benchmark_runner_owner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/software.rs' 'tokio_benchmark_software_owner'
require_literal 'schema_version' 'tokio_benchmark_schema_version_contract'
require_literal 'claim_eligible=false' 'tokio_benchmark_software_not_claim_eligible'
require_literal 'software_direct_async_diagnostic' 'tokio_benchmark_software_target'
require_literal 'direct_async' 'tokio_benchmark_direct_async_row'
require_literal 'direct_sync' 'tokio_benchmark_direct_sync_row'
require_literal 'stdout/artifact byte equality' 'tokio_benchmark_stdout_artifact_equality'
require_literal 'forbidden payload fields' 'tokio_benchmark_payload_guard'

require_literal 'hw-eval/src/main.rs' 'hw_eval_main_owner'
require_literal 'hw-eval/src/config.rs' 'hw_eval_config_owner'
require_literal 'hw-eval/src/report.rs' 'hw_eval_report_owner'
require_literal 'hw-eval/src/methodology/mod.rs' 'hw_eval_methodology_namespace'
require_literal 'hw-eval/src/methodology/software.rs' 'hw_eval_software_methodology_owner'
require_literal 'hw-eval/src/methodology/dsa.rs' 'hw_eval_dsa_methodology_owner'
require_literal 'hw-eval/src/methodology/iax.rs' 'hw_eval_iax_methodology_owner'
require_literal 'hw-eval/src/dsa.rs' 'hw_eval_dsa_helper_owner'
require_literal 'hw-eval/src/iax.rs' 'hw_eval_iax_helper_owner'
require_literal 'hw-eval/src/submit.rs' 'hw_eval_submit_owner'
require_literal 'hw-eval/src/sw.rs' 'hw_eval_sw_owner'
require_literal 'BenchmarkConfig' 'hw_eval_config_contract'
require_literal 'metadata' 'hw_eval_metadata_json_field'
require_literal 'latency' 'hw_eval_latency_json_field'
require_literal 'throughput' 'hw_eval_throughput_json_field'
require_literal 'malformed `--sizes`' 'hw_eval_malformed_sizes_contract'
require_literal 'invalid pin-core' 'hw_eval_invalid_pin_contract'
require_literal 'Missing-WQ diagnostics' 'hw_eval_missing_wq_contract'
require_literal 'CAP_SYS_RAWIO' 'hw_eval_capability_hint'
require_literal 'dsa_launcher' 'hw_eval_launcher_hint'
require_literal 'matrix coverage' 'hw_eval_benchmark_matrix_non_change'
require_literal 'hot-loop semantics remain stable' 'hw_eval_hot_loop_non_change'

require_literal 'idxd-sys/src/lib.rs' 'idxd_sys_facade_owner'
require_literal 'idxd-sys/src/descriptor.rs' 'idxd_sys_descriptor_owner'
require_literal 'idxd-sys/src/portal.rs' 'idxd_sys_portal_owner'
require_literal 'idxd-sys/src/completion.rs' 'idxd_sys_completion_owner'
require_literal 'idxd-sys/src/timing.rs' 'idxd_sys_timing_owner'
require_literal 'idxd-sys/src/topology.rs' 'idxd_sys_topology_owner'
require_literal 'idxd-sys/src/cache.rs' 'idxd_sys_cache_owner'
require_literal 'root-level re-exports' 'idxd_sys_root_reexport_contract'
require_literal 'idxd_uapi' 'idxd_sys_uapi_owner'
require_literal 'mmap/munmap lifetime' 'idxd_sys_mmap_lifetime_boundary'
require_literal 'raw MOVDIR64B/ENQCMD submission' 'idxd_sys_raw_submission_boundary'
require_literal 'std::io::Result' 'idxd_sys_raw_io_result_boundary'
require_literal 'volatile status reads' 'idxd_sys_volatile_status_boundary'
require_literal 'raw assembly' 'idxd_sys_raw_assembly_boundary'
require_literal 'does not gain a broad builder' 'idxd_sys_no_broad_builder_non_change'
require_literal 'domain-error layer' 'idxd_sys_no_domain_error_non_change'
require_literal 'hides ABI layout' 'idxd_sys_no_hidden_abi_non_change'

require_literal 'no-payload' 'no_payload_language'
require_literal 'payload bytes, copied source bytes, destination bytes, raw payload dumps, or payload-content examples' 'no_payload_forbidden_examples'
require_literal 'Prepared-host hardware success is not claimed' 'prepared_host_not_claimed_language'
require_literal 'ordinary-host' 'ordinary_host_claim_limit_language'
require_literal 'host-free' 'host_free_guard_language'
require_literal 'Public APIs, public import surfaces, CLI flags, JSON report schemas, benchmark artifact fields' 'intentional_non_change_public_surfaces'
require_literal 'not a refactor plan' 'intentional_non_change_scope'
require_literal 'Fresh verification log' 's06_fresh_verification_log'
require_literal 'S06 shell guard and Cargo wrapper are now implemented and included in the executed matrix above' 's06_guard_execution_evidence'

require_literal 'bash idxd-rust/scripts/check_m009_s01_readability_baseline.sh' 's01_guard_command_reference'
require_literal 'bash idxd-rust/scripts/check_m009_s02_direct_async_readability.sh' 's02_guard_command_reference'
require_literal 'bash idxd-rust/scripts/check_m009_s03_tokio_benchmark_readability.sh' 's03_guard_command_reference'
require_literal 'bash idxd-rust/scripts/check_m009_s06_integrated_readability.sh' 's06_guard_command_reference'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test direct_async_contract_guard --test direct_async_readability_contract --test async_benchmark_cli_contract --test async_benchmark_verifier_contract --test async_benchmark_readability_contract --test integrated_readability_contract -- --nocapture' 'idxd_rust_final_integrated_cargo_command'
require_literal 'cargo test --manifest-path ./Cargo.toml -p hw-eval --test cli_contract --bin hw-eval -- --nocapture' 'hw_eval_final_verification_command'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-sys --test raw_boundary_contract --test dsa_descriptor_layout -- --nocapture' 'idxd_sys_final_verification_command'
require_literal 'direct_async_contract_guard' 'direct_async_contract_surface'
require_literal 'direct_async_readability_contract' 'direct_async_readability_surface'
require_literal 'async_benchmark_cli_contract' 'tokio_benchmark_cli_contract_surface'
require_literal 'async_benchmark_verifier_contract' 'tokio_benchmark_verifier_contract_surface'
require_literal 'async_benchmark_readability_contract' 'tokio_benchmark_readability_surface'
require_literal 'integrated_readability_contract' 'integrated_readability_cargo_surface'
require_literal 'hw-eval/tests/cli_contract.rs' 'hw_eval_cli_contract_surface'
require_literal 'idxd-sys/tests/raw_boundary_contract.rs' 'idxd_sys_raw_boundary_contract_surface'
require_literal 'idxd-sys/tests/dsa_descriptor_layout.rs' 'idxd_sys_layout_contract_surface'

require_regex 'prepared-host hardware success is not claimed|Prepared-host hardware success is not claimed' 'claim_limit_exact_phrase'
require_regex 'no-payload.*(docs|tests|shell guards|CLI diagnostics|verifier diagnostics|JSON/text reports|artifacts)' 'no_payload_scope_phrase'
require_regex 'software diagnostic success.*do not become DSA/IAX hardware performance evidence' 'software_diagnostic_claim_limit'
require_regex 'If they need real DSA/IAX success or performance evidence, they must use the prepared-host launcher/verifier path' 'prepared_host_launcher_requirement'

reject_regex 'prepared-host hardware success is claimed' 'prepared_host_success_claimed'
reject_regex 'ordinary-host.*(proves|claims).*DSA/IAX (throughput|latency|performance|hardware success)' 'ordinary_host_overclaims_hardware'
reject_regex 'software diagnostic success (is|proves|claims) hardware (success|performance)' 'software_diagnostic_overclaims_hardware'
reject_regex 'permission to include payload bytes (is|was) (granted|allowed)|may include payload bytes|can include payload bytes' 'payload_permission_claim'
reject_regex 'should add .*safe wrapper that hides|safe wrapper that hides .* is required|safe wrapper should hide|should hide raw unsafe semantics' 'hidden_raw_semantics_claim'
reject_regex 'blocking fallback is now (allowed|required|the default)|Direct async code adds a blocking fallback|introduce a blocking fallback for direct async' 'direct_async_blocking_fallback_claim'
reject_regex 'new public API is introduced for callers|new library API is introduced for callers|introduces a new public API' 'new_public_api_claim'

log "verdict=pass report=${REPORT_PATH}"
