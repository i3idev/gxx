//! Command dispatch for clap-based CLI.

use crate::cli::{
    Cli, CollAction, Command, DbCommand, FamilyAction, GramCommand, ModelCommand, SenseAction,
};
use crate::cmd::db::{cmd_db, cmd_init};
use crate::cmd::grammar::cmd_group;
use crate::cmd::load::cmd_load;
use crate::cmd::tr_syn_ant::{cmd_syn_ant, cmd_tr};
use crate::cmd::word::{
    cmd_add, cmd_coll, cmd_del, cmd_ex_word, cmd_family, cmd_find, cmd_list, cmd_say, cmd_sense,
    cmd_set_word, cmd_show,
};
use crate::error::GxxError;
use crate::tts::registry::{
    clear_active_model, load_model_config, set_active_model, ModelRegistry,
};
use rusqlite::Connection;

/// Run a clap-parsed CLI (handles global flags and subcommands).
pub fn run_cli(cli: Cli) -> Result<(), GxxError> {
    // Handle global flags first
    if cli.quiet {
        crate::model::set_quiet(true);
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
pub fn run_clap(
    cmd: Command,
    db_override: Option<&str>,
    json_output: bool,
) -> Result<(), GxxError> {
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
        Command::Paragraph(args) => run_paragraph(args, db_override),
        Command::Read(args) => run_read(args, db_override),
        Command::Pos(args) => run_field("pos", args, db_override),
        Command::Lvl(args) => run_field("lvl", args, db_override),
        Command::Pron(args) => run_field("pron", args, db_override),
        Command::Def(args) => run_field("def", args, db_override),
        Command::Note(args) => run_field("note", args, db_override),
        Command::Gram(sub) => run_gram("gram", sub, db_override, json_output),
        Command::Style(sub) => run_gram("style", sub, db_override, json_output),
        Command::Form(sub) => run_gram("form", sub, db_override, json_output),
        Command::Load(args) => cmd_load(&[args.file], db_override),
        Command::Model(sub) => run_model(sub, json_output),
        Command::External(args) => {
            // Handle special legacy commands and direct word lookup
            if args.is_empty() {
                print_usage();
                return Ok(());
            }
            let first = &args[0];
            match first.as_str() {
                "app" => Err(GxxError::msg(
                    "reading mode GUI not available in this build (requires Slint)",
                )),
                "read" | "-r" | "--read" => {
                    if args.len() < 2 {
                        return Err(GxxError::msg(
                            "usage: gxx read <file.txt> [--auto|--voice CODE]",
                        ));
                    }
                    let read_args = crate::cli::ReadArgs {
                        file: std::path::PathBuf::from(&args[1]),
                        voice: None,
                        auto: false,
                        speed: 1.0,
                        volume: 1.0,
                        pause_ms: 300,
                        length_scale: None,
                        noise_scale: None,
                        out: None,
                        start: None,
                    };
                    run_read(read_args, db_override)
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
                        print_entry_json(&con, std::slice::from_ref(word))?;
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
            let args = if let Some(n) = name {
                vec!["info".to_string(), n]
            } else {
                vec!["info".to_string()]
            };
            cmd_db(&con, &args)
        }
        DbCommand::Del { name, yes } => {
            let args = if yes {
                vec!["del".to_string(), name, "--yes".to_string()]
            } else {
                vec!["del".to_string(), name]
            };
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

fn run_del(args: crate::cli::DelArgs, db_override: Option<&str>) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let argv = if args.yes {
        vec![args.word, "--yes".to_string()]
    } else {
        vec![args.word]
    };
    cmd_del(&con, &argv)
}

fn run_tr(args: crate::cli::TrArgs, db_override: Option<&str>) -> Result<(), GxxError> {
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

fn run_ex(args: crate::cli::ExArgs, db_override: Option<&str>) -> Result<(), GxxError> {
    let con = crate::db::connect(db_override)?;
    let mut argv = vec![args.word];
    if let Some(n) = args.number {
        argv.push(n.to_string());
    }
    argv.extend(args.texts);
    cmd_ex_word(&con, &argv)
}

fn run_sense(args: crate::cli::SenseArgs, db_override: Option<&str>) -> Result<(), GxxError> {
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

fn run_coll(args: crate::cli::CollArgs, db_override: Option<&str>) -> Result<(), GxxError> {
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

fn run_family(args: crate::cli::FamilyArgs, db_override: Option<&str>) -> Result<(), GxxError> {
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

fn run_say(args: crate::cli::SayArgs, db_override: Option<&str>) -> Result<(), GxxError> {
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

/// Find a model by language code ("en", "fa") or voice name ("cori", "amir")
/// and load its PiperConfig.
fn apply_inference_overrides(
    config: &mut crate::tts::model::PiperConfig,
    length_scale: Option<f32>,
    noise_scale: Option<f32>,
) {
    if let Some(v) = length_scale {
        config.inference.length_scale = v;
    }
    if let Some(v) = noise_scale {
        config.inference.noise_scale = v;
    }
}

fn load_config_for_voice(
    query: &str,
    length_scale: Option<f32>,
    noise_scale: Option<f32>,
) -> Result<crate::tts::model::PiperConfig, GxxError> {
    let registry = crate::tts::registry::ModelRegistry::discover()?;
    let entry = registry.find_for_query(query).ok_or_else(|| {
        GxxError::msg(format!(
            "no available model for voice '{}'. run `gxx model list`",
            query
        ))
    })?;
    let mut config = crate::tts::model::load_config(&entry.config_path)?;
    config.onnx_path = Some(entry.onnx_path.clone());
    apply_inference_overrides(&mut config, length_scale, noise_scale);
    Ok(config)
}

/// Like [`load_config_for_voice`] but returns `None` instead of erroring
/// when no model matches. Used by `--auto` to skip unknown-language segments.
fn try_load_config_for_voice(
    query: &str,
    length_scale: Option<f32>,
    noise_scale: Option<f32>,
) -> Option<crate::tts::model::PiperConfig> {
    let registry = crate::tts::registry::ModelRegistry::discover().ok()?;
    let entry = registry.find_for_query(query)?;
    let mut config = crate::tts::model::load_config(&entry.config_path).ok()?;
    config.onnx_path = Some(entry.onnx_path.clone());
    apply_inference_overrides(&mut config, length_scale, noise_scale);
    Some(config)
}

/// Ctrl+C signal flag shared across `read` invocations.
static READ_STOP_FLAG: std::sync::OnceLock<std::sync::Arc<std::sync::atomic::AtomicBool>> =
    std::sync::OnceLock::new();

fn get_read_stop_flag() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    READ_STOP_FLAG
        .get_or_init(|| {
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let f = std::sync::Arc::clone(&flag);
            let _ = ctrlc::set_handler(move || {
                f.store(true, std::sync::atomic::Ordering::Release);
                eprintln!("\nTTS: interrupted, stopping after current paragraph");
            });
            flag
        })
        .clone()
}

fn run_read(args: crate::cli::ReadArgs, _db_override: Option<&str>) -> Result<(), GxxError> {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    let content = std::fs::read_to_string(&args.file)
        .map_err(|e| GxxError::msg(format!("read {}: {}", args.file.display(), e)))?;

    let all_paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    if all_paragraphs.is_empty() {
        return Err(GxxError::msg("file has no readable text"));
    }

    let start = args.start.unwrap_or(1).max(1) - 1;
    if start >= all_paragraphs.len() {
        return Err(GxxError::msg(format!(
            "start paragraph {} out of range (file has {})",
            start + 1,
            all_paragraphs.len()
        )));
    }
    let paragraphs = &all_paragraphs[start..];
    let total = paragraphs.len();

    // --out: render all paragraphs to WAV instead of playing
    if let Some(out) = args.out.as_deref() {
        let auto_for_cfg = args.auto
            || args
                .voice
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("auto"))
                .unwrap_or(false);
        let voice_for_cfg = args
            .voice
            .clone()
            .filter(|v| !v.eq_ignore_ascii_case("auto"));

        // each paragraph may itself be language-segmented
        let mut chunks: Vec<(String, String)> = Vec::new();
        for para in paragraphs {
            if auto_for_cfg {
                let segs = crate::tts::phonemizer::segment_by_language(para);
                chunks.extend(
                    segs.into_iter()
                        .filter(|(t, lang)| *lang != "neutral" && !t.trim().is_empty())
                        .map(|(t, l)| (t, l.to_string())),
                );
            } else if voice_for_cfg.is_some() {
                chunks.push(((*para).to_string(), "voiced".to_string()));
            } else {
                chunks.push(((*para).to_string(), "active".to_string()));
            }
        }

        render_chunks_to_wav(&chunks, out, args.pause_ms, |lang| {
            if auto_for_cfg {
                try_load_config_for_voice(lang, args.length_scale, args.noise_scale)
            } else if let Some(ref v) = voice_for_cfg {
                try_load_config_for_voice(v, args.length_scale, args.noise_scale)
            } else {
                crate::tts::registry::get_active_config().ok()
            }
        })?;
        return Ok(());
    }

    let stop_flag = get_read_stop_flag();
    stop_flag.store(false, Ordering::Release);

    let make_opts = || crate::tts::PlaybackOptions {
        repeat: 1,
        speed: args.speed,
        volume: args.volume,
    };

    for (i, para) in paragraphs.iter().enumerate() {
        if stop_flag.load(Ordering::Acquire) {
            eprintln!("TTS: stopped at paragraph {}", start + i + 1);
            break;
        }

        eprintln!("TTS: paragraph {}/{}", start + i + 1, all_paragraphs.len());

        let auto_mode = args.auto
            || args
                .voice
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("auto"))
                .unwrap_or(false);

        if auto_mode {
            let segments = crate::tts::phonemizer::segment_by_language(para);
            let speakable: Vec<(String, &'static str)> = segments
                .into_iter()
                .filter(|(t, lang)| *lang != "neutral" && !t.trim().is_empty())
                .collect();
            for (chunk, lang) in &speakable {
                if stop_flag.load(Ordering::Acquire) {
                    break;
                }
                match try_load_config_for_voice(lang, args.length_scale, args.noise_scale) {
                    Some(config) => {
                        crate::tts::say_text(
                            chunk,
                            &config,
                            make_opts(),
                            Some(Arc::clone(&stop_flag)),
                        )?;
                    }
                    None => {
                        eprintln!(
                            "TTS: warning: no model for '{}', skipping segment {:?}",
                            lang, chunk
                        );
                    }
                }
            }
        } else if let Some(voice) = args
            .voice
            .as_deref()
            .filter(|v| !v.eq_ignore_ascii_case("auto"))
        {
            let config = load_config_for_voice(voice, args.length_scale, args.noise_scale)?;
            crate::tts::say_text(para, &config, make_opts(), Some(Arc::clone(&stop_flag)))?;
        } else {
            crate::tts::say_with_active_model(para, make_opts(), Some(Arc::clone(&stop_flag)))?;
        }

        if i + 1 < total && args.pause_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(args.pause_ms));
        }
    }
    Ok(())
}

/// Print a table of discovered models with the active one marked.
fn print_voice_list() -> Result<(), GxxError> {
    let registry = crate::tts::registry::ModelRegistry::discover()?;
    let active_id = crate::tts::registry::load_model_config()
        .ok()
        .and_then(|c| c.active_model_id);

    println!(
        "{:<14} {:<8} {:<8} {:<8} {:<6} ACTIVE",
        "ID", "LANG", "VOICE", "QUALITY", "SRATE"
    );
    for m in registry.all() {
        let mark = if Some(&m.id) == active_id.as_ref() {
            "✓"
        } else {
            ""
        };
        println!(
            "{:<14} {:<8} {:<8} {:<8} {:<6} {}",
            m.id, m.language, m.voice, m.quality, m.sample_rate, mark
        );
    }
    Ok(())
}

/// Render a list of pre-split (text, config) chunks into a single WAV file.
///
/// - Synthesizes each chunk without playing.
/// - Inserts `pause_ms` silence between chunks.
/// - Errors if two chunks have different sample rates.
/// - `config_for` decides which model to use per chunk (returns None to skip).
fn render_chunks_to_wav<F>(
    chunks: &[(String, String)],
    out: &std::path::Path,
    pause_ms: u64,
    config_for: F,
) -> Result<usize, GxxError>
where
    F: Fn(&str) -> Option<crate::tts::model::PiperConfig>,
{
    use crate::tts::audio::write_wav;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut sample_rate: Option<u32> = None;
    let mut spoken = 0usize;

    for (chunk, lang) in chunks {
        let config = match config_for(lang.as_str()) {
            Some(c) => c,
            None => {
                eprintln!(
                    "TTS: warning: no model for '{}', skipping segment {:?}",
                    lang, chunk
                );
                continue;
            }
        };
        let rate = config.audio.sample_rate;
        match sample_rate {
            None => sample_rate = Some(rate),
            Some(r) if r != rate => {
                return Err(GxxError::msg(format!(
                    "cannot concatenate segments with different sample rates: {} vs {}",
                    r, rate
                )));
            }
            _ => {}
        }
        let samples = crate::tts::synthesize_text(chunk, &config)?;
        if spoken > 0 && pause_ms > 0 {
            let silence_len = (rate as u64 * pause_ms / 1000) as usize;
            all_samples.extend(std::iter::repeat_n(0.0f32, silence_len));
        }
        all_samples.extend(samples);
        spoken += 1;
    }

    let rate = sample_rate.ok_or_else(|| GxxError::msg("no segment had a matching model"))?;
    let wav = write_wav(&all_samples, rate)?;
    std::fs::write(out, wav)
        .map_err(|e| GxxError::msg(format!("write {}: {}", out.display(), e)))?;
    eprintln!(
        "TTS: wrote {} ({} samples, {} Hz)",
        out.display(),
        all_samples.len(),
        rate
    );
    Ok(spoken)
}

fn run_paragraph(
    args: crate::cli::ParagraphArgs,
    _db_override: Option<&str>,
) -> Result<(), GxxError> {
    if args.list_voices {
        return print_voice_list();
    }

    // 1) gather text
    let text = match (args.text.as_deref(), args.file.as_deref()) {
        (Some(t), None) => t.to_string(),
        (None, Some(p)) => std::fs::read_to_string(p)
            .map_err(|e| GxxError::msg(format!("read {}: {}", p.display(), e)))?,
        (Some(_), Some(_)) => {
            return Err(GxxError::msg("give text OR --file, not both"));
        }
        (None, None) => {
            return Err(GxxError::msg("no text and no --file"));
        }
    };

    // --out: render to WAV instead of playing
    if let Some(out) = args.out.as_deref() {
        let auto_mode = args.auto
            || args
                .voice
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("auto"))
                .unwrap_or(false);

        let chunks: Vec<(String, String)> = if auto_mode {
            let segs = crate::tts::phonemizer::segment_by_language(&text);
            segs.into_iter()
                .filter(|(t, lang)| *lang != "neutral" && !t.trim().is_empty())
                .map(|(t, l)| (t, l.to_string()))
                .collect()
        } else if let Some(v) = args
            .voice
            .as_deref()
            .filter(|v| !v.eq_ignore_ascii_case("auto"))
        {
            vec![(text.clone(), v.to_string())]
        } else {
            // use active model
            vec![(text.clone(), "active".to_string())]
        };

        let auto_for_cfg = auto_mode;
        let voice_for_cfg = args
            .voice
            .clone()
            .filter(|v| !v.eq_ignore_ascii_case("auto"));
        render_chunks_to_wav(&chunks, out, args.paragraph_pause_ms, |lang| {
            if auto_for_cfg {
                try_load_config_for_voice(lang, args.length_scale, args.noise_scale)
            } else if let Some(ref v) = voice_for_cfg {
                try_load_config_for_voice(v, args.length_scale, args.noise_scale)
            } else {
                crate::tts::registry::get_active_config().ok()
            }
        })?;
        return Ok(());
    }

    // helper to build fresh options (PlaybackOptions may not be Clone)
    let make_opts = || crate::tts::PlaybackOptions {
        repeat: args.repeat,
        speed: args.speed,
        volume: args.volume,
    };

    // `--voice auto` is an alias for `--auto`
    let auto_mode = args.auto
        || args
            .voice
            .as_deref()
            .map(|v| v.eq_ignore_ascii_case("auto"))
            .unwrap_or(false);

    // 2a) manual IPA path: skip eSpeak and language guard
    if let Some(ipa) = args.phonemes {
        if let Some(voice) = args
            .voice
            .as_deref()
            .filter(|v| !v.eq_ignore_ascii_case("auto"))
        {
            let config = load_config_for_voice(voice, args.length_scale, args.noise_scale)?;
            crate::tts::say_text_manual(&ipa, &config, make_opts(), None)?;
        } else {
            crate::tts::say_manual_with_active_model(&ipa, make_opts(), None)?;
        }
        return Ok(());
    }

    // 2a') auto mode: segment by language and use matching model per segment
    if auto_mode {
        let segments = crate::tts::phonemizer::segment_by_language(&text);
        let speakable: Vec<(String, &'static str)> = segments
            .into_iter()
            .filter(|(t, lang)| *lang != "neutral" && !t.trim().is_empty())
            .collect();
        if speakable.is_empty() {
            return Err(GxxError::msg("no speakable text"));
        }
        let mut spoken = 0usize;
        for (chunk, lang) in speakable.iter() {
            match try_load_config_for_voice(lang, args.length_scale, args.noise_scale) {
                Some(config) => {
                    crate::tts::say_text(chunk, &config, make_opts(), None)?;
                    spoken += 1;
                    std::thread::sleep(std::time::Duration::from_millis(args.paragraph_pause_ms));
                }
                None => {
                    eprintln!(
                        "TTS: warning: no model for '{}', skipping segment {:?}",
                        lang, chunk
                    );
                }
            }
        }
        if spoken == 0 {
            return Err(GxxError::msg("no segment had a matching model"));
        }
        return Ok(());
    }

    // 2b) voice override path
    if let Some(voice) = args
        .voice
        .as_deref()
        .filter(|v| !v.eq_ignore_ascii_case("auto"))
    {
        let config = load_config_for_voice(voice, args.length_scale, args.noise_scale)?;
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if paragraphs.is_empty() {
            return Err(GxxError::msg("no text to speak"));
        }
        let total = paragraphs.len();
        for (i, para) in paragraphs.iter().enumerate() {
            crate::tts::say_text(para, &config, make_opts(), None)?;
            if i + 1 < total && args.paragraph_pause_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(args.paragraph_pause_ms));
            }
        }
        return Ok(());
    }

    // 3) split on blank lines (paragraphs)
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    if paragraphs.is_empty() {
        return Err(GxxError::msg("no text to speak"));
    }

    // 4) speak each paragraph, with a small pause between
    let total = paragraphs.len();
    for (i, para) in paragraphs.iter().enumerate() {
        crate::tts::say_with_active_model(para, make_opts(), None)?;
        if i + 1 < total && args.paragraph_pause_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(args.paragraph_pause_ms));
        }
    }
    Ok(())
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

fn run_model(sub: ModelCommand, _json_output: bool) -> Result<(), GxxError> {
    match sub {
        ModelCommand::List => {
            let registry = ModelRegistry::discover()?;
            if registry.is_empty() {
                println!("no models found");
                return Ok(());
            }

            let config = load_model_config()?;
            let active_id = config.active_model_id.as_deref();

            println!("ID           NAME                          LANGUAGE  VOICE   QUALITY  SRATE  STATUS");
            for model in registry.all() {
                let status = if active_id == Some(model.id.as_str()) {
                    " (active)"
                } else {
                    ""
                };
                let available = if model.available { "ok" } else { "missing" };
                println!(
                    "{:<12} {:<28} {:<8} {:<7} {:<7} {:<6} {}{}",
                    model.id,
                    truncate(&model.name, 28),
                    model.language,
                    model.voice,
                    model.quality,
                    model.sample_rate,
                    available,
                    status
                );
            }
            Ok(())
        }
        ModelCommand::Current => {
            let config = load_model_config()?;
            if let Some(active_id) = config.active_model_id {
                let registry = ModelRegistry::discover()?;
                if let Some(model) = registry.get(&active_id) {
                    println!("{}", model.id);
                } else {
                    println!("{} (not found)", active_id);
                }
            } else {
                println!("auto (not set)");
            }
            Ok(())
        }
        ModelCommand::Use { id } => {
            set_active_model(&id)?;
            println!("active model: {}", id);
            Ok(())
        }
        ModelCommand::Info { id } => {
            let registry = ModelRegistry::discover()?;
            if let Some(model) = registry.get(&id) {
                println!("ID          : {}", model.id);
                println!("Name        : {}", model.name);
                println!("Language    : {}", model.language);
                println!("Voice       : {}", model.voice);
                println!("Quality     : {}", model.quality);
                println!("Sample Rate : {} Hz", model.sample_rate);
                println!("ONNX Path   : {}", model.onnx_path.display());
                println!("Config Path : {}", model.config_path.display());
                if let Some(card) = &model.card_path {
                    println!("Card Path   : {}", card.display());
                }
                println!(
                    "Available   : {}",
                    if model.available { "yes" } else { "no" }
                );
            } else {
                return Err(GxxError::msg(format!("model '{}' not found", id)));
            }
            Ok(())
        }
        ModelCommand::Clear => {
            clear_active_model()?;
            println!("active model cleared (using auto-discovery)");
            Ok(())
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
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
