// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Converting the rasterizer's ARGB pixels into the encoder's BGRA frames.
//!
//! The blitter writes ARGB8888 (`0xAARRGGBB`) as 32-bit words; ffmpeg is fed
//! BGRA bytes. On a little-endian machine those are the same four bytes in a
//! different order, so the conversion is a byte-order read — but it is
//! written out explicitly rather than reinterpreted, so the result does not
//! depend on the host's endianness and the crate stays free of `unsafe`.

/// Reorder one ARGB8888 word into BGRA bytes.
#[inline]
pub const fn argb_to_bgra(px: u32) -> [u8; 4] {
    let [a, r, g, b] = px.to_be_bytes();
    [b, g, r, a]
}

/// Convert a whole ARGB frame into BGRA bytes.
///
/// `out` is resized to the frame length; the caller owns it so the same
/// buffer can be reused for every frame of a render.
pub fn argb_frame_to_bgra(argb: &[u32], out: &mut Vec<u8>) {
    let needed = argb.len().saturating_mul(4);
    if out.len() != needed {
        out.clear();
        out.resize(needed, 0);
    }
    for (slot, px) in out.chunks_exact_mut(4).zip(argb.iter()) {
        slot.copy_from_slice(&argb_to_bgra(*px));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_stays_red() {
        // 0xAARRGGBB with R=255, G=0, B=0.
        let px = 0xFFFF_0000u32;
        assert_eq!(argb_to_bgra(px), [0, 0, 255, 255]);
    }

    #[test]
    fn blue_stays_blue() {
        // 0xAARRGGBB with R=0, G=0, B=255.
        let px = 0xFF00_00FFu32;
        assert_eq!(argb_to_bgra(px), [255, 0, 0, 255]);
    }

    #[test]
    fn green_stays_green() {
        let px = 0xFF00_FF00u32;
        assert_eq!(argb_to_bgra(px), [0, 255, 0, 255]);
    }

    #[test]
    fn the_channel_order_is_bgra() {
        assert_eq!(argb_to_bgra(0x1122_3344), [0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn a_frame_converts_end_to_end() {
        let argb = vec![0xFFFF_0000u32, 0xFF00_00FFu32];
        let mut out = Vec::new();
        argb_frame_to_bgra(&argb, &mut out);
        assert_eq!(out.len(), 8);
        assert_eq!(&out[0..4], &[0, 0, 255, 255]);
        assert_eq!(&out[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn the_buffer_is_reused_not_reallocated() {
        let argb = vec![0u32; 16];
        let mut out = Vec::new();
        argb_frame_to_bgra(&argb, &mut out);
        let ptr = out.as_ptr();
        argb_frame_to_bgra(&argb, &mut out);
        assert_eq!(ptr, out.as_ptr());
    }
}
