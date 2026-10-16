//! CLI definitions using clap.
//!
//! This module defines the entire command-line surface of `gxx` using
//! [`clap`](https://docs.rs/clap) derive macros. It contains:
//!
//! - [`Cli`] — the root argument parser with global flags
//! - [`Command`] — the top-level subcommand enum
//! - [`DbCommand`] — database management subcommands (`new`, `list`, `use`, `info`, `del`)
//! - [`AddArgs`], [`ShowArgs`], [`FindArgs`], [`ListArgs`], [`DelArgs`] — core word commands
//! - [`TrArgs`], [`SynAntArgs`] — translation / synonym / antonym management
//! - [`ExArgs`] — example management (add / edit by number / delete)
//! - [`SenseArgs`], [`CollArgs`], [`FamilyArgs`] — word relation management
//! - [`SayArgs`], [`ParagraphArgs`], [`ReadArgs`] — TTS commands
//! - [`ModelCommand`] — TTS model management (`list`, `use`, `current`, `info`, `clear`)
//! - [`FieldArgs`] — field get/set commands (`pos`, `lvl`, `pron`, `def`, `note`)
//! - [`GramCommand`] — grammar / style / form command group
//! - [`TopicsArgs`], [`LoadArgs`] — utility commands
//!
//! Global flags (`--db`, `--vault/-v`, `--json`, `--quiet/-q`, `--no-color`)
//! are defined on [`Cli`] and apply to every subcommand.

use clap::{Args, Parser, Subcommand};

/// Root CLI argument parser.
///
/// [`Cli`] is parsed by [`clap::Parser::parse`] in [`crate::main`].
/// Global options are available before or after any subcommand.
///
/// Example:
/// ```no_run
/// use gxx::cli::Cli;
/// let cli = Cli::parse_from(["gxx", "--quiet", "add", "book", "کتاب"]);
/// assert!(cli.quiet);
/// ```
#[derive(Parser, Debug)]
#[command(
    name = "gxx",
    version,
    about = "Bilingual (English / Persian) word and grammar notebook",
    long_about = "gxx - A CLI tool for managing bilingual vocabulary and grammar notes.\nStores data in SQLite databases under ~/.gxx/db/ or ./.gxx/db/.",
    allow_hyphen_values = true
)]
pub struct Cli {
    /// Database to use for this command (overrides the active database).
    ///
    /// Resolves to `<data-dir>/db/<NAME>.db`. The data directory is `~/.gxx/`
    /// by default, or `<vault>/.gxx/` when `--vault` / `GXX_VAULT` is set.
    #[arg(long, global = true, value_name = "NAME")]
    pub db: Option<String>,

    /// Path to vault folder (project root) containing `.gxx/` data.
    ///
    /// When set, `gxx` stores all data inside `<vault>/.gxx/` instead of
    /// `~/.gxx/`. Short form `-v`. Takes precedence over `GXX_VAULT`.
    #[arg(long, short = 'v', global = true, value_name = "PATH")]
    pub vault: Option<String>,

    /// Output results as JSON instead of human-readable text.
    ///
    /// Supported by `show`, `find`, `list` and their `load` equivalents.
    #[arg(long, global = true)]
    pub json: bool,

    /// Quiet mode — suppress non-error informational output.
    ///
    /// Errors are still printed. Success messages ("added", "updated",
    /// "ok:") are suppressed.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable ANSI colors in output.
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level subcommand enum.
///
/// Each variant maps to a handler function in [`crate::run`] and, for most
/// variants, to a command implementation in [`crate::cmd`].
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize the gxx data directory and config file.
    ///
    /// Creates `<data-dir>/` and `<data-dir>/db/`, plus an empty `config`
    /// file. Prints the data directory path.
    Init,

    /// Database management: create, list, switch, inspect, delete databases.
    #[command(subcommand)]
    Db(DbCommand),

    /// Add a new word entry, or update an existing one.
    ///
    /// Translations are auto-directed to the opposite language of the
    /// word's detected script. Duplicates are skipped.
    Add(AddArgs),

    /// Show entry details for a word or grammar/style/form entry.
    ///
    /// Optional section filters: `tr`, `syn`, `ant`, `ex`, `sense`,
    /// `coll`, `family`, `note` (words); `rule`, `desc`, `ex`, `err`,
    /// `note` (grammar/style/form).
    Show(ShowArgs),

    /// Search entries by title or content (substring or `*` glob).
    Find(FindArgs),

    /// List word entries with optional POS / level / language filters.
    List(ListArgs),

    /// Delete a word entry (interactive confirmation unless `--yes`).
    Del(DelArgs),

    /// Manage translations for a word entry (`+add`, `-remove`).
    Tr(TrArgs),

    /// Manage synonyms for a word entry (`+add`, `-remove`).
    Syn(SynAntArgs),

    /// Manage antonyms for a word entry (`+add`, `-remove`).
    Ant(SynAntArgs),

    /// Manage examples for a word entry.
    ///
    /// Syntax:
    ///
    /// - List: `gxx ex <word>`
    /// - Add: `gxx ex <word> ["en text"] ["fa text"]`
    /// - Edit example N: `gxx ex <word> N "text"`
    /// - Delete example N: `gxx ex <word> --delete --number N`
    Ex(ExArgs),

    /// Manage polysemy senses (numbered definitions with optional notes).
    Sense(SenseArgs),

    /// Manage collocations (common word pairings).
    Coll(CollArgs),

    /// Manage word-family members (related words with optional POS).
    Family(FamilyArgs),

    /// Speak a notebook word aloud using TTS.
    Say(SayArgs),

    /// Read a text file aloud paragraph by paragraph.
    Read(ReadArgs),

    /// Speak arbitrary text (no DB lookup).
    Paragraph(ParagraphArgs),

    /// TTS model management: list, use, current, info, clear.
    #[command(subcommand)]
    Model(ModelCommand),

    /// Print extended help on a topic (`tts`, `models`, `language`,
    /// `bookmarks`, `examples`).
    Topics(TopicsArgs),

    /// Set or get the part of speech of a word entry.
    Pos(FieldArgs),

    /// Set or get the proficiency level of a word entry.
    Lvl(FieldArgs),

    /// Set or get the pronunciation of a word entry.
    Pron(FieldArgs),

    /// Set or get the definition of a word entry.
    Def(FieldArgs),

    /// Set or get the notes of a word entry.
    Note(FieldArgs),

    /// Grammar entries (`grammar` kind).
    #[command(subcommand)]
    Gram(GramCommand),

    /// Style entries (`style` kind).
    #[command(subcommand)]
    Style(GramCommand),

    /// Form entries (`form` kind).
    #[command(subcommand)]
    Form(GramCommand),

    /// Load and execute commands from a `.gxx` text file.
    Load(LoadArgs),

    /// Direct word lookup (legacy `gxx <word> [sections...]` form).
    ///
    /// Equivalent to `gxx show <word> [sections...]`.
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// Database management subcommands.
#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Create a new SQLite database.
    ///
    /// The name must match `^[\w-]+$` (letters, digits, underscore,
    /// hyphen). If no active database is set, the new database becomes
    /// active automatically.
    New { name: String },

    /// List all databases with entry counts and last-updated timestamps.
    List,

    /// Set the active database by name.
    Use { name: String },

    /// Show database info: path, active status, per-kind entry counts.
    ///
    /// If no name is given, shows info for the active database.
    Info { name: Option<String> },

    /// Delete a database file.
    ///
    /// Requires interactive confirmation (type the name) unless `--yes`
    /// is passed. If the deleted database was active, the active setting
    /// is cleared.
    Del {
        name: String,
        /// Skip the interactive confirmation prompt.
        #[arg(long)]
        yes: bool,
    },
}

/// Arguments for `gxx add <word> [translations...] [-p POS] [-l LEVEL]`.
#[derive(Args, Debug)]
pub struct AddArgs {
    /// Word to add (English or Persian; language auto-detected).
    pub word: String,

    /// Translations in the target language (auto-detected from the word).
    #[arg(value_name = "TRANSLATION")]
    pub translations: Vec<String>,

    /// Part of speech (e.g. `n`, `v`, `adj`, `adv`).
    #[arg(short, long, value_name = "POS")]
    pub pos: Option<String>,

    /// Proficiency level (e.g. `A1`, `A2`, `B1`, `B2`, `C1`, `C2`).
    #[arg(short, long, value_name = "LEVEL")]
    pub level: Option<String>,
}

/// Arguments for `gxx show <word> [sections...]`.
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Word to look up (fuzzy / normalised matching supported).
    pub word: String,

    /// Sections to display. If omitted, the full entry is rendered.
    #[arg(value_name = "SECTION")]
    pub sections: Vec<String>,
}

/// Arguments for `gxx find <query> [--all]`.
#[derive(Args, Debug)]
pub struct FindArgs {
    /// Search query (case-insensitive substring match on normalised text;
    /// `*` wildcards supported).
    pub query: String,

    /// Search across all databases instead of only the active one.
    #[arg(long)]
    pub all: bool,
}

/// Arguments for `gxx list [--pos POS] [--lvl LEVEL] [--lang LANG]`.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter entries by part of speech (case-insensitive).
    #[arg(long, value_name = "POS")]
    pub pos: Option<String>,

    /// Filter entries by proficiency level (case-insensitive).
    #[arg(long, value_name = "LEVEL")]
    pub lvl: Option<String>,

    /// Filter entries by language (`en` or `fa`).
    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,
}

/// Arguments for `gxx del <word> [--yes]`.
#[derive(Args, Debug)]
pub struct DelArgs {
    /// Word to delete.
    pub word: String,

    /// Skip the interactive confirmation prompt.
    #[arg(long)]
    pub yes: bool,
}

/// Arguments for `gxx tr <word> [lang] +x -y …`.
#[derive(Args, Debug)]
pub struct TrArgs {
    /// Word to modify.
    pub word: String,

    /// Target language code (`fa`, `en`, etc.). Auto-detected from the
    /// word's script if omitted.
    #[arg(value_name = "LANG")]
    pub lang: Option<String>,

    /// Operations: `+text` adds, `-text` removes, plain `text` adds.
    /// Multiple operations may be chained.
    #[arg(value_name = "OP", allow_hyphen_values = true, trailing_var_arg = true)]
    pub ops: Vec<String>,
}

/// Arguments for `gxx syn <word> +x -y …` and `gxx ant <word> +x -y …`.
#[derive(Args, Debug)]
pub struct SynAntArgs {
    /// Word to modify.
    pub word: String,

    /// Operations: `+text` adds, `-text` removes, plain `text` adds.
    #[arg(value_name = "OP", allow_hyphen_values = true, trailing_var_arg = true)]
    pub ops: Vec<String>,
}

/// Arguments for `gxx ex <word> ["text"] | N "text" | --delete --number N`.
#[derive(Args, Debug)]
pub struct ExArgs {
    /// Word to modify.
    pub word: String,

    /// Example text(s). Language auto-detected. Provide one English and
    /// one Persian text for a bilingual pair.
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,

    /// Example number (1-based) to edit or delete.
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,

    /// Delete the specified example. Requires `--number`. Cannot be
    /// combined with example text.
    #[arg(long, requires = "number")]
    pub delete: bool,
}

/// Arguments for `gxx sense <word> [add|del|show]`.
#[derive(Args, Debug)]
pub struct SenseArgs {
    /// Word to modify.
    pub word: String,

    /// Sense action: `add`, `del`, or `show`.
    #[command(subcommand)]
    pub action: Option<SenseAction>,
}

/// Sense subcommand actions.
#[derive(Subcommand, Debug)]
pub enum SenseAction {
    /// Add a new numbered sense with a definition and optional note.
    Add {
        /// The sense definition.
        definition: String,
        /// Optional note attached to the sense.
        #[arg(long)]
        note: Option<String>,
    },
    /// Delete sense number N.
    Del { number: usize },
    /// Show sense number N (definition, note, examples).
    Show { number: usize },
}

/// Arguments for `gxx coll <word> [add|del]`.
#[derive(Args, Debug)]
pub struct CollArgs {
    /// Word to modify.
    pub word: String,

    /// Collocation action: `add` or `del`.
    #[command(subcommand)]
    pub action: Option<CollAction>,
}

/// Collocation subcommand actions.
#[derive(Subcommand, Debug)]
pub enum CollAction {
    /// Add a collocation text.
    Add { text: String },
    /// Delete collocation number N.
    Del { number: usize },
}

/// Arguments for `gxx family <word> [add|del]`.
#[derive(Args, Debug)]
pub struct FamilyArgs {
    /// Word to modify.
    pub word: String,

    /// Family action: `add` or `del`.
    #[command(subcommand)]
    pub action: Option<FamilyAction>,
}

/// Family subcommand actions.
#[derive(Subcommand, Debug)]
pub enum FamilyAction {
    /// Add a related word as a family member, optionally with a POS.
    Add {
        /// Related word (e.g. `textbook` for `book`).
        word: String,
        /// Part of speech for the family member.
        #[arg(short, long)]
        pos: Option<String>,
    },
    /// Delete a family member by related word.
    Del { word: String },
}

/// Arguments for `gxx say <word> [options]`.
#[derive(Args, Debug)]
pub struct SayArgs {
    /// Word to speak aloud.
    pub word: String,

    /// Speak example N's English text instead of the word.
    #[arg(long, value_name = "N")]
    pub example: Option<usize>,

    /// Repeat playback N times (1-10, default 1).
    #[arg(long, value_name = "N", default_value = "1")]
    pub repeat: u32,

    /// Slow playback speed (0.7x).
    #[arg(long)]
    pub slow: bool,

    /// Very slow playback speed (0.5x).
    #[arg(long)]
    pub very_slow: bool,

    /// Soft volume (0.6x).
    #[arg(long)]
    pub soft: bool,

    /// Manual IPA phonemes (bypasses eSpeak and language guard).
    #[arg(long, value_name = "IPA")]
    pub phonemes: Option<String>,
}

/// Arguments for `gxx read <file> [options]`.
#[derive(Args, Debug)]
pub struct ReadArgs {
    /// Text file to narrate paragraph by paragraph.
    pub file: std::path::PathBuf,

    /// Override voice by language code (`en`, `fa`) or voice name
    /// (`cori`, `ryan`, `amir`).
    #[arg(long, value_name = "CODE")]
    pub voice: Option<String>,

    /// Auto-detect language per segment and route to matching model.
    #[arg(long)]
    pub auto: bool,

    /// Playback speed multiplier (default 1.0, range 0.1–2.0).
    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,

    /// Playback volume multiplier (default 1.0).
    #[arg(long, default_value_t = 1.0)]
    pub volume: f32,

    /// Pause between paragraphs in milliseconds (default 300).
    #[arg(long, value_name = "MS", default_value_t = 300)]
    pub pause_ms: u64,

    /// Override Piper `length_scale` (higher = slower, more deliberate).
    #[arg(long, value_name = "F32")]
    pub length_scale: Option<f32>,

    /// Override Piper `noise_scale` (lower = flatter prosody).
    #[arg(long, value_name = "F32")]
    pub noise_scale: Option<f32>,

    /// Override Piper `noise_w` (phoneme duration variance).
    #[arg(long, value_name = "F32")]
    pub noise_w: Option<f32>,

    /// Extra `ː` marks after each long vowel (0-3, higher = more stretched).
    #[arg(long, value_name = "N")]
    pub vowel_stretch: Option<u32>,

    /// Write all paragraphs to a single WAV file instead of playing.
    #[arg(long, value_name = "PATH")]
    pub out: Option<std::path::PathBuf>,

    /// Start from paragraph N (1-based).
    #[arg(long, value_name = "N")]
    pub start: Option<usize>,

    /// Split paragraphs into sentences and pause between them.
    #[arg(long)]
    pub sentence_split: bool,

    /// Pause between sentences when `--sentence-split` is used (ms,
    /// default 150).
    #[arg(long, value_name = "MS", default_value_t = 150)]
    pub sentence_pause_ms: u64,

    /// Resume from the last saved paragraph for this file.
    #[arg(long)]
    pub resume: bool,

    /// Auto-save progress after each paragraph.
    #[arg(long)]
    pub save_progress: bool,

    /// Forget saved progress for this file and exit.
    #[arg(long)]
    pub clear_progress: bool,
}

/// Arguments for `gxx paragraph <text> [options]`.
#[derive(Args, Debug)]
pub struct ParagraphArgs {
    /// Text to speak (quote if it contains spaces).
    pub text: Option<String>,

    /// Read text from a file instead of inline text.
    #[arg(short = 'f', long, value_name = "PATH")]
    pub file: Option<std::path::PathBuf>,

    /// Manual IPA phonemes (skips eSpeak and language guard).
    #[arg(long, value_name = "IPA")]
    pub phonemes: Option<String>,

    /// Override model by language code (`en`, `fa`) or voice name
    /// (`cori`, `ryan`, `amir`). `--voice auto` is an alias for `--auto`.
    #[arg(long, value_name = "CODE")]
    pub voice: Option<String>,

    /// Auto-select model per language segment (mixed text supported).
    #[arg(long)]
    pub auto: bool,

    /// Repeat playback N times (1-10, default 1).
    #[arg(long, value_name = "N", default_value = "1")]
    pub repeat: u32,

    /// Playback speed multiplier (default 1.0).
    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,

    /// Playback volume multiplier (default 1.0).
    #[arg(long, default_value_t = 1.0)]
    pub volume: f32,

    /// Pause between paragraphs in milliseconds (default 300).
    #[arg(long, value_name = "MS", default_value_t = 300)]
    pub paragraph_pause_ms: u64,

    /// Override Piper `length_scale`.
    #[arg(long, value_name = "F32")]
    pub length_scale: Option<f32>,

    /// Override Piper `noise_scale`.
    #[arg(long, value_name = "F32")]
    pub noise_scale: Option<f32>,

    /// Override Piper `noise_w`.
    #[arg(long, value_name = "F32")]
    pub noise_w: Option<f32>,

    /// Extra `ː` marks after each long vowel (0-3).
    #[arg(long, value_name = "N")]
    pub vowel_stretch: Option<u32>,

    /// Write output to a WAV file instead of playing.
    #[arg(long, value_name = "PATH")]
    pub out: Option<std::path::PathBuf>,

    /// List available voices and exit.
    #[arg(long)]
    pub list_voices: bool,

    /// Split paragraphs into sentences and pause between them.
    #[arg(long)]
    pub sentence_split: bool,

    /// Pause between sentences when `--sentence-split` is used (ms,
    /// default 150).
    #[arg(long, value_name = "MS", default_value_t = 150)]
    pub sentence_pause_ms: u64,

    /// Resume from the last saved paragraph (only meaningful with
    /// `--file`).
    #[arg(long)]
    pub resume: bool,

    /// Auto-save progress after each paragraph (only meaningful with
    /// `--file`).
    #[arg(long)]
    pub save_progress: bool,

    /// Forget saved progress and exit (only meaningful with `--file`).
    #[arg(long)]
    pub clear_progress: bool,
}

/// Arguments for `gxx topics [<topic>]`.
#[derive(Args, Debug)]
pub struct TopicsArgs {
    /// Topic name (`tts`, `models`, `language`, `bookmarks`,
    /// `examples`). Omit to list all available topics.
    pub topic: Option<String>,
}

/// Arguments for field get/set commands (`pos`, `lvl`, `pron`, `def`,
/// `note`).
///
/// Without a `VALUE`, prints the current field value (or `-` if unset).
/// With a `VALUE`, sets the field.
#[derive(Args, Debug)]
pub struct FieldArgs {
    /// Word to operate on.
    pub word: String,

    /// Value to set. Omit to read the current value.
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Grammar / style / form command group.
///
/// All three (`gram`, `style`, `form`) share the same subcommand set and
/// differ only in the entry `kind` stored in the database.
#[derive(Subcommand, Debug)]
pub enum GramCommand {
    /// Add a new grammar/style/form entry.
    Add(AddGramArgs),

    /// List entries, optionally filtered by name prefix.
    List { prefix: Option<String> },

    /// Show a full entry.
    Show { name: String },

    /// Delete an entry.
    Del { name: String },

    /// Set or get the `rule` field.
    Rule(RuleArgs),

    /// Set or get the `desc` field.
    Desc(DescArgs),

    /// Set or get the `cat` (category) field.
    Cat(CatArgs),

    /// Set or get the `level` field.
    Lvl(LvlArgs),

    /// Set or get the `notes` field.
    Note(NoteArgs),

    /// Manage examples (same semantics as word examples).
    Ex(ExGramArgs),

    /// Manage error examples (single-text learner mistakes).
    Err(ErrArgs),
}

/// Arguments for `gxx gram|style|form add <name> [--cat C] [--lvl L] [--rule R] [--desc D]`.
#[derive(Args, Debug)]
pub struct AddGramArgs {
    /// Entry name (may contain spaces).
    pub name: String,

    /// Category (e.g. `tense`, `article`, `punctuation`).
    #[arg(short, long)]
    pub cat: Option<String>,

    /// Proficiency level (e.g. `A1`, `B2`).
    #[arg(short, long)]
    pub lvl: Option<String>,

    /// Rule text.
    #[arg(short, long)]
    pub rule: Option<String>,

    /// Description text.
    #[arg(short, long)]
    pub desc: Option<String>,
}

/// Arguments for `gxx gram|style|form rule <name> [value]`.
#[derive(Args, Debug)]
pub struct RuleArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `gxx gram|style|form desc <name> [value]`.
#[derive(Args, Debug)]
pub struct DescArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `gxx gram|style|form cat <name> [value]`.
#[derive(Args, Debug)]
pub struct CatArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `gxx gram|style|form lvl <name> [value]`.
#[derive(Args, Debug)]
pub struct LvlArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `gxx gram|style|form note <name> [value]`.
#[derive(Args, Debug)]
pub struct NoteArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

/// Arguments for `gxx gram|style|form ex <name> ["text"] | N "text" | -N`.
#[derive(Args, Debug)]
pub struct ExGramArgs {
    pub name: String,
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,
}

/// Arguments for `gxx gram|style|form err <name> ["text"] | -N`.
#[derive(Args, Debug)]
pub struct ErrArgs {
    pub name: String,
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,
}

/// Arguments for `gxx load <file.gxx>`.
#[derive(Args, Debug)]
pub struct LoadArgs {
    /// Path to a `.gxx` command file (one command per line, `#`
    /// comments allowed).
    pub file: String,
}

/// TTS model management subcommands.
#[derive(Subcommand, Debug)]
pub enum ModelCommand {
    /// List all discovered TTS models with language, voice, quality,
    /// sample rate, and active status.
    List,

    /// Show the currently active model ID.
    Current,

    /// Set the active model by ID (stored in `model_config.json`).
    Use { id: String },

    /// Show detailed information about a model.
    Info { id: String },

    /// Clear the active model (revert to auto-discovery).
    Clear,
}
