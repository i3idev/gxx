//! Example commands: ex (shared by word and grammar kinds).

use rusqlite::Connection;
use serde_json::Value;

use crate::db::{load_rec_fallback, save_rec};
use crate::error::GxxError;
use crate::model::lang_of;
use crate::render::ex_lines;

/// `gxx ex <name> ["text"] | N "text" | -N`
pub fn cmd_ex(con: &Connection, kind: &str, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg(
            "usage: gxx ex <name> [\"text\"] | N \"text\" | -N",
        ));
    }
    let mut rec = load_rec_fallback(con, kind, &args[0])?;
    let rest = &args[1..];

    // Ensure examples array exists
    if rec.get("examples").is_none() {
        rec["examples"] = Value::Array(vec![]);
    }

    if rest.is_empty() {
        // List examples
        let exs = rec["examples"].as_array().unwrap();
        println!("{}", ex_lines(exs));
        return Ok(());
    }

    // Check for -N (remove example N)
    if let Some(first) = rest.first() {
        if first.starts_with('-') && first[1..].chars().all(|c| c.is_ascii_digit()) {
            let n: usize = first[1..].parse().unwrap();
            {
                let exs = rec["examples"].as_array_mut().unwrap();
                check_n(exs, n, "example")?;
                exs.remove(n - 1);
            }
            save_rec(con, kind, &mut rec)?;
            if !crate::model::STATE.lock().unwrap().quiet {
                let exs = rec["examples"].as_array().unwrap();
                println!("{}", ex_lines(exs));
            }
            return Ok(());
        }
    }

    // Check for N "text" (set one side of example N)
    if let Some(first) = rest.first() {
        if first.chars().all(|c| c.is_ascii_digit()) {
            let n: usize = first.parse().unwrap();
            {
                let exs = rec["examples"].as_array_mut().unwrap();
                check_n(exs, n, "example")?;
                if rest.len() != 2 {
                    return Err(GxxError::msg("usage: gxx ex <name> N \"text\""));
                }
                let lang = lang_of(&rest[1]);
                if exs[n - 1].get(lang).is_some() {
                    // Allow overwrite
                }
                exs[n - 1][lang] = Value::String(rest[1].clone());
            }
            save_rec(con, kind, &mut rec)?;
            if !crate::model::STATE.lock().unwrap().quiet {
                let exs = rec["examples"].as_array().unwrap();
                println!("{}", ex_lines(exs));
            }
            return Ok(());
        }
    }

    // Add new example: one or two texts, different languages
    if rest.len() > 2 {
        return Err(GxxError::msg(
            "one example per command: quote each sentence",
        ));
    }
    {
        let exs = rec["examples"].as_array_mut().unwrap();
        let mut ex = serde_json::Map::new();
        for t in rest {
            let lg = lang_of(t);
            if ex.contains_key(lg) {
                return Err(GxxError::msg(
                    "two texts in the same language; give one en and one fa",
                ));
            }
            ex.insert(lg.to_string(), Value::String(t.clone()));
        }
        exs.push(Value::Object(ex));
    }
    save_rec(con, kind, &mut rec)?;
    if !crate::model::STATE.lock().unwrap().quiet {
        let exs = rec["examples"].as_array().unwrap();
        println!("{}", ex_lines(exs));
    }
    Ok(())
}

fn check_n(items: &mut [Value], n: usize, what: &str) -> Result<(), GxxError> {
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
