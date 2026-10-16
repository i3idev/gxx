//! Piper TTS inference backed by ONNX Runtime.
//!
//! The ONNX Runtime binaries are provided through `ort`'s
//! `download-binaries` feature, with dynamic loading disabled. The
//! runtime API is resolved through the linked `OrtGetApiBase` symbol,
//! so inference should not require an external `libonnxruntime.so` at
//! runtime.
//!
//! A single session is shared across synthesis requests. Access is
//! serialized through a mutex because a session is mutable during
//! inference and must not be accessed concurrently without
//! synchronization.

use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::GxxError;
use crate::tts::model::PiperConfig;
use crate::tts::phonemizer::{phonemize, phonemize_manual};

/// Cached ONNX session keyed by its model path.
type CachedSession = (PathBuf, Arc<Mutex<Session>>);

/// Process-wide ONNX session shared by all synthesis requests.
///
/// `OnceCell` ensures that the session is initialized lazily and
/// retained for subsequent calls. `Arc` allows callers to share
/// ownership, while `Mutex` serializes access to the mutable session.
static PIPER_SESSION: OnceCell<Mutex<Option<CachedSession>>> = OnceCell::new();

/// Register the ONNX Runtime environment configuration.
///
/// Called before session creation to establish the runtime environment
/// under the application-specific name `gxx-tts`.
///
/// # Errors
///
/// Returns an error if environment initialization fails.
fn init_ort_environment() -> Result<(), GxxError> {
    ort::init().with_name("gxx-tts").commit();
    Ok(())
}

/// Lazily initialize and retrieve the shared Piper inference session.
///
/// The model path is resolved by the model subsystem, keeping
/// filesystem and model-location policy outside the inference
/// implementation. The session is created only when the first
/// synthesis request is processed.
///
/// # Errors
///
/// Returns an error if the model path cannot be resolved, the session
/// builder cannot be created, or the ONNX model cannot be loaded.
fn init_ort_session(config_onnx_path: Option<&Path>) -> Result<Arc<Mutex<Session>>, GxxError> {
    init_ort_environment()?;

    let model_path = match config_onnx_path {
        Some(p) => p.to_path_buf(),
        None => crate::tts::registry::get_active_onnx_path()?,
    };
    let model_path = model_path.canonicalize().unwrap_or(model_path);

    let cache = PIPER_SESSION.get_or_init(|| Mutex::new(None));
    let mut cached = cache
        .lock()
        .map_err(|_| GxxError::msg("TTS session cache lock was poisoned"))?;

    if let Some((path, session)) = cached.as_ref() {
        if path == &model_path {
            return Ok(Arc::clone(session));
        }
    }

    eprintln!("TTS: loading ONNX model = {}", model_path.display());

    let session = Session::builder()
        .map_err(|e| GxxError::msg(format!("Failed to create session builder: {}", e)))?
        .commit_from_file(&model_path)
        .map_err(|e| GxxError::msg(format!("Failed to load ONNX model: {}", e)))?;

    let session = Arc::new(Mutex::new(session));
    *cached = Some((model_path, Arc::clone(&session)));
    Ok(session)
}

/// Convert text into speech using the configured Piper ONNX model.
///
/// Text is first converted into phoneme IDs by the phonemizer, then
/// passed to the inference pipeline. The returned samples represent
/// mono audio at the sample rate specified by
/// `config.audio.sample_rate`.
///
/// # Errors
///
/// Returns an error if phonemization, tensor construction, ONNX
/// inference, or output extraction fails.
pub fn synthesize(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let phoneme_ids = phonemize(text, &config.espeak.voice, config)?;
    synthesize_ids(&phoneme_ids, config)
}

/// Synthesize speech from manually supplied IPA.
///
/// This entry point bypasses eSpeak and maps the supplied phonetic
/// representation directly to Piper's model-specific phoneme IDs.
///
/// The IPA input must be compatible with the phoneme mapping
/// configured for the selected model.
///
/// # Errors
///
/// Returns an error if IPA conversion, tensor construction, ONNX
/// inference, or output extraction fails.
pub fn synthesize_manual(ipa: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    eprintln!("TTS: manual IPA = {:?}", ipa);

    let phoneme_ids = phonemize_manual(ipa, &config.phoneme_id_map)?;

    eprintln!("TTS: manual phoneme IDs = {:?}", phoneme_ids);

    synthesize_ids(&phoneme_ids, config)
}

/// Run Piper inference for a sequence of model-specific phoneme IDs.
///
/// This function owns the tensor preparation, model execution, and extraction
/// of generated audio samples. It expects the model to expose the named inputs
/// `input`, `input_lengths`, and `scales`, and an output named `output`.
///
/// # Input contract
///
/// - `phoneme_ids`: a sequence of phoneme IDs accepted by the loaded model.
/// - `config`: model inference parameters and audio configuration.
///
/// # Errors
///
/// Returns an error if session initialization or locking fails, input tensors
/// cannot be constructed, inference fails, or the output tensor cannot be
/// retrieved or decoded.
fn synthesize_ids(phoneme_ids: &[u32], config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let session_mutex = init_ort_session(config.onnx_path.as_deref())?;

    // A session requires exclusive mutable access during inference. Keep the
    // critical section limited to session execution and output extraction.
    let mut session = session_mutex
        .lock()
        .map_err(|e| GxxError::msg(format!("TTS session mutex poisoned: {}", e)))?;

    // Piper expects three inputs:
    // `input`:         int64 [1, sequence_length]
    // `input_lengths`: int64 [1]
    // `scales`:        float32 [3] = [noise_scale, length_scale, noise_w]
    let seq_len = phoneme_ids.len();

    // ONNX model inputs use signed 64-bit integers, whereas the phonemizer
    // exposes IDs as u32. Convert them to the model's expected element type.
    let input_ids: Vec<i64> = phoneme_ids.iter().map(|&x| x as i64).collect();

    // Control generation variability, speaking rate, and noise contribution
    // using the inference parameters associated with the selected model.
    let scales = vec![
        config.inference.noise_scale,
        config.inference.length_scale,
        config.inference.noise_w,
    ];

    // Build tensors with explicit shapes matching the model's input contract.
    let input_tensor = ort::value::Tensor::from_array(
        ndarray::Array::from_shape_vec((1, seq_len), input_ids)?.into_dyn(),
    )?;

    let input_lengths_tensor = ort::value::Tensor::from_array(
        ndarray::Array::from_shape_vec((1,), vec![seq_len as i64])?.into_dyn(),
    )?;

    let scales_tensor =
        ort::value::Tensor::from_array(ndarray::Array::from_shape_vec((3,), scales)?.into_dyn())?;

    // Execute the model with its expected input names. ONNX Runtime returns
    // the output collection, from which the generated waveform is extracted.
    let outputs = session.run(ort::inputs![
        "input" => input_tensor,
        "input_lengths" => input_lengths_tensor,
        "scales" => scales_tensor,
    ])?;

    // Resolve the named waveform output explicitly so that an incompatible
    // model signature produces a descriptive error instead of an unwrap panic.
    let audio_output = outputs
        .get("output")
        .ok_or_else(|| GxxError::msg("No audio output found in model results"))?;

    // Extract the contiguous f32 waveform from the ONNX output tensor.
    let (_, audio_data) = audio_output.try_extract_tensor::<f32>()?;
    let audio_samples: Vec<f32> = audio_data.to_vec();

    eprintln!("TTS: output samples = {}", audio_samples.len());

    // Emit diagnostic statistics only when the model produced audio. These
    // values help identify silent output, clipping, and unexpected durations.
    if !audio_samples.is_empty() {
        let min = audio_samples.iter().copied().fold(f32::INFINITY, f32::min);

        let max = audio_samples
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        // Calculate RMS in f64 to reduce accumulation error when processing
        // longer waveforms containing many floating-point samples.
        let rms = (audio_samples
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            / audio_samples.len() as f64)
            .sqrt();

        // Duration is derived from the configured sample rate; it assumes
        // that the model output uses this rate without additional resampling.
        let duration = audio_samples.len() as f64 / config.audio.sample_rate as f64;

        eprintln!("TTS: output min = {min}");
        eprintln!("TTS: output max = {max}");
        eprintln!("TTS: output RMS = {rms}");
        eprintln!("TTS: output duration = {duration:.3}s");
    }

    Ok(audio_samples)
}
