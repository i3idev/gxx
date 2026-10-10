//! Piper ONNX inference for TTS synthesis.
//!
//! ONNX Runtime is statically linked into the binary via `ort`'s default
//! `download-binaries` feature (the `load-dynamic` feature is intentionally
//! disabled). At runtime `ort` resolves the API through the statically-linked
//! `OrtGetApiBase` symbol, so no external `libonnxruntime.so` is required.

use once_cell::sync::OnceCell;
use ort::session::Session;
use std::sync::{Arc, Mutex};

use crate::error::GxxError;
use crate::tts::model::PiperConfig;
use crate::tts::phonemizer::{phonemize, phonemize_manual};

static PIPER_SESSION: OnceCell<Arc<Mutex<Session>>> = OnceCell::new();

/// Initialize the ONNX Runtime environment (one-time, global).
///
/// With `load-dynamic` disabled this does not load any shared library; it only
/// prepares the global `Environment` used by sessions. Any failure here is a
/// real configuration/runtime error and is propagated as a `GxxError` instead of
/// panicking.
fn init_ort_environment() -> Result<(), GxxError> {
    ort::init().with_name("gxx-tts").commit();
    Ok(())
}

/// Initialize the ONNX Runtime session for the Piper model (one-time, global).
fn init_ort_session() -> Result<Arc<Mutex<Session>>, GxxError> {
    // Ensure the environment is initialized before creating a session.
    init_ort_environment()?;

    let model_path = crate::tts::model::model_onnx_path()
        .map_err(|e| GxxError::msg(format!("Failed to get model path: {}", e)))?;

    eprintln!("TTS: ONNX model = {}", model_path.display());

    let session = PIPER_SESSION.get_or_try_init(|| {
        let mut builder = Session::builder()
            .map_err(|e| GxxError::msg(format!("Failed to create session builder: {}", e)))?;
        let session = builder
            .commit_from_file(&model_path)
            .map_err(|e| GxxError::msg(format!("Failed to load ONNX model: {}", e)))?;
        Ok::<_, GxxError>(Arc::new(Mutex::new(session)))
    })?;

    Ok(Arc::clone(session))
}

/// Synthesize audio from text using the Piper ONNX model.
/// Returns audio samples as f32 (mono, 22050 Hz).
pub fn synthesize(text: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let phoneme_ids = phonemize(text, &config.espeak.voice, config)?;
    synthesize_ids(&phoneme_ids, config)
}

/// Synthesize directly from manually supplied IPA.
///
/// eSpeak is bypassed completely.
pub fn synthesize_manual(ipa: &str, config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    eprintln!("TTS: manual IPA = {:?}", ipa);
    let phoneme_ids = phonemize_manual(ipa, &config.phoneme_id_map)?;
    eprintln!("TTS: manual phoneme IDs = {:?}", phoneme_ids);
    synthesize_ids(&phoneme_ids, config)
}

fn synthesize_ids(phoneme_ids: &[u32], config: &PiperConfig) -> Result<Vec<f32>, GxxError> {
    let session_mutex = init_ort_session()?;
    let mut session = session_mutex
        .lock()
        .map_err(|e| GxxError::msg(format!("TTS session mutex poisoned: {}", e)))?;

    // Prepare input tensors
    // Piper expects: input (int64, shape [1, seq_len]), input_lengths (int64, shape [1]),
    // scales (float32, shape [3])
    let seq_len = phoneme_ids.len();

    // ONNX input tensor uses int64.
    let input_ids: Vec<i64> = phoneme_ids.iter().map(|&x| x as i64).collect();

    // Scales: [noise_scale, length_scale, noise_w]
    let scales = vec![
        config.inference.noise_scale,
        config.inference.length_scale,
        config.inference.noise_w,
    ];

    // Create input tensors
    let input_tensor = ort::value::Tensor::from_array(
        ndarray::Array::from_shape_vec((1, seq_len), input_ids)?.into_dyn(),
    )?;

    let input_lengths_tensor = ort::value::Tensor::from_array(
        ndarray::Array::from_shape_vec((1,), vec![seq_len as i64])?.into_dyn(),
    )?;

    let scales_tensor =
        ort::value::Tensor::from_array(ndarray::Array::from_shape_vec((3,), scales)?.into_dyn())?;

    // Run inference
    let outputs = session.run(ort::inputs![
        "input" => input_tensor,
        "input_lengths" => input_lengths_tensor,
        "scales" => scales_tensor,
    ])?;

    // Extract audio output
    let audio_output = outputs
        .get("output")
        .ok_or_else(|| GxxError::msg("No audio output found in model results"))?;

    // Extract tensor directly from the output value
    // try_extract_tensor returns (&Shape, &[f32])
    let (_, audio_data) = audio_output.try_extract_tensor::<f32>()?;
    let audio_samples: Vec<f32> = audio_data.to_vec();

    eprintln!("TTS: output samples = {}", audio_samples.len());

    if !audio_samples.is_empty() {
        let min = audio_samples.iter().copied().fold(f32::INFINITY, f32::min);

        let max = audio_samples
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        let rms = (audio_samples
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            / audio_samples.len() as f64)
            .sqrt();

        let duration = audio_samples.len() as f64 / config.audio.sample_rate as f64;

        eprintln!("TTS: output min = {min}");
        eprintln!("TTS: output max = {max}");
        eprintln!("TTS: output RMS = {rms}");
        eprintln!("TTS: output duration = {duration:.3}s");
    }

    Ok(audio_samples)
}
