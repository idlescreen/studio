// SPDX-License-Identifier: Apache-2.0

use super::*;

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
