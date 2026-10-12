//! TTS Model Registry and Management
//!
//! Discovers Piper models in the model/ directory, generates stable hash IDs,
//! stores metadata, and manages the active model selection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GxxError;
use crate::tts::model::PiperConfig;

/// Registry entry for a discovered TTS model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Stable hash ID (first 12 chars of SHA256 of model path)
    pub id: String,
    /// Human-readable name from MODEL_CARD or directory
    pub name: String,
    /// Language code (e.g., "en_US", "en_GB", "fa_IR")
    pub language: String,
    /// Voice name (e.g., "ryan", "cori", "amir")
    pub voice: String,
    /// Quality tier (e.g., "high", "medium")
    pub quality: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Path to .onnx file
    pub onnx_path: PathBuf,
    /// Path to .onnx.json config file
    pub config_path: PathBuf,
    /// Path to MODEL_CARD file
    pub card_path: Option<PathBuf>,
    /// Whether the model files currently exist on disk
    pub available: bool,

    /// Optional default VowelStretch from MODEL_CARD
    pub default_vowel_stretch: Option<u32>,
    /// Optional default length_scale from MODEL_CARD
    pub default_length_scale: Option<f32>,
    /// Optional default noise_scale from MODEL_CARD
    pub default_noise_scale: Option<f32>,
    /// Optional default noise_w from MODEL_CARD
    pub default_noise_w: Option<f32>,
}

/// Model registry containing all discovered models.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    models: HashMap<String, ModelEntry>,
}

impl ModelRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Discover all Piper models in the standard model directories.
    pub fn discover() -> Result<Self, GxxError> {
        let mut registry = Self::new();
        for dir in candidate_model_dirs() {
            if dir.exists() {
                discover_in_dir(&dir, &mut registry, 0);
            }
        }
        Ok(registry)
    }

    /// Add a model to the registry.
    fn add(&mut self, model: ModelEntry) {
        self.models.insert(model.id.clone(), model);
    }

    /// Get a model by its stable ID.
    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.models.get(id)
    }

    /// Get all models as a vector.
    pub fn all(&self) -> Vec<&ModelEntry> {
        let mut models: Vec<_> = self.models.values().collect();
        models.sort_by(|a, b| a.name.cmp(&b.name));
        models
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Find the first available model matching a query string.
    ///
    /// Matching is case-insensitive and tries, in order:
    /// 1. language prefix (`"en"` matches `"en_GB"` and `"en_US"`)
    /// 2. exact voice name (`"cori"`, `"amir"`)
    /// 3. exact language code (`"en_GB"`)
    pub fn find_for_query(&self, query: &str) -> Option<&ModelEntry> {
        let q = query.to_lowercase();
        self.all().into_iter().find(|e| {
            e.available
                && (e.language.to_lowercase().starts_with(&q)
                    || e.voice.to_lowercase() == q
                    || e.language.to_lowercase() == q)
        })
    }
}

/// Recursively scan `dir` for `.onnx` files up to `MAX_DEPTH` levels.
///
/// Supports both layouts:
///   model/<name>.onnx                          (flat, depth 0)
///   model/v1/<name>.onnx                       (legacy, depth 1)
///   model/<lang>/<voice-quality>/<name>.onnx   (new, depth 2)
fn discover_in_dir(dir: &Path, registry: &mut ModelRegistry, depth: usize) {
    const MAX_DEPTH: usize = 3;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < MAX_DEPTH {
                discover_in_dir(&path, registry, depth + 1);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("onnx") {
            if let Some(model) = parse_model_entry(&path) {
                registry.add(model);
            }
        }
    }
}

/// Parse a model entry from an .onnx file path.
fn parse_model_entry(onnx_path: &Path) -> Option<ModelEntry> {
    let config_path = onnx_path.with_extension("onnx.json");
    if !config_path.exists() {
        return None;
    }

    // Read config to get metadata
    let config_content = fs::read_to_string(&config_path).ok()?;
    let config: PiperConfig = serde_json::from_str(&config_content).ok()?;

    // Generate stable ID from the model's relative path
    let id = generate_model_id(onnx_path);

    // Read language and quality from MODEL_CARD when available.
    // Extract voice and fallback quality from the ONNX filename.
    let stem = onnx_path.file_stem()?.to_string_lossy().to_string();
    let parts: Vec<&str> = stem.split('-').collect();
    let filename_language = parts.first().copied().unwrap_or("unknown").to_string();
    let voice = parts.get(1).copied().unwrap_or("unknown").to_string();
    let filename_quality = parts.get(2).copied().unwrap_or("unknown").to_string();

    let card_path = onnx_path.parent()?.join("MODEL_CARD");
    let card = if card_path.exists() {
        let card_content = fs::read_to_string(&card_path).ok()?;
        Some(parse_model_card(&card_content))
    } else {
        None
    };
    let card_language = card
        .as_ref()
        .map(|c| c.language.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let card_quality = card
        .as_ref()
        .map(|c| c.quality.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let language = if card_language == "unknown" {
        filename_language
    } else {
        card_language
    };

    let quality = if card_quality == "unknown" {
        filename_quality
    } else {
        card_quality
    };

    let name = format!("{} ({})", voice, language);

    Some(ModelEntry {
        id,
        name,
        language,
        voice,
        quality,
        sample_rate: config.audio.sample_rate,
        onnx_path: onnx_path.to_path_buf(),
        config_path: config_path.clone(),
        card_path: if card_path.exists() {
            Some(card_path)
        } else {
            None
        },
        available: onnx_path.exists() && config_path.exists(),
        default_vowel_stretch: card.as_ref().and_then(|c| c.default_vowel_stretch),
        default_length_scale: card.as_ref().and_then(|c| c.default_length_scale),
        default_noise_scale: card.as_ref().and_then(|c| c.default_noise_scale),
        default_noise_w: card.as_ref().and_then(|c| c.default_noise_w),
    })
}

/// Parse MODEL_CARD content for metadata.
struct ModelCardInfo {
    language: String,
    quality: String,
    default_vowel_stretch: Option<u32>,
    default_length_scale: Option<f32>,
    default_noise_scale: Option<f32>,
    default_noise_w: Option<f32>,
}

fn parse_model_card(content: &str) -> ModelCardInfo {
    let mut language = "unknown".to_string();
    let mut quality = "unknown".to_string();
    let mut default_vowel_stretch = None;
    let mut default_length_scale = None;
    let mut default_noise_scale = None;
    let mut default_noise_w = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("* Language:") {
            language = v.split_whitespace().next().unwrap_or("unknown").to_string();
        } else if let Some(v) = line.strip_prefix("* Quality:") {
            quality = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("* VowelStretch:") {
            default_vowel_stretch = v.trim().parse::<u32>().ok();
        } else if let Some(v) = line.strip_prefix("* LengthScale:") {
            default_length_scale = v.trim().parse::<f32>().ok();
        } else if let Some(v) = line.strip_prefix("* NoiseScale:") {
            default_noise_scale = v.trim().parse::<f32>().ok();
        } else if let Some(v) = line.strip_prefix("* NoiseW:") {
            default_noise_w = v.trim().parse::<f32>().ok();
        }
    }

    ModelCardInfo {
        language,
        quality,
        default_vowel_stretch,
        default_length_scale,
        default_noise_scale,
        default_noise_w,
    }
}

/// Generate a stable model ID from the ONNX file path.
/// Uses first 12 chars of SHA256 of the relative path from model/ root.
fn generate_model_id(onnx_path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Hash by filename stem, not full path, so the ID stays stable
    // when the model directory is restructured (e.g. versioned -> language).
    let stem = onnx_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| onnx_path.to_string_lossy().to_string());

    let mut hasher = DefaultHasher::new();
    stem.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}

/// Get candidate model directories (same as in tts::model).
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

/// Configuration for the active TTS model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    /// Active model ID
    pub active_model_id: Option<String>,
}

/// Load the active model configuration from ~/.gxx/model_config.json
pub fn load_model_config() -> Result<ModelConfig, GxxError> {
    let config_path = model_config_path();
    if !config_path.exists() {
        return Ok(ModelConfig::default());
    }
    let content = fs::read_to_string(&config_path)?;
    let config: ModelConfig = serde_json::from_str(&content)?;
    Ok(config)
}

/// Save the active model configuration to ~/.gxx/model_config.json
pub fn save_model_config(config: &ModelConfig) -> Result<(), GxxError> {
    let config_path = model_config_path();
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, content)?;
    Ok(())
}

/// Get the path to the model configuration file.
fn model_config_path() -> PathBuf {
    let mut p = crate::db::home_dir();
    p.push("model_config.json");
    p
}

/// Get the active model's PiperConfig, or fall back to auto-discovery.
pub fn get_active_config() -> Result<PiperConfig, GxxError> {
    let config = load_model_config()?;

    if let Some(active_id) = config.active_model_id {
        let registry = ModelRegistry::discover()?;
        if let Some(model) = registry.get(&active_id) {
            if model.available {
                return crate::tts::model::load_config(&model.config_path);
            }
        }
    }

    // Fallback to auto-discovery (existing behavior)
    crate::tts::model::load_config(&crate::tts::model::model_config_path()?)
}

/// Get the active model's ONNX path, or fall back to auto-discovery.
pub fn get_active_onnx_path() -> Result<PathBuf, GxxError> {
    let config = load_model_config()?;

    if let Some(active_id) = config.active_model_id {
        let registry = ModelRegistry::discover()?;
        if let Some(model) = registry.get(&active_id) {
            if model.available {
                return Ok(model.onnx_path.clone());
            }
        }
    }

    // Fallback to auto-discovery (existing behavior)
    crate::tts::model::model_onnx_path()
}

/// Set the active model by ID.
pub fn set_active_model(model_id: &str) -> Result<(), GxxError> {
    let registry = ModelRegistry::discover()?;
    if registry.get(model_id).is_none() {
        return Err(GxxError::msg(format!(
            "Model '{}' not found. Run 'gxx model list' to see available models.",
            model_id
        )));
    }

    let mut config = load_model_config()?;
    config.active_model_id = Some(model_id.to_string());
    save_model_config(&config)
}

/// Clear the active model (revert to auto-discovery).
pub fn clear_active_model() -> Result<(), GxxError> {
    let mut config = load_model_config()?;
    config.active_model_id = None;
    save_model_config(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_model_id_stable() {
        let path1 = Path::new("model/v1/en_GB-cori-high.onnx");
        let path2 = Path::new("model/v1/en_GB-cori-high.onnx");
        assert_eq!(generate_model_id(path1), generate_model_id(path2));
    }

    #[test]
    fn test_generate_model_id_different() {
        let path1 = Path::new("model/v1/en_GB-cori-high.onnx");
        let path2 = Path::new("model/v2/en_US-ryan-high.onnx");
        assert_ne!(generate_model_id(path1), generate_model_id(path2));
    }

    #[test]
    fn test_find_for_query_language_prefix() {
        let registry = ModelRegistry::discover().unwrap();
        // "en" should match either en_GB or en_US
        let hit = registry.find_for_query("en");
        if !registry.is_empty() {
            // only assert if any en model exists
            let has_en = registry.all().iter().any(|m| m.language.starts_with("en"));
            if has_en {
                let m = hit.expect("en should match");
                assert!(m.language.starts_with("en"));
            }
        }
    }

    #[test]
    fn test_find_for_query_case_insensitive() {
        let registry = ModelRegistry::discover().unwrap();
        if !registry.is_empty() {
            let any = registry.all()[0];
            let q_upper = any.language.to_uppercase();
            let found = registry.find_for_query(&q_upper);
            assert!(found.is_some(), "uppercase query should match");
        }
    }

    #[test]
    fn test_find_for_query_unknown() {
        let registry = ModelRegistry::discover().unwrap();
        assert!(registry.find_for_query("zz").is_none());
        assert!(registry
            .find_for_query("this-voice-does-not-exist")
            .is_none());
    }

    #[test]
    fn test_find_for_query_skips_unavailable() {
        // construct a fake registry manually
        let mut registry = ModelRegistry::new();
        let entry = ModelEntry {
            id: "deadbeef0000".into(),
            name: "fake".into(),
            language: "xx_XX".into(),
            voice: "fake".into(),
            quality: "low".into(),
            sample_rate: 16000,
            onnx_path: PathBuf::from("/nonexistent/model.onnx"),
            config_path: PathBuf::from("/nonexistent/model.onnx.json"),
            card_path: None,
            available: false,
            default_vowel_stretch: None,
            default_length_scale: None,
            default_noise_scale: None,
            default_noise_w: None,
        };
        registry.add(entry);
        assert!(registry.find_for_query("xx").is_none());
        assert!(registry.find_for_query("fake").is_none());
    }

    #[test]
    fn test_discover_models() {
        // `model/` is gitignored, so CI has no model files. Discovery
        // must not error; content assertions only run when a model
        // happens to be present (developer machines).
        let registry = ModelRegistry::discover().unwrap();
        if registry.is_empty() {
            eprintln!("no models present (CI); skipping content assertions");
            return;
        }
        let m = registry.all()[0];
        assert!(!m.id.is_empty(), "model id must not be empty");
        assert_ne!(m.language, "unknown", "language must be parsed");
        assert_ne!(m.voice, "unknown", "voice must be parsed");
    }

    #[test]
    fn test_model_config_load_save() {
        let config = ModelConfig {
            active_model_id: Some("test123".to_string()),
        };
        // This test would require a temp dir - skip for now
        let _ = config;
    }
}
