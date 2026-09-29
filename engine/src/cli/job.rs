// SPDX-License-Identifier: Apache-2.0

use super::*;

impl Args {
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
