//! Grammar/Style/Form commands: add, list, del, err, field setters, show.

use rusqlite::Connection;
use serde_json::Value;

use crate::cmd::ex::cmd_ex;
use crate::db::{load_rec, load_rec_fallback, save_rec};
use crate::error::GxxError;
use crate::model::{clean, key_of, new_grammar, title_of};
use crate::render::{render_entry, render_gram_list};

/// Grammar/Style/Form field mapping
const GRAM_FIELDS: &[(&str, &str)] = &[
    ("rule", "rule"),
    ("desc", "desc"),
    ("cat", "cat"),
    ("lvl", "level"),
    ("note", "notes"),
];

/// `gxx gram|style|form add <name> [--cat c] [--lvl l] [--rule r] [--desc d]`
pub fn cmd_gadd(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    let (pos, opts) = parse_flags(
        args,
        &[
            ("--cat", "cat"),
            ("-c", "cat"),
            ("--lvl", "level"),
            ("-l", "level"),
            ("--rule", "rule"),
            ("-r", "rule"),
            ("--desc", "desc"),
            ("-d", "desc"),
        ],
    );
    if pos.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} add <name> [--cat c] [--lvl l] [--rule r] [--desc d]",
            crate::model::label(kind)
        )));
    }
    let name = if pos[0].contains(' ') {
        pos[0].trim().to_string()
    } else {
        clean(&pos[0])
    };
    let existing = load_rec(con, kind, &name)?;
    let created = existing.is_none();
    let mut rec = if created {
        new_grammar(kind, &name)
    } else {
        existing.unwrap()
    };
    for (k, v) in opts {
        if !v.is_empty() {
            rec[k] = serde_json::Value::String(v);
        } else {
            rec.as_object_mut().unwrap().remove(&k);
        }
    }
    save_rec(con, kind, &mut rec)?;
    if !crate::model::STATE.lock().unwrap().quiet {
        println!("{}: {}", if created { "added" } else { "updated" }, name);
    }
    Ok(())
}

/// `gxx gram|style|form list [prefix]`
pub fn cmd_glist(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    let prefix = if args.is_empty() {
        String::new()
    } else {
        key_of(&args[0])
    };
    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind=? ORDER BY lower(title)")?;
    let mut rows = stmt.query([kind])?;
    let mut n = 0;
    while let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&json_str)?;
        let k = key_of(&title_of(&rec));
        if !prefix.is_empty() && !(k == prefix || k.starts_with(&format!("{}.", prefix))) {
            continue;
        }
        println!("{}", render_gram_list(&rec));
        n += 1;
    }
    println!("-- {} {} entries", n, crate::model::label(kind));
    Ok(())
}

/// `gxx gram|style|form del <name>`
pub fn cmd_gdel(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx del <name>"));
    }
    load_rec_fallback(con, kind, &args[0])?; // verify exists
    con.execute(
        "DELETE FROM entries WHERE id = ?1",
        [crate::model::entry_id(kind, &args[0])],
    )?;
    if !crate::model::STATE.lock().unwrap().quiet {
        println!("deleted: {}", clean(&args[0]));
    }
    Ok(())
}

/// `gxx gram|style|form err <name> ["text"] [-N]`
pub fn cmd_gerr(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} err <name> [\"text\"] [-N]",
            crate::model::label(kind)
        )));
    }
    let mut rec = load_rec_fallback(con, kind, &args[0])?;
    let rest = &args[1..];

    // Ensure errors array exists
    if rec.get("errors").is_none() {
        rec["errors"] = serde_json::Value::Array(vec![]);
    }

    if rest.is_empty() {
        let errs = rec["errors"].as_array().unwrap();
        if errs.is_empty() {
            println!("no errors listed");
        } else {
            for (i, e) in errs.iter().enumerate() {
                if let Some(t) = e.as_str() {
                    println!(" {}> {}", i + 1, t);
                }
            }
        }
        return Ok(());
    }

    if let Some(first) = rest.first() {
        if first.starts_with('-') && first[1..].chars().all(|c| c.is_ascii_digit()) {
            let n: usize = first[1..].parse().unwrap();
            {
                let errs = rec["errors"].as_array_mut().unwrap();
                check_n(errs, n, "error")?;
                errs.remove(n - 1);
            }
            save_rec(con, kind, &mut rec)?;
            if !crate::model::STATE.lock().unwrap().quiet {
                let errs = rec["errors"].as_array().unwrap();
                println!("ok: {} error(s)", errs.len());
            }
            return Ok(());
        }
    }

    // Add new error
    {
        let errs = rec["errors"].as_array_mut().unwrap();
        errs.push(serde_json::Value::String(rest.join(" ")));
    }
    save_rec(con, kind, &mut rec)?;
    if !crate::model::STATE.lock().unwrap().quiet {
        let errs = rec["errors"].as_array().unwrap();
        println!("ok: {} error(s)", errs.len());
    }
    Ok(())
}

/// `gxx gram|style|form rule|desc|cat|lvl|note <name> [value]`
pub fn cmd_gset(
    con: &Connection,
    kind: &str,
    field: &str,
    args: &[String],
) -> Result<(), GxxError> {
    let key = GRAM_FIELDS
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, k)| *k);
    let key = match key {
        Some(k) => k,
        None => return Err(GxxError::msg("unknown field")),
    };
    if args.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} <name> [value]",
            field
        )));
    }
    let mut rec = load_rec_fallback(con, kind, &args[0])?;
    if args.len() == 1 {
        let val = rec.get(key).and_then(|v| v.as_str()).unwrap_or("-");
        println!("{}", val);
        return Ok(());
    }
    let value = args[1..].join(" ").trim().to_string();
    if !value.is_empty() {
        rec[key] = serde_json::Value::String(value.clone());
    } else {
        rec.as_object_mut().unwrap().remove(key);
    }
    save_rec(con, kind, &mut rec)?;
    if !crate::model::STATE.lock().unwrap().quiet {
        println!("ok: {} = {}", field, value.trim().trim_start());
    }
    Ok(())
}

/// `gxx gram|style|form show <name> [sections...]`
pub fn cmd_gshow(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} show <name>",
            crate::model::label(kind)
        )));
    }
    let name = &args[0];
    let sections = args[1..].to_vec();
    let rec = load_rec_fallback(con, kind, name)?;
    println!("{}", render_entry(kind, &rec, &sections));
    Ok(())
}

/// Main entry point for gram/style/form group commands
pub fn cmd_group(con: &Connection, grp: &str, args: &[String]) -> Result<(), GxxError> {
    let kind = match grp {
        "gram" => "grammar",
        "style" => "style",
        "form" => "form",
        _ => return Err(GxxError::msg(format!("unknown group: {}", grp))),
    };
    if args.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} add|list|del|ex|err|rule|desc|cat|lvl|note|show ...",
            grp
        )));
    }
    let sub = &args[0];
    let rest = &args[1..];
    match sub.as_str() {
        "add" => cmd_gadd(con, kind, rest),
        "list" => cmd_glist(con, kind, rest),
        "del" => cmd_gdel(con, kind, rest),
        "err" => cmd_gerr(con, kind, rest),
        "ex" => cmd_ex(con, kind, rest),
        "show" => cmd_gshow(con, kind, rest),
        "rule" | "desc" | "cat" | "lvl" | "note" => cmd_gset(con, kind, sub, rest),
        _ => Err(GxxError::msg(format!("unknown: gxx {} {}", grp, sub))),
    }
}

fn parse_flags(
    args: &[String],
    spec: &[(&str, &str)],
) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut opts = std::collections::HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let mut matched = false;
        for (flag, key) in spec {
            if a == *flag {
                if i + 1 >= args.len() {
                    return (vec![], std::collections::HashMap::new());
                }
                opts.insert(key.to_string(), args[i + 1].clone());
                i += 2;
                matched = true;
                break;
            }
        }
        if !matched {
            if a.starts_with("--") && a.len() > 2 {
                return (vec![], std::collections::HashMap::new());
            }
            positional.push(a.clone());
            i += 1
        }
    }
    (positional, opts)
}

fn check_n(items: &[Value], n: usize, what: &str) -> Result<(), GxxError> {
    if n == 0 || n > items.len() {
        return Err(GxxError::msg(format!(
            "no {} #{} (have {})",
            what,
            n,
            items.len()
        )));
    }
    Ok(())
}
