//! Self-contained PNG codec: RGBA8 encode + decode.
//!
//! Replaces the `png` crate. Encoder emits zlib/DEFLATE (greedy LZ77 with
//! fixed Huffman codes; stored blocks when incompressible) over unfiltered
//! scanlines — byte-deterministic for identical input. Decoder implements
//! full inflate (stored/fixed/dynamic blocks) and all five PNG filters for
//! 8-bit grayscale, gray+alpha, RGB, and RGBA sources — enough to read both
//! our own output and baselines produced by other encoders.

const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

// ---------- CRC32 / Adler32 ----------

fn crc32_table() -> &'static [u32; 256] {
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

fn adler32(data: &[u8]) -> u32 {
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

struct BitWriter {
    out: Vec<u8>,
    bitbuf: u64,
    bitcnt: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            bitbuf: 0,
            bitcnt: 0,
        }
    }
    /// Write `n` bits of `v`, LSB-first.
    fn bits(&mut self, v: u32, n: u32) {
        self.bitbuf |= (v as u64) << self.bitcnt;
        self.bitcnt += n;
        while self.bitcnt >= 8 {
            self.out.push(self.bitbuf as u8);
            self.bitbuf >>= 8;
            self.bitcnt -= 8;
        }
    }
    /// Write a Huffman code (codes are emitted MSB-of-code first).
    fn huff(&mut self, code: u32, n: u32) {
        // Reverse the n low bits of code, then emit LSB-first.
        let mut rev = 0u32;
        for i in 0..n {
            rev |= ((code >> i) & 1) << (n - 1 - i);
        }
        self.bits(rev, n);
    }
    fn align(&mut self) {
        if self.bitcnt > 0 {
            self.out.push(self.bitbuf as u8);
            self.bitbuf = 0;
            self.bitcnt = 0;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
}

/// Fixed literal/length code for `sym` (0..=287) per RFC 1951 §3.2.6.
fn fixed_lit_code(sym: u16) -> (u32, u32) {
    match sym {
        0..=143 => (0x30 + sym as u32, 8),
        144..=255 => (0x190 + (sym as u32 - 144), 9),
        256..=279 => (sym as u32 - 256, 7),
        _ => (0xC0 + (sym as u32 - 280), 8),
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn len_sym(len: usize) -> (u16, u32, u32) {
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

fn dist_sym(dist: usize) -> (u16, u32, u32) {
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
fn deflate_fixed(data: &[u8]) -> Vec<u8> {
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

fn zlib_encode(data: &[u8]) -> Vec<u8> {
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

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bitbuf: u64,
    bitcnt: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bitbuf: 0,
            bitcnt: 0,
        }
    }
    fn bits(&mut self, n: u32) -> Result<u32, String> {
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
    fn align(&mut self) {
        self.bitbuf = 0;
        self.bitcnt = 0;
    }
}

/// Canonical Huffman decoder: `lens[sym]` = code length (0 = unused).
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lens: &[u8]) -> Result<Self, String> {
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

    fn decode(&self, r: &mut BitReader) -> Result<u16, String> {
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

fn inflate(r: &mut BitReader, out: &mut Vec<u8>) -> Result<(), String> {
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

fn read_dynamic_tables(r: &mut BitReader) -> Result<(Huffman, Huffman), String> {
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

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
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

fn paeth(a: u8, b: u8, c: u8) -> u8 {
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
pub fn decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bytes.len() < 8 || bytes[..8] != SIG {
        return Err("png: bad signature".into());
    }
    let mut pos = 8;
    let (mut w, mut h, mut depth, mut color, mut interlace) = (0u32, 0u32, 0u8, 0u8, 0u8);
    let mut idat = Vec::new();
    let mut palette: Vec<u8> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start + len;
        if data_end + 4 > bytes.len() {
            return Err("png: truncated chunk".into());
        }
        let data = &bytes[data_start..data_end];
        let crc = u32::from_be_bytes(bytes[data_end..data_end + 4].try_into().unwrap());
        let mut crc_data = Vec::with_capacity(4 + len);
        crc_data.extend_from_slice(kind);
        crc_data.extend_from_slice(data);
        if crc32(&crc_data) != crc {
            return Err(format!(
                "png: CRC mismatch in {}",
                std::str::from_utf8(kind).unwrap_or("????")
            ));
        }
        match kind {
            b"IHDR" => {
                if len != 13 {
                    return Err("png: bad IHDR".into());
                }
                w = u32::from_be_bytes(data[0..4].try_into().unwrap());
                h = u32::from_be_bytes(data[4..8].try_into().unwrap());
                depth = data[8];
                color = data[9];
                if data[10] != 0 || data[11] != 0 {
                    return Err("png: bad compression/filter method".into());
                }
                interlace = data[12];
            }
            b"PLTE" => palette = data.to_vec(),
            b"tRNS" => trns = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        pos = data_end + 4;
    }
    if w == 0 || h == 0 {
        return Err("png: missing IHDR".into());
    }
    if interlace != 0 {
        return Err("png: interlaced images unsupported".into());
    }
    if depth != 8 {
        return Err(format!("png: bit depth {depth} unsupported (need 8)"));
    }
    let bpp = match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        c => return Err(format!("png: color type {c} unsupported")),
    };
    if color == 3 && palette.is_empty() {
        return Err("png: palette image without PLTE".into());
    }
    let stride = w as usize * bpp;
    let raw = zlib_decode(&idat)?;
    let expected = (stride + 1) * h as usize;
    if raw.len() < expected {
        return Err(format!(
            "png: image data too short ({} < {expected})",
            raw.len()
        ));
    }
    // Unfilter into RGBA8.
    let mut out = vec![0u8; w as usize * h as usize * 4];
    let mut prev = vec![0u8; stride];
    let mut cur = vec![0u8; stride];
    for y in 0..h as usize {
        let row_start = y * (stride + 1);
        let filter = raw[row_start];
        cur.copy_from_slice(&raw[row_start + 1..row_start + 1 + stride]);
        match filter {
            0 => {}
            1 => {
                for x in bpp..stride {
                    cur[x] = cur[x].wrapping_add(cur[x - bpp]);
                }
            }
            2 => {
                for x in 0..stride {
                    cur[x] = cur[x].wrapping_add(prev[x]);
                }
            }
            3 => {
                for x in 0..stride {
                    let a = if x >= bpp { cur[x - bpp] } else { 0 };
                    cur[x] = cur[x].wrapping_add(((a as u32 + prev[x] as u32) / 2) as u8);
                }
            }
            4 => {
                for x in 0..stride {
                    let a = if x >= bpp { cur[x - bpp] } else { 0 };
                    let c = if x >= bpp { prev[x - bpp] } else { 0 };
                    cur[x] = cur[x].wrapping_add(paeth(a, prev[x], c));
                }
            }
            f => return Err(format!("png: bad filter {f}")),
        }
        for x in 0..w as usize {
            let px = x * bpp;
            let o = (y * w as usize + x) * 4;
            match color {
                0 => {
                    let g = cur[px];
                    out[o] = g;
                    out[o + 1] = g;
                    out[o + 2] = g;
                    out[o + 3] = 255;
                }
                2 => {
                    out[o] = cur[px];
                    out[o + 1] = cur[px + 1];
                    out[o + 2] = cur[px + 2];
                    out[o + 3] = 255;
                }
                3 => {
                    let idx = cur[px] as usize * 3;
                    if idx + 2 >= palette.len() {
                        return Err("png: palette index out of range".into());
                    }
                    out[o] = palette[idx];
                    out[o + 1] = palette[idx + 1];
                    out[o + 2] = palette[idx + 2];
                    out[o + 3] = trns.get(cur[px] as usize).copied().unwrap_or(255);
                }
                4 => {
                    let g = cur[px];
                    out[o] = g;
                    out[o + 1] = g;
                    out[o + 2] = g;
                    out[o + 3] = cur[px + 1];
                }
                _ => {
                    out[o..o + 4].copy_from_slice(&cur[px..px + 4]);
                }
            }
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    Ok((w, h, out))
}

#[cfg(test)]
mod tests {
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
            0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03,
            0x08, 0xb1,
        ];
        let out = zlib_decode(z).expect("inflate");
        assert_eq!(out, b"hello hello hello hello");
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_rgba8(b"not a png").is_err());
        assert!(zlib_decode(&[0x78, 0x9c, 0x00]).is_err());
    }
}
