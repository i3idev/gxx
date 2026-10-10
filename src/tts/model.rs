//! TTS model configuration and loading.

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error::GxxError;

/// Piper model configuration loaded from .onnx.json
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PiperConfig {
    pub audio: AudioConfig,
    pub inference: InferenceConfig,

    /// Present in newer Piper configs, absent in older configs such as Ryan.
    #[serde(default)]
    pub phoneme_type: Option<String>,

    /// Piper's phoneme -> ONNX input ID mapping.
    pub phoneme_id_map: std::collections::HashMap<String, Vec<u32>>,

    pub num_symbols: usize,
    pub num_speakers: usize,

    #[serde(default)]
    pub speaker_id_map: std::collections::HashMap<String, u32>,

    pub espeak: EspeakConfig,
    pub piper_version: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub quality: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InferenceConfig {
    pub noise_scale: f32,
    pub length_scale: f32,
    pub noise_w: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EspeakConfig {
    pub voice: String,
}

/// A resolved Piper model and its matching configuration.
#[derive(Debug, Clone)]
pub struct PiperModel {
    pub onnx_path: PathBuf,
    pub config_path: PathBuf,
}

/// Resolve the Ryan Piper model.
///
/// Model and config are deliberately resolved together so they can never
/// accidentally come from different model versions.
fn try_resolve_ryan(dir: &Path) -> Option<PiperModel> {
    let onnx_path = dir.join("v2/en_US-ryan-high.onnx");
    let config_path = dir.join("v2/en_US-ryan-high.onnx.json");

    if onnx_path.exists() && config_path.exists() {
        Some(PiperModel {
            onnx_path,
            config_path,
        })
    } else {
        None
    }
}

/// Resolve the Cori Piper model.
fn try_resolve_cori(dir: &Path) -> Option<PiperModel> {
    let onnx_path = dir.join("en_GB-cori-high.onnx");
    let config_path = dir.join("en_GB-cori-high.onnx.json");

    if onnx_path.exists() && config_path.exists() {
        Some(PiperModel {
            onnx_path,
            config_path,
        })
    } else {
        None
    }
}

/// Resolve the TTS model directory.
fn candidate_model_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Installed binary: <prefix>/bin/gxx -> <prefix>/bin/model
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            dirs.push(exe_dir.join("model"));
        }
    }

    // Development build.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest_dir).join("model"));
    }

    // Current working directory.
    dirs.push(PathBuf::from("model"));

    dirs
}

/// Resolve the Piper model.
///
/// Ryan is preferred because it is the explicit US English model.
/// Cori remains available as a fallback.
pub fn resolve_model() -> Result<PiperModel, GxxError> {
    for dir in candidate_model_dirs() {
        if let Some(model) = try_resolve_ryan(&dir) {
            return Ok(model);
        }

        if let Some(model) = try_resolve_cori(&dir) {
            return Ok(model);
        }
    }

    Err(GxxError::msg(
        "TTS model not found. Expected one of:\n\
         model/v2/en_US-ryan-high.onnx + .json\n\
         model/en_GB-cori-high.onnx + .json",
    ))
}

/// Load the selected Piper model configuration.
/// Load Piper model configuration from .onnx.json.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<PiperConfig, GxxError> {
    let content = std::fs::read_to_string(path)?;
    let config: PiperConfig = serde_json::from_str(&content)?;
    Ok(config)
}

/// Compatibility helper.
pub fn model_onnx_path() -> Result<PathBuf, GxxError> {
    Ok(resolve_model()?.onnx_path)
}

/// Compatibility helper.
pub fn model_config_path() -> Result<PathBuf, GxxError> {
    Ok(resolve_model()?.config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_finds_cori() {
        // The model directory ships with en_GB-cori-high.onnx + .json
        let result = resolve_model();
        assert!(result.is_ok(), "Should find a Piper model in the project");
        let model = result.unwrap();
        assert!(model.onnx_path.exists(), "ONNX file should exist");
        assert!(model.config_path.exists(), "Config file should exist");
        assert!(model.onnx_path.to_string_lossy().ends_with(".onnx"));
        assert!(model.config_path.to_string_lossy().ends_with(".onnx.json"));
    }

    #[test]
    fn test_load_config_valid() {
        let config_path = model_config_path().expect("model config should be found");
        let config = load_config(&config_path).expect("config should load");
        assert_ne!(config.audio.sample_rate, 0, "sample rate should be set");
        assert!(!config.espeak.voice.is_empty(), "voice should be set");
    }

    #[test]
    fn test_resolve_model_missing_dir() {
        // When no model dirs are accessible, resolve_model should error
        // (We test the error path by checking a temp dir without models)
        let dir = tempfile::tempdir().unwrap();
        let model = try_resolve_ryan(dir.path());
        assert!(model.is_none(), "No Ryan model in empty dir");
        let model = try_resolve_cori(dir.path());
        assert!(model.is_none(), "No Cori model in empty dir");
    }

    #[test]
    fn test_candidate_model_dirs_not_empty() {
        let dirs = candidate_model_dirs();
        assert!(!dirs.is_empty(), "Should have at least one candidate dir");
    }
}
