#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

import claim_package_contract as claim_contract

REQUIRED_STAGE_NAMES = [
    "encode",
    "decode",
    "compress",
    "decompress",
    "buffer_reserve",
    "body_accum",
    "frame_header",
]
REQUIRED_METRIC_NAMES = [
    "requests_completed",
    "bytes_sent",
    "bytes_received",
    "duration_ms",
    "throughput_rps",
    "throughput_mib_s",
    "latency_us_p50",
    "latency_us_p95",
    "latency_us_p99",
    "latency_us_max",
]
REQUIRED_REPORT_METADATA = [
    "mode",
    "endpoint_role",
    "run_id",
    "selected_path",
    "workload_label",
    "instrumentation",
]
CSV_COLUMNS = [
    "workload_label",
    "endpoint_role",
    "device_path",
    "software_baseline_run_id",
    "software_attribution_run_id",
    "idxd_attribution_run_id",
    "software_baseline_throughput_rps",
    "software_attribution_throughput_rps",
    "idxd_attribution_throughput_rps",
    "idxd_vs_software_baseline_throughput_ratio",
    "idxd_vs_software_attribution_throughput_ratio",
    "software_baseline_latency_us_p50",
    "software_attribution_latency_us_p50",
    "idxd_attribution_latency_us_p50",
    "software_attribution_stage_nanos_total",
    "idxd_attribution_stage_nanos_total",
    "idxd_minus_software_attribution_stage_nanos_total",
]


class SummaryError(RuntimeError):
    pass


def fail(message: str) -> None:
    print(message, file=sys.stderr)


def require_dict(value: Any, *, scope: str) -> Dict[str, Any]:
    if not isinstance(value, dict):
        raise SummaryError(f"phase=summarization {scope} must be an object")
    return value


def require_value(parent: Dict[str, Any], key: str, *, scope: str) -> Any:
    if key not in parent:
        raise SummaryError(f"phase=summarization {scope} missing {key}")
    return parent[key]


def require_str(parent: Dict[str, Any], key: str, *, scope: str) -> str:
    value = require_value(parent, key, scope=scope)
    if not isinstance(value, str) or not value:
        raise SummaryError(f"phase=summarization {scope}.{key} must be a non-empty string")
    return value


def require_bool(parent: Dict[str, Any], key: str, *, scope: str) -> bool:
    value = require_value(parent, key, scope=scope)
    if not isinstance(value, bool):
        raise SummaryError(f"phase=summarization {scope}.{key} must be a bool")
    return value


def require_number(parent: Dict[str, Any], key: str, *, scope: str) -> float:
    value = require_value(parent, key, scope=scope)
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise SummaryError(f"phase=summarization {scope}.{key} must be numeric")
    return float(value)


def load_json_object(path: Path, *, phase: str, scope: str) -> Dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as err:
        raise SummaryError(f"phase={phase} {scope} read failed artifact={path}: {err}") from err
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as err:
        raise SummaryError(f"phase={phase} {scope} parse failed artifact={path}: {err}") from err
    if not isinstance(value, dict):
        raise SummaryError(f"phase={phase} {scope} root must be an object artifact={path}")
    return value


def resolve_run_root(manifest: Dict[str, Any], run_root_override: str | None) -> Path:
    raw = run_root_override or manifest["run_root"]
    return claim_contract.resolve_repo_path(raw).resolve()


def output_paths(manifest: Dict[str, Any], run_root: Path) -> Dict[str, Path]:
    return {
        key: run_root / relpath
        for key, relpath in manifest["derived_outputs"].items()
    }


def stage_totals(stages: Dict[str, Any]) -> Dict[str, float]:
    total_nanos = 0.0
    total_bytes = 0.0
    for stage_name in REQUIRED_STAGE_NAMES:
        stage = require_dict(stages.get(stage_name), scope=f"stages.{stage_name}")
        total_nanos += require_number(stage, "nanos", scope=f"stages.{stage_name}")
        total_bytes += require_number(stage, "bytes", scope=f"stages.{stage_name}")
    return {
        "total_nanos": total_nanos,
        "total_bytes": total_bytes,
    }


def safe_ratio(numerator: float, denominator: float) -> float | None:
    if denominator == 0:
        return None
    return numerator / denominator


def render_float(value: float | None, digits: int = 3) -> str:
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}"


def load_report(
    report_path: Path,
    *,
    label: str,
    endpoint_role: str,
    run_family: str,
    instrumentation: str,
    selected_path: str,
) -> Dict[str, Any]:
    context = (
        f"label={label} phase=summarization endpoint_role={endpoint_role} "
        f"run_family={run_family} artifact={report_path}"
    )
    report = load_json_object(report_path, phase="summarization", scope=context)
    metadata = require_dict(report.get("metadata"), scope=f"{context} metadata")
    metrics = require_dict(report.get("metrics"), scope=f"{context} metrics")
    stages = require_dict(report.get("stages"), scope=f"{context} stages")

    for field in REQUIRED_REPORT_METADATA:
        require_str(metadata, field, scope=f"{context} metadata")
    for field in REQUIRED_METRIC_NAMES:
        require_number(metrics, field, scope=f"{context} metrics")
    enabled = require_bool(stages, "enabled", scope=f"{context} stages")

    actual_role = metadata["endpoint_role"]
    actual_run_id = metadata["run_id"]
    actual_label = metadata["workload_label"]
    actual_instrumentation = metadata["instrumentation"]
    actual_selected_path = metadata["selected_path"]
    if actual_role != endpoint_role:
        raise SummaryError(
            f"{context} metadata.endpoint_role={actual_role!r} expected {endpoint_role!r}"
        )
    if actual_label != label:
        raise SummaryError(f"{context} metadata.workload_label={actual_label!r} expected {label!r}")
    if actual_instrumentation != instrumentation:
        raise SummaryError(
            f"{context} metadata.instrumentation={actual_instrumentation!r} expected {instrumentation!r}"
        )
    if actual_selected_path != selected_path:
        raise SummaryError(
            f"{context} metadata.selected_path={actual_selected_path!r} expected {selected_path!r}"
        )
    if (instrumentation == "on") != enabled:
        raise SummaryError(
            f"{context} stages.enabled={enabled!r} expected {instrumentation == 'on'!r}"
        )
    if not isinstance(actual_run_id, str) or not actual_run_id:
        raise SummaryError(f"{context} metadata.run_id must be a non-empty string")

    for stage_name in REQUIRED_STAGE_NAMES:
        stage = require_dict(stages.get(stage_name), scope=f"{context} stages.{stage_name}")
        for field in ["count", "nanos", "millis", "bytes", "avg_nanos"]:
            require_number(stage, field, scope=f"{context} stages.{stage_name}")

    device_path = metadata.get("accelerated_device_path")
    if selected_path == "idxd":
        if not isinstance(device_path, str) or not device_path:
            raise SummaryError(
                f"{context} metadata.accelerated_device_path must be a non-empty string for idxd summaries"
            )
    else:
        if device_path is not None:
            raise SummaryError(
                f"{context} metadata.accelerated_device_path must be null for software summaries"
            )

    return {
        "artifact": str(report_path),
        "metadata": metadata,
        "metrics": metrics,
        "stages": stages,
        "stage_totals": stage_totals(stages),
    }


def load_control_floor(manifest: Dict[str, Any]) -> Dict[str, Any]:
    control_floor_path = claim_contract.resolve_repo_path(manifest["inputs"]["control_floor_summary"])
    summary = load_json_object(
        control_floor_path,
        phase="control-floor-validation",
        scope=f"control_floor_summary={control_floor_path}",
    )
    benchmarks = require_dict(summary.get("benchmarks"), scope="control_floor_summary.benchmarks")

    software_manifest_path = claim_contract.resolve_repo_path(manifest["inputs"]["software_manifest"])
    software_manifest = load_json_object(
        software_manifest_path,
        phase="control-floor-validation",
        scope=f"software_manifest={software_manifest_path}",
    )
    expected_names = software_manifest.get("expected_benchmarks")
    if not isinstance(expected_names, list) or not expected_names:
        raise SummaryError(
            f"phase=control-floor-validation software_manifest={software_manifest_path} missing expected_benchmarks"
        )

    rendered: Dict[str, Dict[str, float | int | str]] = {}
    for raw_name in expected_names:
        if not isinstance(raw_name, str) or not raw_name:
            raise SummaryError(
                f"phase=control-floor-validation software_manifest={software_manifest_path} expected_benchmarks contains a non-string entry"
            )
        benchmark_value = benchmarks.get(raw_name)
        if not isinstance(benchmark_value, dict):
            raise SummaryError(
                f"phase=control-floor-validation control_floor_summary missing benchmark={raw_name}"
            )
        benchmark = benchmark_value
        benchmark_name = require_str(
            benchmark, "benchmark_name", scope=f"control_floor_summary.benchmarks.{raw_name}"
        )
        if benchmark_name != raw_name:
            raise SummaryError(
                f"phase=control-floor-validation control_floor_summary benchmark={raw_name} benchmark_name={benchmark_name!r} expected {raw_name!r}"
            )
        rendered[raw_name] = {
            "benchmark_name": benchmark_name,
            "mean_ns": require_number(
                benchmark, "mean_ns", scope=f"control_floor_summary.benchmarks.{raw_name}"
            ),
            "median_ns": require_number(
                benchmark, "median_ns", scope=f"control_floor_summary.benchmarks.{raw_name}"
            ),
            "std_dev_ns": require_number(
                benchmark, "std_dev_ns", scope=f"control_floor_summary.benchmarks.{raw_name}"
            ),
            "sample_count": int(
                require_number(
                    benchmark,
                    "sample_count",
                    scope=f"control_floor_summary.benchmarks.{raw_name}",
                )
            ),
        }
    return {
        "path": str(control_floor_path),
        "suite_name": summary.get("suite_name"),
        "schema_version": summary.get("schema_version"),
        "benchmarks": rendered,
    }


def load_family_reports(manifest: Dict[str, Any], run_root: Path) -> Dict[str, Dict[Tuple[str, str], Dict[str, Any]]]:
    loaded: Dict[str, Dict[Tuple[str, str], Dict[str, Any]]] = {}
    for family in manifest["artifact_families"]:
        run_family = family["run_family"]
        family_reports: Dict[Tuple[str, str], Dict[str, Any]] = {}
        for entry in family["endpoint_reports"]:
            label = entry["workload_label"]
            endpoint_role = entry["endpoint_role"]
            artifact_path = (run_root / entry["artifact"]).resolve()
            report = load_report(
                artifact_path,
                label=label,
                endpoint_role=endpoint_role,
                run_family=run_family,
                instrumentation=family["instrumentation"],
                selected_path=family["selected_path"],
            )
            family_reports[(label, endpoint_role)] = report
        loaded[run_family] = family_reports

    for family in manifest["artifact_families"]:
        run_family = family["run_family"]
        for label in manifest["scope"]["workload_labels"]:
            client = loaded[run_family][(label, "client")]
            server = loaded[run_family][(label, "server")]
            client_run_id = client["metadata"]["run_id"]
            server_run_id = server["metadata"]["run_id"]
            if client_run_id != server_run_id:
                raise SummaryError(
                    f"label={label} phase=pairing-mismatch endpoint_role=client/server run_family={run_family} "
                    f"artifact={run_root / family['selected_path']} run_id mismatch client={client_run_id!r} server={server_run_id!r}"
                )
    return loaded


def build_rows(
    manifest: Dict[str, Any],
    run_root: Path,
    family_reports: Dict[str, Dict[Tuple[str, str], Dict[str, Any]]],
) -> List[Dict[str, Any]]:
    rows: List[Dict[str, Any]] = []
    for label in manifest["scope"]["workload_labels"]:
        for endpoint_role in claim_contract.EXPECTED_ENDPOINT_ROLES:
            baseline = family_reports["software_baseline"][(label, endpoint_role)]
            sw_on = family_reports["software_attribution"][(label, endpoint_role)]
            idxd = family_reports["idxd_attribution"][(label, endpoint_role)]
            device_path = idxd["metadata"]["accelerated_device_path"]

            baseline_rps = float(baseline["metrics"]["throughput_rps"])
            sw_on_rps = float(sw_on["metrics"]["throughput_rps"])
            idxd_rps = float(idxd["metrics"]["throughput_rps"])
            baseline_p50 = float(baseline["metrics"]["latency_us_p50"])
            sw_on_p50 = float(sw_on["metrics"]["latency_us_p50"])
            idxd_p50 = float(idxd["metrics"]["latency_us_p50"])
            sw_on_stage_nanos = float(sw_on["stage_totals"]["total_nanos"])
            idxd_stage_nanos = float(idxd["stage_totals"]["total_nanos"])

            row = {
                "workload_label": label,
                "endpoint_role": endpoint_role,
                "device_path": device_path,
                "software_baseline": {
                    "artifact": baseline["artifact"],
                    "run_id": baseline["metadata"]["run_id"],
                    "throughput_rps": baseline_rps,
                    "latency_us_p50": baseline_p50,
                    "latency_us_p95": float(baseline["metrics"]["latency_us_p95"]),
                    "stage_nanos_total": float(baseline["stage_totals"]["total_nanos"]),
                    "stage_bytes_total": float(baseline["stage_totals"]["total_bytes"]),
                },
                "software_attribution": {
                    "artifact": sw_on["artifact"],
                    "run_id": sw_on["metadata"]["run_id"],
                    "throughput_rps": sw_on_rps,
                    "latency_us_p50": sw_on_p50,
                    "latency_us_p95": float(sw_on["metrics"]["latency_us_p95"]),
                    "stage_nanos_total": sw_on_stage_nanos,
                    "stage_bytes_total": float(sw_on["stage_totals"]["total_bytes"]),
                },
                "idxd_attribution": {
                    "artifact": idxd["artifact"],
                    "run_id": idxd["metadata"]["run_id"],
                    "throughput_rps": idxd_rps,
                    "latency_us_p50": idxd_p50,
                    "latency_us_p95": float(idxd["metrics"]["latency_us_p95"]),
                    "stage_nanos_total": idxd_stage_nanos,
                    "stage_bytes_total": float(idxd["stage_totals"]["total_bytes"]),
                },
                "comparisons": {
                    "throughput_baseline": {
                        "idxd_minus_software_baseline_rps": idxd_rps - baseline_rps,
                        "idxd_vs_software_baseline_throughput_ratio": safe_ratio(idxd_rps, baseline_rps),
                        "idxd_minus_software_baseline_latency_us_p50": idxd_p50 - baseline_p50,
                    },
                    "attribution": {
                        "idxd_minus_software_attribution_rps": idxd_rps - sw_on_rps,
                        "idxd_vs_software_attribution_throughput_ratio": safe_ratio(idxd_rps, sw_on_rps),
                        "idxd_minus_software_attribution_latency_us_p50": idxd_p50 - sw_on_p50,
                        "idxd_minus_software_attribution_stage_nanos_total": idxd_stage_nanos - sw_on_stage_nanos,
                    },
                },
            }
            rows.append(row)
            print(
                " ".join(
                    [
                        "phase=summarization",
                        f"label={label}",
                        f"endpoint_role={endpoint_role}",
                        "instrumentation=baseline+attribution",
                        f"run_root={run_root}",
                        f"device_path={device_path}",
                    ]
                ),
                flush=True,
            )
    return rows


def write_json(path: Path, value: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: List[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=CSV_COLUMNS)
        writer.writeheader()
        for row in rows:
            writer.writerow(
                {
                    "workload_label": row["workload_label"],
                    "endpoint_role": row["endpoint_role"],
                    "device_path": row["device_path"],
                    "software_baseline_run_id": row["software_baseline"]["run_id"],
                    "software_attribution_run_id": row["software_attribution"]["run_id"],
                    "idxd_attribution_run_id": row["idxd_attribution"]["run_id"],
                    "software_baseline_throughput_rps": render_float(row["software_baseline"]["throughput_rps"]),
                    "software_attribution_throughput_rps": render_float(row["software_attribution"]["throughput_rps"]),
                    "idxd_attribution_throughput_rps": render_float(row["idxd_attribution"]["throughput_rps"]),
                    "idxd_vs_software_baseline_throughput_ratio": render_float(
                        row["comparisons"]["throughput_baseline"]["idxd_vs_software_baseline_throughput_ratio"]
                    ),
                    "idxd_vs_software_attribution_throughput_ratio": render_float(
                        row["comparisons"]["attribution"]["idxd_vs_software_attribution_throughput_ratio"]
                    ),
                    "software_baseline_latency_us_p50": render_float(row["software_baseline"]["latency_us_p50"], 0),
                    "software_attribution_latency_us_p50": render_float(row["software_attribution"]["latency_us_p50"], 0),
                    "idxd_attribution_latency_us_p50": render_float(row["idxd_attribution"]["latency_us_p50"], 0),
                    "software_attribution_stage_nanos_total": render_float(
                        row["software_attribution"]["stage_nanos_total"], 0
                    ),
                    "idxd_attribution_stage_nanos_total": render_float(
                        row["idxd_attribution"]["stage_nanos_total"], 0
                    ),
                    "idxd_minus_software_attribution_stage_nanos_total": render_float(
                        row["comparisons"]["attribution"]["idxd_minus_software_attribution_stage_nanos_total"],
                        0,
                    ),
                }
            )


def write_claim_table(path: Path, rows: List[Dict[str, Any]], control_floor: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# S04 ordinary versus IDXD claim table",
        "",
        "## Workload and endpoint comparisons",
        "",
        "| workload_label | endpoint_role | sw baseline rps | sw attribution rps | idxd attribution rps | idxd/sw baseline | idxd/sw attribution | sw attribution stage ns | idxd attribution stage ns | device_path |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    for row in rows:
        lines.append(
            "| {workload_label} | {endpoint_role} | {sw_base} | {sw_attr} | {idxd_attr} | {base_ratio} | {attr_ratio} | {sw_stage} | {idxd_stage} | {device_path} |".format(
                workload_label=row["workload_label"],
                endpoint_role=row["endpoint_role"],
                sw_base=render_float(row["software_baseline"]["throughput_rps"]),
                sw_attr=render_float(row["software_attribution"]["throughput_rps"]),
                idxd_attr=render_float(row["idxd_attribution"]["throughput_rps"]),
                base_ratio=render_float(
                    row["comparisons"]["throughput_baseline"]["idxd_vs_software_baseline_throughput_ratio"]
                ),
                attr_ratio=render_float(
                    row["comparisons"]["attribution"]["idxd_vs_software_attribution_throughput_ratio"]
                ),
                sw_stage=render_float(row["software_attribution"]["stage_nanos_total"], 0),
                idxd_stage=render_float(row["idxd_attribution"]["stage_nanos_total"], 0),
                device_path=row["device_path"],
            )
        )
    lines.extend(
        [
            "",
            "## Async control-floor reference",
            "",
            "| benchmark | mean_ns | median_ns | std_dev_ns | sample_count |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for benchmark_name in sorted(control_floor["benchmarks"]):
        benchmark = control_floor["benchmarks"][benchmark_name]
        lines.append(
            "| {benchmark} | {mean_ns} | {median_ns} | {std_dev_ns} | {sample_count} |".format(
                benchmark=benchmark_name,
                mean_ns=render_float(float(benchmark["mean_ns"])),
                median_ns=render_float(float(benchmark["median_ns"])),
                std_dev_ns=render_float(float(benchmark["std_dev_ns"])),
                sample_count=benchmark["sample_count"],
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def verify_outputs(paths: Dict[str, Path]) -> None:
    for key, path in paths.items():
        if not path.exists():
            raise SummaryError(f"phase=output-validation missing {key} output artifact={path}")
        if path.stat().st_size == 0:
            raise SummaryError(f"phase=output-validation empty {key} output artifact={path}")


def build_summary_document(
    manifest_path: Path,
    run_root: Path,
    manifest: Dict[str, Any],
    rows: List[Dict[str, Any]],
    control_floor: Dict[str, Any],
    outputs: Dict[str, Path],
) -> Dict[str, Any]:
    return {
        "schema_version": 1,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "manifest_path": str(manifest_path),
        "run_root": str(run_root),
        "pairing_keys": manifest["scope"]["pairing_keys"],
        "summary_outputs": {
            key: str(path) for key, path in outputs.items()
        },
        "control_floor": control_floor,
        "rows": rows,
    }


def summarize(manifest_path: Path, run_root_override: str | None, verify_only: bool) -> Dict[str, Path]:
    manifest = claim_contract.load_manifest(manifest_path)
    run_root = resolve_run_root(manifest, run_root_override)
    outputs = output_paths(manifest, run_root)
    print(
        " ".join(
            [
                "phase=summarization-start",
                f"manifest={manifest_path}",
                f"run_root={run_root}",
                f"summary_path={outputs['comparison_summary_json']}",
                f"csv_path={outputs['ordinary_vs_idxd_csv']}",
                f"claim_table_path={outputs['claim_table_md']}",
            ]
        ),
        flush=True,
    )

    control_floor = load_control_floor(manifest)
    family_reports = load_family_reports(manifest, run_root)
    rows = build_rows(manifest, run_root, family_reports)
    summary_doc = build_summary_document(manifest_path, run_root, manifest, rows, control_floor, outputs)

    write_json(outputs["comparison_summary_json"], summary_doc)
    write_csv(outputs["ordinary_vs_idxd_csv"], rows)
    write_claim_table(outputs["claim_table_md"], rows, control_floor)
    verify_outputs(outputs)

    print(
        " ".join(
            [
                "phase=summarization-done",
                f"run_root={run_root}",
                f"verify_only={str(verify_only).lower()}",
                f"summary_path={outputs['comparison_summary_json']}",
                f"csv_path={outputs['ordinary_vs_idxd_csv']}",
                f"claim_table_path={outputs['claim_table_md']}",
                f"control_floor_path={control_floor['path']}",
            ]
        ),
        flush=True,
    )
    return outputs


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Summarize the frozen S04 ordinary-versus-IDXD run tree into stable JSON/CSV/Markdown outputs."
    )
    parser.add_argument("--manifest", default=str(claim_contract.DEFAULT_MANIFEST))
    parser.add_argument("--run-root", help="Override the manifest run_root for fixture-driven verification")
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Stay local to existing artifacts; this still regenerates the deterministic summary outputs.",
    )
    args = parser.parse_args(list(argv) if argv is not None else None)

    try:
        manifest_path = Path(args.manifest).resolve()
        summarize(manifest_path, args.run_root, args.verify_only)
        return 0
    except (claim_contract.ManifestError, SummaryError) as err:
        fail(str(err))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
