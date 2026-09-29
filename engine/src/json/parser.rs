// SPDX-License-Identifier: Apache-2.0

use super::*;

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

pub(crate) struct Parser<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn err(&self, msg: impl Into<String>) -> Error {
        Error::new(msg, self.line, self.col)
    }

    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(crate) fn bump(&mut self) -> Option<u8> {
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

    pub(crate) fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.bump();
        }
    }

    pub(crate) fn expect(&mut self, b: u8) -> Result<(), Error> {
        if self.bump() == Some(b) {
            Ok(())
        } else {
            Err(self.err(format!("expected '{}'", b as char)))
        }
    }

    pub(crate) fn lit(&mut self, word: &str, v: Value) -> Result<Value, Error> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            for _ in 0..word.len() {
                self.bump();
            }
            Ok(v)
        } else {
            Err(self.err("expected value"))
        }
    }

    pub(crate) fn value(&mut self, depth: usize) -> Result<Value, Error> {
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

    pub(crate) fn object(&mut self, depth: usize) -> Result<Value, Error> {
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

    pub(crate) fn array(&mut self, depth: usize) -> Result<Value, Error> {
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
}
