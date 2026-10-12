#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::invalid_html_tags)]
#![allow(rustdoc::bare_urls)]

//! # gxx
//!
//! Bilingual (English / Persian) vocabulary and grammar notebook CLI with a
//! multi-voice Text-to-Speech subsystem.
//!
//! ## Overview
//!
//! `gxx` is a local, offline-first tool. Data lives in SQLite databases
//! under `~/.gxx/db/`, and TTS uses Piper voices discovered under
//! `model/<lang>/<voice-quality>/`.
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
//! Per-model defaults (VowelStretch, LengthScale, NoiseScale, NoiseW) may be
//! declared in each voice's `MODEL_CARD`.
//!
//! See the repository README for the full user guide.

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
