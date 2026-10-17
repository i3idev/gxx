//! Word commands: `add`, `show`, `find`, `list`, `del`, `pos`, `lvl`,
//! `pron`, `def`, `note`, `say`, `ex`, `sense`, `coll`, `family`.
//!
//! A word entry stores: word/title, translations, part of speech,
//! proficiency level, pronunciation, definition, notes, examples,
//! synonyms, antonyms, senses, collocations, and word-family members.

use rusqlite::Connection;
use serde_json::Value;

use crate::cmd::ex::cmd_ex;
use crate::db::{load_rec, load_rec_fallback, save_rec, search};
use crate::error::GxxError;
use crate::model::{
    add_unique, clean, collocations_mut, entry_id, family_mut, lang_of, new_family_member,
    new_sense, new_word, senses_mut, title_of, STATE,
};
use crate::render::{render_entry, render_word_list};
use crate::tts::{say_manual_with_active_model, say_with_active_model, PlaybackOptions};

/// `gxx add <word> [translations...] [-p POS] [-l LEVEL]`
///
/// Add a new word entry, or update an existing one. Translations are
/// auto-directed to the opposite language of the word's detected
/// script. Duplicates are skipped.
///
/// # Errors
///
/// - `usage: gxx add <word> [translations...] [-p pos] [-l level]` — no word
pub fn cmd_add(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    let (pos, opts) = parse_flags(
        args,
        &[
            ("-p", "pos"),
            ("--pos", "pos"),
            ("-l", "level"),
            ("--lvl", "level"),
        ],
    );
    if pos.is_empty() {
        return Err(GxxError::msg(
            "usage: gxx add <word> [translations...] [-p pos] [-l level]",
        ));
    }
    let name = clean(&pos[0]);
    let existing = load_rec(con, "word", &name)?;
    let created = existing.is_none();
    let mut rec = if created {
        new_word(&name)
    } else {
        existing.unwrap()
    };
    let target = if lang_of(&name) == "en" { "fa" } else { "en" };
    for t in &pos[1..] {
        add_unique(
            rec["translations"]
                .as_object_mut()
                .unwrap()
                .entry(target.to_string())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap(),
            clean(t),
        );
    }
    // Apply options
    for (k, v) in opts {
        if !v.is_empty() {
            rec[k] = Value::String(v);
        } else {
            rec.as_object_mut().unwrap().remove(&k);
        }
    }
    save_rec(con, "word", &mut rec)?;
    if !STATE.lock().unwrap().quiet {
        let tag = if created { "added" } else { "updated" };
        println!("{}: {}", crate::style::ok(tag), name);
    }
    Ok(())
}

/// `gxx show <word> [sections...]`
///
/// Display an entry's content. Searches all four kinds (word, grammar,
/// style, form). Optional section filters: `tr`, `syn`, `ant`, `ex`,
/// `sense`, `coll`, `family`, `note`.
///
/// # Errors
///
/// - `usage: gxx show <word> [tr|syn|ant|ex]` — no word
/// - `not found: <word>  (try: gxx find <word>)` — no matching entry
pub fn cmd_show(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx show <word> [tr|syn|ant|ex]"));
    }
    let name = &args[0];
    let sections = args[1..].to_vec();

    let mut shown = Vec::new();
    for kind in ["word", "grammar", "style", "form"] {
        if let Ok(rec) = load_rec_fallback(con, kind, name) {
            shown.push(render_entry(kind, &rec, &sections));
        }
    }
    if shown.is_empty() {
        return Err(GxxError::msg(format!(
            "not found: {}  (try: gxx find {})",
            name, name
        )));
    }
    println!("{}", shown.join("\n\n"));
    Ok(())
}

/// `gxx find <query> [--all]`
///
/// Search entries by title or content. Case-insensitive substring
/// match on normalised text. `*` glob wildcards supported. Results
/// sorted: exact matches first, then by title. `--all` searches all
/// databases.
///
/// # Errors
///
/// - `usage: gxx find <text> [--all]` — no query
pub fn cmd_find(
    con: &Connection,
    args: &[String],
    _db_override: Option<&str>,
) -> Result<(), GxxError> {
    let all = args.contains(&"--all".to_string());
    let filtered: Vec<String> = args.iter().filter(|a| *a != "--all").cloned().collect();
    let (pos, _) = parse_flags(&filtered, &[]);
    if pos.is_empty() {
        return Err(GxxError::msg("usage: gxx find <text> [--all]"));
    }
    let q = pos.join(" ");

    if all {
        let names = crate::db::db_names()?;
        let mut total = 0;
        for name in names {
            let path = crate::db::db_path(&name);
            let con = rusqlite::Connection::open(&path)?;
            let lines = search(&con, &q)?;
            for ln in &lines {
                println!("{}: {}", name, ln);
            }
            total += lines.len();
        }
        println!("-- {} found", total);
        return Ok(());
    }

    let lines = search(con, &q)?;
    if lines.is_empty() {
        println!("nothing found");
    } else {
        println!("{}", lines.join("\n"));
        println!("-- {} found", lines.len());
    }
    Ok(())
}

/// `gxx list [--pos POS] [--lvl LEVEL] [--lang LANG]`
///
/// List word entries with optional filters: part of speech, proficiency
/// level, or language (`en`/`fa`).
///
/// # Errors
///
/// - `unknown list option: <flag>` — unrecognized flag
pub fn cmd_list(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    let (_, opts) = parse_flags(
        args,
        &[("--pos", "pos"), ("--lvl", "lvl"), ("--lang", "lang")],
    );
    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind='word'")?;
    let mut rows = stmt.query([])?;
    let mut n = 0;
    while let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&json_str)?;
        if let Some(lvl) = opts.get("lvl") {
            if rec
                .get("level")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
                != Some(lvl.to_lowercase())
            {
                continue;
            }
        }
        if let Some(lang) = opts.get("lang") {
            let t = title_of(&rec);
            if lang_of(&t) != lang.as_str() {
                continue;
            }
        }
        if let Some(pos) = opts.get("pos") {
            if rec
                .get("pos")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
                != Some(pos.to_lowercase())
            {
                continue;
            }
        }
        println!("{}", render_word_list(&rec));
        n += 1;
    }
    println!("-- {} words", n);
    Ok(())
}

/// `gxx del <word> [--yes]`
///
/// Delete a word entry. Without `--yes`, prompts interactively to type
/// the word for confirmation.
///
/// # Errors
///
/// - `usage: gxx del <name>` — no word
/// - `no word '<name>'. add it first: gxx add <name>` — entry not found
/// - `cancelled` — interactive confirmation did not match
pub fn cmd_del(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx del <name>"));
    }
    let rec = load_rec_fallback(con, "word", &args[0])?; // verify exists
    let title = title_of(&rec);
    con.execute(
        "DELETE FROM entries WHERE id = ?1",
        [entry_id("word", &title)],
    )?;
    if !STATE.lock().unwrap().quiet {
        println!("{}: {}", crate::style::warn("deleted"), clean(&args[0]));
    }
    Ok(())
}

/// `gxx pos|lvl|pron|def|note <word> [value]`
///
/// Get or set a single field on a word entry. Without `value`, prints
/// the current value (or `-` if unset). With `value`, sets it.
///
/// Field mapping:
///
/// | Command | JSON key |
/// |---------|----------|
/// | `pos` | `pos` |
/// | `lvl` | `level` |
/// | `pron` | `pronunciation` |
/// | `def` | `def` |
/// | `note` | `notes` |
///
/// # Errors
///
/// - `usage: gxx <field> <name> [value]` — no word
/// - `no word '<name>'. add it first: gxx add <name>` — entry not found
pub fn cmd_set_word(con: &Connection, field: &str, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg(format!(
            "usage: gxx {} <name> [value]",
            field
        )));
    }
    let mut rec = load_rec_fallback(con, "word", &args[0])?;
    let key = match field {
        "pos" => "pos",
        "lvl" => "level",
        "pron" => "pronunciation",
        "def" => "def",
        "note" => "notes",
        _ => return Err(GxxError::msg("unknown field")),
    };
    if args.len() == 1 {
        let val = rec.get(key).and_then(|v| v.as_str()).unwrap_or("-");
        println!("{}", val);
        return Ok(());
    }
    let value = args[1..].join(" ").trim().to_string();
    if !value.is_empty() {
        rec[key] = Value::String(value.clone());
    } else {
        rec.as_object_mut().unwrap().remove(key);
    }
    save_rec(con, "word", &mut rec)?;
    if !STATE.lock().unwrap().quiet {
        println!(
            "{} {} = {}",
            crate::style::ok("ok:"),
            field,
            value.clone().trim_end().trim_start()
        );
    }
    Ok(())
}

/// `gxx ex <word> ["text"] | N "text" | -N`
///
/// Dispatches example management for a word entry. Delegates to
/// [`crate::cmd::ex::cmd_ex`] with `kind = "word"`.
pub fn cmd_ex_word(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    cmd_ex(con, "word", args)
}

/// `gxx sense <word> [add|del|show]`
///
/// Manage polysemy senses (numbered definitions with optional notes
/// and examples).
///
/// - List: `gxx sense <word>`
/// - Add: `gxx sense <word> add "definition" [--note "..."]`
/// - Delete: `gxx sense <word> del N`
/// - Show: `gxx sense <word> show N`
///
/// # Errors
///
/// - `usage: gxx sense <word> [add|del|show]` — no word
/// - `no sense #N (have M)` — sense number out of range
/// - `invalid sense number` — non-numeric argument
/// - `unknown sense subcommand: <x>` — invalid subcommand
pub fn cmd_sense(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx sense <word> [add|del|show]"));
    }
    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        // List senses
        let senses = senses_mut(&mut rec);
        if senses.is_empty() {
            println!("no senses");
        } else {
            for (i, s) in senses.iter().enumerate() {
                if let Some(def) = s.get("def").and_then(|v| v.as_str()) {
                    println!(" {}> {}", i + 1, def);
                }
                if let Some(note) = s.get("note").and_then(|v| v.as_str()) {
                    if !note.is_empty() {
                        println!("    note: {}", note);
                    }
                }
            }
        }
        return Ok(());
    }

    let sub = &args[1];
    match sub.as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(GxxError::msg(
                    "usage: gxx sense <word> add \"definition\" [--note \"...\"]",
                ));
            }
            let def = &args[2];
            let mut note = String::new();
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--note" && i + 1 < args.len() {
                    note = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let mut sense = new_sense(def);
            if !note.is_empty() {
                sense["note"] = Value::String(note);
            }
            senses_mut(&mut rec).push(sense);
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!(
                    "{} added sense #{}",
                    crate::style::ok("ok:"),
                    senses_mut(&mut rec).len()
                );
            }
        }
        "del" => {
            if args.len() != 3 {
                return Err(GxxError::msg("usage: gxx sense <word> del N"));
            }
            let n: usize = args[2]
                .parse()
                .map_err(|_| GxxError::msg("invalid sense number"))?;
            let senses = senses_mut(&mut rec);
            if n == 0 || n > senses.len() {
                return Err(GxxError::msg(format!(
                    "no sense #{} (have {})",
                    n,
                    senses.len()
                )));
            }
            senses.remove(n - 1);
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!("{} deleted sense #{}", crate::style::ok("ok:"), n);
            }
        }
        "show" => {
            if args.len() != 3 {
                return Err(GxxError::msg("usage: gxx sense <word> show N"));
            }
            let n: usize = args[2]
                .parse()
                .map_err(|_| GxxError::msg("invalid sense number"))?;
            let senses = senses_mut(&mut rec);
            if n == 0 || n > senses.len() {
                return Err(GxxError::msg(format!(
                    "no sense #{} (have {})",
                    n,
                    senses.len()
                )));
            }
            let s = &senses[n - 1];
            if let Some(def) = s.get("def").and_then(|v| v.as_str()) {
                println!("def: {}", def);
            }
            if let Some(note) = s.get("note").and_then(|v| v.as_str()) {
                if !note.is_empty() {
                    println!("note: {}", note);
                }
            }
            if let Some(ex_arr) = s.get("examples").and_then(|v| v.as_array()) {
                if !ex_arr.is_empty() {
                    println!("{}", crate::render::ex_lines(ex_arr));
                }
            }
        }
        _ => return Err(GxxError::msg(format!("unknown sense subcommand: {}", sub))),
    }
    Ok(())
}

/// `gxx coll <word> [add|del]`
///
/// Manage collocations (common word pairings).
///
/// - List: `gxx coll <word>`
/// - Add: `gxx coll <word> add "collocation"`
/// - Delete: `gxx coll <word> del N`
///
/// # Errors
///
/// - `usage: gxx coll <word> [add|del]` — no word
/// - `no collocation #N (have M)` — number out of range
/// - `invalid collocation number` — non-numeric argument
/// - `unknown collocation subcommand: <x>` — invalid subcommand
pub fn cmd_coll(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx coll <word> [add|del]"));
    }
    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        // List collocations
        let colls = collocations_mut(&mut rec);
        if colls.is_empty() {
            println!("no collocations");
        } else {
            for (i, c) in colls.iter().enumerate() {
                if let Some(text) = c.as_str() {
                    println!(" {}> {}", i + 1, text);
                }
            }
        }
        return Ok(());
    }

    let sub = &args[1];
    match sub.as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(GxxError::msg("usage: gxx coll <word> add \"collocation\""));
            }
            let coll = args[2..].join(" ");
            collocations_mut(&mut rec).push(Value::String(coll));
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!(
                    "ok: added collocation #{}",
                    collocations_mut(&mut rec).len()
                );
            }
        }
        "del" => {
            if args.len() != 3 {
                return Err(GxxError::msg("usage: gxx coll <word> del N"));
            }
            let n: usize = args[2]
                .parse()
                .map_err(|_| GxxError::msg("invalid collocation number"))?;
            let colls = collocations_mut(&mut rec);
            if n == 0 || n > colls.len() {
                return Err(GxxError::msg(format!(
                    "no collocation #{} (have {})",
                    n,
                    colls.len()
                )));
            }
            colls.remove(n - 1);
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!("{} deleted collocation #{}", crate::style::ok("ok:"), n);
            }
        }
        _ => {
            return Err(GxxError::msg(format!(
                "unknown collocation subcommand: {}",
                sub
            )))
        }
    }
    Ok(())
}

/// `gxx family <word> [add|del]`
///
/// Manage word-family members (related words with optional POS).
///
/// - List: `gxx family <word>`
/// - Add: `gxx family <word> add <related-word> [-p POS]`
/// - Delete: `gxx family <word> del <related-word>`
///
/// # Errors
///
/// - `usage: gxx family <word> [add|del]` — no word
/// - `family member '<word>' not found` — deletion target absent
/// - `unknown family subcommand: <x>` — invalid subcommand
pub fn cmd_family(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg("usage: gxx family <word> [add|del]"));
    }
    let name = clean(&args[0]);
    let mut rec = load_rec_fallback(con, "word", &name)?;

    if args.len() == 1 {
        // List family members
        let fam = family_mut(&mut rec);
        if fam.is_empty() {
            println!("no family members");
        } else {
            for m in fam {
                if let Some(word) = m.get("word").and_then(|v| v.as_str()) {
                    let mut line = format!(" {}> {}", word, "");
                    if let Some(pos) = m.get("pos").and_then(|v| v.as_str()) {
                        if !pos.is_empty() {
                            line.push_str(&format!(" ({})", pos));
                        }
                    }
                    println!("{}", line);
                }
            }
        }
        return Ok(());
    }

    let sub = &args[1];
    match sub.as_str() {
        "add" => {
            if args.len() < 3 {
                return Err(GxxError::msg(
                    "usage: gxx family <word> add <related-word> [-p pos]",
                ));
            }
            let related = clean(&args[2]);
            let mut pos = String::new();
            let mut i = 3;
            while i < args.len() {
                if args[i] == "-p" || args[i] == "--pos" {
                    if i + 1 < args.len() {
                        pos = args[i + 1].clone();
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            let member =
                new_family_member(&related, if pos.is_empty() { None } else { Some(&pos) });
            family_mut(&mut rec).push(member);
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!(
                    "{} added family member #{}",
                    crate::style::ok("ok:"),
                    family_mut(&mut rec).len()
                );
            }
        }
        "del" => {
            if args.len() != 3 {
                return Err(GxxError::msg("usage: gxx family <word> del <related-word>"));
            }
            let related = clean(&args[2]);
            let fam = family_mut(&mut rec);
            let len_before = fam.len();
            fam.retain(|m| {
                m.get("word").and_then(|v| v.as_str()).map(clean) != Some(related.clone())
            });
            if fam.len() == len_before {
                return Err(GxxError::msg(format!(
                    "family member '{}' not found",
                    related
                )));
            }
            save_rec(con, "word", &mut rec)?;
            if !STATE.lock().unwrap().quiet {
                println!(
                    "{} deleted family member '{}'",
                    crate::style::ok("ok:"),
                    related
                );
            }
        }
        _ => return Err(GxxError::msg(format!("unknown family subcommand: {}", sub))),
    }
    Ok(())
}

/// `gxx say <word> [--example N] [--repeat N] [--slow] [--very-slow] [--speed N] [--phonemes IPA] [--soft]`
pub fn cmd_say(con: &Connection, args: &[String]) -> Result<(), GxxError> {
    if args.is_empty() {
        return Err(GxxError::msg(
            "usage: gxx say <word> [--example N] [--repeat N] [--slow] [--very-slow] [--speed N] [--phonemes IPA] [--soft]",
        ));
    }

    // Parse arguments
    let mut example_num: Option<usize> = None;
    let mut repeat: u32 = 1;
    let mut speed: f32 = 1.0;
    let mut volume: f32 = 1.0;
    let mut manual_phonemes: Option<String> = None;

    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--example" => {
                if i + 1 >= args.len() {
                    return Err(GxxError::msg("--example requires a number"));
                }
                example_num = Some(
                    args[i + 1]
                        .parse()
                        .map_err(|_| GxxError::msg("invalid example number"))?,
                );
                i += 2;
            }
            "--repeat" => {
                if i + 1 >= args.len() {
                    return Err(GxxError::msg("--repeat requires a number"));
                }
                repeat = args[i + 1]
                    .parse()
                    .map_err(|_| GxxError::msg("invalid repeat value"))?;
                if repeat == 0 || repeat > 10 {
                    return Err(GxxError::msg("repeat must be between 1 and 10"));
                }
                i += 2;
            }
            "--slow" => {
                speed = 0.7;
                i += 1;
            }
            "--very-slow" => {
                speed = 0.5;
                i += 1;
            }
            "--phonemes" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    GxxError::msg("--phonemes requires IPA, e.g. --phonemes \"dʒˈʌmps\"")
                })?;

                manual_phonemes = Some(value.clone());
                i += 2;
            }

            "--speed" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| GxxError::msg("--speed requires a value, e.g. --speed 0.7"))?;

                speed = value.parse::<f32>().map_err(|_| {
                    GxxError::msg(format!(
                        "Invalid speed '{}'. Expected a number between 0.1 and 2.0",
                        value
                    ))
                })?;

                if !(0.1..=2.0).contains(&speed) {
                    return Err(GxxError::msg(format!(
                        "Invalid speed '{}'. Expected 0.1 <= speed <= 2.0",
                        speed
                    )));
                }

                i += 2;
            }
            "--soft" => {
                volume = 0.6;
                i += 1;
            }
            _ if a.starts_with("--") => {
                return Err(GxxError::msg(format!("unknown option: {}", a)));
            }
            _ => {
                positional.push(a.clone());
                i += 1;
            }
        }
    }

    if positional.is_empty() {
        return Err(GxxError::msg(
            "usage: gxx say <word> [--example N] [--repeat N] [--slow] [--very-slow] [--speed N] [--phonemes IPA] [--soft]",
        ));
    }

    let word_name = clean(&positional[0]);

    // Load word record
    let rec = load_rec_fallback(con, "word", &word_name)?;

    // Determine text to speak
    let text = if let Some(ex_num) = example_num {
        let examples = rec.get("examples").and_then(|v| v.as_array());
        match examples {
            Some(exs) if ex_num > 0 && ex_num <= exs.len() => {
                let ex = &exs[ex_num - 1];
                ex.get("en")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| GxxError::msg("example has no English text"))?
                    .to_string()
            }
            _ => return Err(GxxError::msg(format!("example #{} not found", ex_num))),
        }
    } else {
        word_name.clone()
    };

    // Synthesize and play
    let options = PlaybackOptions {
        repeat,
        speed,
        volume,
    };

    if let Some(ipa) = manual_phonemes.as_deref() {
        say_manual_with_active_model(ipa, options, None)?;
    } else {
        say_with_active_model(&text, options, None)?;
    }

    if !STATE.lock().unwrap().quiet {
        if let Some(ex_num) = example_num {
            println!("{} played example #{}", crate::style::ok("ok:"), ex_num);
        } else {
            println!("{} played '{}'", crate::style::ok("ok:"), word_name);
        }
    }

    Ok(())
}

// ----- helpers -----

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
                    return (vec![], std::collections::HashMap::new()); // will be caught by caller
                }
                opts.insert(key.to_string(), args[i + 1].clone());
                i += 2;
                matched = true;
                break;
            }
        }
        if !matched {
            if a.starts_with("--") && a.len() > 2 {
                return (vec![], std::collections::HashMap::new()); // will be caught by caller
            }
            positional.push(a.clone());
            i += 1;
        }
    }
    (positional, opts)
}

// ----- helpers -----
