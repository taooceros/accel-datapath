#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)

if [[ $# -gt 1 ]]; then
  printf '[check_m011_s05_elegance_audit] verdict=fail usage="%s [report-path]"\n' "$0" >&2
  exit 1
fi

REPORT_PATH=${1:-"${REPO_ROOT}/docs/report/architecture/017.generic_idxd_elegance_audit.md"}

if ! command -v python3 >/dev/null 2>&1; then
  printf '[check_m011_s05_elegance_audit] verdict=fail missing_tool=python3\n' >&2
  exit 1
fi

python3 - <<'PY' "${REPORT_PATH}" "${REPO_ROOT}" "${CRATE_DIR}"
import re
import sys
from pathlib import Path

PREFIX = "[check_m011_s05_elegance_audit]"

report_path = Path(sys.argv[1])
repo_root = Path(sys.argv[2])
crate_dir = Path(sys.argv[3])


def fail(message: str) -> None:
    raise SystemExit(f"{PREFIX} verdict=fail {message}")


def read_required_file(path: Path, description: str) -> str:
    if not path.is_file():
        fail(f"missing {description} path={path}")
    if path.stat().st_size == 0:
        fail(f"empty {description} path={path}")
    return path.read_text(encoding="utf-8")


def require_snippets(text: str, snippets: list[str], description: str) -> None:
    missing = [snippet for snippet in snippets if snippet not in text]
    if missing:
        fail(f"{description} missing required terms: " + ", ".join(missing))


def reject_snippets(text: str, snippets: list[str], description: str) -> None:
    found = [snippet for snippet in snippets if snippet in text]
    if found:
        fail(f"{description} contains forbidden terms: " + ", ".join(found))


def reject_regexes(text: str, patterns: list[tuple[str, str]], description: str) -> None:
    for label, pattern in patterns:
        if re.search(pattern, text, re.IGNORECASE | re.MULTILINE):
            fail(f"{description} contains stale overclaim {label}: regex={pattern}")


def require_count(text: str, snippet: str, minimum: int, description: str) -> None:
    count = text.count(snippet)
    if count < minimum:
        fail(f"{description} expected at least {minimum} occurrences of {snippet!r}, found {count}")


report = read_required_file(report_path, "report")

required_headings = [
    "## Purpose and R024 claim boundary",
    "## Handoff verdict",
    "## Source inputs",
    "## Core architecture audit",
    "## Duplication audit",
    "## Simplicity and scope audit",
    "## Diagnostics and no-payload audit",
    "## Known compromises",
    "## Deferred work handoff",
    "## Verification matrix",
    "## Reader-test result",
]
require_snippets(report, required_headings, "report")

required_report_snippets = [
    "M011",
    "S05",
    "R024",
    "IdxdSession<Accel>",
    "IdxdSession<Dsa>",
    "IdxdSession<Iax>",
    "Dsa",
    "Iax",
    "Iaa",
    "run_blocking_operation",
    "BlockingOperation",
    "WqPortal::submit_desc64",
    "docs/report/hw_eval/011.m011_s03_representative_ops_2026-04-30.md",
    "docs/report/benchmarking/015.m011_representative_idxd_numbers_2026-04-30.md",
    "live_idxd_op",
    "idxd_representative_bench",
    "verify_idxd_representative_ops.sh",
    "verify_idxd_representative_bench.sh",
    "Handoff verdict: pass, with named first-version compromises.",
    "Duplication audit",
    "Known compromises",
    "Deferred work",
    "raw buffer bytes",
    "source bytes",
    "destination bytes",
    "payload dumps",
    "descriptors",
    "tokens",
    "secrets",
    "consumes the API by opening `IdxdSession::<Dsa>` and `IdxdSession::<Iax>`",
    "does not define the API",
    "does not bypass the generic session boundary",
    "not a benchmark matrix",
    "not a performance characterization",
    "not a final performance comparison",
    "DsaConfig` per call",
    "zero-residue scalar",
    "diagnostic surface is intentionally large",
    "non-core duplication",
    "**third** proof or verifier",
    "extracted only",
    "no full DSA surface",
    "no full IAX surface",
    "no full IAA surface",
    "no AECS-heavy flows",
    "no pooling/registry",
    "no scheduler",
    "no worker batching",
    "no MOVDIR64/MOVDIR64B strategy framework",
    "no benchmark matrix/framework",
    "no RPC/Tonic integration",
    "no public operation trait",
    "no runtime accelerator dispatcher",
]
require_snippets(report, required_report_snippets, "report")

stale_overclaim_patterns = [
    ("full_dsa_surface", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bfull\s+DSA\s+surface\b"),
    ("full_iax_surface", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bfull\s+IAX\s+surface\b"),
    ("full_iaa_surface", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bfull\s+IAA\s+surface\b"),
    ("production_scheduler", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bproduction[- ]?(grade\s+)?scheduler\b"),
    ("benchmark_matrix", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bbenchmark\s+(matrix|framework)\b"),
    ("software_only_closure", r"\bsoftware[- ]only\s+closure\b"),
    ("payload_examples", r"\b(payload\s+byte\s+examples|payload\s+examples\s+(are|were|is|was)\s+(included|logged|shown|allowed))\b"),
    ("raw_buffer_bytes_logged", r"\braw\s+buffer\s+bytes\b[^.\n]*(are|were|is|was)\s+(logged|included|dumped|shown|printed|emitted)\b"),
    ("source_bytes_logged", r"\bsource\s+bytes\b[^.\n]*(are|were|is|was)\s+(logged|included|dumped|shown|printed|emitted)\b"),
    ("destination_bytes_logged", r"\bdestination\s+bytes\b[^.\n]*(are|were|is|was)\s+(logged|included|dumped|shown|printed|emitted)\b"),
    ("payload_dumps_allowed", r"\bpayload\s+dumps\b[^.\n]*(are|were|is|was)\s+(allowed|included|logged|emitted)\b"),
    ("pooling_registry", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bpooling/registry\b"),
    ("worker_batching", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bworker\s+batching\b"),
    ("movdir_strategy", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bMOVDIR64/MOVDIR64B\s+strategy\s+framework\b"),
    ("rpc_tonic", r"\bM011\s+(adds|added|delivers|delivered|implements|implemented|provides|provided|ships|shipped)\b[^.\n]*\bRPC/Tonic\s+integration\b"),
]
reject_regexes(report, stale_overclaim_patterns, "report")

source_paths = {
    "lifecycle": crate_dir / "src/lifecycle.rs",
    "direct_memmove": crate_dir / "src/direct_memmove.rs",
    "iax_crc64": crate_dir / "src/iax_crc64.rs",
    "session": crate_dir / "src/session.rs",
    "portal": repo_root / "idxd-sys/src/portal.rs",
    "live_idxd_op": crate_dir / "src/bin/live_idxd_op.rs",
    "idxd_representative_bench": crate_dir / "src/bin/idxd_representative_bench.rs",
}
sources = {name: read_required_file(path, f"source_{name}") for name, path in source_paths.items()}

require_snippets(
    sources["lifecycle"],
    [
        "pub(crate) enum BlockingOperationDecision",
        "pub(crate) trait BlockingOperation",
        "pub(crate) fn run_blocking_operation",
        "O: BlockingOperation",
        "operation.reset_and_fill_descriptor()",
        "operation.submit(portal)",
        "operation.observe_completion()",
        "operation.classify_completion(completion)?",
    ],
    "source lifecycle",
)
reject_snippets(
    sources["lifecycle"],
    [
        "pub enum BlockingOperationDecision",
        "pub trait BlockingOperation",
        "pub fn run_blocking_operation",
        "dyn BlockingOperation",
        "Box<dyn",
    ],
    "source lifecycle",
)

require_snippets(
    sources["direct_memmove"],
    [
        "use crate::lifecycle::{BlockingOperation, BlockingOperationDecision, run_blocking_operation};",
        "pub(crate) struct DirectMemmoveOperation",
        "impl BlockingOperation for DirectMemmoveOperation<'_>",
        "portal.submit(self.state.descriptor())",
        "run_blocking_operation(portal, &mut operation)",
        "verify_initialized_destination",
    ],
    "source direct_memmove",
)

require_snippets(
    sources["iax_crc64"],
    [
        "use crate::lifecycle::{BlockingOperation, BlockingOperationDecision, run_blocking_operation};",
        "pub(crate) struct IaxCrc64State",
        "impl BlockingOperation for IaxCrc64State<'_>",
        "portal.submit_iax(&self.desc)",
        "run_blocking_operation(portal, &mut operation)",
        "pub(crate) fn run_iax_crc64",
    ],
    "source iax_crc64",
)

require_snippets(
    sources["session"],
    [
        "mod sealed",
        "pub trait Accelerator: sealed::Sealed",
        "pub struct Dsa",
        "pub struct Iax",
        "pub type Iaa = Iax",
        "pub struct IdxdSession<Accel: Accelerator>",
        "impl IdxdSession<Dsa>",
        "impl IdxdSession<Iax>",
        "run_direct_memmove(",
        "run_iax_crc64(&self.portal",
        "Self::open_config(IdxdSessionConfig::<Accel>::new(device_path)?)",
    ],
    "source session",
)

require_snippets(
    sources["portal"],
    [
        "pub unsafe fn submit_desc64(&self, desc: *const u8)",
        "if self.dedicated",
        "self.submit_movdir64b_desc64(desc)",
        "self.submit_enqcmd_desc64(desc)",
        "pub unsafe fn submit(&self, desc: &DsaHwDesc)",
        "pub unsafe fn submit_iax(&self, desc: &IaxHwDesc)",
    ],
    "source portal",
)
require_count(
    sources["portal"],
    "self.submit_desc64(desc.as_raw_ptr().cast::<u8>())",
    2,
    "source portal typed DSA/IAX wrappers",
)

for binary_name in ["live_idxd_op", "idxd_representative_bench"]:
    source = sources[binary_name]
    require_snippets(
        source,
        [
            "IdxdSession::<Dsa>::open",
            "IdxdSession::<Iax>::open",
            "session.memmove(&mut dst, &src)",
            "session.crc64(&src)",
        ],
        f"source {binary_name}",
    )
    reject_snippets(
        source,
        [
            "DsaSession",
            "WqPortal",
            "submit_movdir64b",
            "submit_enqcmd",
            "submit_desc64",
            "run_direct_memmove",
            "run_iax_crc64",
            "portal.submit(",
            "hw-eval",
            "pub trait",
            "unsafe",
        ],
        f"source {binary_name}",
    )

require_snippets(
    sources["idxd_representative_bench"],
    [
        "crc64_t10dif_field",
        "profile()",
        "\"release\"",
        "claim_eligible",
    ],
    "source idxd_representative_bench",
)

print(
    f"{PREFIX} verdict=pass path={report_path} report_checks=pass source_checks=pass proof_boundary=generic_sessions_only"
)
PY
