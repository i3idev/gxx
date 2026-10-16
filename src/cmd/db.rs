//! Database commands: `init`, `db new|list|use|info|del`.
//!
//! This module implements the database management surface of `gxx`:
//!
//! - [`cmd_init`] — initialize the gxx data directory
//! - [`cmd_db`] — dispatch `gxx db` subcommands (`new`, `list`, `use`,
//!   `info`, `del`)
//!
//! Database files are SQLite databases named `<name>.db`, stored under
//! `<data-dir>/db/`. The active database name is recorded in
//! `<data-dir>/config`.

use regex::Regex;
use rusqlite::Connection;

use crate::db::{active_name, db_names, db_path, DB_NAME_RE, SCHEMA};
use crate::error::GxxError;
use crate::render::{render_db_info, render_db_list_line};

/// Initialize the gxx data directory and config file.
///
/// Creates `<data-dir>/` and `<data-dir>/db/`, plus an empty `config`
/// file. Prints the data directory path and, if no databases exist,
/// suggests `gxx db new <name>`.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn cmd_init() -> Result<(), GxxError> {
    std::fs::create_dir_all(crate::db::db_dir())?;
    let cfg = crate::db::config_path();
    if !cfg.exists() {
        std::fs::write(&cfg, "")?;
    }
    println!("ready: {}", crate::db::home_dir().display());
    let names = db_names()?;
    if names.is_empty() {
        println!("next : gxx db new <name>");
    }
    Ok(())
}

/// Dispatch the `gxx db` subcommand.
///
/// Supported subcommands: `new`, `list`, `use`, `info`, `del`.
///
/// All subcommands require an initialized data directory (run `gxx init`
/// first); `new` also requires it so it cannot silently create a home
/// in a different scope.
///
/// # Errors
///
/// Returns an error for unknown subcommands or missing initialization.
pub fn cmd_db(_con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx db new|list|use|info|del [name]"));
    }
    let sub = &args[0];
    let rest = &args[1..];

    // Ensure init for subcommands that need it
    if sub != "new" && !crate::db::config_path().exists() {
        return Err(GxxError::msg("not initialised. run: gxx init"));
    }

    match sub.as_str() {
        "new" => cmd_db_new(rest),
        "list" => cmd_db_list(),
        "use" => cmd_db_use(rest),
        "info" => cmd_db_info(rest),
        "del" => cmd_db_del(rest),
        _ => Err(GxxError::msg(format!("unknown: gxx db {}", sub))),
    }
}

/// Create a new SQLite database.
///
/// The name must match `^[\w-]+$` (letters, digits, underscore,
/// hyphen). If the database already exists, reports an error. If no
/// active database is set, the new database becomes active.
///
/// # Errors
///
/// - `db name: letters, digits, - and _ only` — invalid name
/// - `'<name>' already exists` — database already exists
fn cmd_db_new(rest: &[String]) -> Result<(), GxxError> {
    if rest.is_empty() {
        return Err(GxxError::msg("usage: gxx db new <name>"));
    }
    let name = &rest[0];
    let re = Regex::new(DB_NAME_RE).unwrap();
    if !re.is_match(name) {
        return Err(GxxError::msg("db name: letters, digits, - and _ only"));
    }
    if !crate::db::home_dir().exists() {
        return Err(GxxError::msg("not initialised. run: gxx init"));
    }
    let path = db_path(name);
    if path.exists() {
        return Err(GxxError::msg(format!("'{}' already exists", name)));
    }
    std::fs::create_dir_all(crate::db::db_dir())?;
    let con = Connection::open(&path)?;
    con.execute_batch(SCHEMA)?;
    println!("created: {}", name);
    // If no active db, set this as active
    let current = active_name(None).unwrap_or_default();
    if current.is_empty() {
        std::fs::write(crate::db::config_path(), name)?;
        println!("active : {}", name);
    }
    Ok(())
}

/// List all databases with entry counts and last-updated timestamps.
///
/// The active database is marked with `*`.
fn cmd_db_list() -> Result<(), GxxError> {
    let names = db_names()?;
    if names.is_empty() {
        println!("no databases. run: gxx db new <name>");
        return Ok(());
    }
    let current = active_name(None).unwrap_or_default();
    println!("  NAME            ENTRIES  UPDATED");
    for name in names {
        let path = db_path(&name);
        let con = Connection::open(&path)?;
        let (cnt, upd): (i64, Option<String>) =
            con.query_row("SELECT COUNT(*), MAX(updated_at) FROM entries", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        println!("{}", render_db_list_line(&current, &name, cnt, upd));
    }
    Ok(())
}

/// Set the active database by name.
///
/// Writes the name to `config`. Fails if the database does not exist.
///
/// # Errors
///
/// - `usage: gxx db use <name>   (see: gxx db list)` — no name or unknown name
fn cmd_db_use(rest: &[String]) -> Result<(), GxxError> {
    if rest.is_empty() {
        return Err(GxxError::msg(
            "usage: gxx db use <name>   (see: gxx db list)",
        ));
    }
    let name = &rest[0];
    let names = db_names()?;
    if !names.contains(name) {
        return Err(GxxError::msg(
            "usage: gxx db use <name>   (see: gxx db list)",
        ));
    }
    std::fs::write(crate::db::config_path(), name)?;
    println!("active : {}", name);
    Ok(())
}

/// Show database info: path, active status, and per-kind entry counts
/// (word, grammar, style, form).
///
/// If no name is given, shows info for the active database.
///
/// # Errors
///
/// - `no database '<name>'` — database does not exist
/// - `no active db. run: gxx init, then: gxx db new <name>` — no name and no active db
fn cmd_db_info(rest: &[String]) -> Result<(), GxxError> {
    let name = if rest.is_empty() {
        active_name(None)?
    } else {
        rest[0].clone()
    };
    let path = db_path(&name);
    if !path.exists() {
        return Err(GxxError::msg(format!("no database '{}'", name)));
    }
    let con = Connection::open(&path)?;
    let current = active_name(None).unwrap_or_default();
    let is_active = name == current;

    let mut counts = Vec::new();
    for kind in ["word", "grammar", "style", "form"] {
        let cnt: i64 = con.query_row(
            "SELECT COUNT(*) FROM entries WHERE kind = ?1",
            [kind],
            |row| row.get(0),
        )?;
        counts.push((kind, cnt));
    }

    for line in render_db_info(&name, &path, is_active, &counts) {
        println!("{}", line);
    }
    Ok(())
}

fn cmd_db_del(rest: &[String]) -> Result<(), GxxError> {
    if rest.is_empty() {
        return Err(GxxError::msg("usage: gxx db del <name> [--yes]"));
    }
    let name = &rest[0];
    let names = db_names()?;
    if !names.contains(name) {
        return Err(GxxError::msg("usage: gxx db del <name> [--yes]"));
    }
    if !rest.contains(&"--yes".to_string()) {
        eprint!("type '{}' to delete it: ", name);
        use std::io::{stdin, stdout, Write};
        stdout().flush()?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        if input.trim() != *name {
            return Err(GxxError::msg("cancelled"));
        }
    }
    std::fs::remove_file(db_path(name))?;
    let current = active_name(None).unwrap_or_default();
    if name == &current {
        std::fs::write(crate::db::config_path(), "")?;
    }
    println!("{}: {}", crate::style::warn("deleted"), name);
    Ok(())
}
