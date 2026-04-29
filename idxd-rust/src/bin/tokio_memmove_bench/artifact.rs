use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::{CliArgs, OutputFormat};

pub(crate) const SCHEMA_VERSION: u32 = 1;
pub(crate) const SOFTWARE_TARGET: &str = "software_direct_async_diagnostic";
pub(crate) const HARDWARE_ASYNC_TARGET: &str = "direct_async";
pub(crate) const HARDWARE_SYNC_TARGET: &str = "direct_sync";

#[derive(Debug, Serialize)]
pub(crate) struct BenchmarkArtifact {
    pub(crate) schema_version: u32,
    pub(crate) ok: bool,
    pub(crate) verdict: &'static str,
    pub(crate) device_path: String,
    pub(crate) backend: &'static str,
    pub(crate) claim_eligible: bool,
    pub(crate) suite: &'static str,
    pub(crate) runtime_flavor: &'static str,
    pub(crate) worker_threads: u32,
    pub(crate) requested_bytes: usize,
    pub(crate) iterations: u64,
    pub(crate) concurrency: u32,
    pub(crate) duration_ms: u64,
    pub(crate) failure_class: Option<&'static str>,
    pub(crate) error_kind: Option<&'static str>,
    pub(crate) direct_failure_kind: Option<&'static str>,
    pub(crate) validation_phase: Option<&'static str>,
    pub(crate) validation_error_kind: Option<&'static str>,
    pub(crate) direct_retry_budget: Option<u32>,
    pub(crate) direct_retry_count: Option<u32>,
    pub(crate) completion_status: Option<String>,
    pub(crate) completion_result: Option<u8>,
    pub(crate) completion_bytes_completed: Option<u32>,
    pub(crate) completion_fault_addr: Option<String>,
    pub(crate) results: Vec<BenchmarkResult>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BenchmarkResult {
    pub(crate) mode: &'static str,
    pub(crate) target: &'static str,
    pub(crate) comparison_target: Option<&'static str>,
    pub(crate) requested_bytes: usize,
    pub(crate) iterations: u64,
    pub(crate) concurrency: u32,
    pub(crate) duration_ms: u64,
    pub(crate) completed_operations: u64,
    pub(crate) failed_operations: u64,
    pub(crate) elapsed_ns: u128,
    pub(crate) min_latency_ns: Option<u128>,
    pub(crate) mean_latency_ns: Option<u128>,
    pub(crate) max_latency_ns: Option<u128>,
    pub(crate) ops_per_sec: Option<f64>,
    pub(crate) bytes_per_sec: Option<f64>,
    pub(crate) verdict: &'static str,
    pub(crate) failure_class: Option<&'static str>,
    pub(crate) error_kind: Option<&'static str>,
    pub(crate) direct_failure_kind: Option<&'static str>,
    pub(crate) validation_phase: Option<&'static str>,
    pub(crate) validation_error_kind: Option<&'static str>,
    pub(crate) direct_retry_budget: Option<u32>,
    pub(crate) direct_retry_count: Option<u32>,
    pub(crate) completion_status: Option<String>,
    pub(crate) completion_result: Option<u8>,
    pub(crate) completion_bytes_completed: Option<u32>,
    pub(crate) completion_fault_addr: Option<String>,
    pub(crate) claim_eligible: bool,
}

pub(crate) fn emit_artifact(args: &CliArgs, artifact: &BenchmarkArtifact) -> Result<(), String> {
    let rendered = match args.format {
        OutputFormat::Json => serde_json::to_string(artifact)
            .map_err(|err| format!("failed to serialize benchmark artifact: {err}"))?,
        OutputFormat::Text => render_text(artifact),
    };

    if let Some(path) = &args.artifact_path {
        write_artifact(path, &rendered)?;
    }

    println!("{rendered}");
    Ok(())
}

fn render_text(artifact: &BenchmarkArtifact) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "verdict={} ok={} backend={} suite={} claim_eligible={}",
        artifact.verdict, artifact.ok, artifact.backend, artifact.suite, artifact.claim_eligible
    );
    for result in &artifact.results {
        let _ = writeln!(
            out,
            "mode={} target={} completed_operations={} failed_operations={} verdict={}",
            result.mode,
            result.target,
            result.completed_operations,
            result.failed_operations,
            result.verdict
        );
    }
    out.trim_end().to_string()
}

fn write_artifact(path: &Path, rendered: &str) -> Result<(), String> {
    validate_artifact_path(path)?;
    let temp_path = temporary_artifact_path(path)?;
    fs::write(&temp_path, rendered)
        .map_err(|err| format!("failed to write artifact `{}`: {err}", path.display()))?;
    fs::rename(&temp_path, path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!("failed to commit artifact `{}`: {err}", path.display())
    })?;
    Ok(())
}

fn temporary_artifact_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "artifact path must include a valid UTF-8 file name".to_string())?;
    Ok(path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id())))
}

pub(crate) fn validate_artifact_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("artifact path must not be empty".to_string());
    }
    if path.exists() {
        let metadata = path.metadata().map_err(|err| {
            format!(
                "failed to inspect artifact path `{}`: {err}",
                path.display()
            )
        })?;
        if metadata.is_dir() {
            return Err(format!(
                "artifact path `{}` expected a writable file path, found directory",
                path.display()
            ));
        }
        if metadata.permissions().readonly() {
            return Err(format!(
                "artifact path `{}` expected a writable file path, found readonly file",
                path.display()
            ));
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        let metadata = parent.metadata().map_err(|err| {
            format!(
                "artifact path `{}` expected an existing writable parent directory: {err}",
                path.display()
            )
        })?;
        if !metadata.is_dir() || metadata.permissions().readonly() {
            return Err(format!(
                "artifact path `{}` expected a writable parent directory",
                path.display()
            ));
        }
    }
    Ok(())
}
