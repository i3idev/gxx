//! Load command: run a file of commands.

use shlex::split;
use std::path::Path;

use crate::error::GxxError;
use crate::model::{set_quiet, STATE};
use crate::run::run_internal;

/// `gxx load <file.gxx>`
pub fn cmd_load(args: &[String], db_override: Option<&str>) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx load <file.gxx>"));
    }
    let path = Path::new(&args[0]);
    if !path.exists() {
        return Err(GxxError::msg(format!("no such file: {}", path.display())));
    }
    let content = std::fs::read_to_string(path)?;
    let mut done = 0;
    let mut bad = 0;
    let previous_quiet = {
        let s = STATE.lock().unwrap();
        s.quiet
    };
    set_quiet(true);

    for (no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = match split(line) {
            Some(p) => p,
            None => {
                eprintln!("line {}: parse error", no + 1);
                bad += 1;
                continue;
            }
        };
        if parts.is_empty() {
            continue;
        }
        let parts: Vec<String> = parts.into_iter().collect();
        // Optional leading "gxx"
        let parts = if parts[0] == "gxx" {
            &parts[1..]
        } else {
            &parts[..]
        };
        if parts.is_empty() {
            continue;
        }
        if parts[0] == "load" {
            eprintln!("line {}: load inside a file is not allowed", no + 1);
            bad += 1;
            continue;
        }
        match run_internal(parts.to_vec(), db_override) {
            Ok(()) => done += 1,
            Err(e) => {
                eprintln!("line {}: {}", no + 1, e);
                bad += 1;
            }
        }
    }

    set_quiet(previous_quiet);
    println!("loaded {} commands, {} errors", done, bad);
    Ok(())
}
