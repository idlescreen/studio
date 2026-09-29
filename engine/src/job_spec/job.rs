// SPDX-License-Identifier: Apache-2.0

use super::*;

impl JobSpec {
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
