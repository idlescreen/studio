// SPDX-License-Identifier: Apache-2.0

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
    pub(crate) fn new(msg: impl Into<String>, line: usize, col: usize) -> Self {
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

pub(crate) const MAX_DEPTH: usize = 128;

/// Parse a JSON document.
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
