//! TTS (Text-to-Speech) subsystem for gxx.
//!
//! This module provides British English text-to-speech using the Piper TTS model
//! (en_GB-cori-high) with eSpeak NG phonemization, ONNX inference, caching, and
//! playback with speed/volume control.

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
/// Uses the existing synthesis cache.
pub fn synthesize_text(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    get_or_synthesize(text, config)
}

/// Synthesize manually supplied IPA without playing it.
pub fn synthesize_text_manual(ipa: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    crate::tts::piper::synthesize_manual(ipa, config)
}

/// Synthesize and play text using the existing system-player backend.
/// Uses the existing synthesis cache (no re-synthesis of identical text).
///
/// If `stop_flag` is provided, it can be set to `true` from another thread
/// to immediately stop playback (for pause/resume support).
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
/// This is the main entry point for `gxx say` command.
pub fn say_with_active_model(
    text: &str,
    options: PlaybackOptions,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<Vec<f32>, GxxError> {
    let config = get_active_config()?;
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
