//! Minimal JSON parse/serialize for the studio job/queue schemas.
//!
//! Replaces `serde_json`: ordered objects, exact u64/i64 integers, recursion
//! cap matching serde_json's default (128). Writers match `to_string` /
//! `to_string_pretty` output style so existing job files stay compatible.

use std::fmt;

/// JSON value. Objects preserve insertion order.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

/// Parse error with serde_json-style position info.
#[derive(Debug)]
pub struct Error {
    msg: String,
    line: usize,
    col: usize,
}

impl Error {
    fn new(msg: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            msg: msg.into(),
            line,
            col,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at line {} column {}", self.msg, self.line, self.col)
    }
}

impl std::error::Error for Error {}

const MAX_DEPTH: usize = 128;

/// Parse a JSON document.
pub fn parse(text: &str) -> Result<Value, Error> {
    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        line: 1,
        col: 1,
    };
    p.skip_ws();
    let v = p.value(0)?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(p.err("trailing characters"));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: impl Into<String>) -> Error {
        Error::new(msg, self.line, self.col)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.bump();
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), Error> {
        if self.bump() == Some(b) {
            Ok(())
        } else {
            Err(self.err(format!("expected '{}'", b as char)))
        }
    }

    fn lit(&mut self, word: &str, v: Value) -> Result<Value, Error> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            for _ in 0..word.len() {
                self.bump();
            }
            Ok(v)
        } else {
            Err(self.err("expected value"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(self.err("recursion limit exceeded"));
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.lit("true", Value::Bool(true)),
            Some(b'f') => self.lit("false", Value::Bool(false)),
            Some(b'n') => self.lit("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.err("expected value")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        self.expect(b'{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Value::Object(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected object key"));
            }
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let v = self.value(depth + 1)?;
            pairs.push((key, v));
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => return Ok(Value::Object(pairs)),
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => return Ok(Value::Array(items)),
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn string(&mut self) -> Result<String, Error> {
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

    fn hex4(&mut self) -> Result<u32, Error> {
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

    fn number(&mut self) -> Result<Value, Error> {
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

impl Value {
    /// Look up `key` in an object value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Unsigned integer view (serde_json-equivalent: signed non-negative ok).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::UInt(u) => Some(*u),
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }

    /// Compact serialization (`serde_json::to_string`).
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, None, 0);
        s
    }

    /// Pretty serialization (`serde_json::to_string_pretty`, 2-space indent).
    pub fn to_json_pretty(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, Some(2), 0);
        s
    }

    fn write(&self, out: &mut String, indent: Option<usize>, level: usize) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Int(i) => out.push_str(&i.to_string()),
            Value::UInt(u) => out.push_str(&u.to_string()),
            Value::Float(f) => out.push_str(&format!("{f:?}")),
            Value::Str(s) => write_str(s, out),
            Value::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline_indent(out, indent, level + 1);
                    v.write(out, indent, level + 1);
                }
                newline_indent(out, indent, level);
                out.push(']');
            }
            Value::Object(pairs) => {
                if pairs.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline_indent(out, indent, level + 1);
                    write_str(k, out);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, level + 1);
                }
                newline_indent(out, indent, level);
                out.push('}');
            }
        }
    }
}

fn newline_indent(out: &mut String, indent: Option<usize>, level: usize) {
    if let Some(n) = indent {
        out.push('\n');
        for _ in 0..n * level {
            out.push(' ');
        }
    }
}

/// Escape a string like serde_json: only what JSON requires.
fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_pretty() {
        let src = r#"{"a": 1, "b": [true, null, "x\n"], "c": {"d": -2}}"#;
        let v = parse(src).unwrap();
        let pretty = v.to_json_pretty();
        let v2 = parse(&pretty).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn escapes_and_unicode() {
        let v = parse(r#""aéb😀""#).unwrap();
        assert_eq!(v.as_str().unwrap(), "a\u{e9}b\u{1F600}");
        let v = parse(r#""\u0041\ud83d\ude00""#).unwrap();
        assert_eq!(v.as_str().unwrap(), "A\u{1F600}");
    }

    #[test]
    fn integers_exact() {
        let v = parse("18446744073709551615").unwrap();
        assert_eq!(v.as_u64(), Some(u64::MAX));
        let v = parse("-5").unwrap();
        assert_eq!(v.as_u64(), None);
        // Floats are not integers for serde-typed fields.
        let v = parse("30.0").unwrap();
        assert_eq!(v.as_u64(), None);
    }

    #[test]
    fn trailing_garbage_rejected() {
        assert!(parse("{} x").is_err());
        assert!(parse("{").is_err());
    }

    #[test]
    fn malformed_inputs_never_panic() {
        // Every prefix of a valid doc — Err or Ok, never panic.
        let good = r#"{"a": [1, 2.5, "x", true, null, {"b": "y"}], "c": "\u00e9"}"#;
        for i in 0..=good.len() {
            if let Some(s) = good.get(..i) {
                let _ = parse(s);
            }
        }
        for bad in [
            "", "{", "[", "\"", "\\", "{\"a\":", "[1,", "-", "1e", "1e+", "0.",
            ".5", "+1", "01", "--1", "{\"a\"}", "[1;2]", "{,}", "[,]", "\"\\u\"",
            "\"\\uZZZZ\"", "\"\\uD800\"", "\"\\uD800x\"", "\"\\uDC00\"", "\"\\x\"",
            "tru", "truex", "nulll", "NaN", "Infinity", "{\"a\":1,}", "{a:1}",
            "1.2.3", "1e9999", "-0", "{}", "[]", "[[[]]]",
        ] {
            let _ = parse(bad);
        }
        // Deep nesting past MAX_DEPTH must Err, not overflow the stack.
        let deep = "[".repeat(1000) + &"]".repeat(1000);
        assert!(parse(&deep).is_err());
        // Control characters inside strings.
        assert!(parse("\"\u{0007}\"").is_err());
        // Lone "-" / "+" start no valid value.
        assert!(parse("[-]").is_err());
    }
}
