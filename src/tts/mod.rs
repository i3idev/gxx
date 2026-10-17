//! Text-to-Speech (TTS) subsystem for `gxx`.
//!
//! This module provides offline text-to-speech using Piper voices with
//! eSpeak NG phonemization, ONNX Runtime inference, synthesis caching,
//! and playback with speed/volume control.
//!
//! Pipeline:
//!
//! 1. Text → eSpeak NG → IPA (language tags stripped)
//! 2. IPA → Piper `phoneme_id_map` → phoneme IDs
//! 3. Phoneme IDs → ONNX Runtime → audio samples
//! 4. Samples → playback (system player) or WAV export
//!
//! Submodules:
//!
//! - [`model`] — Piper config and model resolution
//! - [`registry`] — voice discovery, selection, and active model
//! - [`phonemizer`] — eSpeak NG phonemization and language handling
//! - [`piper`] — ONNX inference
//! - [`cache`] — synthesis cache
//! - [`audio`] — WAV generation, time-stretching, volume
//! - [`playback`] — audio playback with speed/volume control

pub mod audio;
pub mod cache;
pub mod model;
pub mod phonemizer;
pub mod piper;
pub mod playback;
pub mod registry;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::error::GxxError;
use crate::tts::cache::get_or_synthesize;
use crate::tts::model::PiperConfig;
use crate::tts::playback::play_audio;
use crate::tts::registry::get_active_config;

/// Synthesize text and return audio samples without playing them.
///
/// Uses the existing synthesis cache (see [`cache`]) so repeated calls
/// with the same text and model config do not re-synthesize.
pub fn synthesize_text(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    get_or_synthesize(text, config)
}

/// Synthesize manually supplied IPA without playing it.
///
/// Bypasses eSpeak and the language guard; the caller controls the
/// exact IPA sequence.
pub fn synthesize_text_manual(ipa: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    crate::tts::piper::synthesize_manual(ipa, config)
}

/// Synthesize and play text using the existing system-player backend.
///
/// Uses the existing synthesis cache (no re-synthesis of identical
/// text).
///
/// If `stop_flag` is provided, it can be set to `true` from another
/// thread to immediately stop playback (for pause/resume support).
pub fn say_text(
    text: &str,
    config: &PiperConfig,
    options: PlaybackOptions,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<Vec<f32>, GxxError> {
    let audio = synthesize_text(text, config)?;
    play_audio(
        &audio,
        config.audio.sample_rate,
        options,
        stop_flag.as_ref(),
    )?;
    Ok(audio)
}

/// Synthesize and play using manually supplied IPA.
pub fn say_text_manual(
    ipa: &str,
    config: &PiperConfig,
    options: PlaybackOptions,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<Vec<f32>, GxxError> {
    let audio = synthesize_text_manual(ipa, config)?;
    play_audio(
        &audio,
        config.audio.sample_rate,
        options,
        stop_flag.as_ref(),
    )?;
    Ok(audio)
}

/// Synthesize and play using the currently active model.
///
/// This is the main entry point for `gxx say`.
pub fn say_with_active_model(
    text: &str,
    options: PlaybackOptions,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<Vec<f32>, GxxError> {
    let config = crate::tts::registry::get_active_config_for_text(text)?;
    say_text(text, &config, options, stop_flag)
}

/// Synthesize and play using manually supplied IPA with the active model.
pub fn say_manual_with_active_model(
    ipa: &str,
    options: PlaybackOptions,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<Vec<f32>, GxxError> {
    let config = get_active_config()?;
    say_text_manual(ipa, &config, options, stop_flag)
}

pub use crate::tts::playback::PlaybackOptions;
