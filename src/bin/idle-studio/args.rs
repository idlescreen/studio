// SPDX-License-Identifier: Apache-2.0

use super::*;

pub(crate) struct Args {
    /// Queue file (JSON)
    pub queue: Option<PathBuf>,

    /// Optional CLI mode; omit to open the TUI
    pub cmd: Option<Cmd>,
}

#[derive(Debug)]
pub(crate) enum Cmd {
    /// Interactive Director TUI (same as default with no subcommand)
    Tui,
    /// Add a job to the queue (scripting; prefer TUI `n` for interactive use)
    Enqueue {
        effect: String,
        output: PathBuf,
        duration: String,
        seed: u64,
        fps: u32,
        width: u32,
        height: u32,
        dry_run: bool,
        id: Option<String>,
        segment: Option<String>,
        audio: Option<PathBuf>,
        resume: bool,
        crf: u8,
        preset: Option<String>,
        no_hw_encode: bool,
        no_gpu_upscale: bool,
    },
    /// List queue entries (scripting)
    List,
    /// Run the next pending job (or all with --all)
    Run { all: bool },
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}\n\n{USAGE}");
    std::process::exit(2);
}

pub(crate) fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut queue = None;
    let mut cmd = None;
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--queue" => {
                i += 1;
                queue = Some(PathBuf::from(
                    argv.get(i)
                        .unwrap_or_else(|| fail("expected value for '--queue'")),
                ));
            }
            "tui" => cmd = Some(Cmd::Tui),
            "list" => cmd = Some(Cmd::List),
            "run" => cmd = Some(parse_run(&argv, &mut i, &mut queue)),
            "enqueue" => cmd = Some(parse_enqueue(&argv, &mut i, &mut queue)),
            other => fail(format!("unexpected argument '{other}'")),
        }
        if cmd.is_some() && i >= argv.len() {
            break;
        }
        i += 1;
    }
    Args { queue, cmd }
}

fn next_val<'a>(argv: &'a [String], i: &mut usize, name: &str) -> &'a str {
    *i += 1;
    argv.get(*i)
        .map(String::as_str)
        .unwrap_or_else(|| fail(format!("expected value for '{name}'")))
}

fn num<T: std::str::FromStr>(name: &str, v: &str) -> T {
    v.parse::<T>()
        .unwrap_or_else(|_| fail(format!("invalid value '{v}' for '{name}'")))
}

fn parse_run(argv: &[String], i: &mut usize, queue: &mut Option<PathBuf>) -> Cmd {
    let mut all = false;
    while let Some(arg) = argv.get(*i + 1) {
        *i += 1;
        match arg.as_str() {
            "--all" => all = true,
            "--queue" => *queue = Some(PathBuf::from(next_val(argv, i, "--queue"))),
            other => fail(format!("unexpected argument '{other}' for 'run'")),
        }
    }
    Cmd::Run { all }
}

fn parse_enqueue(argv: &[String], i: &mut usize, queue: &mut Option<PathBuf>) -> Cmd {
    let mut effect = None;
    let mut output = None;
    let mut duration = "10s".to_string();
    let mut seed = 0x00C0_FFEEu64;
    let mut fps = 30u32;
    let mut width = 1280u32;
    let mut height = 720u32;
    let mut dry_run = false;
    let mut id = None;
    let mut segment = None;
    let mut audio = None;
    let mut resume = false;
    let mut crf = 35u8;
    let mut preset = None;
    let mut no_hw_encode = false;
    let mut no_gpu_upscale = false;
    while let Some(arg) = argv.get(*i + 1).cloned() {
        *i += 1;
        match arg.as_str() {
            "-e" | "--effect" => effect = Some(next_val(argv, i, "--effect").to_string()),
            "-o" | "--output" => output = Some(PathBuf::from(next_val(argv, i, "--output"))),
            "--duration" => duration = next_val(argv, i, "--duration").to_string(),
            "--seed" => seed = num("--seed", next_val(argv, i, "--seed")),
            "--fps" => fps = num("--fps", next_val(argv, i, "--fps")),
            "--width" => width = num("--width", next_val(argv, i, "--width")),
            "--height" => height = num("--height", next_val(argv, i, "--height")),
            "--dry-run" => dry_run = true,
            "--id" => id = Some(next_val(argv, i, "--id").to_string()),
            "--segment" => segment = Some(next_val(argv, i, "--segment").to_string()),
            "--audio" => audio = Some(PathBuf::from(next_val(argv, i, "--audio"))),
            "--resume" => resume = true,
            "--crf" => crf = num("--crf", next_val(argv, i, "--crf")),
            "--preset" => preset = Some(next_val(argv, i, "--preset").to_string()),
            "--no-hw-encode" => no_hw_encode = true,
            "--no-gpu-upscale" => no_gpu_upscale = true,
            "--queue" => *queue = Some(PathBuf::from(next_val(argv, i, "--queue"))),
            other => fail(format!("unexpected argument '{other}' for 'enqueue'")),
        }
    }
    Cmd::Enqueue {
        effect: effect.unwrap_or_else(|| fail("required: --effect <NAME>")),
        output: output.unwrap_or_else(|| fail("required: --output <PATH>")),
        duration,
        seed,
        fps,
        width,
        height,
        dry_run,
        id,
        segment,
        audio,
        resume,
        crf,
        preset,
        no_hw_encode,
        no_gpu_upscale,
    }
}
