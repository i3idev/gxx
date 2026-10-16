#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::invalid_html_tags)]
#![allow(rustdoc::bare_urls)]

//! # gxx
//!
//! Bilingual (English / Persian) vocabulary and grammar notebook CLI with a
//! multi-voice Text-to-Speech subsystem.
//!
//! This page contains the complete practical tutorial for every `gxx` CLI
//! command. The Rust API documentation follows further down this page.
//!
//! ## Table of contents
//!
//! - [Overview](#overview)
//! - [Global options](#global-options)
//! - [Data directory and initialization](#data-directory-and-initialization)
//! - [Database management](#database-management)
//! - [Words: core workflow](#words-core-workflow)
//! - [Word fields](#word-fields)
//! - [Translations](#translations)
//! - [Synonyms and antonyms](#synonyms-and-antonyms)
//! - [Examples](#examples)
//! - [Senses (polysemy)](#senses-polysemy)
//! - [Collocations](#collocations)
//! - [Word families](#word-families)
//! - [Grammar, style, and form entries](#grammar-style-and-form-entries)
//! - [Searching and listing](#searching-and-listing)
//! - [Deleting entries](#deleting-entries)
//! - [Text-to-Speech: say](#text-to-speech-say)
//! - [Text-to-Speech: paragraph](#text-to-speech-paragraph)
//! - [Text-to-Speech: read](#text-to-speech-read)
//! - [TTS model management](#tts-model-management)
//! - [Batch loading](#batch-loading)
//! - [Topics](#topics)
//! - [JSON output](#json-output)
//! - [Sequential tutorial: create a database and add words](#sequential-tutorial-create-a-database-and-add-words)
//! - [Sequential tutorial: enrich a word entry](#sequential-tutorial-enrich-a-word-entry)
//! - [Sequential tutorial: organize word relationships](#sequential-tutorial-organize-word-relationships)
//! - [Sequential tutorial: manage grammar, style, and form entries](#sequential-tutorial-manage-grammar-style-and-form-entries)
//! - [Sequential tutorial: use TTS](#sequential-tutorial-use-tts)
//! - [Restrictions, limitations, and common mistakes](#restrictions-limitations-and-common-mistakes)
//!
//! ## Overview
//!
//! `gxx` is a local, offline-first tool. Data lives in SQLite databases
//! under `<data-dir>/db/` (default `~/.gxx/db/`), and TTS uses Piper
//! voices discovered under `model/<lang>/<voice-quality>/`.
//!
//! The "data directory" is resolved in this order:
//!
//! 1. `--vault PATH` / `-v PATH` (CLI flag, highest priority)
//! 2. `GXX_VAULT` environment variable
//! 3. `GXX_HOME` environment variable
//! 4. `~/.gxx`
//!
//! When a vault is active, data is stored in `<vault>/.gxx/`. All
//! databases share one data directory; switch between them with
//! `gxx db use`.
//!
//! ## Modules
//!
//! - [`cli`] — command-line surface (clap)
//! - [`run`] — command dispatch and orchestration
//! - [`cmd`] — individual subcommand implementations
//! - [`db`] — SQLite storage layer
//! - [`model`] — in-memory entry model
//! - [`render`] — plain-text rendering of entries
//! - [`tts`] — Text-to-Speech subsystem (Piper, phonemizer, playback)
//! - [`error`] — shared error type
//!
//! ## Quick start
//!
//! ```text
//! gxx init
//! gxx db new demo
//! gxx add book کتاب -p n -l A1
//! gxx say book
//! gxx paragraph "Hello world."
//! gxx read notes.txt --auto
//! ```
//!
//! ## TTS pipeline
//!
//! 1. Text → eSpeak NG → IPA (with language tags stripped)
//! 2. IPA → Piper `phoneme_id_map` → phoneme IDs
//! 3. Phoneme IDs → ONNX Runtime → audio samples
//! 4. Samples → playback (system player) or WAV export
//!
//! Per-model defaults (`VowelStretch`, `LengthScale`, `NoiseScale`,
//! `NoiseW`) may be declared in each voice's `MODEL_CARD`.
//!
//! ## Global options
//!
//! These flags are defined on the root parser and are accepted before or
//! after any subcommand.
//!
//! - `--db NAME` — use database `NAME` for this command, overriding the
//!   active database. Resolves to `<data-dir>/db/<NAME>.db`.
//! - `--vault PATH`, `-v PATH` — use `<PATH>/.gxx/` as the data
//!   directory for this command. Overrides `GXX_VAULT`.
//! - `--json` — emit JSON instead of human-readable text. Supported by
//!   `show`, `find`, `list`, and direct word lookup. JSON is written to
//!   stdout.
//! - `--quiet`, `-q` — suppress non-error informational output. Errors
//!   are still printed. Success messages ("added", "updated", "ok:",
//!   "deleted", playback status) are suppressed.
//! - `--no-color` — disable ANSI colors in output.
//!
//! Example:
//!
//! ```text
//! gxx --vault ~/project --db demo --json list
//! ```
//!
//! `gxx` with no subcommand prints the usage summary and exits
//! successfully. `gxx <word> [sections...]` is a legacy shortcut for
//! `gxx show <word> [sections...]`.
//!
//! ## Data directory and initialization
//!
//! ### `gxx init`
//!
//! ```text
//! gxx init
//! ```
//!
//! Creates the data directory, `<data-dir>/db/`, and an empty `config`
//! file if they do not exist. Prints `ready: <data-dir>`. If no
//! databases exist yet, prints `next : gxx db new <name>`.
//!
//! All subcommands require initialization (a `config` file must
//! exist); run `gxx init` first. Even `gxx db new` needs the data
//! directory created by `gxx init`.
//!
//! ## Database management
//!
//! Manage the SQLite database files themselves. Database commands
//! ignore `--db` and operate on the data directory directly.
//!
//! ### `gxx db new NAME`
//!
//! ```text
//! gxx db new demo
//! ```
//!
//! Create a new SQLite database named `NAME`. `NAME` must match
//! `^[\w-]+$` (letters, digits, underscore, hyphen). The file is
//! `<data-dir>/db/<NAME>.db` and is created with the gxx schema. If no
//! active database is set, the new database becomes active
//! automatically and `active : NAME` is printed.
//!
//! Errors: invalid name (`db name: letters, digits, - and _ only`);
//! `'<NAME>' already exists`.
//!
//! ### `gxx db list`
//!
//! ```text
//! gxx db list
//! ```
//!
//! List all databases with entry counts and last-updated timestamps.
//! The active database is marked with `*`:
//!
//! ```text
//!   NAME            ENTRIES  UPDATED
//! * demo            42       2026-...
//!   test             3        2026-...
//! ```
//!
//! If none exist, prints `no databases. run: gxx db new <name>`.
//!
//! ### `gxx db use NAME`
//!
//! ```text
//! gxx db use demo
//! ```
//!
//! Set the active database by writing `NAME` to `config`. `NAME` must
//! exist (`gxx db list` shows valid names); otherwise usage is
//! printed.
//!
//! ### `gxx db info [NAME]`
//!
//! ```text
//! gxx db info
//! gxx db info demo
//! ```
//!
//! Show database info: path, active status, and per-kind entry counts
//! (word, grammar, style, form). Without `NAME`, shows the active
//! database. Errors: `no database '<NAME>'`,
//! `no active db. run: gxx init, then: gxx db new <name>`.
//!
//! Verify creation with `gxx db list` or `gxx db info`.
//!
//! ### `gxx db del NAME [--yes]`
//!
//! ```text
//! gxx db del old
//! gxx db del old --yes
//! ```
//!
//! Delete a database file. Without `--yes`, prompts `type '<NAME>' to
//! delete it:` and requires typing the name exactly; otherwise prints
//! `cancelled`. With `--yes`, skips the prompt. If the deleted database
//! was active, the active setting is cleared (config becomes empty).
//!
//! Restrictions: deletion is permanent; there is no undo. Do not delete
//! the active database without creating/switching to another one. Do not
//! delete a name that does not exist (usage error).
//!
//! ## Words: core workflow
//!
//! Words are entries of kind `word`. They store a title, translations,
//! part of speech, level, pronunciation, definition, notes, examples,
//! synonyms, antonyms, senses, collocations, and family members.
//!
//! ### `gxx add WORD [TRANSLATION...] [-p POS] [-l LEVEL]`
//!
//! ```text
//! gxx add book کتاب -p n -l A1
//! gxx add run "run, execute" -p v -l B1
//! ```
//!
//! Add a new word entry, or update an existing one. Language is
//! auto-detected from the word's script (English vs. Persian). Each
//! `TRANSLATION` is auto-directed to the opposite language of the word.
//! Duplicates are skipped. `-p/--pos` sets part of speech; `-l/--level` sets proficiency
//! level. In the
//! interactive examples flags are shown as `-p` and `-l`.
//!
//! Empty option values remove the field. Prints `added: WORD` or
//! `updated: WORD` unless `--quiet`.
//!
//! Errors: `usage: gxx add <word> [translations...] [-p pos] [-l level]`
//! when no word is given.
//!
//! ### Direct lookup (legacy)
//!
//! ```text
//! gxx book tr syn
//! ```
//!
//! Equivalent to `gxx show book tr syn`.
//!
//! ## Word fields
//!
//! The commands `pos`, `lvl`, `pron`, `def`, `note` get or set a single
//! field. Without a value they print the current value, or `-` if unset;
//! with a value they set it (multi-word values are joined with spaces).
//! A non-quiet run prints `ok: <field> = <value>`.
//!
//! ```text
//! gxx pos book n
//! gxx pos book
//! gxx lvl book A2
//! gxx pron book /bʊk/
//! gxx def book "a written or printed work"
//! gxx note book "IR regular noun; countable"
//! ```
//!
//! Field mapping: `pos` → `pos`, `lvl` → `level`, `pron` →
//! `pronunciation`, `def` → `def`, `note` → `notes`.
//!
//! Empty value removes the field. Re-running `gxx add` with
//! `-p`/`-l` sets the same fields.
//!
//! Verify with `gxx show book` or `gxx <field> book`.
//!
//! ## Translations
//!
//! ### `gxx tr WORD [LANG] OP...`
//!
//! ```text
//! gxx tr book fa +کتاب +کتابچه
//! gxx tr book fa -کتابچه
//! gxx tr book +kitab            # auto-detects opposite language
//! ```
//!
//! Manage the `translations` object keyed by language code. `LANG` is a
//! two-letter code (e.g. `fa`, `en`); if omitted it is auto-detected
//! from the word's script (English word → `fa` target; Persian word →
//! `en` target). Operations: `+text` adds, `-text` removes, plain `text`
//! adds. Duplicates are skipped; removing a missing item prints
//! `not found: ...` to stderr. If a language list becomes empty, its
//! key is removed. Prints `ok: <lang> = <comma list>` or `-`.
//!
//! Errors: `usage: gxx tr <word> [lang] +x -y`;
//! `no word '<WORD>'. add it first: gxx add <WORD>`.
//! Restrictions: `LANG` must match `^[a-z]{2}$` to be treated as a
//! language code; otherwise it is treated as an operation.
//!
//! ## Synonyms and antonyms
//!
//! ### `gxx syn WORD OP...`
//! ### `gxx ant WORD OP...`
//!
//! ```text
//! gxx syn book +volume +tome
//! gxx syn book -tome
//! gxx ant book +cover
//! ```
//!
//! Manage the `synonyms` / `antonyms` arrays. `+text` adds, `-text`
//! removes, plain `text` adds, deduplicated. With no operations, prints
//! the current comma-separated list. Removing a missing item prints
//! `not found: ...`.
//!
//! Verify with `gxx show book syn` or `gxx syn book`.
//!
//! ## Examples
//!
//! ### `gxx ex WORD ...` (words)
//! ### `gxx gram|style|form ex NAME ...` (grammar/style/form)
//!
//! ```text
//! gxx ex book "I read a book." "کتابی خواندم."
//! gxx ex book 1 "I read a big book."
//! gxx ex book --delete --number 2
//! ```
//!
//! Examples are bilingual pairs with an English side (`en`) and a
//! Persian side (`fa`); language is auto-detected from Unicode script.
//!
//! Operations:
//!
//! - List all: `gxx ex WORD`
//! - Add a pair or single side: `gxx ex WORD "en text" "fa text"`
//!   (one or two texts; maximum two; both sides must be different
//!   languages).
//! - Edit one side of example N: `gxx ex WORD N "text"` or
//!   `gxx ex WORD --number N "text"`.
//! - Delete example N: `gxx ex WORD --delete --number N` or
//!   `gxx ex WORD -- -N`.
//!
//! For grammar/style/form entries the CLI form is
//! `gxx gram ex NAME ["text"]`, `gxx gram ex NAME N "text"`, or
//! `gxx gram ex NAME -- -N` (deletion via a negative number, because there
//! is no `--delete` flag on that group). Editing uses the `--number`
//! flag: `gxx gram ex NAME --number 2 "text"`.
//!
//! Restrictions and errors: `one example per command: quote each
//! sentence` (>2 texts); `two texts in the same language; give one en and
//! one fa`; `no example #N (have M)`; `usage: gxx ex <name> N "text"`.
//! Deletion requires `--number` on the word form and cannot be combined
//! with example text.
//!
//! Verify with `gxx show book ex` or `gxx ex book`.
//!
//! ## Senses (polysemy)
//!
//! ### `gxx sense WORD [add|del|show]`
//!
//! ```text
//! gxx sense book add "written or printed work consisting of pages"
//! gxx sense book add "unit of something" --note "mathematical"
//! gxx sense book show 1
//! gxx sense book del 2
//! gxx sense book
//! ```
//!
//! Manage numbered senses (definitions with optional notes and examples).
//! `gxx sense WORD` with no action lists senses (`1> definition`,
//! optional `note:`). `add` takes a definition and optional `--note`.
//! `del N` deletes sense N. `show N` prints `def:`, optional `note:`,
//! and any examples for that sense.
//!
//! Errors: `usage: ...`; `no sense #N (have M)`; `invalid sense
//! number`; `unknown sense subcommand`.
//!
//! ## Collocations
//!
//! ### `gxx coll WORD [add|del]`
//!
//! ```text
//! gxx coll book add "book club"
//! gxx coll book
//! gxx coll book del 1
//! ```
//!
//! Manage collocations (common word pairings). `gxx coll WORD` lists
//! them numbered (`1> text`) or prints `no collocations`. `add TEXT`
//! adds (text may contain spaces). `del N` deletes number N.
//!
//! Errors: `usage: gxx coll <word> add "collocation"`,
//! `no collocation #N (have M)`, `invalid collocation number`,
//! `unknown collocation subcommand`.
//!
//! ## Word families
//!
//! ### `gxx family WORD [add|del]`
//!
//! ```text
//! gxx family book add textbook -p n
//! gxx family book add booklet
//! gxx family book del booklet
//! gxx family book
//! ```
//!
//! Manage word-family members (related words with optional POS). List
//! shows `word  (pos)`. Add: `gxx family WORD add RELATED [-p POS]`.
//! Delete by related word (`gxx family WORD del RELATED`). Errors if the
//! member is not found: `family member '<WORD>' not found`.
//!
//! ## Grammar, style, and form entries
//!
//! `gram`, `style`, and `form` share one subcommand set and differ only
//! in the stored entry kind (`grammar`, `style`, `form`). All use the
//! same handler.
//!
//! ```text
//! gxx gram add "present perfect" --cat tense --lvl B1 --rule "have + V3" --desc "..."
//! gxx style add "formal register" --cat style
//! gxx form add "plural nouns" --cat morphology
//! ```
//!
//! Subcommands (shown for `gram`; identical for `style`/`form`):
//!
//! - `gxx gram add NAME [--cat C] [--lvl L] [--rule R] [--desc D]`
//!   (flags also accept `-c`, `-l`, `-r`, `-d`)
//! - `gxx gram list [PREFIX]` — list by title; filter by key/prefix.
//! - `gxx gram show NAME [rule|desc|ex|err|note]` — show (optional
//!   sections).
//! - `gxx gram del NAME` — delete.
//! - `gxx gram rule NAME [VALUE]` — get/set `rule`.
//! - `gxx gram desc NAME [VALUE]` — get/set `desc`.
//! - `gxx gram cat NAME [VALUE]` — get/set `cat`.
//! - `gxx gram lvl NAME [VALUE]` — get/set `level`.
//! - `gxx gram note NAME [VALUE]` — get/set `notes`.
//! - `gxx gram ex NAME ...` — examples (same semantics as word
//!   examples).
//! - `gxx gram err NAME ...` — error examples (single text). List, add
//!   (`"text"`), delete (`-N`).
//!
//! Empty value removes a field. Adding an existing name updates it
//! (`updated`); new names print `added`.
//!
//! Sections accepted by `show`: `rule`, `desc`, `ex`, `err`, `note`
//! (grammar/style/form). Word `show` accepts `tr`, `syn`, `ant`, `ex`,
//! `sense`, `coll`, `family`, `note`.
//!
//! Verify with `gxx gram show "present perfect"` and `gxx gram list`.
//!
//! ## Searching and listing
//!
//! ### `gxx show WORD [SECTIONS...]`
//!
//! ```text
//! gxx show book
//! gxx show book tr syn ex
//! gxx show book --json
//! ```
//!
//! Display an entry (searches all four kinds: word, grammar, style,
//! form). Optional section filters narrow output. If the name matches
//! multiple kinds, all are rendered. Not found:
//! `not found: WORD  (try: gxx find WORD)`.
//!
//! ### `gxx find QUERY [--all]`
//!
//! ```text
//! gxx find book
//! gxx find "b*" --all
//! ```
//!
//! Case-insensitive substring search on normalised text; `*` wildcards.
//! Results sorted with exact matches first, then by title. `--all`
//! searches every database (printing `DB: ...`). Prints `--
//! N found` (or `nothing found`). With `--json`, prints a JSON array.
//!
//! ### `gxx list [--pos POS] [--lvl LEVEL] [--lang en|fa]`
//!
//! ```text
//! gxx list
//! gxx list --pos n --lvl A1 --lang en
//! ```
//!
//! List word entries with optional filters (case-insensitive for POS
//! and level). Prints `-- N words`.
//!
//! ## Deleting entries
//!
//! ### `gxx del WORD [--yes]`
//!
//! ```text
//! gxx del obsolete
//! gxx del obsolete --yes
//! ```
//!
//! Delete a word entry. Without `--yes`, asks for interactive
//! confirmation: type the word. Prints `cancelled` on mismatch.
//! On success prints `deleted: WORD`. Errors:
//! `usage: gxx del <name>`,
//! `no word '<WORD>'. add it first: gxx add <WORD>`.
//! Restriction: `del` targets word entries only; grammar/style/form use
//! their own `del` subcommand.
//!
//! ## Text-to-Speech: say
//!
//! ### `gxx say WORD [options]`
//!
//! ```text
//! gxx say book
//! gxx say book --example 1 --repeat 3 --slow
//! gxx say book --phonemes "bʊk"
//! ```
//!
//! Speak a notebook word aloud. By default speaks the word; `--example N`
//! speaks example N's English text. Options:
//!
//! - `--example N` — speak example N's English text (error if no English
//!   text).
//! - `--repeat N` — repeat playback 1-10 (default 1).
//! - `--slow` — speed 0.7x.
//! - `--very-slow` — speed 0.5x.
//! - `--speed N` — playback speed 0.1-2.0 (overrides slow flags).
//! - `--soft` — volume 0.6x.
//! - `--phonemes IPA` — manual IPA, bypasses eSpeak and language guard.
//!
//! Language guard: active voice language must match the text language;
//! otherwise an error is returned. Bypass with `--phonemes` or switch
//! model with `gxx model use`.
//!
//! Verify: listens to playback or use `gxx paragraph ... --out` for
//! file export (say itself plays only).
//!
//! ## Text-to-Speech: paragraph
//!
//! ### `gxx paragraph [TEXT] [options]`
//!
//! ```text
//! gxx paragraph "Hello world."
//! gxx paragraph --voice fa "سلام دنیا"
//! gxx paragraph --auto "hello سلام"
//! gxx paragraph -f text.txt --sentence-split --out out.wav
//! gxx paragraph --list-voices
//! ```
//!
//! Speak arbitrary text (no DB lookup). Text can be inline, from a file
//! (`-f/--file PATH`), or manual IPA (`--phonemes`). Options common to
//! `read` and paragraph:
//!
//! - `--voice CODE` — override voice by language code (`en`, `fa`) or
//!   voice name (`cori`, `ryan`, `amir`). `--voice auto` is an alias
//!   for `--auto`.
//! - `--auto` — segment text by language and route each segment to a
//!   matching model. Unsupported-language segments are skipped with
//!   `TTS: warning: no model for '...'`.
//! - `--repeat N` — repeat playback (default 1, 1-10).
//! - `--speed N` — playback speed multiplier (default 1.0).
//! - `--volume N` — playback volume (default 1.0).
//! - `--paragraph-pause-ms MS` — gap between paragraphs (default 300).
//! - `--length-scale F32` — Piper length scale (higher = slower).
//! - `--noise-scale F32` — Piper noise scale.
//! - `--noise-w F32` — Piper noise W.
//! - `--vowel-stretch N` — extra marks after long vowels (0-3).
//! - `--out PATH` — write WAV instead of playing.
//! - `--sentence-split` — split into sentences with pauses.
//! - `--sentence-pause-ms MS` — gap between sentences (default 150).
//! - `--list-voices` — print voice table and exit.
//! - `--phonemes IPA` — manual IPA, skips eSpeak and language guard.
//! - `--progress` flags (`--resume`, `--save-progress`,
//!   `--clear-progress`) are only meaningful with `--file`.
//!
//! Errors: no text and no `--file` (`no text and no --file`);
//! giving both text and `--file` (`give text OR --file, not both`).
//!
//! ## Text-to-Speech: read
//!
//! ### `gxx read FILE [options]`
//!
//! ```text
//! gxx read notes.txt
//! gxx read notes.txt --auto
//! gxx read notes.txt --out notes.wav
//! gxx read notes.txt --save-progress
//! gxx read notes.txt --resume
//! ```
//!
//! Narrate a text file paragraph by paragraph. Same synthesis options
//! as `paragraph`, plus reading-specific:
//!
//! - `--pause-ms MS` — pause between paragraphs when playing
//!   (default 300).
//! - `--start N` — start at paragraph N (1-based).
//! - `--voice`, `--auto`, `--speed`, `--volume`, `--length-scale`,
//!   `--noise-scale`, `--noise-w`, `--vowel-stretch`, `--sentence-split`,
//!   `--sentence-pause-ms`, `--out` — as above.
//! - `--save-progress` — save next paragraph index after each paragraph.
//! - `--resume` — start from saved progress.
//! - `--clear-progress` — forget saved progress and exit.
//!
//! Empty files or files with no readable text error out
//! (`file has no readable text`). Progress is stored under
//! `~/.local/share/gxx/progress/<hash>.txt`, keyed by path/size/mtime,
//! so editing the file invalidates the bookmark. Ctrl+C finishes the
//! current paragraph then stops
//! (`TTS: interrupted, stopping after current paragraph`).
//!
//! Printing during a run includes `TTS: paragraph N/M`. If finished,
//! `TTS: already finished (N of N paragraphs read)` and use
//! `--clear-progress` to restart.
//!
//! ## TTS model management
//!
//! ### `gxx model list`
//! ### `gxx model current`
//! ### `gxx model use ID`
//! ### `gxx model info ID`
//! ### `gxx model clear`
//!
//! ```text
//! gxx model list
//! gxx model use <ID>
//! gxx model current
//! gxx model info <ID>
//! gxx model clear
//! ```
//!
//! Discover and select Piper voices under `model/`. `list` prints a
//! table (ID, name, language, voice, quality, sample rate, availability,
//! active mark) or `no models found`. `current` prints the active id
//! or `auto (not set)`. `use ID` stores the active model in
//! `model_config.json`. `info ID` prints paths and availability. `clear`
//! reverts to auto-discovery (`active model cleared (using
//! auto-discovery)`). Unknown id: `model '...' not found`.
//!
//! `gxx paragraph --list-voices` shows a similar table including
//! MODEL_CARD defaults.
//!
//! ## Batch loading
//!
//! ### `gxx load FILE.gxx`
//!
//! ```text
//! gxx load demo.gxx
//! ```
//!
//! Execute commands from a text file (one command per line; `#` starts a
//! comment; empty lines skipped). Leading `gxx` token is optional.
//! Nested `load` is forbidden. Errors on individual lines are reported
//! (`line N: ...`) but processing continues. Quiet mode is forced during
//! processing. Prints `loaded N commands, M errors` at the end.
//!
//! Missing file: `no such file: PATH`. Parse error: `line N: parse
//! error`.
//!
//! ## Topics
//!
//! ### `gxx topics [TOPIC]`
//!
//! ```text
//! gxx topics
//! gxx topics tts
//! ```
//!
//! Print extended help. Topics: `tts`, `models`, `language`,
//! `bookmarks`, `examples`. Without a topic, lists them. Unknown topic:
//! `unknown topic '...'`.
//!
//! ## JSON output
//!
//! With `--json`, `show` prints the entry (or array when several kinds
//! match), `find` prints a JSON array of records, and `list` prints a
//! JSON array of word records. JSON keys mirror stored fields.
//!
//! ## Sequential tutorial: create a database and add words
//!
//! 1. Initialize and create a database:
//!
//! ```text
//! gxx init
//! gxx db new vocab
//! gxx db use vocab
//! gxx db info
//! ```
//!
//! 2. Add entries:
//!
//! ```text
//! gxx add book کتاب -p n -l A1
//! gxx add run دویدن -p v -l A2
//! ```
//!
//! 3. Verify:
//!
//! ```text
//! gxx show book
//! gxx list
//! gxx find book
//! ```
//!
//! ## Sequential tutorial: enrich a word entry
//!
//! ```text
//! gxx pos book n
//! gxx lvl book A2
//! gxx pron book /bʊk/
//! gxx def book "a written or printed work"
//! gxx note book "countable"
//! gxx tr book fa +کتاب +کتابچه
//! gxx syn book +volume +tome
//! gxx ant book +cover
//! gxx sense book add "written work" --note "primary"
//! gxx coll book add "book club"
//! gxx family book add textbook -p n
//! gxx ex book "I read a book." "کتابی خواندم."
//! ```
//!
//! Verify each step with `gxx show book` and the relevant
//! `gxx show book <section>`.
//!
//! ## Sequential tutorial: organize word relationships
//!
//! ```text
//! gxx syn book +volume +tome
//! gxx syn book -tome
//! gxx ant book +cover
//! gxx family book add chapter -p n
//! gxx family book del chapter
//! gxx coll book add "book shelf"
//! gxx coll book del 1
//! ```
//!
//! List current values with `gxx syn book`, `gxx ant book`,
//! `gxx family book`, `gxx coll book`.
//!
//! ## Sequential tutorial: manage grammar, style, and form entries
//!
//! ```text
//! gxx gram add "present perfect" --cat tense --lvl B1 --rule "have + V3"
//! gxx gram desc "present perfect" "completed action with present result"
//! gxx gram ex "present perfect" "I have finished." "تمام کرده‌ام."
//! gxx gram err "present perfect" "I have went"
//! gxx gram show "present perfect"
//! gxx gram list
//! ```
//!
//! Same commands work with `style` and `form` instead of `gram`.
//!
//! ## Sequential tutorial: use TTS
//!
//! ```text
//! gxx model list
//! gxx model use <id>
//! gxx say book
//! gxx paragraph "Hello world."
//! gxx paragraph --voice fa "سلام دنیا"
//! gxx paragraph --auto "hello سلام"
//! gxx paragraph --out greeting.wav "Hello"
//! printf 'First paragraph.\n\nSecond paragraph.' > notes.txt
//! gxx read notes.txt --auto --sentence-split
//! gxx read notes.txt --save-progress
//! gxx read notes.txt --resume
//! ```
//!
//! Use `gxx topics tts`, `gxx topics models`,
//! `gxx topics language`, `gxx topics bookmarks` for deeper reference.
//!
//! ## Restrictions, limitations, and common mistakes
//!
//! - Run `gxx init` before any subcommand except `gxx db new`.
//! - `--db` overrides the active DB but database commands (`gxx db ...`)
//!   ignore it.
//! - `gxx add` requires at least one word; translations are pushed into
//!   the opposite language of the word; duplicates are skipped.
//! - `del` (word) needs confirmation unless `--yes`.
//! - `gxx ex` word form: `--delete` requires `--number` and cannot
//!   combine with text; maximum two texts and they must be different
//!   languages.
//! - Grammar/style/form entries use negative numbers (`-N`) to delete
//!   examples/errors from the CLI; there is no `--delete` flag on those.
//! - Field setters accept only one value token stream (joined with
//!   spaces); an empty value removes the field.
//! - `tr` LANG detection: a two-letter lowercase token is interpreted
//!   as the target language; quote or avoid such tokens as operations.
//! - `find` uses `*` glob syntax; other regex metacharacters are not
//!   special.
//! - TTS requires models under `model/`; missing models produce
//!   `no models found` or synthesis warnings. `say` enforces language
//!   matching with the active voice unless `--phonemes` is given.
//! - `read --resume` on a finished file prints "already finished"; use
//!   `--clear-progress`.
//! - Do not create a separate manual or `docs/` directory; this page
//!   is the canonical command tutorial.
//!
//! ### TTS pipeline
//!
//! 1. Text → eSpeak NG → IPA (with language tags stripped)
//! 2. IPA → Piper `phoneme_id_map` → phoneme IDs
//! 3. Phoneme IDs → ONNX Runtime → audio samples
//! 4. Samples → playback (system player) or WAV export
//!
//! Per-model defaults (VowelStretch, LengthScale, NoiseScale, NoiseW)
//! may be declared in each voice's `MODEL_CARD`.

mod cli;
mod cmd;
mod db;
mod error;
mod model;
mod render;
mod run;
mod style;
mod topics;
mod tts;

use crate::error::GxxError;
use crate::run::run_cli;
use clap::Parser;
use cli::Cli;

fn main() {
    // Ensure stdout encoding on Windows
    #[cfg(windows)]
    {
        use std::io::{stdout, Write};
        let _ = stdout().flush();
    }

    let cli = Cli::parse();

    if cli.no_color {
        crate::style::set_mode(1);
    }

    let result = run_cli(cli);

    match result {
        Ok(()) => {}
        Err(e) => {
            let msg = match e {
                GxxError::Msg(msg) => msg,
                _ => e.to_string(),
            };
            eprintln!("{} {}", crate::style::err("error:"), msg);
            std::process::exit(1);
        }
    }
}
