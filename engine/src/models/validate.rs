// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Validation of a render job before any buffer is allocated.

use crate::error::RenderError;
use crate::paths::deny_parent_dirs;

use super::{RenderJob, MAX_FRAME_DIMENSION, MAX_FRAME_PIXELS};

impl RenderJob {
    pub fn validate(&self) -> Result<(), RenderError> {
        if self.fps == 0 {
            return Err(RenderError::Job("fps must be > 0".into()));
        }
        if self.width == 0 || self.height == 0 {
            return Err(RenderError::Job("width and height must be > 0".into()));
        }
        if self.width > MAX_FRAME_DIMENSION || self.height > MAX_FRAME_DIMENSION {
            return Err(RenderError::Job(format!(
                "width and height must be <= {MAX_FRAME_DIMENSION}"
            )));
        }
        // The frame buffer is 4 bytes per pixel and is allocated once per
        // rendered frame. Without a pixel budget, `--width 100000 --height
        // 100000` asks for a 40 GB allocation before the first frame is
        // written.
        let pixels = u64::from(self.width) * u64::from(self.height);
        if pixels > MAX_FRAME_PIXELS {
            return Err(RenderError::Job(format!(
                "frame size {pixels} pixels exceeds the {MAX_FRAME_PIXELS} pixel budget"
            )));
        }
        if self.duration.is_zero() {
            return Err(RenderError::Job("duration must be > 0".into()));
        }
        if self.effect.trim().is_empty() && self.plugin_path.is_none() {
            return Err(RenderError::Job(
                "effect name or --plugin-path required".into(),
            ));
        }
        if self.output.as_os_str().is_empty() {
            return Err(RenderError::Job("output path required".into()));
        }
        if self.crf > 63 {
            return Err(RenderError::Job("crf must be 0..=63".into()));
        }
        if self.update_baselines
            && std::env::var("RENDER_FORCE_UPDATE_BASELINES").as_deref() != Ok("1")
        {
            return Err(RenderError::Job(
                "--update-baselines requires RENDER_FORCE_UPDATE_BASELINES=1 in the environment"
                    .into(),
            ));
        }
        deny_parent_dirs(&self.output, "output")?;
        if let Some(p) = &self.plugin_path {
            deny_parent_dirs(p, "plugin_path")?;
        }
        if let Some(a) = &self.audio {
            deny_parent_dirs(a, "audio")?;
        }
        if let Some(seg) = self.segment {
            if seg.is_zero() {
                return Err(RenderError::Job("segment duration must be > 0".into()));
            }
        }
        if let Some(p) = &self.baseline_dir {
            deny_parent_dirs(p, "baseline_dir")?;
        }
        Ok(())
    }
}
