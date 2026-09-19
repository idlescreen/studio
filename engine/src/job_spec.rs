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

    /// Build the JSON object for this spec (all fields, declaration order —
    /// matches the previous serde output).
    pub fn to_value(&self) -> Value {
        fn opt_str(o: &Option<impl AsRef<str>>) -> Value {
            match o {
                Some(s) => Value::Str(s.as_ref().to_string()),
                None => Value::Null,
            }
        }
        fn opt_path(o: &Option<PathBuf>) -> Value {
            match o {
                Some(p) => Value::Str(p.display().to_string()),
                None => Value::Null,
            }
        }
        Value::Object(vec![
            ("effect".into(), Value::Str(self.effect.clone())),
            ("plugin_path".into(), opt_path(&self.plugin_path)),
            ("seed".into(), Value::UInt(self.seed)),
            ("fps".into(), Value::UInt(self.fps as u64)),
            ("duration".into(), Value::Str(self.duration.clone())),
            (
                "output".into(),
                Value::Str(self.output.display().to_string()),
            ),
            ("width".into(), Value::UInt(self.width as u64)),
            ("height".into(), Value::UInt(self.height as u64)),
            (
                "cols".into(),
                self.cols.map_or(Value::Null, |v| Value::UInt(v as u64)),
            ),
            (
                "rows".into(),
                self.rows.map_or(Value::Null, |v| Value::UInt(v as u64)),
            ),
            ("dry_run".into(), Value::Bool(self.dry_run)),
            ("raw".into(), Value::Bool(self.raw)),
            ("segment".into(), opt_str(&self.segment)),
            ("audio".into(), opt_path(&self.audio)),
            ("resume".into(), Value::Bool(self.resume)),
            ("crf".into(), Value::UInt(self.crf as u64)),
            ("preset".into(), opt_str(&self.preset)),
            ("encoder".into(), opt_str(&self.encoder)),
            ("prefer_hw".into(), Value::Bool(self.prefer_hw)),
            ("gpu_upscale".into(), Value::Bool(self.gpu_upscale)),
            ("format".into(), opt_str(&self.format)),
            ("container".into(), opt_str(&self.container)),
            ("baseline_dir".into(), opt_path(&self.baseline_dir)),
            (
                "snapshot_last_only".into(),
                Value::Bool(self.snapshot_last_only),
            ),
            (
                "update_baselines".into(),
                Value::Bool(self.update_baselines),
            ),
            ("cpu_raster".into(), Value::Bool(self.cpu_raster)),
        ])
    }

    /// Inverse of [`JobSpec::to_value`]; also used for `StudioJob`'s
    /// serde-flatten (unknown keys — e.g. `id` — are ignored).
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let obj = v
            .as_object()
            .ok_or_else(|| "invalid type: expected object".to_string())?;
        fn field<'a>(obj: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
            obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        }
        fn opt_str(obj: &[(String, Value)], key: &str) -> Result<Option<String>, String> {
            match field(obj, key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Str(s)) => Ok(Some(s.clone())),
                Some(_) => Err(format!("invalid type for `{key}`: expected string")),
            }
        }
        fn opt_path(obj: &[(String, Value)], key: &str) -> Result<Option<PathBuf>, String> {
            Ok(opt_str(obj, key)?.map(PathBuf::from))
        }
        fn opt_usize(obj: &[(String, Value)], key: &str) -> Result<Option<usize>, String> {
            match field(obj, key) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v
                    .as_u64()
                    .map(|u| Some(u as usize))
                    .ok_or_else(|| format!("invalid type for `{key}`: expected unsigned integer")),
            }
        }
        fn req_str(obj: &[(String, Value)], key: &str) -> Result<String, String> {
            match field(obj, key) {
                None => Err(format!("missing field `{key}`")),
                Some(Value::Str(s)) => Ok(s.clone()),
                Some(_) => Err(format!("invalid type for `{key}`: expected string")),
            }
        }
        fn num<T>(obj: &[(String, Value)], key: &str, dflt: T) -> Result<T, String>
        where
            T: TryFrom<u64>,
        {
            match field(obj, key) {
                None => Ok(dflt),
                Some(v) => v
                    .as_u64()
                    .and_then(|u| T::try_from(u).ok())
                    .ok_or_else(|| format!("invalid type for `{key}`: expected unsigned integer")),
            }
        }
        fn boolean(obj: &[(String, Value)], key: &str, dflt: bool) -> Result<bool, String> {
            match field(obj, key) {
                None => Ok(dflt),
                Some(v) => v
                    .as_bool()
                    .ok_or_else(|| format!("invalid type for `{key}`: expected bool")),
            }
        }
        Ok(JobSpec {
            effect: req_str(obj, "effect")?,
            plugin_path: opt_path(obj, "plugin_path")?,
            seed: num(obj, "seed", default_seed())?,
            fps: num(obj, "fps", default_fps())?,
            duration: req_str(obj, "duration")?,
            output: PathBuf::from(req_str(obj, "output")?),
            width: num(obj, "width", default_w())?,
            height: num(obj, "height", default_h())?,
            cols: opt_usize(obj, "cols")?,
            rows: opt_usize(obj, "rows")?,
            dry_run: boolean(obj, "dry_run", false)?,
            raw: boolean(obj, "raw", false)?,
            segment: opt_str(obj, "segment")?,
            audio: opt_path(obj, "audio")?,
            resume: boolean(obj, "resume", false)?,
            crf: num(obj, "crf", default_crf())?,
            preset: opt_str(obj, "preset")?,
            encoder: opt_str(obj, "encoder")?,
            prefer_hw: boolean(obj, "prefer_hw", true)?,
            gpu_upscale: boolean(obj, "gpu_upscale", true)?,
            format: opt_str(obj, "format")?,
            container: opt_str(obj, "container")?,
            baseline_dir: opt_path(obj, "baseline_dir")?,
            snapshot_last_only: boolean(obj, "snapshot_last_only", false)?,
            update_baselines: boolean(obj, "update_baselines", false)?,
            cpu_raster: boolean(obj, "cpu_raster", false)?,
        })
    }

    pub fn save_path(&self, path: &Path) -> Result<(), RenderError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| RenderError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let raw = self.to_json();
        // Atomic: tmp + rename so a crash can't leave a half-written job file
        // for `render --job-file` to choke on.
        let tmp = path.with_extension("job.tmp");
        std::fs::write(&tmp, raw).map_err(|source| RenderError::Io {
            path: tmp.clone(),
            source,
        })?;
        std::fs::rename(&tmp, path).map_err(|source| RenderError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn into_job(self) -> Result<(RenderJob, EncodeBackend), RenderError> {
        let duration = parse_duration_secs(&self.duration)?;
        let segment = match self.segment {
            Some(s) => Some(parse_duration_secs(&s)?),
            None => None,
        };
        let format = if self.raw {
            OutputFormat::Raw
        } else {
            match &self.format {
                Some(s) => parse_format(s)?,
                None => OutputFormat::Mp4,
            }
        };
        let container = match &self.container {
            Some(s) => parse_container(s)?,
            None => Container::Mkv,
        };
        // `raw` in JSON also toggles the stdout-raw flag (legacy alias).
        let stdout_raw = self.raw;
        let job = RenderJob {
            effect: self.effect,
            plugin_path: self.plugin_path,
            seed: self.seed,
            fps: self.fps,
            duration,
            width: self.width,
            height: self.height,
            output: self.output,
            cols: self.cols,
            rows: self.rows,
            dry_run: self.dry_run,
            segment,
            audio: self.audio,
            resume: self.resume,
            crf: self.crf,
            preset: self.preset,
            encoder: self.encoder,
            prefer_hw: self.prefer_hw,
            gpu_upscale: self.gpu_upscale,
            format,
            container,
            baseline_dir: self.baseline_dir,
            snapshot_last_only: self.snapshot_last_only,
            update_baselines: self.update_baselines,
            cpu_raster: self.cpu_raster,
        };
        job.validate()?;
        let backend = match job.format {
            OutputFormat::Png => EncodeBackend::PngSequence,
            OutputFormat::Raw if stdout_raw => EncodeBackend::StdoutRaw,
            OutputFormat::Raw => EncodeBackend::RawDump,
            OutputFormat::Mp4 if job.dry_run => EncodeBackend::RawDump,
            OutputFormat::Mp4 if matches!(job.container, Container::Mp4) => {
                EncodeBackend::FfmpegH264
            }
            OutputFormat::Mp4 => EncodeBackend::FfmpegAv1,
        };
        Ok((job, backend))
    }
}

#[cfg(test)]
#[path = "job_spec_tests.rs"]
mod tests;
