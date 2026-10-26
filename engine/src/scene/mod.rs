// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Offline scene rendering: the daemon's built-in savers, headlessly.
//!
//! The daemon crate owns the 53 scenes and the cell rasterizer. This module
//! drives them: it resolves an effect name, sizes the glyph grid, and turns
//! the daemon's cell grid into the BGRA frames the encoder consumes.

pub mod effects;
pub mod frame_buffer;
pub mod geometry;
pub mod session;

pub use effects::{effect_names, resolve_effect};
pub use geometry::GridGeometry;
pub use session::{SceneSession, DEFAULT_TEXT};
