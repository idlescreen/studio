// SPDX-License-Identifier: Apache-2.0

use super::*;

pub struct Args {
    /// JSON job file (Studio / automation). When set, flag fields below are ignored.
    pub job_file: Option<PathBuf>,

    /// Effect name (allowlisted saver basename, e.g. ripple)
    pub effect: Option<String>,

    /// Explicit path to plugin .so (skips discovery)
    pub plugin_path: Option<PathBuf>,

    /// RNG seed
    pub seed: u64,

    /// Output timeline fps
    pub fps: u32,

    /// Duration: 10s, 5m, 2h, 1d (or bare seconds)
    pub duration: String,

    /// Optional segment length for long encodes (e.g. 1h)
    pub segment: Option<String>,

    /// Optional audio bed (muxed after video)
    pub audio: Option<PathBuf>,

    /// Output path (.mkv recommended). Optional with `--dry-run`/`--stdout-raw`.
    pub output: Option<PathBuf>,

    /// Pixel width
    pub width: u32,

    /// Pixel height
    pub height: u32,

    /// Optional simulation grid columns
    pub cols: Option<usize>,

    /// Optional simulation grid rows
    pub rows: Option<usize>,

    /// Validate and print plan only
    pub dry_run: bool,

    /// Write raw BGRA dump instead of AV1 (debug/tests). Alias for `--format raw`.
    pub raw: bool,

    /// Resume: skip encode for existing non-empty segment parts
    pub resume: bool,

    /// AV1 quality 0–63 (CRF / CQ). Default 35.
    pub crf: u8,

    /// Encoder preset (SVT numeric or NVENC p1–p7)
    pub preset: Option<String>,

    /// Force ffmpeg video encoder name
    pub encoder: Option<String>,

    /// Force software AV1 only
    pub no_hw_encode: bool,

    /// Force CPU upscale
    pub no_gpu_upscale: bool,

    /// Output family: `mp4` (video, default), `png` (per-frame), `raw` (stdout BGRA).
    pub format: CliFormat,

    /// Container for video output: `mkv` (AV1 default) or `mp4` (H.264 default).
    pub container: CliContainer,

    /// Stream raw BGRA + 16-byte header to stdout. Equivalent to `--format raw`.
    pub stdout_raw: bool,

    /// Snapshot compare directory (when set, --snapshot-last-only compares last frame).
    pub baseline_dir: Option<PathBuf>,

    /// Overwrite baseline files instead of comparing (dev only).
    pub update_baselines: bool,

    /// Only compare final frame against baseline (skips all-but-last encode work).
    pub snapshot_last_only: bool,

    /// Force CPU rendering path (deterministic; bypass GPU variance for tests).
    pub cpu_raster: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFormat {
    Mp4,
    Png,
    Raw,
}

impl CliFormat {
    pub(crate) fn parse(s: &str) -> Result<Self, String> {
        match s {
            "mp4" => Ok(Self::Mp4),
            "png" => Ok(Self::Png),
            "raw" => Ok(Self::Raw),
            _ => Err(format!(
                "invalid value '{s}' for '--format' [possible values: mp4, png, raw]"
            )),
        }
    }
}

impl From<CliFormat> for OutputFormat {
    fn from(v: CliFormat) -> Self {
        match v {
            CliFormat::Mp4 => OutputFormat::Mp4,
            CliFormat::Png => OutputFormat::Png,
            CliFormat::Raw => OutputFormat::Raw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliContainer {
    Mkv,
    Mp4,
}

impl CliContainer {
    pub(crate) fn parse(s: &str) -> Result<Self, String> {
        match s {
            "mkv" => Ok(Self::Mkv),
            "mp4" => Ok(Self::Mp4),
            _ => Err(format!(
                "invalid value '{s}' for '--container' [possible values: mkv, mp4]"
            )),
        }
    }
}

impl From<CliContainer> for Container {
    fn from(v: CliContainer) -> Self {
        match v {
            CliContainer::Mkv => Container::Mkv,
            CliContainer::Mp4 => Container::Mp4,
        }
    }
}

pub(crate) fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}\n\n{USAGE}");
    std::process::exit(2);
}
