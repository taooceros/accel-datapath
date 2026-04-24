#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
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
DEFAULT_MANIFEST = SCRIPT_DIR.parent / "workloads" / "s02_trustworthy_matrix.json"
EXPECTED_BENCHMARKS = [
    "tokio_spawn_join",
    "tokio_oneshot_completion",
    "tokio_mpsc_round_trip",
    "tokio_same_thread_wake",
    "tokio_cross_thread_wake",
]


class ManifestError(RuntimeError):
    pass


class RunError(RuntimeError):
    pass



def fail(message: str) -> None:
    print(message, file=sys.stderr)



def required_str(obj: Dict[str, Any], key: str, *, scope: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{scope} missing {key}")
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
        raise ManifestError(f"manifest read failed ({path}): {err}") from err
    except json.JSONDecodeError as err:
        raise ManifestError(f"manifest parse failed ({path}): {err}") from err

    if not isinstance(manifest, dict):
        raise ManifestError("manifest root must be an object")

    defaults = manifest.get("defaults") or {}
    if not isinstance(defaults, dict):
        raise ManifestError("manifest.defaults must be an object")

    expected_benchmarks = manifest.get("expected_benchmarks")
    if not isinstance(expected_benchmarks, list) or not expected_benchmarks:
        raise ManifestError("manifest.expected_benchmarks must be a non-empty array")
    validated_benchmarks: List[str] = []
    for index, name in enumerate(expected_benchmarks):
        if not isinstance(name, str) or not name:
            raise ManifestError(
                f"manifest.expected_benchmarks[{index}] must be a non-empty string"
            )
        validated_benchmarks.append(name)
    if sorted(validated_benchmarks) != sorted(EXPECTED_BENCHMARKS):
        raise ManifestError(
            "manifest.expected_benchmarks must match the async control-floor suite: "
            + ", ".join(EXPECTED_BENCHMARKS)
        )

    workloads = manifest.get("workloads")
    if not isinstance(workloads, list) or not workloads:
        raise ManifestError("manifest.workloads must be a non-empty array")

    validated_workloads: List[Dict[str, Any]] = []
    seen_labels: set[str] = set()
    for index, entry in enumerate(workloads):
        validated_workloads.append(validate_workload(index, entry, defaults, seen_labels))

    required_labels = {
        "ordinary/unary-bytes/repeated-64",
        "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
    }
    labels = {entry["label"] for entry in validated_workloads}
    missing_labels = sorted(required_labels - labels)
    if missing_labels:
        raise ManifestError(
            "manifest.workloads missing required trust-verification labels: "
            + ", ".join(missing_labels)
        )

    return {
        "version": manifest.get("version"),
        "defaults": defaults,
        "expected_benchmarks": validated_benchmarks,
        "workloads": validated_workloads,
    }



def validate_workload(
    index: int, entry: Any, defaults: Dict[str, Any], seen_labels: set[str]
) -> Dict[str, Any]:
    scope = f"workloads[{index}]"
    if not isinstance(entry, dict):
        raise ManifestError(f"{scope} must be an object")

    label = required_str(entry, "label", scope=scope)
    if label in seen_labels:
        raise ManifestError(f"{scope} label={label!r} is duplicated")
    seen_labels.add(label)

    rpc = required_str(entry, "rpc", scope=scope)
    endpoint_artifacts = entry.get("endpoint_artifacts")
    if not isinstance(endpoint_artifacts, dict):
        raise ManifestError(f"{scope} missing endpoint_artifacts object")

    merged: Dict[str, Any] = dict(defaults)
    merged.update(entry)
    merged["label"] = label
    merged["rpc"] = rpc
    merged["endpoint_artifacts"] = validate_endpoint_artifacts(index, label, endpoint_artifacts)

    warmup_ms = merged.get("warmup_ms")
    if not isinstance(warmup_ms, int) or warmup_ms < 0:
        raise ManifestError(f"{scope} field warmup_ms must be a non-negative integer")
    for field in ["measure_ms", "requests", "concurrency"]:
        required_positive_int(merged, field, scope=scope)
    for field in ["runtime", "compression", "buffer_policy"]:
        required_str(merged, field, scope=scope)

    if rpc == "unary-bytes":
        required_positive_int(merged, "payload_size", scope=scope)
        required_str(merged, "payload_kind", scope=scope)
    elif rpc == "unary-proto-shape":
        required_str(merged, "proto_shape", scope=scope)
        required_str(merged, "response_shape", scope=scope)
    else:
        raise ManifestError(f"{scope} label={label!r} has unsupported rpc mode {rpc!r}")

    return merged



def validate_endpoint_artifacts(index: int, label: str, endpoint_artifacts: Any) -> Dict[str, Dict[str, str]]:
    scope = f"workloads[{index}] label={label!r} endpoint_artifacts"
    if not isinstance(endpoint_artifacts, dict):
        raise ManifestError(f"{scope} must be an object")

    validated: Dict[str, Dict[str, str]] = {}
    seen_paths: set[str] = set()
    for instrumentation in ("off", "on"):
        pair = endpoint_artifacts.get(instrumentation)
        if not isinstance(pair, dict):
            raise ManifestError(f"{scope} missing {instrumentation} object")
        validated_pair: Dict[str, str] = {}
        for role in EXPECTED_ENDPOINT_ROLES:
            artifact = required_str(pair, role, scope=f"{scope}.{instrumentation}")
            if artifact in seen_paths:
                raise ManifestError(
                    f"{scope}.{instrumentation}.{role} duplicates artifact path {artifact!r}"
                )
            seen_paths.add(artifact)
            validated_pair[role] = artifact
        validated[instrumentation] = validated_pair
    return validated



def locate_binary(explicit: str | None) -> Path:
    path = Path(explicit) if explicit else Path("accel-rpc/target/release/tonic-profile")
    if not path.exists():
        raise RunError(
            f"tonic-profile binary not found at {path}; build it first with cargo build --release -p tonic-profile --manifest-path accel-rpc/Cargo.toml"
        )
    return path.resolve()



def reserve_addr() -> str:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()
        return f"{host}:{port}"



def drain_process_output(proc: subprocess.Popen[bytes]) -> tuple[str, str]:
    stdout, stderr = proc.communicate(timeout=2)
    return (
        stdout.decode("utf-8", errors="replace"),
        stderr.decode("utf-8", errors="replace"),
    )



def terminate_process(proc: subprocess.Popen[bytes], *, label: str, phase: str, artifact_path: Path) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        fail(
            f"label={label} phase={phase} endpoint_role=server artifact={artifact_path} process did not exit after SIGTERM; sending SIGKILL"
        )
        proc.kill()
        proc.wait(timeout=2)



def wait_for_port(
    proc: subprocess.Popen[bytes],
    addr: str,
    timeout_s: float,
    *,
    label: str,
    instrumentation: str,
    artifact_path: Path,
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
                        f"label={label} phase=server-startup instrumentation={instrumentation} endpoint_role=server artifact={artifact_path} exit={proc.returncode}",
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
        f"label={label} phase=server-startup instrumentation={instrumentation} endpoint_role=server artifact={artifact_path} timeout after {timeout_s:.1f}s"
    )



def build_common_args(entry: Dict[str, Any], addr: str) -> List[str]:
    return build_rpc_args(entry, addr)



def run_manifest(
    manifest: Dict[str, Any],
    binary: Path,
    output_dir: Path,
    server_start_timeout: float,
    client_timeout: float,
    server_flush_timeout: float,
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for entry in manifest["workloads"]:
        label = entry["label"]
        for instrumentation in ("off", "on"):
            addr = reserve_addr()
            run_id = f"run-s02-{int(time.time() * 1_000_000)}-{instrumentation}-{abs(hash((label, instrumentation, addr))) % 1_000_000}"
            common_args = build_common_args(entry, addr)
            client_artifact = output_dir / entry["endpoint_artifacts"][instrumentation]["client"]
            server_artifact = output_dir / entry["endpoint_artifacts"][instrumentation]["server"]
            server_cmd = [
                str(binary),
                "--mode",
                "server",
                "--instrumentation",
                instrumentation,
                *common_args,
                "--run-id",
                run_id,
                "--shutdown-after-requests",
                str(entry["requests"]),
                "--server-json-out",
                str(server_artifact),
            ]
            client_cmd = [
                str(binary),
                "--mode",
                "client",
                "--instrumentation",
                instrumentation,
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
                context={"instrumentation": instrumentation},
                cleanup_phase=f"cleanup instrumentation={instrumentation}",
            )



def parse_json(path: Path) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise RunError(f"artifact read failed ({path}): {err}") from err
    if not raw.strip():
        raise RunError(f"artifact validation failed path={path}: file is empty")
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as err:
        raise RunError(f"artifact validation failed path={path}: invalid json: {err}") from err
    if not isinstance(value, dict):
        raise RunError(f"artifact validation failed path={path}: root must be an object")
    return value



def require_key(obj: Dict[str, Any], path: Path, key: str) -> Any:
    if key not in obj:
        raise RunError(f"artifact validation failed path={path}: missing {key}")
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



def validate_artifacts(manifest: Dict[str, Any], output_dir: Path) -> None:
    on_signatures: Dict[str, Dict[str, tuple[int, int, int, int]]] = {
        role: {} for role in EXPECTED_ENDPOINT_ROLES
    }

    for entry in manifest["workloads"]:
        label = entry["label"]
        pair_reports: Dict[str, Dict[str, Dict[str, Any]]] = {"off": {}, "on": {}}
        for instrumentation in ("off", "on"):
            for endpoint_role in EXPECTED_ENDPOINT_ROLES:
                artifact_path = output_dir / entry["endpoint_artifacts"][instrumentation][endpoint_role]
                if not artifact_path.exists():
                    raise RunError(
                        f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} missing artifact"
                    )
                report = parse_json(artifact_path)
                pair_reports[instrumentation][endpoint_role] = report
                validate_report(
                    entry,
                    instrumentation,
                    endpoint_role,
                    artifact_path,
                    report,
                )
                print(
                    f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} verdict=pass",
                    flush=True,
                )

                if instrumentation == "on":
                    on_signatures[endpoint_role][label] = (
                        int(report["stages"]["encode"]["bytes"]),
                        int(report["stages"]["decode"]["bytes"]),
                        int(report["stages"]["body_accum"]["bytes"]),
                        int(report["stages"]["frame_header"]["bytes"]),
                    )

            client_report = pair_reports[instrumentation]["client"]
            server_report = pair_reports[instrumentation]["server"]
            client_run_id = client_report["metadata"].get("run_id")
            server_run_id = server_report["metadata"].get("run_id")
            if client_run_id != server_run_id:
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role=client/server artifact={output_dir} run_id mismatch client={client_run_id!r} server={server_run_id!r}"
                )
            if client_report["metadata"].get("workload_label") != server_report["metadata"].get("workload_label"):
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role=client/server artifact={output_dir} workload_label mismatch"
                )

    for endpoint_role in EXPECTED_ENDPOINT_ROLES:
        signatures = on_signatures[endpoint_role]
        unique = set(signatures.values())
        if len(unique) <= 1:
            labels = ", ".join(sorted(signatures))
            raise RunError(
                f"phase=workload-sensitivity instrumentation=on endpoint_role={endpoint_role} artifact={output_dir} labels={labels} stage signatures were identical across workloads"
            )
        print(
            f"phase=workload-sensitivity instrumentation=on endpoint_role={endpoint_role} artifact={output_dir} verdict=pass workloads={len(signatures)}",
            flush=True,
        )



def validate_report(
    entry: Dict[str, Any],
    instrumentation: str,
    endpoint_role: str,
    artifact_path: Path,
    report: Dict[str, Any],
) -> None:
    label = entry["label"]
    metadata = require_key(report, artifact_path, "metadata")
    metrics = require_key(report, artifact_path, "metrics")
    stages = require_key(report, artifact_path, "stages")
    if not isinstance(metadata, dict):
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata must be an object"
        )
    if not isinstance(metrics, dict):
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metrics must be an object"
        )
    if not isinstance(stages, dict):
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} stages must be an object"
        )

    expected_mode = endpoint_role
    expected_pairs = {
        "mode": expected_mode,
        "endpoint_role": endpoint_role,
        "ordinary_path": "software",
        "seam": "codec_body",
        "rpc": entry["rpc"],
        "workload_label": label,
        "instrumentation": instrumentation,
        "buffer_policy": entry["buffer_policy"],
    }
    for key, expected in expected_pairs.items():
        actual = metadata.get(key)
        if actual != expected:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.{key}={actual!r} expected {expected!r}"
            )

    run_id = metadata.get("run_id")
    if not isinstance(run_id, str) or not run_id:
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.run_id must be a non-empty string"
        )

    for field in ("effective_codec_buffer_size", "effective_codec_yield_threshold"):
        value = metadata.get(field)
        if not isinstance(value, int) or value <= 0:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.{field} must be a positive integer"
            )

    for field in ("request_serialized_size", "response_serialized_size"):
        value = metadata.get(field)
        if not isinstance(value, int) or value <= 0:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.{field} must be a positive integer"
            )

    if entry["rpc"] == "unary-bytes":
        if metadata.get("payload_size") != entry["payload_size"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.payload_size mismatch"
            )
        if metadata.get("payload_kind") != entry["payload_kind"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.payload_kind mismatch"
            )
    else:
        if metadata.get("request_shape") != entry["proto_shape"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.request_shape mismatch"
            )
        if metadata.get("response_shape") != entry["response_shape"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metadata.response_shape mismatch"
            )

    requests_completed = metrics.get("requests_completed")
    if not isinstance(requests_completed, int) or requests_completed <= 0:
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} metrics.requests_completed must be a positive integer"
        )

    enabled = stages.get("enabled")
    expected_enabled = instrumentation == "on"
    if enabled is not expected_enabled:
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} stages.enabled={enabled!r} expected {expected_enabled!r}"
        )

    for stage_name in REQUIRED_STAGE_NAMES:
        stage = require_key(stages, artifact_path, stage_name)
        if not isinstance(stage, dict):
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} stages.{stage_name} must be an object"
            )
        for counter_field in REQUIRED_STAGE_COUNTER_FIELDS:
            if counter_field not in stage:
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} missing stages.{stage_name}.{counter_field}"
                )

    if instrumentation == "on" and placeholder_only(report):
        raise RunError(
            f"label={label} phase=artifact-validation instrumentation={instrumentation} endpoint_role={endpoint_role} artifact={artifact_path} counters stayed placeholder-only"
        )



def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Run or validate the curated S02 trustworthy-evidence workload matrix."
    )
    parser.add_argument("--manifest", default=str(DEFAULT_MANIFEST))
    parser.add_argument("--output-dir")
    parser.add_argument("--binary")
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
            f"phase=manifest-parse manifest={manifest_path} workloads={len(manifest['workloads'])} expected_benchmarks={len(manifest['expected_benchmarks'])}",
            flush=True,
        )

        if args.validate_only:
            return 0

        if not args.output_dir:
            raise RunError("--output-dir is required unless --validate-only is used")
        output_dir = Path(args.output_dir).resolve()

        if args.verify_only:
            validate_artifacts(manifest, output_dir)
            print(f"phase=artifact-validation-complete output_dir={output_dir}", flush=True)
            return 0

        binary = locate_binary(args.binary)
        run_manifest(
            manifest,
            binary,
            output_dir,
            args.server_start_timeout,
            args.client_timeout,
            args.server_flush_timeout,
        )
        validate_artifacts(manifest, output_dir)
        print(f"phase=run-complete output_dir={output_dir}", flush=True)
        return 0
    except (ManifestError, RunError) as err:
        fail(str(err))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
