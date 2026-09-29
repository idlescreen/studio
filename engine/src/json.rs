//! Minimal JSON parse/serialize for the studio job/queue schemas.
//!
//! Replaces `serde_json`: ordered objects, exact u64/i64 integers, recursion
//! cap matching serde_json's default (128). Writers match `to_string` /
//! `to_string_pretty` output style so existing job files stay compatible.

mod parser;
mod parser_str;
mod value;

pub use parser::parse;
pub(crate) use parser::*;
pub(crate) use value::*;
pub use value::{Error, Value};

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
