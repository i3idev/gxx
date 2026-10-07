//! Translation, synonym, antonym commands: tr, syn, ant.

use rusqlite::Connection;
use serde_json::Value;

use crate::db::{load_rec_fallback, save_rec};
use crate::error::GxxError;
use crate::model::{add_unique, clean, lang_of, remove_item};

/// `gxx tr <word> [lang] +x -y`
pub fn cmd_tr(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx tr <word> [lang] +x -y"));
    }
    let mut rec = load_rec_fallback(con, "word", &args[0])?;
    let rest = &args[1..];
    let title = crate::model::title_of(&rec);
    let mut lang = if lang_of(&title) == "en" { "fa" } else { "en" };
    let mut ops = rest;
    if let Some(first) = rest.first() {
        if regex::Regex::new(r"^[a-z]{2}$").unwrap().is_match(first) {
            lang = first;
            ops = &rest[1..];
        }
    }

    // Ensure translations object exists
    if rec.get("translations").is_none() {
        rec["translations"] = Value::Object(serde_json::Map::new());
    }

    {
        let trans = rec["translations"].as_object_mut().unwrap();
        let arr = trans
            .entry(lang.to_string())
            .or_insert_with(|| Value::Array(vec![]))
            .as_array_mut()
            .unwrap();
        let (add, rem) = parse_ops(ops);
        for t in add {
            add_unique(arr, t);
        }
        for t in rem {
            if !remove_item(arr, &t) {
                eprintln!("not found: {}", t);
            }
        }
        if arr.is_empty() {
            trans.remove(lang);
        }
    }
    save_rec(con, "word", &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        let trans = rec["translations"].as_object().unwrap();
        let vals: Vec<String> = trans
            .get(lang)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let joined = vals.join(", ");
        println!(
            "ok: {} = {}",
            lang,
            if joined.is_empty() { "-" } else { &joined }
        );
    }
    Ok(())
}

/// `gxx syn|ant <word> +x -y`
pub fn cmd_syn_ant(con: &Connection, field: &str, args: &[String]) -> Result<(), GxxError> {
    let key = if field == "syn" {
        "synonyms"
    } else {
        "antonyms"
    };
    if args.is_empty() {
        return Err(GxxError::msg(format!("usage: gxx {} <word> +x -y", field)));
    }
    let mut rec = load_rec_fallback(con, "word", &args[0])?;
    if args.len() == 1 {
        let vals: Vec<String> = rec
            .get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        println!("{}", vals.join(", "));
        return Ok(());
    }

    // Ensure array exists
    if rec.get(key).is_none() {
        rec[key] = Value::Array(vec![]);
    }

    {
        let arr = rec[key].as_array_mut().unwrap();
        let (add, rem) = parse_ops(&args[1..]);
        for t in add {
            add_unique(arr, t);
        }
        for t in rem {
            if !remove_item(arr, &t) {
                eprintln!("not found: {}", t);
            }
        }
    }
    save_rec(con, "word", &mut rec)?;

    if !crate::model::STATE.lock().unwrap().quiet {
        let arr = rec[key].as_array().unwrap();
        let vals: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        let joined = vals.join(", ");
        println!("ok: {}", if joined.is_empty() { "-" } else { &joined });
    }
    Ok(())
}

fn parse_ops(tokens: &[String]) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut rem = Vec::new();
    for t in tokens {
        if t.starts_with('-') && t.len() > 1 && !t.starts_with("--") {
            rem.push(clean(&t[1..]));
        } else if t.starts_with('+') && t.len() > 1 {
            add.push(clean(&t[1..]));
        } else {
            add.push(clean(t));
        }
    }
    (add, rem)
}
