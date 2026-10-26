// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Driving a daemon scene and turning it into frames.
//!
//! The daemon's `EngineSession` advances a scene and hands back a frame of
//! ANSI text; `paint_frame` turns that text into a grid of coloured cells;
//! the blitter turns the grid into pixels. This session owns all three
//! steps and keeps every buffer it needs, so a render allocates nothing per
//! frame.

use std::time::Duration;

use idlescreen::ascii::draw::paint_frame;
use idlescreen::ascii::effects::EffectKind;
use idlescreen::ascii::engine::EngineSession;
use idlescreen::ascii::sys_info::detect_system_theme;
use idlescreen::cell::TerminalCell;
use idlescreen::raster::blit::blit_cells_to_argb;

use crate::error::RenderError;
use crate::scene::effects::resolve_effect;
use crate::scene::frame_buffer::argb_frame_to_bgra;
use crate::scene::geometry::GridGeometry;

use self::art::build_art;

pub use self::art::DEFAULT_TEXT;

mod art;

/// The foreground the daemon uses for scene text.
const SCENE_FG: (u8, u8, u8) = (248, 248, 242);

/// A live scene, ready to be stepped and rasterized.
pub struct SceneSession {
    effect: EffectKind,
    text: String,
    seed: u64,
    fps: u32,
    grid: GridGeometry,
    engine: EngineSession,
    cells: Vec<TerminalCell>,
    argb: Vec<u32>,
    frame: String,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
}

impl SceneSession {
    /// Build a session for `effect`, animating `text` on a `grid` of cells.
    ///
    /// The output size is not needed here: the blitter scales the grid's
    /// glyphs into whatever frame it is given.
    pub fn new(
        effect: &str,
        text: &str,
        grid: GridGeometry,
        seed: u64,
        fps: u32,
    ) -> Result<Self, RenderError> {
        let kind = resolve_effect(effect)?;
        let art = build_art(text, grid);
        if art.trim().is_empty() {
            return Err(RenderError::Job(
                "the scene has nothing to animate: pass --text with the words to render".into(),
            ));
        }
        let engine = EngineSession::new(&art, kind, grid.cols, grid.rows, seed, fps.max(1))
            .ok_or_else(|| {
                RenderError::Job(format!(
                    "the daemon could not start the '{effect}' scene on a {}x{} grid",
                    grid.cols, grid.rows
                ))
            })?;
        let theme = detect_system_theme();
        Ok(Self {
            effect: kind,
            text: art,
            seed,
            fps: fps.max(1),
            cells: vec![TerminalCell::default(); grid.cells()],
            grid,
            engine,
            argb: Vec::new(),
            frame: String::new(),
            fg: SCENE_FG,
            bg: theme.bg_rgb,
        })
    }

    /// The scene this session renders.
    pub fn effect(&self) -> EffectKind {
        self.effect
    }

    /// The glyph grid the scene is drawn on.
    pub fn grid(&self) -> GridGeometry {
        self.grid
    }

    /// Advance the scene by one frame and refresh the cell grid.
    ///
    /// A scene is finite: it plays through its phases and then ends. A video
    /// is not, so the scene is rebuilt with the same seed when it finishes —
    /// the same animation, played again.
    pub fn tick(&mut self) {
        let Some(frame) = self.engine.next_frame() else {
            self.restart();
            return;
        };
        self.frame.clear();
        self.frame.push_str(&frame);
        paint_frame(
            &self.frame,
            &mut self.cells,
            self.grid.cols,
            self.grid.rows,
            self.fg,
            self.bg,
        );
    }

    /// Rasterize the current cell grid into `out` as BGRA bytes.
    ///
    /// `out` is resized to the frame length and then reused for every frame
    /// of the render.
    pub fn render_into(&mut self, width: u32, height: u32, out: &mut Vec<u8>) {
        let pixels = (width as usize).saturating_mul(height as usize);
        if self.argb.len() != pixels {
            self.argb.clear();
            self.argb.resize(pixels, 0);
        }
        blit_cells_to_argb(
            &self.cells,
            self.grid.cols,
            self.grid.rows,
            &mut self.argb,
            width as usize,
            height as usize,
        );
        argb_frame_to_bgra(&self.argb, out);
    }

    /// Rebuild the scene so the animation plays again.
    fn restart(&mut self) {
        if let Some(engine) = EngineSession::new(
            &self.text,
            self.effect,
            self.grid.cols,
            self.grid.rows,
            self.seed,
            self.fps,
        ) {
            self.engine = engine;
        }
    }

    /// The time step one frame represents.
    pub fn frame_step(&self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.fps.max(1)))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
