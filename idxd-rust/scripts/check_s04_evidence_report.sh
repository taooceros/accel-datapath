#!/usr/bin/env bash
set -euo pipefail

REPORT_PATH=${1:-docs/report/architecture/009.direct_tokio_baseline_evidence.md}

fail() {
  printf '[check_s04_evidence_report] verdict=fail report=%s reason=%s\n' "${REPORT_PATH}" "$*" >&2
  exit 1
}

require_file() {
  [[ -f "${REPORT_PATH}" ]] || fail 'report file is missing'
  [[ -s "${REPORT_PATH}" ]] || fail 'report file is empty'
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
  '## Status and scope' \
  '## Evidence categories' \
  '## Command evidence' \
  '## Direct async versus direct sync interpretation' \
  '## Requirement coverage' \
  '## Ordinary-host rules and non-claims' \
  '## Evidence paths'
do
  require_literal "${heading}"
done

for requirement in R002 R004 R005 R006 R007 R008 R009 R015; do
  require_literal "${requirement}"
done

require_literal 'software_direct_async_diagnostic'
require_literal 'claim_eligible=false'
require_literal 'direct_async'
require_literal 'direct_sync'
require_regex 'expected_failure|verdict=pass'
require_regex 'verdict=pass[[:space:]]+claim_eligible=true|claim_eligible=true'
require_literal 'failure_phase'
require_literal 'launcher_status'
require_literal 'prepared-host'
require_literal 'non-claim'

# R015 permits discussing payload redaction, but the report must not introduce
# obvious source/destination payload dump field names that future evidence could
# cargo-cult into logs or artifacts.
reject_regex '\b(source|src|destination|dst)_payload\b'
reject_regex '\b(payload_bytes|payload_dump|raw_payload|dumped_payload)\b'
reject_regex '\b(source|src|destination|dst)_(bytes|contents|data)_dump\b'

printf '[check_s04_evidence_report] verdict=pass report=%s\n' "${REPORT_PATH}"
