// SPDX-License-Identifier: Apache-2.0

use super::*;

pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bitbuf: u64,
    bitcnt: u32,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bitbuf: 0,
            bitcnt: 0,
        }
    }
    pub(crate) fn bits(&mut self, n: u32) -> Result<u32, String> {
        while self.bitcnt < n {
            let b = *self
                .data
                .get(self.pos)
                .ok_or("deflate: unexpected end of input")?;
            self.pos += 1;
            self.bitbuf |= (b as u64) << self.bitcnt;
            self.bitcnt += 8;
        }
        let v = (self.bitbuf & ((1u64 << n) - 1)) as u32;
        self.bitbuf >>= n;
        self.bitcnt -= n;
        Ok(v)
    }
    pub(crate) fn align(&mut self) {
        self.bitbuf = 0;
        self.bitcnt = 0;
    }
}

/// Canonical Huffman decoder: `lens[sym]` = code length (0 = unused).
pub(crate) struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    pub(crate) fn new(lens: &[u8]) -> Result<Self, String> {
        let mut counts = [0u16; 16];
        for &l in lens {
            if l > 15 {
                return Err("deflate: code length > 15".into());
            }
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        if counts.iter().all(|&c| c == 0) {
            return Err("deflate: empty code".into());
        }
        let mut offsets = [0u16; 16];
        for i in 1..15 {
            offsets[i + 1] = offsets[i] + counts[i];
        }
        let mut symbols = vec![0u16; lens.iter().filter(|&&l| l != 0).count()];
        for (sym, &l) in lens.iter().enumerate() {
            if l != 0 {
                symbols[offsets[l as usize] as usize] = sym as u16;
                offsets[l as usize] += 1;
            }
        }
        Ok(Self { counts, symbols })
    }

    pub(crate) fn decode(&self, r: &mut BitReader) -> Result<u16, String> {
        let mut code = 0u32;
        let mut first = 0u32;
        let mut index = 0usize;
        for len in 1..16u32 {
            code |= r.bits(1)?;
            let count = self.counts[len as usize] as u32;
            if code < first + count {
                return Ok(self.symbols[index + (code - first) as usize]);
            }
            index += count as usize;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("deflate: invalid huffman code".into())
    }
}

pub(crate) fn inflate(r: &mut BitReader, out: &mut Vec<u8>) -> Result<(), String> {
    loop {
        let bfinal = r.bits(1)?;
        let btype = r.bits(2)?;
        match btype {
            0 => {
                r.align();
                let len = r.bits(16)? as usize;
                let nlen = r.bits(16)? as usize;
                if len != (!nlen & 0xFFFF) {
                    return Err("deflate: bad stored length".into());
                }
                for _ in 0..len {
                    let b = r.bits(8)? as u8;
                    out.push(b);
                }
            }
            1 | 2 => {
                let (lit, dist) = if btype == 1 {
                    let mut lit_lens = [0u8; 288];
                    for (i, l) in lit_lens.iter_mut().enumerate() {
                        *l = match i {
                            0..=143 => 8,
                            144..=255 => 9,
                            256..=279 => 7,
                            _ => 8,
                        };
                    }
                    let dist_lens = [5u8; 30];
                    (Huffman::new(&lit_lens)?, Huffman::new(&dist_lens)?)
                } else {
                    read_dynamic_tables(r)?
                };
                loop {
                    let sym = lit.decode(r)?;
                    match sym {
                        0..=255 => out.push(sym as u8),
                        256 => break,
                        257..=285 => {
                            let idx = (sym - 257) as usize;
                            let len =
                                LEN_BASE[idx] as usize + r.bits(LEN_EXTRA[idx] as u32)? as usize;
                            let dsym = dist.decode(r)? as usize;
                            if dsym >= 30 {
                                return Err("deflate: bad distance symbol".into());
                            }
                            let dist_val = DIST_BASE[dsym] as usize
                                + r.bits(DIST_EXTRA[dsym] as u32)? as usize;
                            if dist_val > out.len() {
                                return Err("deflate: distance too far back".into());
                            }
                            let start = out.len() - dist_val;
                            for k in 0..len {
                                let b = out[start + k];
                                out.push(b);
                            }
                        }
                        _ => return Err("deflate: bad literal symbol".into()),
                    }
                }
            }
            _ => return Err("deflate: reserved block type".into()),
        }
        if bfinal == 1 {
            return Ok(());
        }
    }
}

const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub(crate) fn read_dynamic_tables(r: &mut BitReader) -> Result<(Huffman, Huffman), String> {
    let hlit = r.bits(5)? as usize + 257;
    let hdist = r.bits(5)? as usize + 1;
    let hclen = r.bits(4)? as usize + 4;
    let mut clen_lens = [0u8; 19];
    for &i in CLEN_ORDER.iter().take(hclen) {
        clen_lens[i] = r.bits(3)? as u8;
    }
    let clen = Huffman::new(&clen_lens)?;
    let mut lens = vec![0u8; hlit + hdist];
    let mut i = 0;
    while i < lens.len() {
        let sym = clen.decode(r)?;
        match sym {
            0..=15 => {
                lens[i] = sym as u8;
                i += 1;
            }
            16 => {
                if i == 0 {
                    return Err("deflate: repeat with no previous".into());
                }
                let prev = lens[i - 1];
                let n = 3 + r.bits(2)? as usize;
                for _ in 0..n {
                    if i >= lens.len() {
                        return Err("deflate: repeat overflow".into());
                    }
                    lens[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let n = 3 + r.bits(3)? as usize;
                i += n;
            }
            18 => {
                let n = 11 + r.bits(7)? as usize;
                i += n;
            }
            _ => return Err("deflate: bad code-length symbol".into()),
        }
        if i > lens.len() {
            return Err("deflate: code-length overflow".into());
        }
    }
    let lit = Huffman::new(&lens[..hlit])?;
    let dist = Huffman::new(&lens[hlit..])?;
    Ok((lit, dist))
}

/// Inflate a zlib stream (RFC 1950 + RFC 1951). Returns the decompressed bytes.
pub fn zlib_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 2 {
        return Err("zlib: truncated header".into());
    }
    let (cmf, flg) = (data[0] as u32, data[1] as u32);
    if cmf & 0x0F != 8 {
        return Err("zlib: not deflate".into());
    }
    if (cmf * 256 + flg) % 31 != 0 {
        return Err("zlib: bad header check".into());
    }
    if flg & 0x20 != 0 {
        return Err("zlib: preset dictionary unsupported".into());
    }
    let body = &data[2..data.len().saturating_sub(4).max(2)];
    let mut r = BitReader::new(body);
    let mut out = Vec::new();
    inflate(&mut r, &mut out)?;
    if data.len() >= 6 {
        let expect = u32::from_be_bytes(data[data.len() - 4..].try_into().unwrap());
        if adler32(&out) != expect {
            return Err("zlib: adler32 mismatch".into());
        }
    }
    Ok(out)
}

// ---------- PNG layer ----------
