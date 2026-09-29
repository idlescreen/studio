//! idle-studio — **TUI-first** UI that drives the render capability.

use idle_render::JobSpec;
use idle_studio::job::StudioJob;
use idle_studio::queue::{default_queue_path, JobQueue, JobStatus};
use idle_studio::runner::run_job;
use idle_studio::tui::run_tui;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
IdleScreen Studio — TUI Director for the render capability

Primary UX: run with no subcommand to open the Director TUI.
CLI subcommands (enqueue/list/run) remain for scripts and automation.

Usage: idle-studio [OPTIONS] [COMMAND]

Commands:
  tui       Interactive Director TUI (same as default with no subcommand)
  enqueue   Add a job to the queue (scripting; prefer TUI `n` for interactive use)
  list      List queue entries (scripting)
  run       Run the next pending job (or all with --all)

Options:
      --queue <PATH>  Queue file (JSON)
  -h, --help          Print help

enqueue options:
  -e, --effect <NAME>      Effect name (required)
  -o, --output <PATH>      Output path (required)
      --duration <DUR>     [default: 10s]
      --seed <N>           [default: 12648430]
      --fps <N>            [default: 30]
      --width <N>          [default: 1280]
      --height <N>         [default: 720]
      --dry-run
      --id <ID>
      --segment <DUR>
      --audio <PATH>
      --resume
      --crf <N>            [default: 35]
      --preset <P>
      --no-hw-encode
      --no-gpu-upscale

run options:
      --all           Run all pending jobs
";

mod args;

use args::*;
fn main() -> ExitCode {
    let args = parse_args();
    let path = args.queue.unwrap_or_else(default_queue_path);

    // TUI is the product: no subcommand, or explicit `tui`.
    if args.cmd.is_none() || matches!(args.cmd, Some(Cmd::Tui)) {
        return match run_tui(&path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("idle-studio: {e}");
                ExitCode::from(1)
            }
        };
    }

    let mut queue = match JobQueue::load_or_recover(&path) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("idle-studio: {e}");
            return ExitCode::from(2);
        }
    };

    match args.cmd.expect("subcommand present") {
        Cmd::Tui => ExitCode::SUCCESS,
        Cmd::Enqueue {
            effect,
            output,
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
        } => {
            let id = id.unwrap_or_else(|| queue.next_id());
            let spec = JobSpec {
                effect,
                plugin_path: None,
                seed,
                fps,
                duration,
                output,
                width,
                height,
                cols: None,
                rows: None,
                dry_run,
                raw: false,
                segment,
                audio,
                resume,
                crf,
                preset,
                encoder: None,
                prefer_hw: !no_hw_encode,
                gpu_upscale: !no_gpu_upscale,
                format: None,
                container: None,
                baseline_dir: None,
                snapshot_last_only: false,
                update_baselines: false,
                cpu_raster: false,
            };
            queue.enqueue(StudioJob::new(id.clone(), spec));
            match queue.save(&path) {
                Ok(()) => {
                    eprintln!("enqueued {id} → {}", path.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("idle-studio: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Cmd::List => {
            for (i, e) in queue.entries.iter().enumerate() {
                println!(
                    "[{i}] {} {:?} {} -> {}",
                    e.job.id,
                    e.status,
                    e.job.spec.effect,
                    e.job.spec.output.display()
                );
            }
            ExitCode::SUCCESS
        }
        Cmd::Run { all } => {
            let mut code = ExitCode::SUCCESS;
            loop {
                let Some(idx) = queue.next_pending_index() else {
                    if !all {
                        eprintln!("no pending jobs");
                    }
                    break;
                };
                queue.entries[idx].status = JobStatus::Running;
                let _ = queue.save(&path);
                let job = queue.entries[idx].job.clone();
                match run_job(&job) {
                    Ok(msg) => {
                        queue.entries[idx].status = JobStatus::Done;
                        queue.entries[idx].message = msg;
                        eprintln!("done {}", job.id);
                    }
                    Err(e) => {
                        queue.entries[idx].status = JobStatus::Failed;
                        queue.entries[idx].message = e.to_string();
                        eprintln!("failed {}: {e}", job.id);
                        code = ExitCode::from(1);
                    }
                }
                let _ = queue.save(&path);
                if !all {
                    break;
                }
            }
            code
        }
    }
}
