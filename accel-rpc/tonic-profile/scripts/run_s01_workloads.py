#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_MANIFEST = SCRIPT_DIR.parent / "workloads" / "s01_ordinary_matrix.json"
REQUIRED_STAGE_NAMES = [
    "encode",
    "decode",
    "compress",
    "decompress",
    "buffer_reserve",
    "body_accum",
    "frame_header",
]
REQUIRED_STAGE_COUNTER_FIELDS = ["count", "nanos", "millis", "bytes", "avg_nanos"]


class ManifestError(RuntimeError):
    pass


class RunError(RuntimeError):
    pass


def fail(message: str) -> None:
    print(message, file=sys.stderr)


def load_manifest(path: Path) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise ManifestError(f"manifest read failed ({path}): {err}") from err
    try:
        manifest = json.loads(raw)
    except json.JSONDecodeError as err:
        raise ManifestError(f"manifest parse failed ({path}): {err}") from err
    if not isinstance(manifest, dict):
        raise ManifestError("manifest root must be an object")

    defaults = manifest.get("defaults", {})
    if defaults is None:
        defaults = {}
    if not isinstance(defaults, dict):
        raise ManifestError("manifest.defaults must be an object")

    workloads = manifest.get("workloads")
    if not isinstance(workloads, list) or not workloads:
        raise ManifestError("manifest.workloads must be a non-empty array")

    validated: List[Dict[str, Any]] = []
    seen_labels = set()
    for index, entry in enumerate(workloads):
        validated.append(validate_workload(index, entry, defaults, seen_labels))

    return {"version": manifest.get("version"), "defaults": defaults, "workloads": validated}


def validate_workload(
    index: int, entry: Any, defaults: Dict[str, Any], seen_labels: set[str]
) -> Dict[str, Any]:
    if not isinstance(entry, dict):
        raise ManifestError(f"workloads[{index}] must be an object")

    label = required_str(entry, index, "label")
    if label in seen_labels:
        raise ManifestError(f"workloads[{index}] label={label!r} is duplicated")
    seen_labels.add(label)

    rpc = required_str(entry, index, "rpc")
    pair = entry.get("artifact_pair")
    if not isinstance(pair, dict):
        raise ManifestError(
            f"workloads[{index}] label={label!r} missing artifact_pair object"
        )
    artifact_off = required_str(pair, index, "off", scope=f"workloads[{index}].artifact_pair")
    artifact_on = required_str(pair, index, "on", scope=f"workloads[{index}].artifact_pair")

    merged: Dict[str, Any] = dict(defaults)
    merged.update(entry)
    merged["label"] = label
    merged["rpc"] = rpc
    merged["artifact_pair"] = {"off": artifact_off, "on": artifact_on}

    for field in ["warmup_ms", "measure_ms", "requests", "concurrency"]:
        if field not in merged:
            raise ManifestError(f"workloads[{index}] label={label!r} missing {field}")
        if not isinstance(merged[field], int) or merged[field] <= 0:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field {field} must be a positive integer"
            )

    for field in ["runtime", "compression", "buffer_policy"]:
        if field not in merged:
            raise ManifestError(f"workloads[{index}] label={label!r} missing {field}")
        if not isinstance(merged[field], str) or not merged[field]:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field {field} must be a non-empty string"
            )

    if rpc == "unary-bytes":
        if "payload_size" not in merged:
            raise ManifestError(
                f"workloads[{index}] label={label!r} missing payload_size for rpc unary-bytes"
            )
        if "payload_kind" not in merged:
            raise ManifestError(
                f"workloads[{index}] label={label!r} missing payload_kind for rpc unary-bytes"
            )
        if not isinstance(merged["payload_size"], int) or merged["payload_size"] <= 0:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field payload_size must be a positive integer"
            )
        if not isinstance(merged["payload_kind"], str) or not merged["payload_kind"]:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field payload_kind must be a non-empty string"
            )
    elif rpc == "unary-proto-shape":
        if "proto_shape" not in merged:
            raise ManifestError(
                f"workloads[{index}] label={label!r} missing proto_shape for rpc unary-proto-shape"
            )
        if "response_shape" not in merged:
            raise ManifestError(
                f"workloads[{index}] label={label!r} missing response_shape for rpc unary-proto-shape"
            )
        if not isinstance(merged["proto_shape"], str) or not merged["proto_shape"]:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field proto_shape must be a non-empty string"
            )
        if not isinstance(merged["response_shape"], str) or not merged["response_shape"]:
            raise ManifestError(
                f"workloads[{index}] label={label!r} field response_shape must be a non-empty string"
            )
    else:
        raise ManifestError(
            f"workloads[{index}] label={label!r} has unsupported rpc mode {rpc!r}"
        )

    return merged


def required_str(
    obj: Dict[str, Any], index: int, key: str, *, scope: str | None = None
) -> str:
    value = obj.get(key)
    label = scope or f"workloads[{index}]"
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{label} missing {key}")
    return value


def locate_binary(explicit: str | None) -> Path:
    if explicit:
        path = Path(explicit)
    else:
        path = Path("accel-rpc/target/release/tonic-profile")
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


def wait_for_port(addr: str, timeout_s: float) -> None:
    host, port_str = addr.rsplit(":", 1)
    port = int(port_str)
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.2)
            if sock.connect_ex((host, port)) == 0:
                return
        time.sleep(0.05)
    raise RunError(f"server startup timeout after {timeout_s:.1f}s for {addr}")


def terminate_process(proc: subprocess.Popen[bytes], label: str, phase: str) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        fail(f"label={label} phase={phase} process did not exit after SIGTERM; sending SIGKILL")
        proc.kill()
        proc.wait(timeout=2)


def build_common_args(entry: Dict[str, Any], addr: str) -> List[str]:
    args = [
        "--rpc",
        entry["rpc"],
        "--bind",
        addr,
        "--target",
        addr,
        "--warmup-ms",
        str(entry["warmup_ms"]),
        "--measure-ms",
        str(entry["measure_ms"]),
        "--requests",
        str(entry["requests"]),
        "--concurrency",
        str(entry["concurrency"]),
        "--runtime",
        entry["runtime"],
        "--compression",
        entry["compression"],
        "--buffer-policy",
        entry["buffer_policy"],
    ]
    if entry["rpc"] == "unary-bytes":
        args.extend(
            [
                "--payload-size",
                str(entry["payload_size"]),
                "--payload-kind",
                entry["payload_kind"],
            ]
        )
    else:
        args.extend(
            [
                "--proto-shape",
                entry["proto_shape"],
                "--response-shape",
                entry["response_shape"],
            ]
        )
    return args


def run_manifest(
    manifest: Dict[str, Any],
    binary: Path,
    output_dir: Path,
    server_start_timeout: float,
    client_timeout: float,
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for entry in manifest["workloads"]:
        label = entry["label"]
        for mode in ["off", "on"]:
            addr = reserve_addr()
            common_args = build_common_args(entry, addr)
            artifact_name = entry["artifact_pair"][mode]
            artifact_path = output_dir / artifact_name
            server: subprocess.Popen[bytes] | None = None
            try:
                server_cmd = [
                    str(binary),
                    "--mode",
                    "server",
                    "--instrumentation",
                    mode,
                    *common_args,
                ]
                print(
                    f"label={label} phase=server-startup instrumentation={mode} bind={addr} binary={binary}",
                    flush=True,
                )
                server = subprocess.Popen(
                    server_cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                wait_for_port(addr, server_start_timeout)

                client_cmd = [
                    str(binary),
                    "--mode",
                    "client",
                    "--instrumentation",
                    mode,
                    *common_args,
                    "--json-out",
                    str(artifact_path),
                ]
                print(
                    f"label={label} phase=client-execution instrumentation={mode} artifact={artifact_path}",
                    flush=True,
                )
                result = subprocess.run(
                    client_cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=client_timeout,
                    check=False,
                )
                if result.returncode != 0:
                    raise RunError(
                        "\n".join(
                            [
                                f"label={label} phase=client-execution instrumentation={mode} exit={result.returncode}",
                                f"artifact={artifact_path}",
                                f"stdout:\n{result.stdout.decode('utf-8', errors='replace')}",
                                f"stderr:\n{result.stderr.decode('utf-8', errors='replace')}",
                            ]
                        )
                    )
            except subprocess.TimeoutExpired as err:
                raise RunError(
                    f"label={label} phase=client-execution instrumentation={mode} timeout after {client_timeout:.1f}s"
                ) from err
            except RunError as err:
                raise err
            except Exception as err:
                raise RunError(
                    f"label={label} phase=server-startup instrumentation={mode} {err}"
                ) from err
            finally:
                if server is not None:
                    terminate_process(server, label, f"server-shutdown instrumentation={mode}")


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


def validate_artifacts(manifest: Dict[str, Any], output_dir: Path) -> None:
    for entry in manifest["workloads"]:
        label = entry["label"]
        for mode in ["off", "on"]:
            artifact_path = output_dir / entry["artifact_pair"][mode]
            if not artifact_path.exists():
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={mode} missing artifact={artifact_path}"
                )
            report = parse_json(artifact_path)
            metadata = require_key(report, artifact_path, "metadata")
            metrics = require_key(report, artifact_path, "metrics")
            stages = require_key(report, artifact_path, "stages")
            if not isinstance(metadata, dict):
                raise RunError(f"artifact validation failed path={artifact_path}: metadata must be an object")
            if not isinstance(metrics, dict):
                raise RunError(f"artifact validation failed path={artifact_path}: metrics must be an object")
            if not isinstance(stages, dict):
                raise RunError(f"artifact validation failed path={artifact_path}: stages must be an object")

            assert_metadata(entry, mode, artifact_path, metadata)
            enabled = require_key(stages, artifact_path, "enabled")
            expected_enabled = mode == "on"
            if enabled is not expected_enabled:
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} stages.enabled={enabled!r} expected {expected_enabled!r}"
                )
            if mode == "on":
                for stage_name in REQUIRED_STAGE_NAMES:
                    stage = require_key(stages, artifact_path, stage_name)
                    if not isinstance(stage, dict):
                        raise RunError(
                            f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} stages.{stage_name} must be an object"
                        )
                    for counter_field in REQUIRED_STAGE_COUNTER_FIELDS:
                        if counter_field not in stage:
                            raise RunError(
                                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} missing stages.{stage_name}.{counter_field}"
                            )
            if not isinstance(metrics.get("requests_completed"), int):
                raise RunError(
                    f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} missing integer metrics.requests_completed"
                )
            print(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path}",
                flush=True,
            )


def assert_metadata(entry: Dict[str, Any], mode: str, artifact_path: Path, metadata: Dict[str, Any]) -> None:
    label = entry["label"]
    expected_pairs = {
        "ordinary_path": "software",
        "seam": "codec_body",
        "rpc": entry["rpc"],
        "workload_label": label,
        "instrumentation": mode,
    }
    for key, expected in expected_pairs.items():
        actual = metadata.get(key)
        if actual != expected:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.{key}={actual!r} expected {expected!r}"
            )

    if entry["rpc"] == "unary-bytes":
        if metadata.get("payload_size") != entry["payload_size"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.payload_size mismatch"
            )
        if metadata.get("payload_kind") != entry["payload_kind"]:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.payload_kind mismatch"
            )
    else:
        expected_request_shape = entry["proto_shape"]
        expected_response_shape = (
            entry["proto_shape"]
            if entry["response_shape"] == "same"
            else entry["response_shape"]
        )
        if metadata.get("request_shape") != expected_request_shape:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.request_shape mismatch"
            )
        if metadata.get("response_shape") != expected_response_shape:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.response_shape mismatch"
            )
    for field in ["request_serialized_size", "response_serialized_size"]:
        value = metadata.get(field)
        if not isinstance(value, int) or value <= 0:
            raise RunError(
                f"label={label} phase=artifact-validation instrumentation={mode} artifact={artifact_path} metadata.{field} must be a positive integer"
            )


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Run or validate the curated S01 ordinary-path workload matrix."
    )
    parser.add_argument("--manifest", default=str(DEFAULT_MANIFEST))
    parser.add_argument("--output-dir")
    parser.add_argument("--binary")
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--server-start-timeout", type=float, default=10.0)
    parser.add_argument("--client-timeout", type=float, default=20.0)
    args = parser.parse_args(list(argv) if argv is not None else None)

    if args.validate_only and args.verify_only:
        fail("--validate-only and --verify-only are mutually exclusive")
        return 2

    try:
        manifest = load_manifest(Path(args.manifest))
        print(
            f"phase=manifest-parse manifest={Path(args.manifest).resolve()} workloads={len(manifest['workloads'])}",
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
        )
        print(f"phase=run-complete output_dir={output_dir}", flush=True)
        return 0
    except (ManifestError, RunError) as err:
        fail(str(err))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
