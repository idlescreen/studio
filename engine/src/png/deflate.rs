// SPDX-License-Identifier: Apache-2.0

use super::*;

pub(crate) struct BitWriter {
    out: Vec<u8>,
    bitbuf: u64,
    bitcnt: u32,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        Self {
            out: Vec::new(),
            bitbuf: 0,
            bitcnt: 0,
        }
    }
    /// Write `n` bits of `v`, LSB-first.
    pub(crate) fn bits(&mut self, v: u32, n: u32) {
        self.bitbuf |= (v as u64) << self.bitcnt;
        self.bitcnt += n;
        while self.bitcnt >= 8 {
            self.out.push(self.bitbuf as u8);
            self.bitbuf >>= 8;
            self.bitcnt -= 8;
        }
    }
    /// Write a Huffman code (codes are emitted MSB-of-code first).
    pub(crate) fn huff(&mut self, code: u32, n: u32) {
        // Reverse the n low bits of code, then emit LSB-first.
        let mut rev = 0u32;
        for i in 0..n {
            rev |= ((code >> i) & 1) << (n - 1 - i);
        }
        self.bits(rev, n);
    }
    pub(crate) fn align(&mut self) {
        if self.bitcnt > 0 {
            self.out.push(self.bitbuf as u8);
            self.bitbuf = 0;
            self.bitcnt = 0;
        }
    }
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
}

/// Fixed literal/length code for `sym` (0..=287) per RFC 1951 §3.2.6.
pub(crate) fn fixed_lit_code(sym: u16) -> (u32, u32) {
    match sym {
        0..=143 => (0x30 + sym as u32, 8),
        144..=255 => (0x190 + (sym as u32 - 144), 9),
        256..=279 => (sym as u32 - 256, 7),
        _ => (0xC0 + (sym as u32 - 280), 8),
    }
}

pub(crate) const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub(crate) const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub(crate) const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub(crate) const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub(crate) fn len_sym(len: usize) -> (u16, u32, u32) {
    debug_assert!((3..=258).contains(&len));
    for i in (0..29).rev() {
        if len >= LEN_BASE[i] as usize && !(i == 28 && len == 258) {
            return (
                257 + i as u16,
                (len - LEN_BASE[i] as usize) as u32,
                LEN_EXTRA[i] as u32,
            );
        }
    }
    (285, 0, 0) // len == 258
}

pub(crate) fn dist_sym(dist: usize) -> (u16, u32, u32) {
    debug_assert!((1..=32768).contains(&dist));
    for i in (0..30).rev() {
        if dist >= DIST_BASE[i] as usize {
            return (
                i as u16,
                (dist - DIST_BASE[i] as usize) as u32,
                DIST_EXTRA[i] as u32,
            );
        }
    }
    unreachable!()
}

/// DEFLATE one block with greedy LZ77 + fixed Huffman codes.
pub(crate) fn deflate_fixed(data: &[u8]) -> Vec<u8> {
    const HASH_BITS: u32 = 15;
    const HASH_SIZE: usize = 1 << HASH_BITS;
    const WINDOW: usize = 32768;
    let hash = |d: &[u8], i: usize| -> usize {
        ((d[i] as usize) << 10 ^ (d[i + 1] as usize) << 5 ^ d[i + 2] as usize) & (HASH_SIZE - 1)
    };
    let mut w = BitWriter::new();
    w.bits(1, 1); // BFINAL
    w.bits(1, 2); // BTYPE = fixed Huffman
    let mut head = vec![-1i32; HASH_SIZE];
    let mut i = 0usize;
    while i < data.len() {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + 3 <= data.len() {
            let h = hash(data, i);
            let cand = head[h];
            head[h] = i as i32;
            if cand >= 0 {
                let c = cand as usize;
                let dist = i - c;
                if dist <= WINDOW {
                    let max = (data.len() - i).min(258);
                    let mut l = 0usize;
                    while l < max && data[c + l] == data[i + l] {
                        l += 1;
                    }
                    if l >= 3 {
                        best_len = l;
                        best_dist = dist;
                    }
                }
            }
        }
        if best_len >= 3 {
            let (sym, extra, ebits) = len_sym(best_len);
            let (code, clen) = fixed_lit_code(sym);
            w.huff(code, clen);
            if ebits > 0 {
                w.bits(extra, ebits);
            }
            let (dsym, dextra, debits) = dist_sym(best_dist);
            w.huff(dsym as u32, 5);
            if debits > 0 {
                w.bits(dextra, debits);
            }
            // Register every position in the match for better future hits.
            let end = i + best_len;
            i += 1;
            while i < end {
                if i + 3 <= data.len() {
                    head[hash(data, i)] = i as i32;
                }
                i += 1;
            }
        } else {
            let (code, clen) = fixed_lit_code(data[i] as u16);
            w.huff(code, clen);
            i += 1;
        }
    }
    let (code, clen) = fixed_lit_code(256); // end of block
    w.huff(code, clen);
    w.finish()
}

pub(crate) fn zlib_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 64);
    out.extend_from_slice(&[0x78, 0x01]); // zlib header, 32K window
    let compressed = deflate_fixed(data);
    if compressed.len() < data.len() + 5 * (data.len() / 65535 + 1) {
        out.extend_from_slice(&compressed);
    } else {
        // Incompressible input: emit stored blocks.
        let mut pos = 0;
        loop {
            let n = (data.len() - pos).min(65535);
            let last = pos + n == data.len();
            out.push(if last { 1 } else { 0 }); // BFINAL + BTYPE=00
            out.extend_from_slice(&(n as u16).to_le_bytes());
            out.extend_from_slice(&(!(n as u16)).to_le_bytes());
            out.extend_from_slice(&data[pos..pos + n]);
            pos += n;
            if last {
                break;
            }
        }
        if data.is_empty() {
            out.push(1);
            out.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

// ---------- zlib / deflate decoder ----------
