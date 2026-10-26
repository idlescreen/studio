// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tests for driving a daemon scene and turning it into frames.

use super::*;

fn session(effect: &str) -> SceneSession {
    let grid = crate::scene::geometry::resolve(None, None).expect("grid");
    SceneSession::new(effect, "IDLESCREEN", grid, 7, 30).expect("session")
}

#[test]
fn a_scene_starts_and_renders_pixels() {
    let mut s = session("beams");
    s.tick();
    let mut out = Vec::new();
    s.render_into(64, 48, &mut out);
    assert_eq!(out.len(), 64 * 48 * 4);
    assert!(out.iter().any(|b| *b != 0), "the frame is entirely black");
}

#[test]
fn every_scene_renders_a_non_empty_frame() {
    for name in crate::scene::effects::effect_names() {
        let mut s = session(name);
        s.tick();
        let mut out = Vec::new();
        s.render_into(64, 48, &mut out);
        assert!(out.iter().any(|b| *b != 0), "{name} rendered nothing");
    }
}

#[test]
fn the_same_seed_gives_the_same_frame() {
    let mut a = session("beams");
    let mut b = session("beams");
    a.tick();
    b.tick();
    let (mut oa, mut ob) = (Vec::new(), Vec::new());
    a.render_into(64, 48, &mut oa);
    b.render_into(64, 48, &mut ob);
    assert_eq!(oa, ob);
}

#[test]
fn a_different_seed_gives_a_different_frame() {
    let grid = crate::scene::geometry::resolve(None, None).expect("grid");
    let mut a = SceneSession::new("beams", "IDLESCREEN", grid, 1, 30).expect("session");
    let mut b = SceneSession::new("beams", "IDLESCREEN", grid, 2, 30).expect("session");
    a.tick();
    b.tick();
    let (mut oa, mut ob) = (Vec::new(), Vec::new());
    a.render_into(64, 48, &mut oa);
    b.render_into(64, 48, &mut ob);
    assert_ne!(oa, ob);
}

#[test]
fn an_unknown_scene_is_refused() {
    let grid = crate::scene::geometry::resolve(None, None).expect("grid");
    assert!(SceneSession::new("nope", "IDLESCREEN", grid, 1, 30).is_err());
}

#[test]
fn an_empty_text_falls_back_to_the_daemon_default() {
    // The daemon falls back to "IDLESCREEN" when its rotation has no
    // text; Studio does the same rather than refusing to render.
    let grid = crate::scene::geometry::resolve(None, None).expect("grid");
    let mut blank = SceneSession::new("beams", "   ", grid, 1, 30).expect("session");
    let mut named = SceneSession::new("beams", DEFAULT_TEXT, grid, 1, 30).expect("session");
    blank.tick();
    named.tick();
    let (mut ob, mut on) = (Vec::new(), Vec::new());
    blank.render_into(64, 48, &mut ob);
    named.render_into(64, 48, &mut on);
    assert_eq!(ob, on);
}

#[test]
fn a_finished_scene_plays_again() {
    // A scene is finite; the session restarts it with the same seed so a
    // video loops instead of freezing on the last frame.
    let grid = crate::scene::geometry::resolve(None, None).expect("grid");
    let mut s = SceneSession::new("beams", "IDLESCREEN", grid, 7, 30).expect("session");
    let mut first = Vec::new();
    s.tick();
    s.render_into(64, 48, &mut first);
    // Run well past the scene's length.
    for _ in 0..2000 {
        s.tick();
    }
    let mut later = Vec::new();
    s.render_into(64, 48, &mut later);
    assert!(!later.is_empty());
    assert_ne!(first, later, "the scene never advanced");
}
