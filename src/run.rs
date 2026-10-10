//! Internal run function for dispatching commands.

use crate::cmd::db::{cmd_db, cmd_init};
use crate::cmd::grammar::cmd_group;
use crate::cmd::load::cmd_load;
use crate::cmd::tr_syn_ant::{cmd_syn_ant, cmd_tr};
use crate::cmd::word::{
    cmd_add, cmd_coll, cmd_del, cmd_ex_word, cmd_family, cmd_find, cmd_list, cmd_say, cmd_sense,
    cmd_set_word, cmd_show,
};
use crate::error::GxxError;
use rusqlite::Connection;

/// Run a command internally (used by load and main).
pub fn run_internal(argv: Vec<String>, db_override: Option<&str>) -> Result<(), GxxError> {
    if argv.is_empty() {
        print_usage();
        return Ok(());
    }

    let mut argv = argv;
    let json_output = argv.iter().any(|arg| arg == "--json");
    argv.retain(|arg| arg != "--json");

    if argv.is_empty() {
        print_usage();
        return Ok(());
    }

    let cmd = &argv[0];
    let rest = &argv[1..];

    match cmd.as_str() {
        "help" | "-h" | "--help" => {
            print_usage();
            return Ok(());
        }
        "init" => return cmd_init(),
        "db" => {
            let con = Connection::open_in_memory()?;
            return cmd_db(&con, rest);
        }
        "load" => return cmd_load(rest, db_override),
        _ => {}
    }

    let con = crate::db::connect(db_override)?;

    match cmd.as_str() {
        "add" => cmd_add(&con, rest),
        "show" => {
            if json_output {
                print_entry_json(&con, rest)?;
                Ok(())
            } else {
                cmd_show(&con, rest)
            }
        }
        "find" => {
            if json_output {
                print_find_json(&con, rest)?;
                Ok(())
            } else {
                cmd_find(&con, rest, db_override)
            }
        }
        "list" => {
            if json_output {
                print_list_json(&con, rest)?;
                Ok(())
            } else {
                cmd_list(&con, rest)
            }
        }
        "del" => cmd_del(&con, rest),
        "tr" => cmd_tr(&con, rest),
        "syn" => cmd_syn_ant(&con, "syn", rest),
        "ant" => cmd_syn_ant(&con, "ant", rest),
        "ex" => cmd_ex_word(&con, rest),
        "sense" => cmd_sense(&con, rest),
        "coll" => cmd_coll(&con, rest),
        "family" => cmd_family(&con, rest),
        "say" => cmd_say(&con, rest),
        "pos" | "lvl" | "pron" | "def" | "note" => cmd_set_word(&con, cmd, rest),
        "gram" | "style" | "form" => cmd_group(&con, cmd, rest),
        _ => {
            let mut args = vec![cmd.clone()];
            args.extend(rest.iter().cloned());
            if json_output {
                print_entry_json(&con, &args)?;
                Ok(())
            } else {
                cmd_show(&con, &args)
            }
        }
    }
}

fn print_usage() {
    println!(
        r#"gxx - bilingual word and grammar notebook

setup
  gxx init
  gxx db new|list|use|info|del [name]

words
  gxx add <word> [translations...] [-p pos] [-l level]
  gxx <word>                 same as: gxx show <word> [tr|syn|ant|ex|sense|coll|family]
  gxx find <text> [--all]    gxx list [--pos x] [--lvl x] [--lang en|fa]
  gxx tr  <word> [lang] +x -y        gxx syn|ant <word> +x -y
  gxx ex  <word> ["text"]            add an example (language is detected)
  gxx ex  <word> N "text"            set one side of example N
  gxx ex  <word> -N                  remove example N
  gxx pos|lvl|pron|def|note <word> [value]
  gxx sense <word> [add|del|show]
  gxx coll <word> [add|del]
  gxx family <word> [add|del]
  gxx say <word> [--example N] [--repeat N] [--slow] [--very-slow] [--soft]
  gxx del <word>

grammar (the same commands work for: style, form)
  gxx gram add <name> [--cat c] [--lvl l] [--rule r] [--desc d]
  gxx gram rule|desc|cat|lvl|note <name> [value]
  gxx gram ex  <name> ...            gxx gram err <name> ["text"] [-N]
  gxx gram list [prefix]             gxx gram del <name>

files
  gxx load <file.gxx>        run a file of commands (one per line, # = comment)

options
  --db <name>                use another database for this command only
"#
    );
}

/// Print word records as a JSON array, preserving list filters.
fn print_list_json(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    use serde_json::Value;

    let mut pos_filter: Option<String> = None;
    let mut lvl_filter: Option<String> = None;
    let mut lang_filter: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--pos" | "--lvl" | "--lang" => {
                let flag = args[i].as_str();
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| GxxError::msg(format!("missing value after {}", flag)))?;
                match flag {
                    "--pos" => pos_filter = Some(value.to_lowercase()),
                    "--lvl" => lvl_filter = Some(value.to_lowercase()),
                    "--lang" => lang_filter = Some(value.to_lowercase()),
                    _ => unreachable!(),
                }
                i += 2;
            }
            other => {
                return Err(GxxError::msg(format!("unknown list option: {}", other)));
            }
        }
    }

    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind = 'word'")?;
    let mut rows = stmt.query([])?;
    let mut records: Vec<Value> = Vec::new();

    while let Some(row) = rows.next()? {
        let raw: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&raw)?;

        if let Some(ref lvl) = lvl_filter {
            let value = rec.get("level").and_then(Value::as_str).unwrap_or("");
            if value.to_lowercase() != *lvl {
                continue;
            }
        }

        if let Some(ref pos) = pos_filter {
            let value = rec.get("pos").and_then(Value::as_str).unwrap_or("");
            if value.to_lowercase() != *pos {
                continue;
            }
        }

        if let Some(ref lang) = lang_filter {
            let title = crate::model::title_of(&rec);
            if crate::model::lang_of(&title) != lang.as_str() {
                continue;
            }
        }

        records.push(rec);
    }

    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}

/// Print search results as a JSON array of complete records.
fn print_find_json(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    let filtered: Vec<String> = args
        .iter()
        .filter(|arg| arg.as_str() != "--all")
        .cloned()
        .collect();

    if filtered.is_empty() {
        return Err(GxxError::msg("usage: gxx find <text> [--all] --json"));
    }

    let query = filtered.join(" ");
    let records = crate::db::search_records(con, &query)?;
    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}

/// Print a word or grammar entry as machine-readable JSON.
fn print_entry_json(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    use crate::db::load_rec_fallback;

    let name = args
        .first()
        .ok_or_else(|| GxxError::msg("usage: gxx <word> --json"))?;

    let mut found = Vec::new();
    for kind in ["word", "grammar", "style", "form"] {
        if let Ok(rec) = load_rec_fallback(con, kind, name) {
            found.push(rec);
        }
    }

    match found.len() {
        0 => Err(GxxError::msg(format!("not found: {}", name))),
        1 => {
            println!("{}", serde_json::to_string_pretty(&found[0])?);
            Ok(())
        }
        _ => {
            println!("{}", serde_json::to_string_pretty(&found)?);
            Ok(())
        }
    }
}
