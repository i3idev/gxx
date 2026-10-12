//! CLI definitions using clap.

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "gxx",
    version,
    about = "Bilingual (English / Persian) word and grammar notebook",
    long_about = "gxx - A CLI tool for managing bilingual vocabulary and grammar notes.\nStores data in SQLite databases under ~/.gxx/db/.",
    allow_hyphen_values = true
)]
pub struct Cli {
    /// Database to use for this command (overrides active database)
    #[arg(long, global = true, value_name = "NAME")]
    pub db: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize config directory and default database
    Init,

    /// Database management
    #[command(subcommand)]
    Db(DbCommand),

    /// Add a new word entry
    Add(AddArgs),

    /// Show entry details
    Show(ShowArgs),

    /// Search entries
    Find(FindArgs),

    /// List entries with optional filters
    List(ListArgs),

    /// Delete an entry
    Del(DelArgs),

    /// Manage translations
    Tr(TrArgs),

    /// Manage synonyms
    Syn(SynAntArgs),

    /// Manage antonyms
    Ant(SynAntArgs),

    /// Manage examples
    Ex(ExArgs),

    /// Manage senses (polysemy)
    Sense(SenseArgs),

    /// Manage collocations
    Coll(CollArgs),

    /// Manage word family
    Family(FamilyArgs),

    /// Text-to-speech playback
    Say(SayArgs),

    /// Read a text file aloud (paragraph by paragraph)
    Read(ReadArgs),

    /// Speak arbitrary text (no DB lookup, no caching by word)
    Paragraph(ParagraphArgs),

    /// Speak arbitrary text (no DB lookup, no caching by word)

    /// TTS model management
    #[command(subcommand)]
    Model(ModelCommand),

    /// Set/get part of speech
    Pos(FieldArgs),

    /// Set/get proficiency level
    Lvl(FieldArgs),

    /// Set/get pronunciation
    Pron(FieldArgs),

    /// Set/get definition
    Def(FieldArgs),

    /// Set/get notes
    Note(FieldArgs),

    /// Grammar entries
    #[command(subcommand)]
    Gram(GramCommand),

    /// Style entries
    #[command(subcommand)]
    Style(GramCommand),

    /// Form entries
    #[command(subcommand)]
    Form(GramCommand),

    /// Load commands from a file
    Load(LoadArgs),

    /// Direct word lookup (same as: gxx show <word>)
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Create a new database
    New { name: String },

    /// List all databases
    List,

    /// Set active database
    Use { name: String },

    /// Show database info
    Info { name: Option<String> },

    /// Delete a database
    Del {
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Word to add
    pub word: String,

    /// Translations (target language auto-detected)
    #[arg(value_name = "TRANSLATION")]
    pub translations: Vec<String>,

    /// Part of speech (n, v, adj, adv, etc.)
    #[arg(short, long, value_name = "POS")]
    pub pos: Option<String>,

    /// Proficiency level (A1, A2, B1, B2, C1, C2)
    #[arg(short, long, value_name = "LEVEL")]
    pub level: Option<String>,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Word to show
    pub word: String,

    /// Sections to display (tr, syn, ant, ex, sense, coll, family, note)
    #[arg(value_name = "SECTION")]
    pub sections: Vec<String>,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    /// Search query
    pub query: String,

    /// Search across all databases
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter by part of speech
    #[arg(long, value_name = "POS")]
    pub pos: Option<String>,

    /// Filter by level
    #[arg(long, value_name = "LEVEL")]
    pub lvl: Option<String>,

    /// Filter by language (en, fa)
    #[arg(long, value_name = "LANG")]
    pub lang: Option<String>,
}

#[derive(Args, Debug)]
pub struct DelArgs {
    /// Word to delete
    pub word: String,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct TrArgs {
    /// Word to modify
    pub word: String,

    /// Target language (auto-detected from word if omitted)
    #[arg(value_name = "LANG")]
    pub lang: Option<String>,

    /// Operations: +add -remove (e.g., +volume -fast)
    #[arg(value_name = "OP", allow_hyphen_values = true, trailing_var_arg = true)]
    pub ops: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SynAntArgs {
    /// Word to modify
    pub word: String,

    /// Operations: +add -remove (e.g., +volume -book)
    #[arg(value_name = "OP", allow_hyphen_values = true, trailing_var_arg = true)]
    pub ops: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ExArgs {
    /// Word to modify
    pub word: String,

    /// Example text(s) - language auto-detected
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,

    /// Example number to set/remove
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,
}

#[derive(Args, Debug)]
pub struct SenseArgs {
    /// Word to modify
    pub word: String,

    #[command(subcommand)]
    pub action: Option<SenseAction>,
}

#[derive(Subcommand, Debug)]
pub enum SenseAction {
    /// Add a new sense
    Add {
        /// Definition
        definition: String,
        /// Optional note
        #[arg(long)]
        note: Option<String>,
    },
    /// Delete a sense by number
    Del { number: usize },
    /// Show a specific sense
    Show { number: usize },
}

#[derive(Args, Debug)]
pub struct CollArgs {
    /// Word to modify
    pub word: String,

    #[command(subcommand)]
    pub action: Option<CollAction>,
}

#[derive(Subcommand, Debug)]
pub enum CollAction {
    /// Add a collocation
    Add { text: String },
    /// Delete a collocation by number
    Del { number: usize },
}

#[derive(Args, Debug)]
pub struct FamilyArgs {
    /// Word to modify
    pub word: String,

    #[command(subcommand)]
    pub action: Option<FamilyAction>,
}

#[derive(Subcommand, Debug)]
pub enum FamilyAction {
    /// Add a family member
    Add {
        /// Related word
        word: String,
        /// Part of speech
        #[arg(short, long)]
        pos: Option<String>,
    },
    /// Delete a family member
    Del { word: String },
}

#[derive(Args, Debug)]
pub struct SayArgs {
    /// Word to speak
    pub word: String,

    /// Speak example N instead of word
    #[arg(long, value_name = "N")]
    pub example: Option<usize>,

    /// Repeat N times (1-10)
    #[arg(long, value_name = "N", default_value = "1")]
    pub repeat: u32,

    /// Slow speed (0.7x)
    #[arg(long)]
    pub slow: bool,

    /// Very slow speed (0.5x)
    #[arg(long)]
    pub very_slow: bool,

    /// Soft volume (0.6x)
    #[arg(long)]
    pub soft: bool,

    /// Manual IPA phonemes
    #[arg(long, value_name = "IPA")]
    pub phonemes: Option<String>,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    /// File to read
    pub file: std::path::PathBuf,

    /// Override voice (language code like en/fa or name like cori/amir)
    #[arg(long, value_name = "CODE")]
    pub voice: Option<String>,

    /// Auto-detect language per segment
    #[arg(long)]
    pub auto: bool,

    /// Playback speed multiplier
    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,

    /// Playback volume multiplier
    #[arg(long, default_value_t = 1.0)]
    pub volume: f32,

    /// Pause between paragraphs (milliseconds)
    #[arg(long, value_name = "MS", default_value_t = 300)]
    pub pause_ms: u64,

    /// Start from paragraph N (1-based)
    #[arg(long, value_name = "N")]
    pub start: Option<usize>,
}

#[derive(Args, Debug)]
pub struct ParagraphArgs {
    /// Text to speak (quote it if it has spaces)
    pub text: Option<String>,

    /// Read text from a file instead of inline
    #[arg(short = 'f', long, value_name = "PATH")]
    pub file: Option<std::path::PathBuf>,

    /// Manual IPA phonemes (skips eSpeak and language check)
    #[arg(long, value_name = "IPA")]
    pub phonemes: Option<String>,

    /// Override model by language code (en, fa) or voice name (cori, ryan, amir)
    #[arg(long, value_name = "CODE")]
    pub voice: Option<String>,

    /// Auto-select model per language segment (mixed text supported)
    #[arg(long)]
    pub auto: bool,

    /// Repeat N times (1-10)
    #[arg(long, value_name = "N", default_value = "1")]
    pub repeat: u32,

    /// Playback speed multiplier
    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,

    /// Playback volume multiplier
    #[arg(long, default_value_t = 1.0)]
    pub volume: f32,

    /// Pause between paragraphs (milliseconds)
    #[arg(long, value_name = "MS", default_value_t = 300)]
    pub paragraph_pause_ms: u64,
}

#[derive(Args, Debug)]
pub struct FieldArgs {
    /// Word to modify
    pub word: String,

    /// Value to set (omit to get current value)
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum GramCommand {
    /// Add a new entry
    Add(AddGramArgs),

    /// List entries
    List { prefix: Option<String> },

    /// Show full entry
    Show { name: String },

    /// Delete entry
    Del { name: String },

    /// Set/get rule
    Rule(RuleArgs),

    /// Set/get description
    Desc(DescArgs),

    /// Set/get category
    Cat(CatArgs),

    /// Set/get level
    Lvl(LvlArgs),

    /// Set/get notes
    Note(NoteArgs),

    /// Manage examples
    Ex(ExGramArgs),

    /// Manage errors
    Err(ErrArgs),
}

#[derive(Args, Debug)]
pub struct AddGramArgs {
    /// Entry name
    pub name: String,

    /// Category
    #[arg(short, long)]
    pub cat: Option<String>,

    /// Level
    #[arg(short, long)]
    pub lvl: Option<String>,

    /// Rule
    #[arg(short, long)]
    pub rule: Option<String>,

    /// Description
    #[arg(short, long)]
    pub desc: Option<String>,
}

#[derive(Args, Debug)]
pub struct RuleArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct DescArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct CatArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct LvlArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct NoteArgs {
    pub name: String,
    #[arg(value_name = "VALUE")]
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct ExGramArgs {
    pub name: String,
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,
}

#[derive(Args, Debug)]
pub struct ErrArgs {
    pub name: String,
    #[arg(value_name = "TEXT")]
    pub texts: Vec<String>,
    #[arg(short, long, value_name = "N")]
    pub number: Option<usize>,
}

#[derive(Args, Debug)]
pub struct LoadArgs {
    /// Path to .gxx command file
    pub file: String,
}

#[derive(Subcommand, Debug)]
pub enum ModelCommand {
    /// List all available TTS models
    List,

    /// Show the currently active model
    Current,

    /// Set the active model by ID
    Use { id: String },

    /// Show detailed info about a model
    Info { id: String },

    /// Clear active model (revert to auto-discovery)
    Clear,
}
