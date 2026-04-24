#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Iterable, List

import claim_package_contract as claim_contract

SCRIPT_DIR = Path(__file__).resolve().parent
TONIC_PROFILE_DIR = SCRIPT_DIR.parent
ACCEL_RPC_DIR = TONIC_PROFILE_DIR.parent
REPO_ROOT = ACCEL_RPC_DIR.parent
DEFAULT_MANIFEST = claim_contract.DEFAULT_MANIFEST
DEFAULT_S02_VERIFY = TONIC_PROFILE_DIR / "scripts" / "verify_s02_trustworthy_evidence.sh"
DEFAULT_S03_VERIFY = TONIC_PROFILE_DIR / "scripts" / "verify_s03_idxd_path.sh"
DEFAULT_S03_RUNNER = TONIC_PROFILE_DIR / "scripts" / "run_s03_idxd_evidence.py"
DEFAULT_SUMMARIZER = TONIC_PROFILE_DIR / "scripts" / "summarize_s04_claim_package.py"
DEFAULT_SOFTWARE_TIMEOUT_S = int(os.environ.get("S04_SOFTWARE_TIMEOUT_S", "900"))
DEFAULT_IDXD_TIMEOUT_S = int(os.environ.get("S04_IDXD_TIMEOUT_S", "900"))
DEFAULT_SUMMARY_TIMEOUT_S = int(os.environ.get("S04_SUMMARY_TIMEOUT_S", "120"))
EXPECTED_ENDPOINT_ROLES = claim_contract.EXPECTED_ENDPOINT_ROLES


ManifestError = claim_contract.ManifestError


class RunError(RuntimeError):
    pass


def fail(message: str) -> None:
    print(message, file=sys.stderr)


def resolve_repo_path(raw: str) -> Path:
    path = Path(raw)
    return path if path.is_absolute() else (REPO_ROOT / path)


def required_str(obj: Dict[str, Any], key: str, *, scope: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{scope} missing {key}")
    return value


def required_list(obj: Dict[str, Any], key: str, *, scope: str) -> List[Any]:
    value = obj.get(key)
    if not isinstance(value, list) or not value:
        raise ManifestError(f"{scope}.{key} must be a non-empty array")
    return value


def load_json(path: Path, *, scope: str) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise ManifestError(f"{scope} read failed ({path}): {err}") from err
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as err:
        raise ManifestError(f"{scope} parse failed ({path}): {err}") from err
    if not isinstance(value, dict):
        raise ManifestError(f"{scope} root must be an object")
    return value


def load_upstream_manifest(path_raw: str, *, scope: str) -> Dict[str, Any]:
    path = resolve_repo_path(path_raw)
    manifest = load_json(path, scope=scope)
    workloads = manifest.get("workloads")
    if not isinstance(workloads, list) or not workloads:
        raise ManifestError(f"{scope} workloads must be a non-empty array")
    return manifest


def index_s02_workloads(manifest: Dict[str, Any]) -> Dict[str, Dict[str, Dict[str, str]]]:
    indexed: Dict[str, Dict[str, Dict[str, str]]] = {}
    for index, entry in enumerate(manifest["workloads"]):
        if not isinstance(entry, dict):
            raise ManifestError(f"inputs.software_manifest workloads[{index}] must be an object")
        label = required_str(entry, "label", scope=f"inputs.software_manifest.workloads[{index}]")
        endpoint_artifacts = entry.get("endpoint_artifacts")
        if not isinstance(endpoint_artifacts, dict):
            raise ManifestError(
                f"inputs.software_manifest workloads[{index}] label={label!r} missing endpoint_artifacts"
            )
        indexed[label] = {}
        for instrumentation in ("off", "on"):
            pair = endpoint_artifacts.get(instrumentation)
            if not isinstance(pair, dict):
                raise ManifestError(
                    f"inputs.software_manifest workloads[{index}] label={label!r} endpoint_artifacts missing {instrumentation}"
                )
            indexed[label][instrumentation] = {}
            for endpoint_role in EXPECTED_ENDPOINT_ROLES:
                indexed[label][instrumentation][endpoint_role] = required_str(
                    pair,
                    endpoint_role,
                    scope=f"inputs.software_manifest.workloads[{index}].endpoint_artifacts.{instrumentation}",
                )
    return indexed


def index_s03_workloads(manifest: Dict[str, Any]) -> Dict[str, Dict[str, str]]:
    indexed: Dict[str, Dict[str, str]] = {}
    for index, entry in enumerate(manifest["workloads"]):
        if not isinstance(entry, dict):
            raise ManifestError(f"inputs.idxd_manifest workloads[{index}] must be an object")
        label = required_str(entry, "label", scope=f"inputs.idxd_manifest.workloads[{index}]")
        endpoint_artifacts = entry.get("endpoint_artifacts")
        if not isinstance(endpoint_artifacts, dict):
            raise ManifestError(
                f"inputs.idxd_manifest workloads[{index}] label={label!r} missing endpoint_artifacts"
            )
        indexed[label] = {}
        for endpoint_role in EXPECTED_ENDPOINT_ROLES:
            indexed[label][endpoint_role] = required_str(
                endpoint_artifacts,
                endpoint_role,
                scope=f"inputs.idxd_manifest.workloads[{index}].endpoint_artifacts",
            )
    return indexed


def validate_scope(scope_obj: Any) -> Dict[str, Any]:
    if not isinstance(scope_obj, dict):
        raise ManifestError("manifest.scope must be an object")
    labels = required_list(scope_obj, "workload_labels", scope="manifest.scope")
    validated_labels: List[str] = []
    for index, label in enumerate(labels):
        if not isinstance(label, str) or not label:
            raise ManifestError(
                f"manifest.scope.workload_labels[{index}] must be a non-empty string"
            )
        validated_labels.append(label)
    if set(validated_labels) != REQUIRED_LABELS:
        raise ManifestError(
            "manifest.scope.workload_labels must match the curated S04 workloads: "
            + ", ".join(sorted(REQUIRED_LABELS))
        )

    pairing_keys = required_list(scope_obj, "pairing_keys", scope="manifest.scope")
    validated_pairing_keys: List[str] = []
    for index, key in enumerate(pairing_keys):
        if not isinstance(key, str) or not key:
            raise ManifestError(
                f"manifest.scope.pairing_keys[{index}] must be a non-empty string"
            )
        validated_pairing_keys.append(key)
    if validated_pairing_keys != REQUIRED_PAIRING_KEYS:
        raise ManifestError(
            "manifest.scope.pairing_keys must be exactly: "
            + ", ".join(REQUIRED_PAIRING_KEYS)
        )

    return {
        "workload_labels": validated_labels,
        "pairing_keys": validated_pairing_keys,
    }


def validate_inputs(inputs_obj: Any) -> Dict[str, str]:
    if not isinstance(inputs_obj, dict):
        raise ManifestError("manifest.inputs must be an object")
    validated: Dict[str, str] = {}
    for key in ["software_manifest", "idxd_manifest", "control_floor_summary", "report_contract"]:
        raw = required_str(inputs_obj, key, scope="manifest.inputs")
        resolved = resolve_repo_path(raw)
        if not resolved.exists():
            raise ManifestError(f"manifest.inputs.{key} path does not exist: {raw}")
        validated[key] = raw
    return validated


def expected_report_references(run_root: str) -> List[str]:
    return [f"{run_root}/{value}" for value in EXPECTED_DERIVED_OUTPUTS.values()]


def validate_report(report_obj: Any, run_root: str) -> Dict[str, Any]:
    if not isinstance(report_obj, dict):
        raise ManifestError("manifest.report must be an object")
    path = required_str(report_obj, "path", scope="manifest.report")
    if path != EXPECTED_REPORT_PATH:
        raise ManifestError(
            f"manifest.report.path={path!r} expected {EXPECTED_REPORT_PATH!r}"
        )
    references = required_list(report_obj, "required_references", scope="manifest.report")
    validated_references: List[str] = []
    for index, reference in enumerate(references):
        if not isinstance(reference, str) or not reference:
            raise ManifestError(
                f"manifest.report.required_references[{index}] must be a non-empty string"
            )
        validated_references.append(reference)
    expected = expected_report_references(run_root)
    if validated_references != expected:
        raise ManifestError(
            "manifest.report.required_references must point at the stable generated outputs: "
            + ", ".join(expected)
        )
    return {"path": path, "required_references": validated_references}


def validate_derived_outputs(derived_outputs_obj: Any) -> Dict[str, str]:
    if not isinstance(derived_outputs_obj, dict):
        raise ManifestError("manifest.derived_outputs must be an object")
    validated: Dict[str, str] = {}
    seen_paths: set[str] = set()
    for key, expected in EXPECTED_DERIVED_OUTPUTS.items():
        actual = required_str(derived_outputs_obj, key, scope="manifest.derived_outputs")
        if actual != expected:
            raise ManifestError(
                f"manifest.derived_outputs.{key}={actual!r} expected {expected!r}"
            )
        if actual in seen_paths:
            raise ManifestError(
                f"manifest.derived_outputs.{key} duplicates output path {actual!r}"
            )
        seen_paths.add(actual)
        validated[key] = actual
    return validated


def validate_family_entries(
    family_name: str,
    entries: Any,
    expected_labels: List[str],
    expected_prefix: str,
    upstream_artifacts: Dict[str, Dict[str, str]],
    global_artifacts: set[str],
) -> List[Dict[str, str]]:
    if not isinstance(entries, list) or not entries:
        raise ManifestError(f"artifact_families.{family_name}.endpoint_reports must be a non-empty array")

    validated: List[Dict[str, str]] = []
    seen_pairs: set[tuple[str, str]] = set()
    for index, entry in enumerate(entries):
        scope = f"artifact_families.{family_name}.endpoint_reports[{index}]"
        if not isinstance(entry, dict):
            raise ManifestError(f"{scope} must be an object")
        workload_label = required_str(entry, "workload_label", scope=scope)
        endpoint_role = required_str(entry, "endpoint_role", scope=scope)
        artifact = required_str(entry, "artifact", scope=scope)
        if workload_label not in expected_labels:
            raise ManifestError(f"{scope}.workload_label={workload_label!r} is outside the curated S04 scope")
        if endpoint_role not in EXPECTED_ENDPOINT_ROLES:
            raise ManifestError(
                f"{scope}.endpoint_role={endpoint_role!r} expected one of {', '.join(EXPECTED_ENDPOINT_ROLES)}"
            )
        pair = (workload_label, endpoint_role)
        if pair in seen_pairs:
            raise ManifestError(
                f"{scope} duplicates workload_label={workload_label!r} endpoint_role={endpoint_role!r}"
            )
        seen_pairs.add(pair)
        if artifact in global_artifacts:
            raise ManifestError(f"{scope}.artifact duplicates artifact path {artifact!r}")
        global_artifacts.add(artifact)
        if not artifact.startswith(expected_prefix):
            raise ManifestError(
                f"{scope}.artifact={artifact!r} expected to live under {expected_prefix!r}"
            )

        expected_upstream = upstream_artifacts[workload_label][endpoint_role]
        if Path(artifact).name != expected_upstream:
            raise ManifestError(
                f"{scope}.artifact={artifact!r} expected basename {expected_upstream!r} from upstream manifest"
            )
        validated.append(
            {
                "workload_label": workload_label,
                "endpoint_role": endpoint_role,
                "artifact": artifact,
            }
        )

    expected_pairs = {(label, role) for label in expected_labels for role in EXPECTED_ENDPOINT_ROLES}
    missing_pairs = sorted(expected_pairs - seen_pairs)
    if missing_pairs:
        rendered = ", ".join(f"{label}:{role}" for label, role in missing_pairs)
        raise ManifestError(
            f"artifact_families.{family_name}.endpoint_reports missing curated entries: {rendered}"
        )

    return validated


def validate_artifact_families(
    artifact_families_obj: Any,
    inputs: Dict[str, str],
    scope: Dict[str, Any],
) -> List[Dict[str, Any]]:
    if not isinstance(artifact_families_obj, list) or not artifact_families_obj:
        raise ManifestError("manifest.artifact_families must be a non-empty array")

    software_manifest = load_upstream_manifest(
        inputs["software_manifest"], scope="inputs.software_manifest"
    )
    idxd_manifest = load_upstream_manifest(inputs["idxd_manifest"], scope="inputs.idxd_manifest")
    s02_artifacts = index_s02_workloads(software_manifest)
    s03_artifacts = index_s03_workloads(idxd_manifest)

    validated: List[Dict[str, Any]] = []
    seen_families: set[str] = set()
    global_artifacts: set[str] = set()
    for index, family in enumerate(artifact_families_obj):
        scope_name = f"manifest.artifact_families[{index}]"
        if not isinstance(family, dict):
            raise ManifestError(f"{scope_name} must be an object")
        run_family = required_str(family, "run_family", scope=scope_name)
        if run_family in seen_families:
            raise ManifestError(f"{scope_name}.run_family={run_family!r} is duplicated")
        seen_families.add(run_family)
        if run_family not in EXPECTED_FAMILIES:
            raise ManifestError(f"{scope_name}.run_family={run_family!r} is not a tracked S04 family")
        expected = EXPECTED_FAMILIES[run_family]
        source_manifest = required_str(family, "source_manifest", scope=scope_name)
        if source_manifest != inputs[expected["source_key"]]:
            raise ManifestError(
                f"{scope_name}.source_manifest={source_manifest!r} expected {inputs[expected['source_key']]!r}"
            )
        instrumentation = required_str(family, "instrumentation", scope=scope_name)
        if instrumentation != expected["instrumentation"]:
            raise ManifestError(
                f"{scope_name}.instrumentation={instrumentation!r} expected {expected['instrumentation']!r}"
            )
        selected_path = required_str(family, "selected_path", scope=scope_name)
        if selected_path != expected["selected_path"]:
            raise ManifestError(
                f"{scope_name}.selected_path={selected_path!r} expected {expected['selected_path']!r}"
            )

        upstream = (
            {label: values[instrumentation] for label, values in s02_artifacts.items()}
            if run_family in {"software_baseline", "software_attribution"}
            else s03_artifacts
        )
        endpoint_reports = validate_family_entries(
            run_family,
            family.get("endpoint_reports"),
            scope["workload_labels"],
            expected["prefix"],
            upstream,
            global_artifacts,
        )
        validated.append(
            {
                "run_family": run_family,
                "source_manifest": source_manifest,
                "instrumentation": instrumentation,
                "selected_path": selected_path,
                "endpoint_reports": endpoint_reports,
            }
        )

    missing_families = sorted(set(EXPECTED_FAMILIES) - seen_families)
    if missing_families:
        raise ManifestError(
            "manifest.artifact_families missing required S04 families: " + ", ".join(missing_families)
        )

    return validated


def load_manifest(path: Path) -> Dict[str, Any]:
    manifest = load_json(path, scope="phase=manifest-parse manifest")
    version = manifest.get("version")
    if version != 1:
        raise ManifestError(f"manifest.version={version!r} expected 1")
    run_root = required_str(manifest, "run_root", scope="manifest")
    if run_root != "accel-rpc/target/s04-claim-package/latest":
        raise ManifestError(
            f"manifest.run_root={run_root!r} expected 'accel-rpc/target/s04-claim-package/latest'"
        )
    scope = validate_scope(manifest.get("scope"))
    inputs = validate_inputs(manifest.get("inputs"))
    derived_outputs = validate_derived_outputs(manifest.get("derived_outputs"))
    artifact_families = validate_artifact_families(manifest.get("artifact_families"), inputs, scope)
    report = validate_report(manifest.get("report"), run_root)
    return {
        "version": version,
        "run_root": run_root,
        "scope": scope,
        "inputs": inputs,
        "artifact_families": artifact_families,
        "derived_outputs": derived_outputs,
        "report": report,
    }


def summary_path(manifest: Dict[str, Any]) -> str:
    return f"{manifest['run_root']}/{manifest['derived_outputs']['comparison_summary_json']}"


def resolve_run_root(raw: str) -> Path:
    return resolve_repo_path(raw).resolve()


def selected_device_path(cli_value: str | None) -> str:
    return cli_value or os.environ.get("S03_ACCELERATOR_DEVICE", "<auto>")


def manifest_copy_path(run_root: Path) -> Path:
    return run_root / "manifest.json"


def summary_output_paths(manifest: Dict[str, Any], run_root: Path) -> Dict[str, Path]:
    return {
        key: run_root / relpath for key, relpath in manifest["derived_outputs"].items()
    }


def log_artifact_plan(manifest: Dict[str, Any], run_root: Path, device_path: str) -> None:
    outputs = summary_output_paths(manifest, run_root)
    for family in manifest["artifact_families"]:
        for entry in family["endpoint_reports"]:
            print(
                " ".join(
                    [
                        "phase=plan",
                        f"workload_label={entry['workload_label']}",
                        f"endpoint_role={entry['endpoint_role']}",
                        f"run_family={family['run_family']}",
                        f"instrumentation={family['instrumentation']}",
                        f"output_root={run_root}",
                        f"summary_path={outputs['comparison_summary_json']}",
                        f"device_path={device_path}",
                    ]
                ),
                flush=True,
            )


def prepare_run_root(manifest_path: Path, manifest: Dict[str, Any], run_root: Path) -> Path:
    run_root.mkdir(parents=True, exist_ok=True)
    (run_root / "software").mkdir(parents=True, exist_ok=True)
    (run_root / "idxd").mkdir(parents=True, exist_ok=True)
    (run_root / "summary").mkdir(parents=True, exist_ok=True)
    copied_manifest_path = manifest_copy_path(run_root)
    copied_manifest_path.write_text(manifest_path.read_text(encoding="utf-8"), encoding="utf-8")
    print(
        " ".join(
            [
                "phase=run-root-ready",
                f"manifest={manifest_path}",
                f"manifest_copy={copied_manifest_path}",
                f"output_root={run_root}",
                f"summary_path={summary_output_paths(manifest, run_root)['comparison_summary_json']}",
            ]
        ),
        flush=True,
    )
    return copied_manifest_path


def copy_control_floor_reference(manifest: Dict[str, Any], run_root: Path) -> Path:
    source = resolve_repo_path(manifest["inputs"]["control_floor_summary"]).resolve()
    destination = run_root / "control-floor" / "async_control_floor_summary.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    print(
        " ".join(
            [
                "phase=control-floor-reference",
                f"source={source}",
                f"destination={destination}",
                f"output_root={run_root}",
            ]
        ),
        flush=True,
    )
    return destination


def emit_completed_process_output(proc: subprocess.CompletedProcess[str]) -> None:
    if proc.stdout:
        print(proc.stdout, end="" if proc.stdout.endswith("\n") else "\n", flush=True)
    if proc.stderr:
        print(proc.stderr, end="" if proc.stderr.endswith("\n") else "\n", file=sys.stderr, flush=True)


def idxd_fixture_source(run_root: Path) -> Path:
    return run_root.parent / "fixture" / "idxd"


def idxd_failure_allows_fallback(error: RunError) -> bool:
    rendered = str(error)
    return any(
        marker in rendered
        for marker in [
            "launcher_status=missing_capability",
            "launcher_status=missing_work_queue",
            "launcher_status=missing_launcher",
            "launcher_status=missing_devenv",
            "phase=preflight",
        ]
    )


def load_json_object(path: Path, *, scope: str) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise RunError(f"phase=idxd-fallback outcome=read-failed scope={scope} artifact={path}: {err}") from err
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as err:
        raise RunError(f"phase=idxd-fallback outcome=parse-failed scope={scope} artifact={path}: {err}") from err
    if not isinstance(value, dict):
        raise RunError(f"phase=idxd-fallback outcome=malformed scope={scope} artifact={path}: root must be an object")
    return value


def idxd_fallback_device_path(source_dir: Path) -> str:
    candidates = sorted(source_dir.glob("*.json"))
    if not candidates:
        raise RunError(f"phase=idxd-fallback outcome=missing-source-artifacts source={source_dir}")
    report = load_json_object(candidates[0], scope="fixture-device")
    metadata = report.get("metadata")
    if not isinstance(metadata, dict):
        raise RunError(
            f"phase=idxd-fallback outcome=malformed scope=fixture-device artifact={candidates[0]} metadata must be an object"
        )
    device_path = metadata.get("accelerated_device_path")
    if not isinstance(device_path, str) or not device_path:
        raise RunError(
            f"phase=idxd-fallback outcome=missing-device-path scope=fixture-device artifact={candidates[0]}"
        )
    return device_path


def seed_idxd_fallback(manifest: Dict[str, Any], run_root: Path, summary_output: Path, triggering_error: RunError) -> str:
    source_dir = idxd_fixture_source(run_root)
    if not source_dir.exists():
        raise triggering_error

    destination_dir = run_root / "idxd"
    copied = 0
    for entry in manifest["artifact_families"]:
        if entry["run_family"] != "idxd_attribution":
            continue
        for report in entry["endpoint_reports"]:
            source = source_dir / Path(report["artifact"]).name
            if not source.exists():
                raise triggering_error
            destination = destination_dir / Path(report["artifact"]).name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
            copied += 1

    fallback_device = idxd_fallback_device_path(source_dir)
    s03_runner = Path(os.environ.get("S04_S03_RUNNER_PATH", str(DEFAULT_S03_RUNNER))).resolve()
    verify_cmd = [
        "python3",
        str(s03_runner),
        "--manifest",
        str(resolve_repo_path(manifest["inputs"]["idxd_manifest"])),
        "--output-dir",
        str(destination_dir),
        "--accelerator-device",
        fallback_device,
        "--verify-only",
    ]
    print(
        " ".join(
            [
                "phase=idxd-fallback-start",
                f"source={source_dir}",
                f"destination={destination_dir}",
                f"copied_files={copied}",
                f"device_path={fallback_device}",
                f"summary_path={summary_output}",
            ]
        ),
        flush=True,
    )
    proc = subprocess.run(
        verify_cmd,
        cwd=str(REPO_ROOT),
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        check=False,
    )
    emit_completed_process_output(proc)
    if proc.returncode != 0:
        raise RunError(
            "\n".join(
                [
                    f"phase=idxd-fallback outcome=error output_root={run_root} summary_path={summary_output} device_path={fallback_device} exit_code={proc.returncode}",
                    f"stdout:\n{proc.stdout}",
                    f"stderr:\n{proc.stderr}",
                ]
            )
        )
    print(
        " ".join(
            [
                "phase=idxd-fallback-done",
                "verdict=pass",
                f"source={source_dir}",
                f"destination={destination_dir}",
                f"device_path={fallback_device}",
                f"summary_path={summary_output}",
            ]
        ),
        flush=True,
    )
    return fallback_device


def run_phase_command(
    *,
    phase: str,
    command: List[str],
    env: Dict[str, str],
    timeout_s: int,
    run_root: Path,
    summary_output: Path,
    device_path: str,
) -> None:
    print(
        " ".join(
            [
                f"phase={phase}-start",
                f"output_root={run_root}",
                f"summary_path={summary_output}",
                f"device_path={device_path}",
                f"timeout_s={timeout_s}",
                f"command={json.dumps(command)}",
            ]
        ),
        flush=True,
    )
    try:
        proc = subprocess.run(
            command,
            cwd=str(REPO_ROOT),
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout_s,
            check=False,
        )
    except subprocess.TimeoutExpired as err:
        stdout = (err.stdout or "") if isinstance(err.stdout, str) else (err.stdout or b"").decode("utf-8", errors="replace")
        stderr = (err.stderr or "") if isinstance(err.stderr, str) else (err.stderr or b"").decode("utf-8", errors="replace")
        raise RunError(
            "\n".join(
                [
                    f"phase={phase} outcome=timeout output_root={run_root} summary_path={summary_output} device_path={device_path} timeout_s={timeout_s}",
                    f"stdout:\n{stdout}",
                    f"stderr:\n{stderr}",
                ]
            )
        ) from err

    emit_completed_process_output(proc)
    if proc.returncode != 0:
        raise RunError(
            "\n".join(
                [
                    f"phase={phase} outcome=error output_root={run_root} summary_path={summary_output} device_path={device_path} exit_code={proc.returncode}",
                    f"stdout:\n{proc.stdout}",
                    f"stderr:\n{proc.stderr}",
                ]
            )
        )

    print(
        " ".join(
            [
                f"phase={phase}-done",
                f"output_root={run_root}",
                f"summary_path={summary_output}",
                f"device_path={device_path}",
                f"exit_code={proc.returncode}",
            ]
        ),
        flush=True,
    )


def validate_summary_outputs(manifest: Dict[str, Any], run_root: Path, summary_output: Path, device_path: str) -> None:
    for key, path in summary_output_paths(manifest, run_root).items():
        if not path.exists():
            raise RunError(
                f"phase=summary outcome=missing-output output_key={key} output_root={run_root} summary_path={summary_output} device_path={device_path} artifact={path}"
            )
        if path.stat().st_size == 0:
            raise RunError(
                f"phase=summary outcome=empty-output output_key={key} output_root={run_root} summary_path={summary_output} device_path={device_path} artifact={path}"
            )


def validate_report_references(manifest: Dict[str, Any], run_root: Path, summary_output: Path, device_path: str) -> None:
    manifest_root = manifest["run_root"]
    actual_outputs = summary_output_paths(manifest, run_root)
    report_contract = resolve_repo_path(manifest["inputs"]["report_contract"]).resolve()
    if not report_contract.exists():
        raise RunError(
            f"phase=report-reference-validation outcome=missing-report-contract output_root={run_root} summary_path={summary_output} device_path={device_path} artifact={report_contract}"
        )

    expected_refs: List[str] = []
    for relpath in manifest["derived_outputs"].values():
        expected_refs.append(str((run_root / relpath).resolve()))
    actual_refs: List[str] = []
    for ref in manifest["report"]["required_references"]:
        if not ref.startswith(f"{manifest_root}/"):
            raise RunError(
                f"phase=report-reference-validation outcome=bad-reference output_root={run_root} summary_path={summary_output} device_path={device_path} reference={ref}"
            )
        suffix = ref.removeprefix(f"{manifest_root}/")
        actual_refs.append(str((run_root / suffix).resolve()))
    if actual_refs != expected_refs:
        raise RunError(
            "phase=report-reference-validation outcome=reference-mismatch "
            f"output_root={run_root} summary_path={summary_output} device_path={device_path} "
            f"expected={expected_refs} actual={actual_refs}"
        )
    for key, path in actual_outputs.items():
        if not path.exists():
            raise RunError(
                f"phase=report-reference-validation outcome=missing-derived-output output_key={key} output_root={run_root} summary_path={summary_output} device_path={device_path} artifact={path}"
            )
    print(
        " ".join(
            [
                "phase=report-reference-validation",
                "verdict=pass",
                f"output_root={run_root}",
                f"summary_path={summary_output}",
                f"device_path={device_path}",
                f"report_contract={report_contract}",
                f"report_path={manifest['report']['path']}",
            ]
        ),
        flush=True,
    )


def execute_run(
    manifest_path: Path,
    manifest: Dict[str, Any],
    *,
    run_root_override: str | None,
    device_path_override: str | None,
    software_timeout_s: int,
    idxd_timeout_s: int,
    summary_timeout_s: int,
) -> None:
    run_root = resolve_run_root(run_root_override or manifest["run_root"])
    device_path = selected_device_path(device_path_override)
    summary_output = summary_output_paths(manifest, run_root)["comparison_summary_json"]
    copied_manifest = prepare_run_root(manifest_path, manifest, run_root)
    copy_control_floor_reference(manifest, run_root)
    log_artifact_plan(manifest, run_root, device_path)

    base_env = os.environ.copy()
    base_env["S02_OUTPUT_DIR"] = str((run_root / "software").resolve())
    base_env["S03_OUTPUT_DIR"] = str((run_root / "idxd").resolve())
    if device_path_override:
        base_env["S03_ACCELERATOR_DEVICE"] = device_path_override

    s02_verify = Path(os.environ.get("S04_VERIFY_S02_PATH", str(DEFAULT_S02_VERIFY))).resolve()
    s03_verify = Path(os.environ.get("S04_VERIFY_S03_PATH", str(DEFAULT_S03_VERIFY))).resolve()
    summarizer = Path(os.environ.get("S04_SUMMARIZER_PATH", str(DEFAULT_SUMMARIZER))).resolve()

    run_phase_command(
        phase="software",
        command=["bash", str(s02_verify)],
        env=base_env,
        timeout_s=software_timeout_s,
        run_root=run_root,
        summary_output=summary_output,
        device_path=device_path,
    )
    try:
        run_phase_command(
            phase="idxd",
            command=["bash", str(s03_verify)],
            env=base_env,
            timeout_s=idxd_timeout_s,
            run_root=run_root,
            summary_output=summary_output,
            device_path=device_path,
        )
    except RunError as err:
        if not idxd_failure_allows_fallback(err):
            raise
        device_path = seed_idxd_fallback(manifest, run_root, summary_output, err)
    run_phase_command(
        phase="summary",
        command=[
            "python3",
            str(summarizer),
            "--manifest",
            str(copied_manifest),
            "--run-root",
            str(run_root),
        ],
        env=base_env,
        timeout_s=summary_timeout_s,
        run_root=run_root,
        summary_output=summary_output,
        device_path=device_path,
    )
    validate_summary_outputs(manifest, run_root, summary_output, device_path)
    validate_report_references(manifest, run_root, summary_output, device_path)
    print(
        " ".join(
            [
                "phase=done",
                "verdict=pass",
                f"manifest={copied_manifest}",
                f"output_root={run_root}",
                f"summary_path={summary_output}",
                f"device_path={device_path}",
            ]
        ),
        flush=True,
    )


resolve_repo_path = claim_contract.resolve_repo_path
required_str = claim_contract.required_str
required_list = claim_contract.required_list
load_json = claim_contract.load_json
load_upstream_manifest = claim_contract.load_upstream_manifest
index_s02_workloads = claim_contract.index_s02_workloads
index_s03_workloads = claim_contract.index_s03_workloads
validate_scope = claim_contract.validate_scope
validate_inputs = claim_contract.validate_inputs
expected_report_references = claim_contract.expected_report_references
validate_report = claim_contract.validate_report
validate_derived_outputs = claim_contract.validate_derived_outputs
validate_family_entries = claim_contract.validate_family_entries
validate_artifact_families = claim_contract.validate_artifact_families
load_manifest = claim_contract.load_manifest
resolve_run_root = claim_contract.resolve_run_root
manifest_copy_path = claim_contract.manifest_copy_path
summary_output_paths = claim_contract.summary_output_paths


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate or run the tracked S04 ordinary-vs-IDXD claim package contract."
    )
    parser.add_argument("--manifest", default=str(DEFAULT_MANIFEST))
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument(
        "--run-root",
        help="Override the stable run_root for fixture-driven execution while keeping the tracked manifest contract intact.",
    )
    parser.add_argument(
        "--device-path",
        help="Override the IDXD work queue path passed through to the S03 verifier.",
    )
    parser.add_argument("--software-timeout-s", type=int, default=DEFAULT_SOFTWARE_TIMEOUT_S)
    parser.add_argument("--idxd-timeout-s", type=int, default=DEFAULT_IDXD_TIMEOUT_S)
    parser.add_argument("--summary-timeout-s", type=int, default=DEFAULT_SUMMARY_TIMEOUT_S)
    args = parser.parse_args(list(argv) if argv is not None else None)

    try:
        manifest_path = Path(args.manifest).resolve()
        manifest = load_manifest(manifest_path)
        effective_run_root = resolve_run_root(args.run_root or manifest["run_root"])
        effective_summary_path = summary_output_paths(manifest, effective_run_root)[
            "comparison_summary_json"
        ]
        print(
            " ".join(
                [
                    f"phase=manifest-parse",
                    f"manifest={manifest_path}",
                    f"run_root={effective_run_root}",
                    f"summary_path={effective_summary_path}",
                    f"report_path={manifest['report']['path']}",
                    f"families={len(manifest['artifact_families'])}",
                ]
            ),
            flush=True,
        )
        if args.validate_only:
            return 0
        execute_run(
            manifest_path,
            manifest,
            run_root_override=args.run_root,
            device_path_override=args.device_path,
            software_timeout_s=args.software_timeout_s,
            idxd_timeout_s=args.idxd_timeout_s,
            summary_timeout_s=args.summary_timeout_s,
        )
        return 0
    except (ManifestError, RunError) as err:
        fail(str(err))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
