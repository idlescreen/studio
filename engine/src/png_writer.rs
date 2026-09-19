//! Pure-Rust PNG encoding for BGRA frames.
//!
//! Wraps `&[u8]` BGRA frames as RGBA8 PNGs via [`crate::png`].
//! Deterministic: same input bytes ⇒ same PNG bytes (CRC32 + deflate).

use crate::error::RenderError;

/// Encode a single BGRA frame as a self-contained PNG byte vector.
///
/// # Panics
/// Panics when `bgra.len() != width * height * 4` (defensive — pipeline
/// guarantees invariants) or when width/height exceed the `png` crate's
/// per-row chunk budget (a hard library limit).
pub fn encode_bgra_frame_to_png(width: u32, height: u32, bgra: &[u8]) -> Vec<u8> {
    let expected = (width as usize).checked_mul(height as usize).map(|n| n * 4);
    let ok = matches!(expected, Some(n) if bgra.len() == n);
    assert!(
        ok,
        "encode_bgra_frame_to_png: bgra length {} does not match {width}x{height}*4",
        bgra.len()
    );

    // RGBA8 output buffer: swap B and R channels on the fly.
    let pixels = width as usize * height as usize;
    let mut rgba = vec![0u8; pixels * 4];
    for (src, dst) in bgra.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        dst[0] = src[2]; // R ← B
        dst[1] = src[1]; // G ← G
        dst[2] = src[0]; // B ← R
        dst[3] = src[3]; // A ← A
    }

    crate::png::encode_rgba8(width, height, &rgba)
}

/// Same as [`encode_bgra_frame_to_png`] but returns a [`RenderError`] for the
/// io / encoding paths (useful when chaining from the pipeline).
pub fn encode_bgra_frame_to_png_result(
    width: u32,
    height: u32,
    bgra: &[u8],
) -> Result<Vec<u8>, RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::Job("png encode: zero dimension".into()));
    }
    if bgra.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err(RenderError::Job("png encode: bgra length mismatch".into()));
    }
    Ok(encode_bgra_frame_to_png(width, height, bgra))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_single_pixel_deterministically() {
        let frame = [10u8, 20, 30, 255]; // BGRA → RGBA(30,20,10,255)
        let a = encode_bgra_frame_to_png(1, 1, &frame);
        let b = encode_bgra_frame_to_png(1, 1, &frame);
        assert_eq!(a, b);
        // PNG signature check.
        assert_eq!(&a[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn rejects_bad_length_via_result() {
        let r = encode_bgra_frame_to_png_result(2, 2, &[0u8; 3]);
        assert!(r.is_err());
    }
}
