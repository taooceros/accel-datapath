#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, Iterable, List

from proof_runner_common import (
    EXPECTED_ENDPOINT_ROLES,
    REQUIRED_STAGE_COUNTER_FIELDS,
    REQUIRED_STAGE_NAMES,
    build_rpc_args,
    run_server_client_pair,
)

SCRIPT_DIR = Path(__file__).resolve().parent
TONIC_PROFILE_DIR = SCRIPT_DIR.parent
ACCEL_RPC_DIR = TONIC_PROFILE_DIR.parent
REPO_ROOT = ACCEL_RPC_DIR.parent
DEFAULT_MANIFEST = TONIC_PROFILE_DIR / "workloads" / "s03_idxd_matrix.json"
DEFAULT_BINARY = ACCEL_RPC_DIR / "target" / "release" / "tonic-profile"
REQUIRED_LABELS = {
    "ordinary/unary-bytes/repeated-64",
    "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
}


class ManifestError(RuntimeError):
    pass


class RunError(RuntimeError):
    pass


EXPECTED_METADATA_FIELDS = {
    "ordinary_path": "software",
    "selected_path": "idxd",
    "seam": "codec_body",
    "accelerated_lane": "codec_memmove",
    "accelerated_direction": "bidirectional",
}


def fail(message: str) -> None:
    print(message, file=sys.stderr)


def required_str(obj: Dict[str, Any], key: str, *, scope: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{scope} missing {key}")
    return value


def required_bool(obj: Dict[str, Any], key: str, *, scope: str) -> bool:
    value = obj.get(key)
    if not isinstance(value, bool):
        raise ManifestError(f"{scope} field {key} must be a boolean")
    return value


def required_positive_int(obj: Dict[str, Any], key: str, *, scope: str) -> int:
    value = obj.get(key)
    if not isinstance(value, int) or value <= 0:
        raise ManifestError(f"{scope} field {key} must be a positive integer")
    return value


def load_manifest(path: Path) -> Dict[str, Any]:
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except OSError as err:
        raise ManifestError(f"phase=manifest-parse manifest={path} read failed: {err}") from err
    except json.JSONDecodeError as err:
        raise ManifestError(f"phase=manifest-parse manifest={path} parse failed: {err}") from err

    if not isinstance(manifest, dict):
        raise ManifestError("phase=manifest-parse manifest root must be an object")

    defaults = manifest.get("defaults") or {}
    if not isinstance(defaults, dict):
        raise ManifestError("phase=manifest-parse manifest.defaults must be an object")

    expected_metadata = manifest.get("expected_metadata")
    if not isinstance(expected_metadata, dict):
        raise ManifestError("phase=manifest-parse manifest.expected_metadata must be an object")
    validated_expected_metadata = validate_expected_metadata(expected_metadata)

    workloads = manifest.get("workloads")
    if not isinstance(workloads, list) or not workloads:
        raise ManifestError("phase=manifest-parse manifest.workloads must be a non-empty array")

    validated_workloads: List[Dict[str, Any]] = []
    seen_labels: set[str] = set()
    for index, entry in enumerate(workloads):
        validated_workloads.append(
            validate_workload(index, entry, defaults, seen_labels, validated_expected_metadata)
        )

    labels = {entry["label"] for entry in validated_workloads}
    missing_labels = sorted(REQUIRED_LABELS - labels)
    if missing_labels:
        raise ManifestError(
            "phase=manifest-parse manifest.workloads missing required idxd labels: "
            + ", ".join(missing_labels)
        )

    return {
        "version": manifest.get("version"),
        "defaults": defaults,
        "expected_metadata": validated_expected_metadata,
        "workloads": validated_workloads,
    }


def validate_expected_metadata(expected_metadata: Dict[str, Any]) -> Dict[str, Any]:
    scope = "manifest.expected_metadata"
    validated: Dict[str, Any] = {}
    for key, expected in EXPECTED_METADATA_FIELDS.items():
        actual = required_str(expected_metadata, key, scope=scope)
        if actual != expected:
            raise ManifestError(
                f"{scope}.{key}={actual!r} expected {expected!r} for tracked S03 idxd validation"
            )
        validated[key] = actual
    validated["require_device_path"] = required_bool(
        expected_metadata, "require_device_path", scope=scope
    )
    return validated


def validate_workload(
    index: int,
    entry: Any,
    defaults: Dict[str, Any],
    seen_labels: set[str],
    expected_metadata: Dict[str, Any],
) -> Dict[str, Any]:
    scope = f"workloads[{index}]"
    if not isinstance(entry, dict):
        raise ManifestError(f"phase=manifest-parse {scope} must be an object")

    label = required_str(entry, "label", scope=scope)
    if label in seen_labels:
        raise ManifestError(f"phase=manifest-parse {scope} label={label!r} is duplicated")
    seen_labels.add(label)

    rpc = required_str(entry, "rpc", scope=scope)
    endpoint_artifacts = entry.get("endpoint_artifacts")
    if not isinstance(endpoint_artifacts, dict):
        raise ManifestError(f"phase=manifest-parse {scope} missing endpoint_artifacts object")

    merged: Dict[str, Any] = dict(defaults)
    merged.update(entry)
    merged["label"] = label
    merged["rpc"] = rpc
    merged["endpoint_artifacts"] = validate_endpoint_artifacts(index, label, endpoint_artifacts)
    merged["expected_metadata"] = expected_metadata

    warmup_ms = merged.get("warmup_ms")
    if not isinstance(warmup_ms, int) or warmup_ms < 0:
        raise ManifestError(f"phase=manifest-parse {scope} field warmup_ms must be a non-negative integer")
    for field in ["measure_ms", "requests", "concurrency"]:
        required_positive_int(merged, field, scope=scope)
    for field in ["runtime", "compression", "buffer_policy", "instrumentation", "accelerated_path"]:
        required_str(merged, field, scope=scope)

    if merged["instrumentation"] != "on":
        raise ManifestError(
            f"phase=manifest-parse {scope} field instrumentation={merged['instrumentation']!r} expected 'on'"
        )
    if merged["accelerated_path"] != "idxd":
        raise ManifestError(
            f"phase=manifest-parse {scope} field accelerated_path={merged['accelerated_path']!r} expected 'idxd'"
        )

    if rpc == "unary-bytes":
        required_positive_int(merged, "payload_size", scope=scope)
        required_str(merged, "payload_kind", scope=scope)
    elif rpc == "unary-proto-shape":
        required_str(merged, "proto_shape", scope=scope)
        required_str(merged, "response_shape", scope=scope)
    else:
        raise ManifestError(f"phase=manifest-parse {scope} label={label!r} has unsupported rpc mode {rpc!r}")

    return merged


def validate_endpoint_artifacts(index: int, label: str, endpoint_artifacts: Any) -> Dict[str, str]:
    scope = f"workloads[{index}] label={label!r} endpoint_artifacts"
    if not isinstance(endpoint_artifacts, dict):
        raise ManifestError(f"phase=manifest-parse {scope} must be an object")

    validated: Dict[str, str] = {}
    seen_paths: set[str] = set()
    for role in EXPECTED_ENDPOINT_ROLES:
        artifact = required_str(endpoint_artifacts, role, scope=scope)
        if artifact in seen_paths:
            raise ManifestError(
                f"phase=manifest-parse {scope}.{role} duplicates artifact path {artifact!r}"
            )
        seen_paths.add(artifact)
        validated[role] = artifact
    return validated


def locate_binary(explicit: str | None) -> Path:
    path = Path(explicit) if explicit else DEFAULT_BINARY
    if not path.exists():
        raise RunError(
            f"phase=build artifact={path} tonic-profile binary not found; build it first with cargo build --release -p tonic-profile --manifest-path accel-rpc/Cargo.toml"
        )
    return path.resolve()


def reserve_addr() -> str:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()
        return f"{host}:{port}"


def drain_process_output(proc: subprocess.Popen[bytes]) -> tuple[str, str]:
    try:
        stdout, stderr = proc.communicate(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout, stderr = proc.communicate(timeout=2)
    return (
        stdout.decode("utf-8", errors="replace"),
        stderr.decode("utf-8", errors="replace"),
    )


def terminate_process(proc: subprocess.Popen[bytes], *, label: str, phase: str, artifact_path: Path, device_path: str) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        fail(
            f"label={label} phase={phase} endpoint_role=server artifact={artifact_path} device_path={device_path} process did not exit after SIGTERM; sending SIGKILL"
        )
        proc.kill()
        proc.wait(timeout=2)


def wait_for_port(
    proc: subprocess.Popen[bytes],
    addr: str,
    timeout_s: float,
    *,
    label: str,
    artifact_path: Path,
    device_path: str,
) -> None:
    host, port_str = addr.rsplit(":", 1)
    port = int(port_str)
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            stdout, stderr = drain_process_output(proc)
            raise RunError(
                "\n".join(
                    [
                        f"label={label} phase=server-startup endpoint_role=server artifact={artifact_path} device_path={device_path} exit={proc.returncode}",
                        f"stdout:\n{stdout}",
                        f"stderr:\n{stderr}",
                    ]
                )
            )
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.2)
            if sock.connect_ex((host, port)) == 0:
                return
        time.sleep(0.05)
    raise RunError(
        f"label={label} phase=server-startup endpoint_role=server artifact={artifact_path} device_path={device_path} timeout after {timeout_s:.1f}s"
    )


def build_common_args(entry: Dict[str, Any], addr: str, device_path: str) -> List[str]:
    args = build_rpc_args(entry, addr)
    args.extend([
        "--instrumentation",
        entry["instrumentation"],
        "--accelerated-path",
        entry["accelerated_path"],
        "--accelerator-device",
        device_path,
    ])
    return args


def launch_prefix() -> List[str]:
    if shutil.which("devenv") is None:
        raise RunError("phase=preflight launcher=devenv status=missing command not found")
    return ["devenv", "shell", "--", "launch"]


def run_manifest(
    manifest: Dict[str, Any],
    binary: Path,
    output_dir: Path,
    accelerator_device: str,
    server_start_timeout: float,
    client_timeout: float,
    server_flush_timeout: float,
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    launch = launch_prefix()
    for entry in manifest["workloads"]:
        label = entry["label"]
        addr = reserve_addr()
        run_id = f"run-s03-{int(time.time() * 1_000_000)}-{abs(hash((label, addr, accelerator_device))) % 1_000_000}"
        common_args = build_common_args(entry, addr, accelerator_device)
        client_artifact = output_dir / entry["endpoint_artifacts"]["client"]
        server_artifact = output_dir / entry["endpoint_artifacts"]["server"]
        server_cmd = [
            *launch,
            str(binary),
            "--mode",
            "server",
            *common_args,
            "--run-id",
            run_id,
            "--shutdown-after-requests",
            str(entry["requests"]),
            "--server-json-out",
            str(server_artifact),
        ]
        client_cmd = [
            *launch,
            str(binary),
            "--mode",
            "client",
            *common_args,
            "--run-id",
            run_id,
            "--json-out",
            str(client_artifact),
        ]
        run_server_client_pair(
            label=label,
            server_cmd=server_cmd,
            client_cmd=client_cmd,
            server_artifact=server_artifact,
            client_artifact=client_artifact,
            bind_addr=addr,
            target_addr=addr,
            server_start_timeout=server_start_timeout,
            client_timeout=client_timeout,
            server_flush_timeout=server_flush_timeout,
            error_cls=RunError,
            fail_fn=fail,
            server_popen_kwargs={"cwd": REPO_ROOT},
            client_run_kwargs={"cwd": REPO_ROOT},
            context={"device_path": accelerator_device, "launcher": "launch"},
            cleanup_phase="cleanup",
        )


def parse_json(path: Path) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise RunError(f"phase=artifact-validation artifact={path} read failed: {err}") from err
    if not raw.strip():
        raise RunError(f"phase=artifact-validation artifact={path} file is empty")
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as err:
        raise RunError(f"phase=artifact-validation artifact={path} invalid json: {err}") from err
    if not isinstance(value, dict):
        raise RunError(f"phase=artifact-validation artifact={path} root must be an object")
    return value


def require_key(obj: Dict[str, Any], path: Path, key: str) -> Any:
    if key not in obj:
        raise RunError(f"phase=artifact-validation artifact={path} missing {key}")
    return obj[key]


def stage_triplet(report: Dict[str, Any], stage_name: str) -> tuple[int, int, int]:
    stage = report["stages"][stage_name]
    return (
        int(stage.get("count", 0)),
        int(stage.get("nanos", 0)),
        int(stage.get("bytes", 0)),
    )


def placeholder_only(report: Dict[str, Any]) -> bool:
    return all(
        stage_triplet(report, stage_name) == (0, 0, 0) for stage_name in REQUIRED_STAGE_NAMES
    )


def validate_artifacts(manifest: Dict[str, Any], output_dir: Path, accelerator_device: str) -> None:
    signatures: Dict[str, Dict[str, tuple[int, int, int, int]]] = {
        role: {} for role in EXPECTED_ENDPOINT_ROLES
    }

    for entry in manifest["workloads"]:
        label = entry["label"]
        pair_reports: Dict[str, Dict[str, Any]] = {}
        for endpoint_role in EXPECTED_ENDPOINT_ROLES:
            artifact_path = output_dir / entry["endpoint_artifacts"][endpoint_role]
            if not artifact_path.exists():
                raise RunError(
                    f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} missing artifact"
                )
            report = parse_json(artifact_path)
            pair_reports[endpoint_role] = report
            validate_report(
                entry,
                endpoint_role,
                artifact_path,
                report,
                accelerator_device,
            )
            print(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} verdict=pass",
                flush=True,
            )
            signatures[endpoint_role][label] = (
                int(report["stages"]["encode"]["bytes"]),
                int(report["stages"]["decode"]["bytes"]),
                int(report["stages"]["body_accum"]["bytes"]),
                int(report["stages"]["frame_header"]["bytes"]),
            )

        client_report = pair_reports["client"]
        server_report = pair_reports["server"]
        client_run_id = client_report["metadata"].get("run_id")
        server_run_id = server_report["metadata"].get("run_id")
        if client_run_id != server_run_id:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role=client/server artifact={output_dir} device_path={accelerator_device} run_id mismatch client={client_run_id!r} server={server_run_id!r}"
            )
        if client_report["metadata"].get("workload_label") != server_report["metadata"].get("workload_label"):
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role=client/server artifact={output_dir} device_path={accelerator_device} workload_label mismatch"
            )

    for endpoint_role in EXPECTED_ENDPOINT_ROLES:
        unique = set(signatures[endpoint_role].values())
        if len(unique) <= 1:
            labels = ", ".join(sorted(signatures[endpoint_role]))
            raise RunError(
                f"phase=workload-sensitivity endpoint_role={endpoint_role} artifact={output_dir} device_path={accelerator_device} labels={labels} stage signatures were identical across workloads"
            )
        print(
            f"phase=workload-sensitivity endpoint_role={endpoint_role} artifact={output_dir} device_path={accelerator_device} verdict=pass workloads={len(signatures[endpoint_role])}",
            flush=True,
        )


def validate_report(
    entry: Dict[str, Any],
    endpoint_role: str,
    artifact_path: Path,
    report: Dict[str, Any],
    accelerator_device: str,
) -> None:
    label = entry["label"]
    metadata = require_key(report, artifact_path, "metadata")
    metrics = require_key(report, artifact_path, "metrics")
    stages = require_key(report, artifact_path, "stages")
    if not isinstance(metadata, dict):
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata must be an object"
        )
    if not isinstance(metrics, dict):
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metrics must be an object"
        )
    if not isinstance(stages, dict):
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} stages must be an object"
        )

    expected_pairs = {
        "mode": endpoint_role,
        "endpoint_role": endpoint_role,
        "ordinary_path": entry["expected_metadata"]["ordinary_path"],
        "selected_path": entry["expected_metadata"]["selected_path"],
        "seam": entry["expected_metadata"]["seam"],
        "rpc": entry["rpc"],
        "workload_label": label,
        "instrumentation": entry["instrumentation"],
        "buffer_policy": entry["buffer_policy"],
        "accelerated_lane": entry["expected_metadata"]["accelerated_lane"],
        "accelerated_direction": entry["expected_metadata"]["accelerated_direction"],
    }
    for key, expected in expected_pairs.items():
        actual = metadata.get(key)
        if actual != expected:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.{key}={actual!r} expected {expected!r}"
            )

    if entry["expected_metadata"]["require_device_path"]:
        actual_device = metadata.get("accelerated_device_path")
        if actual_device != accelerator_device:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.accelerated_device_path={actual_device!r} expected {accelerator_device!r}"
            )

    run_id = metadata.get("run_id")
    if not isinstance(run_id, str) or not run_id:
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.run_id must be a non-empty string"
        )

    for field in ("effective_codec_buffer_size", "effective_codec_yield_threshold"):
        value = metadata.get(field)
        if not isinstance(value, int) or value <= 0:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.{field} must be a positive integer"
            )

    for field in ("request_serialized_size", "response_serialized_size"):
        value = metadata.get(field)
        if not isinstance(value, int) or value <= 0:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.{field} must be a positive integer"
            )

    if entry["rpc"] == "unary-bytes":
        if metadata.get("payload_size") != entry["payload_size"]:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.payload_size mismatch"
            )
        if metadata.get("payload_kind") != entry["payload_kind"]:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.payload_kind mismatch"
            )
    else:
        if metadata.get("request_shape") != entry["proto_shape"]:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.request_shape mismatch"
            )
        if metadata.get("response_shape") != entry["response_shape"]:
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metadata.response_shape mismatch"
            )

    requests_completed = metrics.get("requests_completed")
    if not isinstance(requests_completed, int) or requests_completed <= 0:
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} metrics.requests_completed must be a positive integer"
        )

    enabled = stages.get("enabled")
    if enabled is not True:
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} stages.enabled={enabled!r} expected True"
        )

    for stage_name in REQUIRED_STAGE_NAMES:
        stage = require_key(stages, artifact_path, stage_name)
        if not isinstance(stage, dict):
            raise RunError(
                f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} stages.{stage_name} must be an object"
            )
        for counter_field in REQUIRED_STAGE_COUNTER_FIELDS:
            if counter_field not in stage:
                raise RunError(
                    f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} missing stages.{stage_name}.{counter_field}"
                )

    if placeholder_only(report):
        raise RunError(
            f"label={label} phase=artifact-validation endpoint_role={endpoint_role} artifact={artifact_path} device_path={accelerator_device} counters stayed placeholder-only"
        )


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Run or validate the curated S03 IDXD hardware-evidence workload matrix."
    )
    parser.add_argument("--manifest", default=str(DEFAULT_MANIFEST))
    parser.add_argument("--output-dir")
    parser.add_argument("--binary")
    parser.add_argument("--accelerator-device")
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--server-start-timeout", type=float, default=10.0)
    parser.add_argument("--client-timeout", type=float, default=20.0)
    parser.add_argument("--server-flush-timeout", type=float, default=10.0)
    args = parser.parse_args(list(argv) if argv is not None else None)

    if args.validate_only and args.verify_only:
        fail("--validate-only and --verify-only are mutually exclusive")
        return 2

    try:
        manifest_path = Path(args.manifest).resolve()
        manifest = load_manifest(manifest_path)
        print(
            f"phase=manifest-parse manifest={manifest_path} workloads={len(manifest['workloads'])}",
            flush=True,
        )

        if args.validate_only:
            return 0

        if not args.accelerator_device:
            raise RunError("phase=preflight accelerator-device is required unless --validate-only is used")
        accelerator_device = args.accelerator_device

        if not args.output_dir:
            raise RunError("phase=artifact-validation --output-dir is required unless --validate-only is used")
        output_dir = Path(args.output_dir).resolve()

        if args.verify_only:
            validate_artifacts(manifest, output_dir, accelerator_device)
            print(
                f"phase=artifact-validation-complete output_dir={output_dir} device_path={accelerator_device}",
                flush=True,
            )
            return 0

        binary = locate_binary(args.binary)
        run_manifest(
            manifest,
            binary,
            output_dir,
            accelerator_device,
            args.server_start_timeout,
            args.client_timeout,
            args.server_flush_timeout,
        )
        validate_artifacts(manifest, output_dir, accelerator_device)
        print(
            f"phase=run-complete output_dir={output_dir} device_path={accelerator_device}",
            flush=True,
        )
        return 0
    except (ManifestError, RunError) as err:
        fail(str(err))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
