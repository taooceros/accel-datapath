#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
REPORT_PATH="${M009_S03_TOKIO_BENCHMARK_REPORT_PATH:-${REPO_ROOT}/docs/report/architecture/014.idxd_rust_tokio_benchmark_readability.md}"

fail() {
  printf '[check_m009_s03_tokio_benchmark_readability] verdict=fail %s\n' "$*" >&2
  exit 1
}

log() {
  printf '[check_m009_s03_tokio_benchmark_readability] %s\n' "$*"
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
  "${REPO_ROOT}/docs/report/architecture/013.idxd_rust_direct_async_readability.md"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/cli.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/failure.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/hardware.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/modes.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/runner.rs"
  "${CRATE_DIR}/src/bin/tokio_memmove_bench/software.rs"
  "${CRATE_DIR}/tests/async_benchmark_cli_contract.rs"
  "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs"
  "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh"
)

for path in "${source_paths[@]}"; do
  require_file "${path}" source_input
done

require_literal '# idxd-rust Tokio benchmark readability map' 'report_title'
require_literal 'R018' 'requirement_r018'
require_literal 'S03' 'slice_s03'
require_literal 'ordinary-host' 'ordinary_host_limit'
require_literal 'prepared-host' 'prepared_host_limit'
require_literal 'host-free' 'host_free_guard_scope'
require_literal 'docs/report/architecture/012.hardware_rust_readability_baseline.md' 's01_baseline_input'
require_literal 'docs/report/architecture/013.idxd_rust_direct_async_readability.md' 's02_direct_async_input'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench.rs' 'current_benchmark_input'
require_literal 'idxd-rust/tests/async_benchmark_cli_contract.rs' 'cli_contract_input'
require_literal 'idxd-rust/tests/async_benchmark_verifier_contract.rs' 'verifier_contract_input'
require_literal 'idxd-rust/scripts/verify_tokio_memmove_bench.sh' 'verifier_script_input'

require_literal 'AsyncDsaSession' 'direct_async_session_consumed'
require_literal 'AsyncDsaHandle' 'direct_async_handle_consumed'
require_literal 'AsyncMemmoveRequest' 'direct_async_request_consumed'
require_literal 'direct failure accessors' 'direct_failure_accessors_preserved'
require_literal 'must not alter those runtime contracts' 'runtime_contract_non_change'

require_literal 'idxd-rust/src/bin/tokio_memmove_bench/cli.rs' 'planned_owner_cli'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/suite.rs' 'planned_owner_suite'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/software.rs' 'planned_owner_software'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/hardware.rs' 'planned_owner_hardware'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/modes.rs' 'planned_owner_modes'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/failure.rs' 'planned_owner_failure'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/runner.rs' 'planned_owner_runner'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench/artifact.rs' 'planned_owner_artifact'
require_literal 'idxd-rust/src/bin/tokio_memmove_bench.rs' 'planned_owner_entrypoint'
require_literal 'idxd-rust/scripts/verify_tokio_memmove_bench.sh' 'planned_owner_verifier'

require_literal 'CLI/config owner' 'section_cli_config'
require_literal 'Suite/mode dispatch owner' 'section_suite_mode_dispatch'
require_literal 'Software diagnostic backend' 'section_software_diagnostic_backend'
require_literal 'Hardware execution and claim gating owner' 'section_hardware_claim_gating'
require_literal 'Artifact schema/rendering/writing owner' 'section_artifact_schema_render_write'
require_literal 'Verifier semantics owner' 'section_verifier_semantics'
require_literal 'No-payload constraints' 'section_no_payload_constraints'
require_literal 'Ordinary-host evidence' 'section_ordinary_host_evidence'
require_literal 'Prepared-host claim boundaries' 'section_prepared_host_claim_boundaries'

require_literal 'schema_version' 'schema_version_field'
require_literal 'serialized field names are stable unless an explicit tested exception is introduced' 'serialized_field_stability'
require_literal 'schema version and serialized field names are stable unless an explicit tested exception is introduced' 'schema_and_serialized_field_stability'
require_literal 'claim_eligible=false' 'software_non_claim_eligible_literal'
require_literal 'software_direct_async_diagnostic' 'software_diagnostic_target'
require_literal 'direct_async' 'hardware_async_target'
require_literal 'direct_sync' 'hardware_sync_target'
require_literal 'Prepared-host hardware claims require' 'prepared_host_claim_criteria'
require_literal 'top-level `verdict=pass`' 'prepared_host_verdict_pass'
require_literal 'top-level `ok=true`' 'prepared_host_ok_true'
require_literal 'top-level `claim_eligible=true`' 'prepared_host_claim_eligible_true'
require_literal 'stdout/artifact equality' 'stdout_artifact_equality'
require_literal 'byte-for-byte' 'stdout_artifact_byte_for_byte'
require_literal 'Expected failures remain expected failures' 'expected_failure_classification'
require_literal 'phase=done' 'verifier_done_phase'
require_literal 'phase=artifact_validation' 'verifier_artifact_validation_phase'
require_literal 'malformed JSON' 'malformed_json_hard_failure'
require_literal 'missing required schema fields' 'missing_schema_fields_hard_failure'
require_literal 'stdout/artifact mismatch' 'stdout_artifact_mismatch_hard_failure'
require_literal 'software artifact or software row with `claim_eligible=true`' 'software_claim_contradiction_hard_failure'
require_literal 'hardware success missing the required `direct_sync` comparison row' 'missing_sync_comparison_hard_failure'
require_literal 'forbidden payload dump fields' 'forbidden_payload_fields_hard_failure'
require_literal 'copied source bytes' 'no_source_bytes'
require_literal 'destination bytes' 'no_destination_bytes'
require_literal 'raw payload dumps' 'no_raw_payload_dumps'
require_literal 'payload-content fields' 'no_payload_content_fields'
require_literal 'does not claim prepared-host success' 'no_prepared_host_claim_from_report'

require_literal 'bash idxd-rust/scripts/check_m009_s03_tokio_benchmark_readability.sh' 'guard_command'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test async_benchmark_readability_contract -- --nocapture' 'cargo_guard_command'
require_literal 'cargo test --manifest-path ./Cargo.toml -p idxd-rust --test async_benchmark_cli_contract --test async_benchmark_verifier_contract -- --nocapture' 'existing_contract_command'

require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'const SCHEMA_VERSION: u32 = 1;' 'source_schema_version_constant'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/cli.rs" 'struct CliArgs' 'source_cli_args_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/cli.rs" 'enum Suite' 'source_suite_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/software.rs" 'struct SoftwareDirectBackend' 'source_software_backend_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/software.rs" 'fn initialize_success_destination' 'source_software_success_copy_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/software.rs" 'claim_eligible: false' 'source_software_non_claim_eligible_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/hardware.rs" 'async fn hardware_artifact' 'source_hardware_artifact_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/hardware.rs" 'fn run_sync_comparison' 'source_sync_comparison_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/hardware.rs" 'claim_eligible: first_failure.is_none()' 'source_hardware_claim_gating_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/modes.rs" 'async fn run_async_mode' 'source_async_mode_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/modes.rs" 'JoinSet' 'source_joinset_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/modes.rs" 'fn deterministic_source' 'source_request_data_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/failure.rs" 'struct RowFailure' 'source_row_failure_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/failure.rs" 'fn async_error' 'source_async_failure_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/failure.rs" 'fn sync_error' 'source_sync_failure_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/runner.rs" 'async fn execute' 'source_runner_dispatch_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'struct BenchmarkArtifact' 'source_artifact_struct_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'fn emit_artifact' 'source_emit_artifact_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'fn write_artifact' 'source_write_artifact_owner'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'SOFTWARE_TARGET: &str = "software_direct_async_diagnostic"' 'source_software_target_constant'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'HARDWARE_ASYNC_TARGET: &str = "direct_async"' 'source_hardware_async_target_constant'
require_literal_in_file "${CRATE_DIR}/src/bin/tokio_memmove_bench/artifact.rs" 'HARDWARE_SYNC_TARGET: &str = "direct_sync"' 'source_hardware_sync_target_constant'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_cli_contract.rs" 'writes_artifact_matching_stdout_exactly' 'cli_contract_stdout_artifact_equality'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_cli_contract.rs" 'assert_no_payload_dump_fields' 'cli_contract_no_payload'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs" 'malformed_json_is_hard_artifact_validation_failure' 'verifier_contract_malformed_json'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs" 'stdout_artifact_mismatch_is_hard_artifact_validation_failure' 'verifier_contract_stdout_artifact_mismatch'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs" 'software_claim_eligible_contradiction_is_hard_failure' 'verifier_contract_software_claim_contradiction'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs" 'hardware_success_missing_sync_comparison_is_hard_failure' 'verifier_contract_missing_sync_comparison'
require_literal_in_file "${CRATE_DIR}/tests/async_benchmark_verifier_contract.rs" 'benchmark_verifier_rejects_payload_dump_fields_in_result_rows' 'verifier_contract_payload_rejection'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'log_phase done' 'verifier_script_done_phase'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'fail_phase artifact_validation' 'verifier_script_artifact_validation_phase'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'stdout and artifact diverged' 'verifier_script_stdout_artifact_mismatch'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'software artifacts must not be claim eligible' 'verifier_script_software_claim_guard'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'hardware success missing direct sync comparison row' 'verifier_script_sync_comparison_guard'
require_literal_in_file "${CRATE_DIR}/scripts/verify_tokio_memmove_bench.sh" 'artifact contains forbidden payload dump field' 'verifier_script_payload_guard'

require_regex 'schema[_ -]?version.*stable|stable.*schema[_ -]?version' 'schema_stability_phrase'
require_regex 'serialized field names.*stable|stable.*serialized field names' 'serialized_field_stability_phrase'
require_regex 'software.*claim_eligible=false|claim_eligible=false.*software' 'software_claim_eligible_false_phrase'
require_regex 'stdout/artifact equality|artifact file content must match stdout' 'stdout_artifact_equality_phrase'
require_regex 'expected[- ]failure classification|Expected failures remain expected failures' 'expected_failure_phrase'
require_regex 'no-payload|copied source bytes|raw payload dumps' 'no_payload_phrase'

reject_regex 'software diagnostic (is|counts as|proves) (prepared-host )?(hardware|DSA|IAX) (success|proof|claim)' 'software_as_hardware_claim'
reject_regex 'ordinary-host.*(prepared-host hardware success|real DSA/IAX completion progress|performance behavior).*may claim|may claim.*ordinary-host.*prepared-host hardware success' 'ordinary_host_prepared_host_claim'
reject_regex 'schema_version (may|can|should) (change|be renamed|be removed) without.*test|serialized field names (may|can|should) (change|be renamed|be removed) without.*test' 'untested_schema_drift_allowed'
reject_regex 'stdout/artifact (equality|byte-for-byte).*not required|artifact.*need not match stdout' 'stdout_artifact_equality_weakened'
reject_regex 'software.*claim_eligible=true.*allowed|claim_eligible=true.*software.*allowed' 'software_claim_eligible_allowed'
reject_regex 'payload bytes? (are|is) logged|source bytes? (are|is) logged|destination bytes? (are|is) logged|payload-content fields? (are|is) allowed' 'payload_logging_allowed'

log "verdict=pass report=${REPORT_PATH}"
