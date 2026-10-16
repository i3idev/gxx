//! Topic-based help for the CLI.
//!
//! `gxx topics` prints extended help on specific topics that go beyond
//! the built-in CLI usage summary. Topics are defined as string
//! constants in this module and printed by [`print_topic`].
//!
//! Available topics:
//!
//! - `tts` — TTS overview: commands, synthesis options
//! - `models` — Voices, model layout, MODEL_CARD presets
//! - `language` — Language detection, `--auto`, error handling
//! - `bookmarks` — `read --save-progress` / `--resume` / `--clear-progress`
//! - `examples` — End-to-end walkthroughs

use crate::error::GxxError;

/// List of available help topics.
///
/// Each entry is `(name, short description)`.
pub const TOPICS: &[(&str, &str)] = &[
    ("tts", "TTS overview: commands, synthesis options"),
    ("models", "Voices, model layout, MODEL_CARD presets"),
    ("language", "Language detection, --auto, error handling"),
    (
        "bookmarks",
        "read --save-progress / --resume / --clear-progress",
    ),
    ("examples", "End-to-end walkthroughs"),
];

/// Print the list of available topics.
pub fn list_topics() {
    use crate::style;
    println!("{}", style::header("gxx topics — extended help"));
    println!();
    println!("Usage: {} <TOPIC>", style::key("gxx topics"));
    println!();
    println!("{}", style::key("Available topics:"));
    for (name, desc) in TOPICS {
        // Pad *before* coloring, so ANSI codes don't break the alignment.
        let padded = format!("{:<12}", name);
        println!("  {} {}", style::accent(&padded), desc);
    }
    println!();
    println!("Run `{}` for details.", style::key("gxx topics <TOPIC>"));
}

/// Print a specific topic, or list all if name is `None`.
///
/// # Errors
///
/// Returns an error for unknown topic names.
pub fn print_topic(name: Option<&str>) -> Result<(), GxxError> {
    let Some(name) = name else {
        list_topics();
        return Ok(());
    };
    let body = match name {
        "tts" => TTS,
        "models" => MODELS,
        "language" => LANGUAGE,
        "bookmarks" => BOOKMARKS,
        "examples" => EXAMPLES,
        other => {
            return Err(GxxError::msg(format!(
                "unknown topic '{}'. run `gxx topics` to list",
                other
            )));
        }
    };
    print!("{}", body);
    Ok(())
}

const TTS: &str = r#"gxx TTS — Text-to-Speech overview

The TTS subsystem uses Piper voices with eSpeak NG phonemization.

COMMANDS
  say        Speak a word from the notebook
  paragraph  Speak arbitrary text (no DB lookup)
  read       Narrate a file paragraph by paragraph
  model      Manage voices (list, use, current, info)

QUICK START
  gxx model list                       # see available voices
  gxx model use <id>                   # set active voice
  gxx paragraph "Hello world."         # free text
  gxx paragraph --voice fa "سلام"      # force a voice
  gxx paragraph --auto "hello سلام"    # per-segment language routing
  gxx read book.txt --auto             # narrate a file
  gxx read book.txt --out book.wav     # export to WAV

SYNTHESIS OPTIONS (paragraph, read)
  --length-scale <F32>   Higher = slower, more deliberate
  --noise-scale <F32>    Lower = flatter prosody
  --noise-w <F32>        Phoneme duration variance
  --vowel-stretch <N>    Extra ː marks after long vowels (0-3)
  --sentence-split       Split paragraphs into sentences
  --sentence-pause-ms    Gap between sentences (ms, default 150)
  --speed <F32>          Playback speed (0-2)
  --volume <F32>         Playback volume (0-2)
  --out <PATH>           Write WAV instead of playing

PLAYBACK VS SYNTHESIS
  --speed is applied to the generated audio (playback-side).
  --length-scale changes the synthesis itself (Piper-side).

  For a slower, more natural read, try:
    --length-scale 1.15 --sentence-split

DETAILED GUIDE
  docs/tts.md — full TTS and reading reference
  docs/models.md — model discovery, selection, MODEL_CARD

See also:
  gxx topics models
  gxx topics language
  gxx topics bookmarks
"#;

const MODELS: &str = r#"gxx TTS — Voices and models

LAYOUT
  model/<lang>/<voice-quality>/<lang>-<voice>-<quality>.onnx
  model/<lang>/<voice-quality>/<lang>-<voice>-<quality>.onnx.json
  model/<lang>/<voice-quality>/MODEL_CARD

EXAMPLE
  model/
    en_GB/cori-high/
      en_GB-cori-high.onnx
      en_GB-cori-high.onnx.json
      MODEL_CARD
    en_US/ryan-high/
      ...
    fa_IR/amir-medium/
      ...

ADDING A VOICE
  1. mkdir -p model/<lang>/<voice-quality>
  2. copy the .onnx and .onnx.json files
  3. (optional) create MODEL_CARD with metadata and presets
  4. run: gxx model list

MODEL_CARD PRESETS (optional)
  * Language: fa_IR (Persian, Iran)
  * Quality: medium
  * VowelStretch: 2
  * LengthScale: 1.10
  * NoiseScale: 0.80
  * NoiseW: 0.80

  These apply automatically unless overridden by CLI flags.

MODEL IDS
  Each model gets a stable ID derived from its filename stem.
  Moving the model directory does not change the ID.

COMMANDS
  gxx model list           # all discovered voices
  gxx model use <id>       # set active voice
  gxx model current        # show active voice id
  gxx model info <id>      # detailed info

  gxx paragraph --list-voices    # table with MODEL_CARD defaults

DETAILED GUIDE
  docs/models.md — model discovery, selection, MODEL_CARD, errors
  docs/tts.md    — TTS commands and synthesis options
"#;

const LANGUAGE: &str = r#"gxx TTS — Language handling

DETECTION
  gxx detects fa (Persian), en (English), other (Cyrillic, CJK, ...)
  via Unicode ranges.

LANGUAGE GUARD
  The active voice's language must match the input text. Otherwise
  gxx returns a clear error instead of garbled audio:

    $ gxx say hello                (with fa_IR active)
    error: text language 'en' does not match active voice 'fa'.
           switch model with `gxx model use <id>` or pass --phonemes

  Bypass the guard with --phonemes <IPA>.

AUTO MODE
  With --auto (or --voice auto), each paragraph is split by language
  and routed to a matching model:

    $ gxx paragraph --auto "hello سلام"
      → "hello" spoken with en_GB, "سلام" spoken with fa_IR

  Segments whose language has no model are skipped with a warning:

    TTS: warning: no model for 'other', skipping segment "привет"

  The active model is never changed by --auto or --voice.

MIXED TEXT IN NON-AUTO MODE
  Mixed text is spoken with the active voice. eSpeak language tags
  like (en) and (fa) are stripped automatically before Piper sees
  the IPA.

DETAILED GUIDE
  docs/tts.md — full TTS and language handling reference
  docs/models.md — model discovery and selection
"#;

const BOOKMARKS: &str = r#"gxx read — Bookmarks and resume

SAVE PROGRESS
  $ gxx read book.txt --save-progress

  After each paragraph, the next paragraph index (1-based) is saved
  to: ~/.local/share/gxx/progress/<hash>.txt

  The <hash> is derived from the file path, size, and mtime, so
  editing the file invalidates the bookmark.

RESUME
  $ gxx read book.txt --resume

  Starts at the saved paragraph. If the file was fully read:

    TTS: already finished (N of N paragraphs read)

  Use --clear-progress to reset.

CLEAR
  $ gxx read book.txt --clear-progress

INTERRUPT
  Press Ctrl+C during playback. The current paragraph finishes, then
  playback stops with:

    TTS: interrupted, stopping after current paragraph

COMBINING WITH OTHER FLAGS
  $ gxx read book.txt --auto --save-progress
  $ gxx read book.txt --auto --resume --sentence-split

DETAILED GUIDE
  docs/tts.md — full reading and progress reference
"#;

const EXAMPLES: &str = r#"gxx topics — End-to-end walkthroughs

SETUP
  gxx init
  gxx db new demo
  gxx add book کتاب -p n -l A1

SPEAK A NOTEBOOK WORD
  gxx say book
  gxx say book --example 1 --repeat 3 --slow
  gxx say book --phonemes "bʊk"          # skip eSpeak

FREE TEXT
  gxx paragraph "Hello world."
  gxx paragraph --voice fa "سلام دنیا"
  gxx paragraph --auto "hello سلام привет"
  gxx paragraph --sentence-split "One. Two. Three."
  gxx paragraph --vowel-stretch 2 --length-scale 1.15 "سلام"
  gxx paragraph --out greeting.wav "Hello"

NARRATE A FILE
  gxx read book.txt                      # paragraph-by-paragraph
  gxx read book.txt --auto               # per-language routing
  gxx read book.txt --out book.wav       # single WAV
  gxx read book.txt --save-progress      # bookmark as you go
  gxx read book.txt --resume             # continue later

DISCOVER VOICES
  gxx model list
  gxx paragraph --list-voices

FINE-TUNING
  gxx paragraph --length-scale 1.2 --noise-scale 0.9 "test"
  gxx paragraph --noise-w 1.2 "test"
  gxx paragraph --sentence-split --sentence-pause-ms 300 "One. Two."

DETAILED GUIDE
  docs/tutorials.md — complete end-to-end workflows
  docs/command-reference.md — full command and option index
"#;
