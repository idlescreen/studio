//! JSON job contract for Studio (and any other driver).
//!
//! Studio writes this file; `render --job-file` runs it. Capability lives here.

use crate::duration::parse_duration_secs;
use crate::encode::EncodeBackend;
use crate::error::RenderError;
use crate::json::Value;
use crate::models::{Container, OutputFormat, RenderJob};
use std::path::{Path, PathBuf};

/// Serializable render request (string durations, JSON-friendly).
#[derive(Debug, Clone, PartialEq)]
pub struct JobSpec {
    pub effect: String,
    pub plugin_path: Option<PathBuf>,
    pub seed: u64,
    pub fps: u32,
    /// Human duration: `10s`, `5m`, `2h`, `1d`, or bare seconds.
    pub duration: String,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub cols: Option<usize>,
    pub rows: Option<usize>,
    pub dry_run: bool,
    pub raw: bool,
    pub segment: Option<String>,
    pub audio: Option<PathBuf>,
    pub resume: bool,
    pub crf: u8,
    pub preset: Option<String>,
    pub encoder: Option<String>,
    /// Prefer hardware AV1 when auto-detecting (default true).
    pub prefer_hw: bool,
    /// Request GPU upscale path (default true).
    pub gpu_upscale: bool,
    /// Output family (`mp4` / `png` / `raw`). Default `mp4`.
    pub format: Option<String>,
    /// Container (`mp4` / `mkv`). Default `mkv`.
    pub container: Option<String>,
    /// Optional baseline directory for snapshot comparison.
    pub baseline_dir: Option<PathBuf>,
    /// Only compare final frame against baseline.
    pub snapshot_last_only: bool,
    /// Overwrite baseline files instead of comparing.
    pub update_baselines: bool,
    /// Force CPU raster (deterministic; bypass GPU variance).
    pub cpu_raster: bool,
}

fn default_seed() -> u64 {
    0x00C0_FFEE
}
fn default_fps() -> u32 {
    30
}
fn default_w() -> u32 {
    1280
}
fn default_h() -> u32 {
    720
}
fn default_crf() -> u8 {
    35
}

fn parse_format(s: &str) -> Result<OutputFormat, RenderError> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "mp4" | "video" => OutputFormat::Mp4,
        "png" | "pngs" | "png-sequence" | "png_sequence" => OutputFormat::Png,
        "raw" | "stdout" | "stdout-raw" | "stdout_raw" => OutputFormat::Raw,
        other => return Err(RenderError::Job(format!("unknown format '{other}'"))),
    })
}
fn parse_container(s: &str) -> Result<Container, RenderError> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "mkv" | "matroska" => Container::Mkv,
        "mp4" => Container::Mp4,
        other => return Err(RenderError::Job(format!("unknown container '{other}'"))),
    })
}

mod job;
mod serde;

impl JobSpec {
    pub fn load_path(path: &Path) -> Result<Self, RenderError> {
        let raw = std::fs::read_to_string(path).map_err(|source| RenderError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_json(&raw)
            .map_err(|e| RenderError::Job(format!("invalid job file {}: {e}", path.display())))
    }

    /// Parse a JSON document into a [`JobSpec`] (serde-equivalent semantics:
    /// unknown keys ignored, missing fields take defaults, required fields
    /// `effect`/`duration`/`output` must be present).
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let v = crate::json::parse(raw).map_err(|e| e.to_string())?;
        Self::from_value(&v)
    }

    /// Serialize to a pretty-printed JSON document (`to_string_pretty` style).
    pub fn to_json(&self) -> String {
        self.to_value().to_json_pretty()
    }
}

#[cfg(test)]
#[path = "job_spec_tests.rs"]
mod tests;
