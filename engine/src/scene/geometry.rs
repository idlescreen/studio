// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The glyph grid an export is rendered on.
//!
//! The daemon draws its scenes on a terminal-sized grid of character cells.
//! The org standard fixes that native grid at 120x36 cells (960x576 pixels,
//! 8x16 per cell), and the compositor scales that buffer to the display. An
//! export does the same thing: it renders the native grid and lets the
//! blitter scale the glyphs into the requested output, so a video looks like
//! the screensaver rather than like a differently-proportioned scene.

use crate::error::RenderError;

/// The daemon's native glyph grid: 120 columns, 36 rows.
pub const NATIVE_COLS: usize = 120;
pub const NATIVE_ROWS: usize = 36;

/// A resolved character grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridGeometry {
    pub cols: usize,
    pub rows: usize,
}

impl GridGeometry {
    /// The number of cells in the grid.
    pub fn cells(&self) -> usize {
        self.cols.saturating_mul(self.rows)
    }
}

/// Resolve the grid for a job.
///
/// Explicit `--cols`/`--rows` win; otherwise the native grid is used. The
/// output resolution does not change the grid: the blitter scales the
/// glyphs, exactly as the compositor scales the screensaver's buffer.
pub fn resolve(cols: Option<usize>, rows: Option<usize>) -> Result<GridGeometry, RenderError> {
    let cols = cols.unwrap_or(NATIVE_COLS);
    let rows = rows.unwrap_or(NATIVE_ROWS);
    if cols == 0 || rows == 0 {
        return Err(RenderError::Job(format!(
            "grid resolved to {cols}x{rows}: a grid needs at least one column and one row"
        )));
    }
    Ok(GridGeometry { cols, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_native_grid() {
        let g = resolve(None, None).expect("grid");
        assert_eq!(g.cols, 120);
        assert_eq!(g.rows, 36);
        assert_eq!(g.cells(), 4320);
    }

    #[test]
    fn explicit_dimensions_win() {
        let g = resolve(Some(80), Some(24)).expect("grid");
        assert_eq!(g.cols, 80);
        assert_eq!(g.rows, 24);
    }

    #[test]
    fn one_dimension_can_be_overridden_alone() {
        let g = resolve(Some(64), None).expect("grid");
        assert_eq!(g.cols, 64);
        assert_eq!(g.rows, 36);
    }

    #[test]
    fn a_zero_dimension_is_rejected() {
        assert!(resolve(Some(0), Some(24)).is_err());
        assert!(resolve(Some(80), Some(0)).is_err());
    }
}
