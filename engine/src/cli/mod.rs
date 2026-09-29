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
mod args;
mod job;
mod parse;

pub(crate) use args::*;
pub use args::{Args, CliContainer, CliFormat};
