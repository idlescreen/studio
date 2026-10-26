// SPDX-License-Identifier: Apache-2.0

//! Self-contained offline saver simulation session for `render`.
//!
//! Decouples `studio`/`render` from the archived `idlescreen/runtime` repository
//! while preserving deterministic frame generation and grid sizing contracts.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum LaunchMode {
    Preview,
}

pub struct PluginSession {
    effect_hash: u64,
    scale: f32,
    cols: usize,
    rows: usize,
    sim_hz: f32,
    elapsed_secs: f64,
    tick_count: u64,
}

impl PluginSession {
    pub fn load_path_with_options(path: &Path, scale: Option<f32>) -> Result<Self, String> {
        if !path.is_file() {
            return Err(format!("plugin path does not exist: {}", path.display()));
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin");
        Self::new_session(name, scale)
    }

    pub fn load_with_options(
        effect: &str,
        _mode: &LaunchMode,
        scale: Option<f32>,
    ) -> Result<Self, String> {
        if effect.trim().is_empty() {
            return Err("effect name cannot be empty".to_string());
        }
        Self::new_session(effect, scale)
    }

    fn new_session(name: &str, scale: Option<f32>) -> Result<Self, String> {
        let seed_env = std::env::var("RENDER_SEED")
            .or_else(|_| std::env::var("IDLE_RENDER_SEED"))
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let mut h = seed_env ^ 0xCBF2_9CE4_8422_2325;
        for b in name.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01B3);
        }
        Ok(Self {
            effect_hash: h,
            scale: scale.unwrap_or(1.0).clamp(0.25, 2.0),
            cols: 80,
            rows: 24,
            sim_hz: 60.0,
            elapsed_secs: 0.0,
            tick_count: 0,
        })
    }

    pub fn grid_for_pixels(&self, width: u32, height: u32) -> (usize, usize) {
        let cell_w = (8.0 / self.scale).max(1.0) as u32;
        let cell_h = (16.0 / self.scale).max(1.0) as u32;
        let cols = (width / cell_w).max(1) as usize;
        let rows = (height / cell_h).max(1) as usize;
        (cols, rows)
    }

    pub fn init(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
    }

    pub fn set_simulation_rate(&mut self, hz: f32) {
        if hz > 0.0 {
            self.sim_hz = hz;
        }
    }

    pub fn tick(&mut self, dt: Duration) {
        self.elapsed_secs += dt.as_secs_f64();
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn render(&mut self, _cols: usize, _rows: usize, width: u32, height: u32) -> Arc<[u8]> {
        // `RenderJob::validate` is the contract for the frame size. Clamp
        // here as a last-resort guard so a caller that skipped validation
        // cannot ask for an unbounded allocation.
        let pixels = u64::from(width)
            .saturating_mul(u64::from(height))
            .min(crate::models::MAX_FRAME_PIXELS);
        let total = (pixels as usize).saturating_mul(4);
        let mut buf = vec![0u8; total];
        let phase = self.tick_count.wrapping_mul(0x9E37_79B9) ^ self.effect_hash;
        for (i, chunk) in buf.chunks_exact_mut(4).enumerate() {
            let v = (i as u64).wrapping_mul(31).wrapping_add(phase);
            chunk[0] = (v & 0xFF) as u8;
            chunk[1] = ((v >> 8) & 0xFF) as u8;
            chunk[2] = ((v >> 16) & 0xFF) as u8;
            chunk[3] = 0xFF;
        }
        Arc::from(buf)
    }
}
