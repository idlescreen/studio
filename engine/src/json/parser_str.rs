// SPDX-License-Identifier: Apache-2.0

use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn string(&mut self) -> Result<String, Error> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let b = self.bump().ok_or_else(|| self.err("unterminated string"))?;
            match b {
                b'"' => return Ok(out),
                b'\\' => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'b') => out.push('\u{0008}'),
                    Some(b'f') => out.push('\u{000C}'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => {
                        let hi = self.hex4()?;
                        let cp = if (0xD800..0xDC00).contains(&hi) {
                            // High surrogate: require \uXXXX low surrogate.
                            if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                return Err(self.err("invalid unicode escape"));
                            }
                            let lo = self.hex4()?;
                            if !(0xDC00..0xE000).contains(&lo) {
                                return Err(self.err("invalid unicode escape"));
                            }
                            0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                        } else {
                            hi
                        };
                        out.push(
                            char::from_u32(cp).ok_or_else(|| self.err("invalid unicode escape"))?,
                        );
                    }
                    _ => return Err(self.err("invalid escape")),
                },
                0x00..=0x1F => return Err(self.err("control character in string")),
                _ => {
                    // Collect the full UTF-8 sequence (input is &str — valid).
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    self.pos += len - 1;
                    self.col += len - 1;
                    out.push_str(
                        std::str::from_utf8(&self.bytes[start..self.pos])
                            .map_err(|_| self.err("invalid utf-8"))?,
                    );
                }
            }
        }
    }

    pub(crate) fn hex4(&mut self) -> Result<u32, Error> {
        let mut v = 0u32;
        for _ in 0..4 {
            let d = match self.bump() {
                Some(c @ b'0'..=b'9') => c - b'0',
                Some(c @ b'a'..=b'f') => c - b'a' + 10,
                Some(c @ b'A'..=b'F') => c - b'A' + 10,
                _ => return Err(self.err("invalid unicode escape")),
            };
            v = v * 16 + d as u32;
        }
        Ok(v)
    }

    pub(crate) fn number(&mut self) -> Result<Value, Error> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => {
                self.bump();
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.bump();
                }
            }
            _ => return Err(self.err("invalid number")),
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.bump();
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.err("invalid number"))?;
        if is_float {
            return text
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| self.err("number out of range"));
        }
        if let Ok(u) = text.parse::<u64>() {
            return Ok(Value::UInt(u));
        }
        if let Ok(i) = text.parse::<i64>() {
            return Ok(Value::Int(i));
        }
        // Out of integer range: serde_json yields f64.
        text.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| self.err("number out of range"))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}
