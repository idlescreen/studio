// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The banner art a scene animates.
//!
//! The daemon renders its rotation text into a big-block banner and hands
//! that to the scene engine. Studio does the same, so an export looks like
//! the screensaver instead of like plain text dropped into a scene.

use idlescreen::ascii::layout::LruFontAtlasCache;

use crate::scene::geometry::GridGeometry;

/// The words the daemon falls back to when its rotation has no text.
pub const DEFAULT_TEXT: &str = "IDLESCREEN";

/// Build the banner art for `text`, exactly the way the daemon does.
///
/// Returns the banner's block-glyph lines joined by newlines, or the plain
/// text when the banner came out empty.
pub fn build_art(text: &str, grid: GridGeometry) -> String {
    let source = if text.trim().is_empty() {
        DEFAULT_TEXT
    } else {
        text.trim()
    };
    let mut cache = LruFontAtlasCache::new(1);
    let banner = cache.get_or_render(source, None, grid.cols, grid.rows);
    if !banner.lines.is_empty() && banner.lines.iter().any(|l| !l.trim().is_empty()) {
        banner.lines.join("\n")
    } else {
        source.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_is_block_glyphs_not_the_plain_word() {
        let grid = crate::scene::geometry::resolve(None, None).expect("grid");
        let art = build_art("IDLESCREEN", grid);
        assert!(art.contains('█'), "the banner is not block art: {art:?}");
        assert!(!art.contains('I'), "the plain word leaked into the banner");
    }

    #[test]
    fn an_empty_text_falls_back_to_the_default() {
        let grid = crate::scene::geometry::resolve(None, None).expect("grid");
        let blank = build_art("   ", grid);
        let named = build_art(DEFAULT_TEXT, grid);
        assert_eq!(blank, named);
    }
}
