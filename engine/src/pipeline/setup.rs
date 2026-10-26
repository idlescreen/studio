// SPDX-License-Identifier: Apache-2.0

//! Pipeline setup helpers: export env, scene resolution, encoder settings,
//! frame counting, and resume fast-forward.

#![allow(unsafe_code)]

use crate::encode::EncodeSettings;
use crate::error::RenderError;
use crate::models::RenderJob;
use crate::scene::geometry::GridGeometry;
use crate::scene::SceneSession;
use std::time::Duration;

pub fn export_seed_env(seed: u64) {
    // SAFETY: single-threaded CLI before the scene is built; values are numeric strings.
    unsafe {
        std::env::set_var("RENDER_SEED", seed.to_string());
        std::env::set_var("IDLE_RENDER_SEED", seed.to_string());
        std::env::set_var("TRANCE_SEED", seed.to_string());
        std::env::set_var("IDLE_DISABLE_SANDBOX", "1");
        // Release sandbox escape requires both flags (see idle-runner sandbox.rs).
        std::env::set_var("IDLE_RENDER_PIPELINE", "1");
        std::env::set_var("TRANCE_DISABLE_SANDBOX", "1");
        std::env::set_var("IDLE_EXPORT_MODE", "1");
        std::env::set_var("TRANCE_EXPORT_MODE", "1");
    }
}

/// Build the scene a job renders.
///
/// The scenes are the daemon's own: the plugin path is gone, and
/// `RenderJob::validate` has already refused a job that still asks for one.
pub(super) fn resolve_scene(
    job: &RenderJob,
    grid: GridGeometry,
) -> Result<SceneSession, RenderError> {
    SceneSession::new(&job.effect, &job.text, grid, job.seed, job.fps)
}

pub(super) fn encode_settings(job: &RenderJob) -> EncodeSettings {
    EncodeSettings {
        crf: job.crf,
        preset: job.preset.clone(),
        encoder: job.encoder.clone(),
        prefer_hw: job.prefer_hw,
    }
}

pub(super) fn frames_for_duration(duration: Duration, fps: u32) -> u64 {
    let secs = duration.as_secs_f64();
    let n = (secs * f64::from(fps)).floor() as u64;
    n.max(1)
}

/// Advance the scene without rasterizing or encoding (used when --resume
/// skips a part).
pub(super) fn fast_forward(session: &mut SceneSession, frames: u64) {
    for _ in 0..frames {
        session.tick();
    }
}
