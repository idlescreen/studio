// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn encode_decode_roundtrip() {
    // Mixed content: solid run + noise to exercise LZ77 and literals.
    let (w, h) = (64u32, 32u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut s = 0x1234_5678u64;
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let v = if i < 200 { 128 } else { s as u8 };
        px[0] = v;
        px[1] = 255u8.wrapping_sub(v);
        px[2] = (i % 256) as u8;
        px[3] = 255;
    }
    let png = encode_rgba8(w, h, &rgba);
    assert_eq!(&png[..8], &SIG);
    let (dw, dh, dec) = decode_rgba8(&png).expect("decode");
    assert_eq!((dw, dh), (w, h));
    assert_eq!(dec, rgba);
}

#[test]
fn decode_dynamic_huffman_zlib() {
    // zlib stream produced by zlib.compress() (dynamic Huffman) over
    // "hello hello hello hello" — exercises the BTYPE=2 path.
    let z: &[u8] = &[
        0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08,
        0xb1,
    ];
    let out = zlib_decode(z).expect("inflate");
    assert_eq!(out, b"hello hello hello hello");
}

#[test]
fn rejects_garbage() {
    assert!(decode_rgba8(b"not a png").is_err());
    assert!(zlib_decode(&[0x78, 0x9c, 0x00]).is_err());
}
