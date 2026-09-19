use crate::duration::parse_duration_secs;
use crate::encode::EncodeBackend;
use crate::error::RenderError;
use crate::job_spec::JobSpec;
use crate::models::{Container, OutputFormat, RenderJob};
use std::path::PathBuf;

const USAGE: &str = "\
IdleScreen offline export capability (saver math → AV1)

Usage: render [OPTIONS]

Options:
      --job-file <PATH>       JSON job file (Studio / automation). When set,
                              flag fields below are ignored.
  -e, --effect <NAME>         Effect name (allowlisted saver basename, e.g. ripple)
      --plugin-path <PATH>    Explicit path to plugin .so (skips discovery)
      --seed <N>              RNG seed [default: 12648430]
      --fps <N>               Output timeline fps [default: 30]
      --duration <DUR>        Duration: 10s, 5m, 2h, 1d (or bare seconds) [default: 10s]
      --segment <DUR>         Optional segment length for long encodes (e.g. 1h)
      --audio <PATH>          Optional audio bed (muxed after video)
  -o, --output <PATH>         Output path (.mkv recommended). Optional with
                              --dry-run/--stdout-raw.
      --width <N>             Pixel width [default: 1280]
      --height <N>            Pixel height [default: 720]
      --cols <N>              Optional simulation grid columns
      --rows <N>              Optional simulation grid rows
      --dry-run               Validate and print plan only
      --raw                   Write raw BGRA dump instead of AV1 (debug/tests).
                              Alias for --format raw.
      --resume                Resume: skip encode for existing non-empty segment parts
      --crf <N>               AV1 quality 0-63 (CRF / CQ) [default: 35]
      --preset <P>            Encoder preset (SVT numeric or NVENC p1-p7)
      --encoder <NAME>        Force ffmpeg video encoder name
      --no-hw-encode          Force software AV1 only
      --no-gpu-upscale        Force CPU upscale
      --format <FMT>          Output family [default: mp4] [possible: mp4, png, raw]
      --container <C>         Container for video output [default: mkv] [possible: mkv, mp4]
      --stdout-raw            Stream raw BGRA + 16-byte header to stdout.
                              Equivalent to --format raw.
      --baseline-dir <PATH>   Snapshot compare directory (when set,
                              --snapshot-last-only compares last frame).
      --update-baselines      Overwrite baseline files instead of comparing (dev only).
      --snapshot-last-only    Only compare final frame against baseline (skips
                              all-but-last encode work).
      --cpu-raster            Force CPU rendering path (deterministic; bypass GPU
                              variance for tests).
  -h, --help                  Print help
";

/// CLI arguments (hand-rolled parser; was `clap::Parser`).
#[derive(Debug)]
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
    fn parse(s: &str) -> Result<Self, String> {
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
    fn parse(s: &str) -> Result<Self, String> {
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

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}\n\n{USAGE}");
    std::process::exit(2);
}

impl Args {
    /// Parse process args (clap `Parser::parse` semantics: `--help` prints
    /// usage and exits 0, errors exit 2).
    pub fn parse() -> Self {
        Self::parse_from(std::env::args())
    }

    /// Parse an argv-style iterator (first element is the program name,
    /// matching clap's `try_parse_from` convention).
    pub fn parse_from<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut a = Args {
            job_file: None,
            effect: None,
            plugin_path: None,
            seed: 0x00C0_FFEE,
            fps: 30,
            duration: "10s".into(),
            segment: None,
            audio: None,
            output: None,
            width: 1280,
            height: 720,
            cols: None,
            rows: None,
            dry_run: false,
            raw: false,
            resume: false,
            crf: 35,
            preset: None,
            encoder: None,
            no_hw_encode: false,
            no_gpu_upscale: false,
            format: CliFormat::Mp4,
            container: CliContainer::Mkv,
            stdout_raw: false,
            baseline_dir: None,
            update_baselines: false,
            snapshot_last_only: false,
            cpu_raster: false,
        };
        let args: Vec<String> = args.into_iter().map(Into::into).skip(1).collect();
        let mut i = 0;
        let next_val = |i: &mut usize, name: &str| -> String {
            *i += 1;
            match args.get(*i) {
                Some(v) => v.clone(),
                None => fail(format!("expected value for '{name}'")),
            }
        };
        fn num<T: std::str::FromStr>(name: &str, v: &str) -> T {
            v.parse::<T>()
                .unwrap_or_else(|_| fail(format!("invalid value '{v}' for '{name}'")))
        }
        while i < args.len() {
            let arg = &args[i];
            // Support `--flag=value` for long options.
            let (arg, inline) = match arg.strip_prefix("--").and_then(|s| s.split_once('=')) {
                Some((name, val)) => (format!("--{name}"), Some(val.to_string())),
                None => (arg.clone(), None),
            };
            macro_rules! val {
                ($name:expr) => {
                    match inline {
                        Some(ref v) => v.clone(),
                        None => next_val(&mut i, $name),
                    }
                };
            }
            match arg.as_str() {
                "-h" | "--help" => {
                    println!("{USAGE}");
                    std::process::exit(0);
                }
                "--job-file" => a.job_file = Some(PathBuf::from(val!("--job-file"))),
                "-e" | "--effect" => a.effect = Some(val!("--effect")),
                "--plugin-path" => a.plugin_path = Some(PathBuf::from(val!("--plugin-path"))),
                "--seed" => a.seed = num("--seed", &val!("--seed")),
                "--fps" => a.fps = num("--fps", &val!("--fps")),
                "--duration" => a.duration = val!("--duration"),
                "--segment" => a.segment = Some(val!("--segment")),
                "--audio" => a.audio = Some(PathBuf::from(val!("--audio"))),
                "-o" | "--output" => a.output = Some(PathBuf::from(val!("--output"))),
                "--width" => a.width = num("--width", &val!("--width")),
                "--height" => a.height = num("--height", &val!("--height")),
                "--cols" => a.cols = Some(num("--cols", &val!("--cols"))),
                "--rows" => a.rows = Some(num("--rows", &val!("--rows"))),
                "--dry-run" => a.dry_run = true,
                "--raw" => a.raw = true,
                "--resume" => a.resume = true,
                "--crf" => a.crf = num("--crf", &val!("--crf")),
                "--preset" => a.preset = Some(val!("--preset")),
                "--encoder" => a.encoder = Some(val!("--encoder")),
                "--no-hw-encode" => a.no_hw_encode = true,
                "--no-gpu-upscale" => a.no_gpu_upscale = true,
                "--format" => {
                    a.format = CliFormat::parse(&val!("--format")).unwrap_or_else(|e| fail(e))
                }
                "--container" => {
                    a.container =
                        CliContainer::parse(&val!("--container")).unwrap_or_else(|e| fail(e))
                }
                "--stdout-raw" => a.stdout_raw = true,
                "--baseline-dir" => a.baseline_dir = Some(PathBuf::from(val!("--baseline-dir"))),
                "--update-baselines" => a.update_baselines = true,
                "--snapshot-last-only" => a.snapshot_last_only = true,
                "--cpu-raster" => a.cpu_raster = true,
                other => fail(format!("unexpected argument '{other}'")),
            }
            i += 1;
        }
        // clap `required_unless_present` semantics.
        if a.job_file.is_none() {
            if a.effect.is_none() {
                fail("the following required arguments were not provided: --effect <NAME>");
            }
            if a.output.is_none() && !a.dry_run && !a.stdout_raw {
                fail("the following required arguments were not provided: --output <PATH>");
            }
        }
        a
    }

    pub fn into_job(self) -> Result<(RenderJob, EncodeBackend), RenderError> {
        if let Some(path) = self.job_file {
            return JobSpec::load_path(&path)?.into_job();
        }
        let effect = self
            .effect
            .ok_or_else(|| RenderError::Job("--effect required without --job-file".into()))?;
        let output = match self.output {
            Some(p) => p,
            None if self.dry_run => PathBuf::from("dry-run.mkv"),
            // StdoutRaw streams to stdout; the path is unused but RenderJob
            // requires one.
            None if self.stdout_raw => PathBuf::from("<stdout>"),
            None => {
                return Err(RenderError::Job(
                    "--output required without --job-file (unless --dry-run/--stdout-raw)".into(),
                ));
            }
        };
        let duration = parse_duration_secs(&self.duration)?;
        let segment = match self.segment {
            Some(s) => Some(parse_duration_secs(&s)?),
            None => None,
        };
        // `--stdout-raw` always streams with the GBRI header; `--raw` (legacy)
        // is a synonym for `--format raw --stdout-raw`. The `format` field
        // itself is just `--format {png,mp4,raw}` — `raw` means raw-BGRA-to-file.
        let format: OutputFormat = if self.raw || self.stdout_raw {
            OutputFormat::Raw
        } else {
            self.format.into()
        };
        // `format == Raw` becomes RawDump (file); `--stdout-raw` upgrades it
        // to StdoutRaw so the 16-byte header is emitted.
        let stdout_raw = self.stdout_raw;
        let job = RenderJob {
            effect,
            plugin_path: self.plugin_path,
            seed: self.seed,
            fps: self.fps,
            duration,
            width: self.width,
            height: self.height,
            output,
            cols: self.cols,
            rows: self.rows,
            dry_run: self.dry_run,
            segment,
            audio: self.audio,
            resume: self.resume,
            crf: self.crf,
            preset: self.preset,
            encoder: self.encoder,
            prefer_hw: !self.no_hw_encode,
            gpu_upscale: !self.no_gpu_upscale,
            format,
            container: self.container.into(),
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
