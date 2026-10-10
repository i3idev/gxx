//! Audio cache management.

use dirs;
use std::path::PathBuf;

use crate::error::GxxError;
use crate::tts::model::PiperConfig;

/// Get the cache directory for TTS audio.
/// Uses ~/.local/share/gxx/audio/ on Linux, equivalent on other platforms.
pub fn cache_dir() -> Result<PathBuf, GxxError> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| GxxError::msg("Could not determine local data directory"))?;
    let cache_dir = base.join("gxx").join("audio");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Generate a cache key for the given text and model config.
/// The key includes:
/// - Text hash (for exact text matching)
/// - Model identity (voice name, version)
/// - Key inference parameters that affect synthesis
///
/// Does NOT include playback parameters (repeat, speed, volume).
pub fn cache_key(text: &str, config: &PiperConfig) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    config.piper_version.hash(&mut hasher);
    config.audio.sample_rate.hash(&mut hasher);
    config.inference.noise_scale.to_bits().hash(&mut hasher);
    config.inference.length_scale.to_bits().hash(&mut hasher);
    config.inference.noise_w.to_bits().hash(&mut hasher);
    // Include phoneme_id_map hash for robustness
    let mut phoneme_keys: Vec<_> = config.phoneme_id_map.keys().collect();
    phoneme_keys.sort();
    for k in phoneme_keys {
        k.hash(&mut hasher);
        config.phoneme_id_map[k].hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

/// Get the cache file path for a given text and model.
/// Creates subdirectory structure: cache_dir/<key>/audio.wav
pub fn cache_path(text: &str, config: &PiperConfig) -> Result<PathBuf, GxxError> {
    let key = cache_key(text, config);
    let dir = cache_dir()?.join(&key);
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("audio.wav"))
}

/// Check if cached audio exists and is valid.
/// Read cached audio samples from WAV file.
pub fn read_cached_audio(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let path = cache_path(text, config)?;
    let mut reader = hound::WavReader::open(&path)?;
    let spec = reader.spec();

    if spec.channels != 1 {
        return Err(GxxError::msg("Cached audio must be mono"));
    }

    let samples: Result<Vec<f32>, _> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect();

    Ok(samples?)
}

/// Write audio samples to cache.
pub fn write_cache(text: &str, config: &PiperConfig, samples: &[f32]) -> Result<(), GxxError> {
    let path = cache_path(text, config)?;
    let wav_bytes = crate::tts::audio::write_wav(samples, config.audio.sample_rate)?;
    std::fs::write(&path, wav_bytes)?;
    Ok(())
}

/// Get or synthesize audio with caching.
/// If cached, returns cached audio. Otherwise synthesizes, caches, and returns.
pub fn get_or_synthesize(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let path = cache_path(text, config)?;

    eprintln!("TTS: cache path = {}", path.display());

    if path.exists() {
        eprintln!("TTS: cache hit for '{}'", text);
        read_cached_audio(text, config)
    } else {
        eprintln!("TTS: cache miss for '{}', synthesizing...", text);
        let audio = crate::tts::piper::synthesize(text, config)?;
        write_cache(text, config, &audio)?;
        Ok(audio)
    }
}
