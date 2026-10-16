//! ANSI colour helpers for CLI output.
//!
//! Colours are enabled only when stdout is a TTY and `NO_COLOR` is
//! unset. The mode can be set explicitly via [`set_mode`] from
//! `main` based on CLI flags (`--no-color`).
//!
//! Modes:
//! - `0` — auto (TTY + no `NO_COLOR`)
//! - `1` — never
//! - `2` — always

use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

/// 0 = auto (TTY + no NO_COLOR), 1 = never, 2 = always.
static MODE: AtomicU8 = AtomicU8::new(0);

/// Set the colour mode. 0 = auto, 1 = never, 2 = always.
pub fn set_mode(mode: u8) {
    MODE.store(mode.min(2), Ordering::Relaxed);
}

/// Whether colours should be emitted right now.
pub fn enabled() -> bool {
    match MODE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal(),
    }
}

fn wrap(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

/// Bold text.
pub fn bold(s: &str) -> String {
    wrap("1", s)
}
/// Dim text.
pub fn dim(s: &str) -> String {
    wrap("2", s)
}
/// Red text.
pub fn red(s: &str) -> String {
    wrap("31", s)
}
/// Green text.
pub fn green(s: &str) -> String {
    wrap("32", s)
}
/// Yellow text.
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
/// Magenta text.
pub fn magenta(s: &str) -> String {
    wrap("35", s)
}
/// Cyan text.
pub fn cyan(s: &str) -> String {
    wrap("36", s)
}

/// Section header (bold).
pub fn header(s: &str) -> String {
    bold(s)
}
/// Key/label (cyan).
pub fn key(s: &str) -> String {
    cyan(s)
}
/// Success marker (green).
pub fn ok(s: &str) -> String {
    green(s)
}
/// Warning marker (yellow).
pub fn warn(s: &str) -> String {
    yellow(s)
}
/// Error marker (red).
pub fn err(s: &str) -> String {
    red(s)
}
/// Accent (magenta).
pub fn accent(s: &str) -> String {
    magenta(s)
}
