//! ANSI colors for CLI output.
//!
//! Colors are enabled only when stdout is a TTY and `NO_COLOR` is unset.
//! Set the mode explicitly via [`set_mode`] from `main` based on CLI flags.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

/// 0 = auto (TTY + no NO_COLOR), 1 = never, 2 = always.
static MODE: AtomicU8 = AtomicU8::new(0);

/// Set the color mode. 0 = auto, 1 = never, 2 = always.
pub fn set_mode(mode: u8) {
    MODE.store(mode.min(2), Ordering::Relaxed);
}

/// Whether colors should be emitted right now.
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

pub fn bold(s: &str) -> String {
    wrap("1", s)
}
pub fn dim(s: &str) -> String {
    wrap("2", s)
}
pub fn red(s: &str) -> String {
    wrap("31", s)
}
pub fn green(s: &str) -> String {
    wrap("32", s)
}
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
pub fn magenta(s: &str) -> String {
    wrap("35", s)
}
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
