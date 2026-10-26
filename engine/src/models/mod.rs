// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The render job model: output formats, the job itself, and its limits.

use std::path::PathBuf;
use std::time::Duration;

/// Pixel layout produced by the cell renderer (BGRA8888).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra8888,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        4
    }
}

/// Output family for a render job (added Sprint 02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// MP4 / MKV video (AV1 default, H.264 via `--format mp4 --container mp4`).
    #[default]
    Mp4,
    /// One PNG per frame (directory).
    Png,
    /// Raw BGRA + 16-byte GBRI header to stdout.
    Raw,
}

/// Largest accepted frame dimension (pixels) on either axis.
pub const MAX_FRAME_DIMENSION: u32 = 16_384;

/// Largest accepted frame size in pixels (8192 × 8192). The frame buffer is
/// 4 bytes per pixel, so this caps a single frame at 256 MiB.
pub const MAX_FRAME_PIXELS: u64 = 67_108_864;

/// Container for video outputs (only meaningful when [`OutputFormat::Mp4`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Container {
    /// Matroska (default; supports AV1, H.264).
    #[default]
    Mkv,
    /// ISOBMFF MP4 (H.264 default; AV1 also supported in modern ffmpeg).
    Mp4,
}

/// Fully validated offline render request.
#[derive(Debug, Clone)]
pub struct RenderJob {
    pub effect: String,
    pub plugin_path: Option<PathBuf>,
    pub seed: u64,
    pub fps: u32,
    pub duration: Duration,
    pub width: u32,
    pub height: u32,
    pub output: PathBuf,
    pub cols: Option<usize>,
    pub rows: Option<usize>,
    pub dry_run: bool,
    /// When set, encode in chunks of this length then concat.
    pub segment: Option<Duration>,
    /// Optional audio bed muxed after video (loop/shorten to video length).
    pub audio: Option<PathBuf>,
    /// Skip encoding segments whose part files already exist; still advances sim.
    pub resume: bool,
    /// AV1 CRF (lower = better/larger). Default 35.
    pub crf: u8,
    /// Optional encoder preset string (e.g. SVT-AV1 speed).
    pub preset: Option<String>,
    /// Optional forced ffmpeg encoder name.
    pub encoder: Option<String>,
    /// Prefer hardware AV1 (nvenc/qsv/amf) when auto-detecting encoder.
    pub prefer_hw: bool,
    /// GPU upscale in idle-runner (wgpu path when available).
    pub gpu_upscale: bool,
    /// Output family (mp4 / png / raw). Added Sprint 02.
    pub format: OutputFormat,
    /// Container for video outputs. Added Sprint 02.
    pub container: Container,
    /// Optional baseline directory for snapshot comparison.
    pub baseline_dir: Option<PathBuf>,
    /// When true, only the final frame is written and compared to baseline.
    pub snapshot_last_only: bool,
    /// When true, overwrite baseline files instead of comparing.
    pub update_baselines: bool,
    /// Force CPU raster/encoder path (deterministic; bypass GPU variance).
    pub cpu_raster: bool,
}

impl RenderJob {
    pub fn frame_count(&self) -> u64 {
        let secs = self.duration.as_secs_f64();
        if secs <= 0.0 || self.fps == 0 {
            return 0;
        }
        let n = (secs * f64::from(self.fps)).floor() as u64;
        n.max(1)
    }

    /// Number of segment files for a segmented encode (1 if no segment).
    pub fn segment_count(&self) -> u64 {
        let Some(seg) = self.segment else {
            return 1;
        };
        if seg.is_zero() {
            return 1;
        }
        let total = self.duration.as_secs_f64();
        let part = seg.as_secs_f64();
        if part <= 0.0 {
            return 1;
        }
        ((total / part).ceil() as u64).max(1)
    }
}

mod validate;

#[cfg(test)]
mod tests;
