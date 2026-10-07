//! Database commands: init, db new|list|use|info|del.

use regex::Regex;
use rusqlite::Connection;

use crate::db::{active_name, db_names, db_path, DB_NAME_RE, SCHEMA};
use crate::error::GxxError;
use crate::render::{render_db_info, render_db_list_line};

/// Initialize the GXX home directory and config.
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

/// Database subcommands: new, list, use, info, del.
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

fn cmd_db_new(rest: &[String]) -> Result<(), GxxError> {
    if rest.is_empty() {
        return Err(GxxError::msg("usage: gxx db new <name>"));
    }
    let name = &rest[0];
    let re = Regex::new(DB_NAME_RE).unwrap();
    if !re.is_match(name) {
        return Err(GxxError::msg("db name: letters, digits, - and _ only"));
    }
    let path = db_path(name);
    if path.exists() {
        return Err(GxxError::msg(format!("'{}' already exists", name)));
    }
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
    println!("deleted: {}", name);
    Ok(())
}
