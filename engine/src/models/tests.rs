// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tests for the render job model.

use super::*;

fn sample() -> RenderJob {
    RenderJob {
        effect: "beams".into(),
        text: "IDLESCREEN".into(),
        plugin_path: None,
        seed: 1,
        fps: 30,
        duration: Duration::from_secs(1),
        width: 320,
        height: 180,
        output: PathBuf::from("/tmp/out.mkv"),
        cols: None,
        rows: None,
        dry_run: true,
        segment: None,
        audio: None,
        resume: false,
        crf: 35,
        preset: None,
        encoder: None,
        prefer_hw: true,
        gpu_upscale: true,
        format: OutputFormat::Mp4,
        container: Container::Mkv,
        baseline_dir: None,
        snapshot_last_only: false,
        update_baselines: false,
        cpu_raster: false,
    }
}

#[test]
fn frame_count_one_second_30fps() {
    assert_eq!(sample().frame_count(), 30);
}

#[test]
fn segment_count_two_hours_hourly() {
    let mut j = sample();
    j.duration = Duration::from_secs(7200);
    j.segment = Some(Duration::from_secs(3600));
    assert_eq!(j.segment_count(), 2);
}

#[test]
fn rejects_oversized_frames() {
    // A pathological --width/--height pair must be refused before the
    // 4-bytes-per-pixel frame buffer is allocated.
    let mut j = sample();
    j.width = 100_000;
    j.height = 100_000;
    assert!(j.validate().is_err());

    let mut j = sample();
    j.width = MAX_FRAME_DIMENSION + 1;
    j.height = 16;
    assert!(j.validate().is_err());

    let mut j = sample();
    j.width = 8192;
    j.height = 8192;
    assert!(j.validate().is_ok(), "8K must still be accepted");
}

#[test]
fn rejects_zero_fps() {
    let mut j = sample();
    j.fps = 0;
    assert!(j.validate().is_err());
}

#[test]
fn rejects_zero_size() {
    let mut j = sample();
    j.width = 0;
    assert!(j.validate().is_err());
}

#[test]
fn update_baselines_requires_env_var() {
    // F7 foot-gun: --update-baselines must fail-closed unless
    // RENDER_FORCE_UPDATE_BASELINES=1 is set in the environment.
    let mut j = sample();
    j.update_baselines = true;

    // No env var set.
    std::env::remove_var("RENDER_FORCE_UPDATE_BASELINES");
    assert!(j.validate().is_err(), "must reject without env var");

    // Env var set to something other than "1".
    std::env::set_var("RENDER_FORCE_UPDATE_BASELINES", "true");
    assert!(
        j.validate().is_err(),
        "must reject when env var is not \"1\""
    );
    std::env::set_var("RENDER_FORCE_UPDATE_BASELINES", "");
    assert!(j.validate().is_err(), "must reject when env var is empty");
    std::env::remove_var("RENDER_FORCE_UPDATE_BASELINES");

    // Env var set to "1".
    std::env::set_var("RENDER_FORCE_UPDATE_BASELINES", "1");
    assert!(
        j.validate().is_ok(),
        "must accept when env var is exactly \"1\""
    );
    std::env::remove_var("RENDER_FORCE_UPDATE_BASELINES");
}

#[test]
fn rejects_bad_crf() {
    let mut j = sample();
    j.crf = 99;
    assert!(j.validate().is_err());
}
