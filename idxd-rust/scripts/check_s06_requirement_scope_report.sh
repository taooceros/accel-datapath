#!/usr/bin/env bash
set -euo pipefail

REPORT_PATH=${1:-docs/report/architecture/011.m007_requirement_scope_remediation.md}

fail() {
  printf '[check_s06_requirement_scope_report] verdict=fail report=%s reason=%s\n' "${REPORT_PATH}" "$*" >&2
  exit 1
}

require_file() {
  [[ -f "${REPORT_PATH}" ]] || fail 'requirement-scope report file is missing'
  [[ -s "${REPORT_PATH}" ]] || fail 'requirement-scope report file is empty'
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
  '## Active M007 scope' \
  '## Deferred requirements' \
  '## Preserved guard' \
  '## Label drift errata' \
  '## Validation rule' \
  '## Proof refresh'
do
  require_literal "${heading}"
done

for classification in \
  'PASS active-scope coverage' \
  'WARN deferred' \
  'PRESERVED GUARD' \
  'LABEL DRIFT'
do
  require_literal "${classification}"
done

for active_requirement in R002 R003 R004 R005 R006 R007 R008 R009; do
  require_regex "\\|[[:space:]]*${active_requirement}[[:space:]]*\\|[[:space:]]*\`?PASS active-scope coverage\`?[[:space:]]*\\|"
done

for deferred_requirement in R010 R011 R012 R013 R014; do
  require_regex "\\|[[:space:]]*${deferred_requirement}[[:space:]]*\\|[[:space:]]*\`?WARN deferred\`?[[:space:]]*\\|"
done

require_regex '\|[[:space:]]*R015[[:space:]]*\|[[:space:]]*`?PRESERVED GUARD`?[[:space:]]*\|'
require_literal 'no-payload'
require_literal 'redaction guard'
require_literal 'copied caller data'

require_literal 'prepared-host'
require_literal 'expected_failure'
require_literal 'claim_eligible=false'
require_regex 'verdict=pass[[:space:]]+claim_eligible=true|claim_eligible=true'

require_literal 'direct Tokio'
require_literal 'direct baseline'
require_literal 'future scope'
require_literal 'not active feature coverage'

# R015 permits discussing payload redaction, but the requirement-scope report must
# not introduce source/destination payload dump labels that future evidence could
# cargo-cult into logs or artifacts.
reject_regex '\b(source|src|destination|dst)_payload\b'
reject_regex '\b(payload_bytes|payload_dump|raw_payload|dumped_payload)\b'
reject_regex '\b(source|src|destination|dst)_(bytes|contents|data)_dump\b'

printf '[check_s06_requirement_scope_report] verdict=pass report=%s\n' "${REPORT_PATH}"
