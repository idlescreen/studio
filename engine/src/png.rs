//! Self-contained PNG codec: RGBA8 encode + decode.
//!
//! Replaces the `png` crate. Encoder emits zlib/DEFLATE (greedy LZ77 with
//! fixed Huffman codes; stored blocks when incompressible) over unfiltered
//! scanlines — byte-deterministic for identical input. Decoder implements
//! full inflate (stored/fixed/dynamic blocks) and all five PNG filters for
//! 8-bit grayscale, gray+alpha, RGB, and RGBA sources — enough to read both
//! our own output and baselines produced by other encoders.

mod decode;
mod deflate;
mod inflate;

pub use decode::decode_rgba8;
pub(crate) use deflate::*;
pub(crate) use inflate::*;

const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

// ---------- CRC32 / Adler32 ----------

pub(crate) fn crc32_table() -> &'static [u32; 256] {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *e = c;
        }
        t
    })
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = crc32_table()[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub(crate) fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

// ---------- zlib / deflate encoder ----------
pub(crate) fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(kind);
    crc_data.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_data).to_be_bytes());
}

/// Encode `rgba` (width*height*4 bytes) as a PNG. Deterministic.
pub fn encode_rgba8(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(
        rgba.len(),
        width as usize * height as usize * 4,
        "encode_rgba8: rgba length mismatch"
    );
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in rgba.chunks_exact(stride) {
        raw.push(0); // filter: None
        raw.extend_from_slice(row);
    }
    let mut out = Vec::with_capacity(raw.len() / 2 + 64);
    out.extend_from_slice(&SIG);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_encode(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

pub(crate) fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (a, b, c) = (a as i32, b as i32, c as i32);
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

/// Decode a PNG into `(width, height, rgba8)`.
///
/// Supports non-interlaced 8-bit gray (0), gray+alpha (4), RGB (2), and
/// RGBA (6) — the formats any sane encoder emits for opaque/alpha content.
#[cfg(test)]
#[path = "png_tests.rs"]
mod tests;
