#!/usr/bin/env bash
set -euo pipefail

REPORT_PATH=${1:-docs/report/architecture/010.worker_runtime_readiness_handoff.md}

fail() {
  printf '[check_s05_worker_readiness_report] verdict=fail report=%s reason=%s\n' "${REPORT_PATH}" "$*" >&2
  exit 1
}

require_file() {
  [[ -f "${REPORT_PATH}" ]] || fail 'readiness report file is missing'
  [[ -s "${REPORT_PATH}" ]] || fail 'readiness report file is empty'
}

require_literal() {
  local needle=$1
  grep -Fq -- "${needle}" "${REPORT_PATH}" || fail "missing required text: ${needle}"
}

require_regex() {
  local pattern=$1
  grep -Eq -- "${pattern}" "${REPORT_PATH}" || fail "missing required pattern: ${pattern}"
}

reject_regex() {
  local pattern=$1
  if grep -Eiq -- "${pattern}" "${REPORT_PATH}"; then
    fail "forbidden payload-dump label matched: ${pattern}"
  fi
}

require_file

for heading in \
  '## Reader and action' \
  '## Readiness classification' \
  '## Proven direct-baseline evidence' \
  '## Non-proven boundary matrix' \
  '## Execution gates for the next worker-runtime milestone' \
  '## Next milestone bridge' \
  '## Failure and redaction rules' \
  '## Requirement preservation checklist' \
  '## Reader-test result'
do
  require_literal "${heading}"
done

for readiness_term in \
  'planning readiness' \
  'execution readiness' \
  'claim readiness' \
  'prepared-host' \
  'claim_eligible=true' \
  'expected_failure' \
  'failure_phase' \
  'launcher_status' \
  'direct_async' \
  'direct_sync' \
  'software_direct_async_diagnostic'
do
  require_literal "${readiness_term}"
done

for boundary_term in \
  'worker batching' \
  'MOVDIR64' \
  'preallocated' \
  'registry' \
  'pool' \
  'Tonic' \
  'RPC' \
  'R006' \
  'R008' \
  'R009'
do
  require_literal "${boundary_term}"
done

# R008 permits discussing payload redaction, but the readiness handoff must not
# introduce field labels that future worker-runtime evidence could cargo-cult
# into payload dumps or caller-buffer contents.
reject_regex '\b(source|src|destination|dst)_payload\b'
reject_regex '\b(payload_bytes|payload_dump|raw_payload|dumped_payload)\b'
reject_regex '\b(source|src|destination|dst)_(bytes|contents|data)_dump\b'

# The report is a documentation contract only. Benchmark JSON parsing and
# hardware claim eligibility remain owned by verify_tokio_memmove_bench.sh and
# the S04 evidence guard.
require_regex 'verdict=pass[[:space:]]+claim_eligible=true|claim_eligible=true'
require_regex 'verdict=expected_failure|expected_failure'

printf '[check_s05_worker_readiness_report] verdict=pass report=%s\n' "${REPORT_PATH}"
