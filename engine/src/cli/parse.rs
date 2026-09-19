// SPDX-License-Identifier: Apache-2.0

use super::*;

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
}
