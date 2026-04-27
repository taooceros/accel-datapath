from __future__ import annotations

import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, Mapping, Sequence, Type

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
EXPECTED_ENDPOINT_ROLES = ("client", "server")


ContextDict = Mapping[str, object]


def fail(message: str) -> None:
    print(message, file=sys.stderr)


def required_str(obj: Dict[str, Any], key: str, *, scope: str, error_cls: Type[Exception]) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value:
        raise error_cls(f"{scope} missing {key}")
    return value


def required_bool(obj: Dict[str, Any], key: str, *, scope: str, error_cls: Type[Exception]) -> bool:
    value = obj.get(key)
    if not isinstance(value, bool):
        raise error_cls(f"{scope} field {key} must be a boolean")
    return value


def required_positive_int(
    obj: Dict[str, Any], key: str, *, scope: str, error_cls: Type[Exception]
) -> int:
    value = obj.get(key)
    if not isinstance(value, int) or value <= 0:
        raise error_cls(f"{scope} field {key} must be a positive integer")
    return value


def locate_binary(
    explicit: str | None,
    *,
    default_path: Path,
    error_cls: Type[Exception],
    missing_message: str,
) -> Path:
    path = Path(explicit) if explicit else default_path
    if not path.exists():
        raise error_cls(missing_message.format(path=path))
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


def _context_line(
    *,
    label: str,
    phase: str,
    endpoint_role: str,
    artifact_path: Path,
    extras: ContextDict | None = None,
) -> str:
    parts = [
        f"label={label}",
        f"phase={phase}",
        f"endpoint_role={endpoint_role}",
        f"artifact={artifact_path}",
    ]
    if extras:
        for key, value in extras.items():
            parts.append(f"{key}={value}")
    return " ".join(parts)


def terminate_process(
    proc: subprocess.Popen[bytes],
    *,
    fail_fn: Callable[[str], None],
    label: str,
    phase: str,
    artifact_path: Path,
    extras: ContextDict | None = None,
) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        fail_fn(
            _context_line(
                label=label,
                phase=phase,
                endpoint_role="server",
                artifact_path=artifact_path,
                extras={**(dict(extras or {})), "warning": "sigkill-after-sigterm"},
            )
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
    error_cls: Type[Exception],
    extras: ContextDict | None = None,
) -> None:
    host, port_str = addr.rsplit(":", 1)
    port = int(port_str)
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            stdout, stderr = drain_process_output(proc)
            raise error_cls(
                "\n".join(
                    [
                        _context_line(
                            label=label,
                            phase="server-startup",
                            endpoint_role="server",
                            artifact_path=artifact_path,
                            extras={**(dict(extras or {})), "exit": proc.returncode},
                        ),
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
    raise error_cls(
        _context_line(
            label=label,
            phase="server-startup",
            endpoint_role="server",
            artifact_path=artifact_path,
            extras={**(dict(extras or {})), "timeout_s": f"{timeout_s:.1f}"},
        )
    )


def build_rpc_args(entry: Dict[str, Any], addr: str) -> list[str]:
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
        args.extend([
            "--payload-size",
            str(entry["payload_size"]),
            "--payload-kind",
            entry["payload_kind"],
        ])
    else:
        args.extend([
            "--proto-shape",
            entry["proto_shape"],
            "--response-shape",
            entry["response_shape"],
        ])
    return args


def run_server_client_pair(
    *,
    label: str,
    server_cmd: Sequence[str],
    client_cmd: Sequence[str],
    server_artifact: Path,
    client_artifact: Path,
    bind_addr: str,
    target_addr: str,
    server_start_timeout: float,
    client_timeout: float,
    server_flush_timeout: float,
    error_cls: Type[Exception],
    fail_fn: Callable[[str], None],
    server_popen_kwargs: Dict[str, Any] | None = None,
    client_run_kwargs: Dict[str, Any] | None = None,
    context: ContextDict | None = None,
    cleanup_phase: str = "cleanup",
) -> None:
    server_artifact.parent.mkdir(parents=True, exist_ok=True)
    client_artifact.parent.mkdir(parents=True, exist_ok=True)

    server: subprocess.Popen[bytes] | None = None
    try:
        print(
            _context_line(
                label=label,
                phase="server-startup",
                endpoint_role="server",
                artifact_path=server_artifact,
                extras={**(dict(context or {})), "bind": bind_addr},
            ),
            flush=True,
        )
        server = subprocess.Popen(  # noqa: S603
            list(server_cmd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            **(server_popen_kwargs or {}),
        )
        wait_for_port(
            server,
            bind_addr,
            server_start_timeout,
            label=label,
            artifact_path=server_artifact,
            error_cls=error_cls,
            extras=context,
        )

        print(
            _context_line(
                label=label,
                phase="client-execution",
                endpoint_role="client",
                artifact_path=client_artifact,
                extras={**(dict(context or {})), "target": target_addr},
            ),
            flush=True,
        )
        result = subprocess.run(  # noqa: S603
            list(client_cmd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=client_timeout,
            check=False,
            **(client_run_kwargs or {}),
        )
        if result.returncode != 0:
            raise error_cls(
                "\n".join(
                    [
                        _context_line(
                            label=label,
                            phase="client-execution",
                            endpoint_role="client",
                            artifact_path=client_artifact,
                            extras={**(dict(context or {})), "exit": result.returncode},
                        ),
                        f"stdout:\n{result.stdout.decode('utf-8', errors='replace')}",
                        f"stderr:\n{result.stderr.decode('utf-8', errors='replace')}",
                    ]
                )
            )

        try:
            server.wait(timeout=server_flush_timeout)
        except subprocess.TimeoutExpired as err:
            raise error_cls(
                _context_line(
                    label=label,
                    phase="server-flush",
                    endpoint_role="server",
                    artifact_path=server_artifact,
                    extras={**(dict(context or {})), "timeout_s": f"{server_flush_timeout:.1f}"},
                )
            ) from err

        if server.returncode != 0:
            stdout, stderr = drain_process_output(server)
            raise error_cls(
                "\n".join(
                    [
                        _context_line(
                            label=label,
                            phase="server-flush",
                            endpoint_role="server",
                            artifact_path=server_artifact,
                            extras={**(dict(context or {})), "exit": server.returncode},
                        ),
                        f"stdout:\n{stdout}",
                        f"stderr:\n{stderr}",
                    ]
                )
            )
        print(
            _context_line(
                label=label,
                phase="server-flush",
                endpoint_role="server",
                artifact_path=server_artifact,
                extras={**(dict(context or {})), "verdict": "pass"},
            ),
            flush=True,
        )
    except subprocess.TimeoutExpired as err:
        raise error_cls(
            _context_line(
                label=label,
                phase="client-execution",
                endpoint_role="client",
                artifact_path=client_artifact,
                extras={**(dict(context or {})), "timeout_s": f"{client_timeout:.1f}"},
            )
        ) from err
    finally:
        if server is not None:
            terminate_process(
                server,
                fail_fn=fail_fn,
                label=label,
                phase=cleanup_phase,
                artifact_path=server_artifact,
                extras=context,
            )


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
