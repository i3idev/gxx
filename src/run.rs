//! Command dispatch for clap-based CLI.

use crate::cli::{Cli, Command, DbCommand, GramCommand, SenseAction, CollAction, FamilyAction};
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

/// Run a clap-parsed CLI (handles global flags and subcommands).
pub fn run_cli(cli: Cli) -> Result<(), GxxError> {
    // Handle global flags first
    if cli.quiet {
        crate::model::set_quiet(true);
    }

    // Handle legacy reading mode flags
    if cli.app {
        return Err(GxxError::msg(
            "reading mode GUI not available in this build (requires Slint)"
        ));
    }
    if let Some(file) = cli.read_file {
        return Err(GxxError::msg(format!(
            "reading mode not available: {}",
            file
        )));
    }

    // Handle subcommand
    if let Some(cmd) = cli.command {
        run_clap(cmd, cli.db.as_deref(), cli.json)
    } else {
        // No subcommand - could be direct word lookup
        print_usage();
        Ok(())
    }
}

/// Run a clap-parsed command (subcommand only).
pub fn run_clap(cmd: Command, db_override: Option<&str>, json_output: bool) -> Result<(), GxxError> {
    match cmd {
        Command::Init => cmd_init(),
        Command::Db(sub) => run_db(sub, db_override),
        Command::Add(args) => run_add(args, db_override, json_output),
        Command::Show(args) => run_show(args, db_override, json_output),
        Command::Find(args) => run_find(args, db_override, json_output),
        Command::List(args) => run_list(args, db_override, json_output),
        Command::Del(args) => run_del(args, db_override),
        Command::Tr(args) => run_tr(args, db_override),
        Command::Syn(args) => run_syn_ant("syn", args, db_override),
        Command::Ant(args) => run_syn_ant("ant", args, db_override),
        Command::Ex(args) => run_ex(args, db_override),
        Command::Sense(args) => run_sense(args, db_override),
        Command::Coll(args) => run_coll(args, db_override),
        Command::Family(args) => run_family(args, db_override),
        Command::Say(args) => run_say(args, db_override),
        Command::Pos(args) => run_field("pos", args, db_override),
        Command::Lvl(args) => run_field("lvl", args, db_override),
        Command::Pron(args) => run_field("pron", args, db_override),
        Command::Def(args) => run_field("def", args, db_override),
        Command::Note(args) => run_field("note", args, db_override),
        Command::Gram(sub) => run_gram("gram", sub, db_override, json_output),
        Command::Style(sub) => run_gram("style", sub, db_override, json_output),
        Command::Form(sub) => run_gram("form", sub, db_override, json_output),
        Command::Load(args) => cmd_load(&[args.file], db_override),
        Command::External(args) => {
            // Handle special legacy commands and direct word lookup
            if args.is_empty() {
                print_usage();
                return Ok(());
            }
            let first = &args[0];
            match first.as_str() {
                "app" | "read" => {
                    // Try to launch GUI reading mode (will fail in headless)
                    Err(GxxError::msg(
                        "reading mode GUI not available in this build (requires Slint)"
                    ))
                }
                "-r" | "--read" => {
                    // Reading mode with file argument
                    if args.len() < 2 {
                        return Err(GxxError::msg("usage: gxx -r <file.txt>"));
                    }
                    Err(GxxError::msg(format!(
                        "reading mode not available: {}",
                        args[1]
                    )))
                }
                "help" | "-h" | "--help" => {
                    print_usage();
                    Ok(())
                }
                _ => {
                    // Direct word lookup: gxx <word> [sections...]
                    let word = &args[0];
                    let sections = &args[1..];
                    let con = crate::db::connect(db_override)?;
                    if json_output {
                        print_entry_json(&con, &[word.clone()])?;
                        Ok(())
                    } else {
                        let mut show_args = vec![word.clone()];
                        show_args.extend(sections.iter().cloned());
                        cmd_show(&con, &show_args)
                    }
                }
            }
        }
    }
}

fn run_db(sub: DbCommand, _db_override: Option<&str>) -> Result<(), GxxError> {
    let con = Connection::open_in_memory()?;
    match sub {
        DbCommand::New { name } => cmd_db(&con, &["new".to_string(), name]),
        DbCommand::List => cmd_db(&con, &["list".to_string()]),
        DbCommand::Use { name } => cmd_db(&con, &["use".to_string(), name]),
        DbCommand::Info { name } => {
            let args = if let Some(n) = name { vec!["info".to_string(), n] } else { vec!["info".to_string()] };
            cmd_db(&con, &args)
        }
        DbCommand::Del { name, yes } => {
            let args = if yes { vec!["del".to_string(), name, "--yes".to_string()] } else { vec!["del".to_string(), name] };
            cmd_db(&con, &args)
        }
    }
}

fn run_add(
    args: crate::cli::AddArgs,
    db_override: Option<&str>,
    _json_output: bool,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    argv.extend(args.translations);
    if let Some(pos) = args.pos {
        argv.push("-p".into());
        argv.push(pos);
    }
    if let Some(lvl) = args.level {
        argv.push("-l".into());
        argv.push(lvl);
    }
    cmd_add(&con, &argv)
}

fn run_show(
    args: crate::cli::ShowArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    argv.extend(args.sections);
    if json_output {
        print_entry_json(&con, &argv)?;
        Ok(())
    } else {
        cmd_show(&con, &argv)
    }
}

fn run_find(
    args: crate::cli::FindArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.query];
    if args.all {
        argv.push("--all".into());
    }
    if json_output {
        print_find_json(&con, &argv)?;
        Ok(())
    } else {
        cmd_find(&con, &argv, db_override)
    }
}

fn run_list(
    args: crate::cli::ListArgs,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = Vec::new();
    if let Some(pos) = args.pos {
        argv.push("--pos".into());
        argv.push(pos);
    }
    if let Some(lvl) = args.lvl {
        argv.push("--lvl".into());
        argv.push(lvl);
    }
    if let Some(lang) = args.lang {
        argv.push("--lang".into());
        argv.push(lang);
    }
    if json_output {
        print_list_json(&con, &argv)?;
        Ok(())
    } else {
        cmd_list(&con, &argv)
    }
}

fn run_del(
    args: crate::cli::DelArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let argv = if args.yes {
        vec![args.word, "--yes".to_string()]
    } else {
        vec![args.word]
    };
    cmd_del(&con, &argv)
}

fn run_tr(
    args: crate::cli::TrArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(lang) = args.lang {
        argv.push(lang);
    }
    argv.extend(args.ops);
    cmd_tr(&con, &argv)
}

fn run_syn_ant(
    field: &str,
    args: crate::cli::SynAntArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    argv.extend(args.ops);
    cmd_syn_ant(&con, field, &argv)
}

fn run_ex(
    args: crate::cli::ExArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(n) = args.number {
        argv.push(n.to_string());
    }
    argv.extend(args.texts);
    cmd_ex_word(&con, &argv)
}

fn run_sense(
    args: crate::cli::SenseArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(action) = args.action {
        match action {
            SenseAction::Add { definition, note } => {
                argv.push("add".into());
                argv.push(definition);
                if let Some(n) = note {
                    argv.push("--note".into());
                    argv.push(n);
                }
            }
            SenseAction::Del { number } => {
                argv.push("del".into());
                argv.push(number.to_string());
            }
            SenseAction::Show { number } => {
                argv.push("show".into());
                argv.push(number.to_string());
            }
        }
    }
    cmd_sense(&con, &argv)
}

fn run_coll(
    args: crate::cli::CollArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(action) = args.action {
        match action {
            CollAction::Add { text } => {
                argv.push("add".into());
                argv.push(text);
            }
            CollAction::Del { number } => {
                argv.push("del".into());
                argv.push(number.to_string());
            }
        }
    }
    cmd_coll(&con, &argv)
}

fn run_family(
    args: crate::cli::FamilyArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(action) = args.action {
        match action {
            FamilyAction::Add { word, pos } => {
                argv.push("add".into());
                argv.push(word);
                if let Some(p) = pos {
                    argv.push("-p".into());
                    argv.push(p);
                }
            }
            FamilyAction::Del { word } => {
                argv.push("del".into());
                argv.push(word);
            }
        }
    }
    cmd_family(&con, &argv)
}

fn run_say(
    args: crate::cli::SayArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(ex) = args.example {
        argv.push("--example".into());
        argv.push(ex.to_string());
    }
    if args.repeat != 1 {
        argv.push("--repeat".into());
        argv.push(args.repeat.to_string());
    }
    if args.slow {
        argv.push("--slow".into());
    }
    if args.very_slow {
        argv.push("--very-slow".into());
    }
    if args.soft {
        argv.push("--soft".into());
    }
    if let Some(ipa) = args.phonemes {
        argv.push("--phonemes".into());
        argv.push(ipa);
    }
    cmd_say(&con, &argv)
}

fn run_field(
    field: &str,
    args: crate::cli::FieldArgs,
    db_override: Option<&str>,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(v) = args.value {
        argv.push(v);
    }
    cmd_set_word(&con, field, &argv)
}

fn run_gram(
    kind: &str,
    sub: GramCommand,
    db_override: Option<&str>,
    _json_output: bool,
) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![sub_to_str(&sub).to_string()];
    match sub {
        GramCommand::Add(args) => {
            argv.push(args.name);
            if let Some(cat) = args.cat {
                argv.push("--cat".into());
                argv.push(cat);
            }
            if let Some(lvl) = args.lvl {
                argv.push("--lvl".into());
                argv.push(lvl);
            }
            if let Some(rule) = args.rule {
                argv.push("--rule".into());
                argv.push(rule);
            }
            if let Some(desc) = args.desc {
                argv.push("--desc".into());
                argv.push(desc);
            }
        }
        GramCommand::List { prefix } => {
            if let Some(p) = prefix {
                argv.push(p);
            }
        }
        GramCommand::Show { name } => {
            argv.push(name);
        }
        GramCommand::Del { name } => {
            argv.push(name);
        }
        GramCommand::Rule(args) => {
            argv.push(args.name);
            if let Some(v) = args.value {
                argv.push(v);
            }
        }
        GramCommand::Desc(args) => {
            argv.push(args.name);
            if let Some(v) = args.value {
                argv.push(v);
            }
        }
        GramCommand::Cat(args) => {
            argv.push(args.name);
            if let Some(v) = args.value {
                argv.push(v);
            }
        }
        GramCommand::Lvl(args) => {
            argv.push(args.name);
            if let Some(v) = args.value {
                argv.push(v);
            }
        }
        GramCommand::Note(args) => {
            argv.push(args.name);
            if let Some(v) = args.value {
                argv.push(v);
            }
        }
        GramCommand::Ex(args) => {
            argv.push(args.name);
            if let Some(n) = args.number {
                argv.push(n.to_string());
            }
            argv.extend(args.texts);
        }
        GramCommand::Err(args) => {
            argv.push(args.name);
            if let Some(n) = args.number {
                argv.push(n.to_string());
            }
            argv.extend(args.texts);
        }
    }
    cmd_group(&con, kind, &argv)
}

fn sub_to_str(sub: &GramCommand) -> &'static str {
    match sub {
        GramCommand::Add(_) => "add",
        GramCommand::List { .. } => "list",
        GramCommand::Show { .. } => "show",
        GramCommand::Del { .. } => "del",
        GramCommand::Rule(_) => "rule",
        GramCommand::Desc(_) => "desc",
        GramCommand::Cat(_) => "cat",
        GramCommand::Lvl(_) => "lvl",
        GramCommand::Note(_) => "note",
        GramCommand::Ex(_) => "ex",
        GramCommand::Err(_) => "err",
    }
}

// Re-export for load command compatibility - uses legacy argument parsing
pub fn run_internal(argv: Vec<String>, db_override: Option<&str>) -> Result<(), GxxError> {
    if argv.is_empty() {
        return Ok(());
    }

    let mut argv = argv;
    let json_output = argv.iter().any(|arg| arg == "--json");
    argv.retain(|arg| arg != "--json");

    if argv.is_empty() {
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
            let con = rusqlite::Connection::open_in_memory()?;
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

// JSON output helpers (kept from original run.rs)
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

fn print_usage() {
    println!(
        r#"gxx - bilingual word and grammar notebook

USAGE:
    gxx <COMMAND> [OPTIONS]

COMMANDS:
    init                    Initialize config directory
    db <SUBCOMMAND>         Database management
    add <WORD> [TRANS...]   Add a word
    show <WORD> [SECTIONS]  Show entry
    find <QUERY>            Search entries
    list                    List entries
    del <WORD>              Delete entry
    tr <WORD> [OPS]         Manage translations
    syn <WORD> [OPS]        Manage synonyms
    ant <WORD> [OPS]        Manage antonyms
    ex <WORD> [TEXTS]       Manage examples
    sense <WORD> <ACTION>   Manage senses
    coll <WORD> <ACTION>    Manage collocations
    family <WORD> <ACTION>  Manage word family
    say <WORD> [OPTS]       Text-to-speech
    pos|lvl|pron|def|note   Set/get field
    gram|style|form <CMD>   Grammar/style/form
    load <FILE>             Load commands from file

OPTIONS:
    --db <NAME>             Use specific database
    --json                  Output as JSON
    -q, --quiet             Suppress non-error output
    -h, --help              Show help
    -V, --version           Show version
"#
    );
}