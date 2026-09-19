//! Minimal stderr logging replacing `tracing`/`tracing-subscriber`.
//!
//! `RUST_LOG` env var selects the minimum level (`error`, `warn`, `info`,
//! `debug`, `trace`, or `off`); default is `info`. Macros are crate-root
//! exported like the tracing ones they replace.

use std::sync::atomic::{AtomicU8, Ordering};

pub const ERROR: u8 = 1;
pub const WARN: u8 = 2;
pub const INFO: u8 = 3;
pub const DEBUG: u8 = 4;
pub const TRACE: u8 = 5;

static LEVEL: AtomicU8 = AtomicU8::new(INFO);

/// Initialize the level from `RUST_LOG` (default `default_level`).
/// Call once at binary start; idempotent.
pub fn init(default_level: &str) {
    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| parse_level(&v))
        .unwrap_or_else(|| parse_level(default_level).unwrap_or(INFO));
    LEVEL.store(level, Ordering::Relaxed);
}

fn parse_level(v: &str) -> Option<u8> {
    // Accept a bare level or an EnvFilter-style `target=level` spec; the
    // rightmost bare level word wins.
    for tok in v.split(',').rev() {
        let word = tok.rsplit('=').next().unwrap_or(tok).trim();
        let l = match word.to_ascii_lowercase().as_str() {
            "off" => 0,
            "error" => ERROR,
            "warn" | "warning" => WARN,
            "info" => INFO,
            "debug" => DEBUG,
            "trace" => TRACE,
            _ => continue,
        };
        return Some(l);
    }
    None
}

#[doc(hidden)]
pub fn enabled(level: u8) -> bool {
    level <= LEVEL.load(Ordering::Relaxed)
}

#[doc(hidden)]
pub fn emit(level: u8, name: &str, args: std::fmt::Arguments) {
    if enabled(level) {
        eprintln!("{name:>5} render: {args}");
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::ERROR, "ERROR", format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::WARN, "WARN", format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::INFO, "INFO", format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::DEBUG, "DEBUG", format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::TRACE, "TRACE", format_args!($($arg)*)) };
}
