//! Shared error type for `gxx`.
//!
//! [`GxxError`] is a single enum covering all failure modes: I/O,
//! SQLite, JSON, WAV, ONNX Runtime, shape, resample, buffer, and
//! anyhow. It is the return type of every command handler and the
//! TTS pipeline.
//!
//! In `main`, a [`GxxError`] is printed as `error: <message>` and the
//! process exits with code 1.

use thiserror::Error;

/// Unified error type for `gxx`.
///
/// Each variant wraps a human-readable message. `From` impls convert
/// common library errors (SQLite, JSON, WAV, ONNX, ndarray, rubato,
/// anyhow) into the appropriate variant.
#[derive(Debug, Error)]
pub enum GxxError {
    /// Generic message error.
    #[error("{0}")]
    Msg(String),
    /// I/O error.
    #[error("io: {0}")]
    Io(String),
    /// SQLite error.
    #[error("sqlite: {0}")]
    Sqlite(String),
    /// JSON serialisation / deserialisation error.
    #[error("json: {0}")]
    Json(String),
    /// WAV file error.
    #[error("wav: {0}")]
    Wav(String),
    /// ONNX Runtime error.
    #[error("onnx: {0}")]
    Onnx(String),
    /// Array shape error.
    #[error("shape: {0}")]
    Shape(String),
    /// Resample error.
    #[error("resample: {0}")]
    Resample(String),
    /// Resampler construction error.
    #[error("resample construction: {0}")]
    ResampleConstruction(String),
    /// Buffer size error.
    #[error("buffer size: {0}")]
    BufferSize(String),
    /// Anyhow error.
    #[error("anyhow: {0}")]
    Anyhow(String),
}

impl GxxError {
    /// Construct a generic message error.
    pub fn msg<S: Into<String>>(s: S) -> Self {
        GxxError::Msg(s.into())
    }
}

impl From<std::io::Error> for GxxError {
    fn from(e: std::io::Error) -> Self {
        GxxError::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for GxxError {
    fn from(e: rusqlite::Error) -> Self {
        GxxError::Sqlite(e.to_string())
    }
}

impl From<serde_json::Error> for GxxError {
    fn from(e: serde_json::Error) -> Self {
        GxxError::Json(e.to_string())
    }
}

impl From<hound::Error> for GxxError {
    fn from(e: hound::Error) -> Self {
        GxxError::Wav(e.to_string())
    }
}

impl From<ort::Error> for GxxError {
    fn from(e: ort::Error) -> Self {
        GxxError::Onnx(e.to_string())
    }
}

impl From<ndarray::ShapeError> for GxxError {
    fn from(e: ndarray::ShapeError) -> Self {
        GxxError::Shape(e.to_string())
    }
}

impl From<rubato::ResampleError> for GxxError {
    fn from(e: rubato::ResampleError) -> Self {
        GxxError::Resample(e.to_string())
    }
}

impl From<rubato::ResamplerConstructionError> for GxxError {
    fn from(e: rubato::ResamplerConstructionError) -> Self {
        GxxError::ResampleConstruction(e.to_string())
    }
}

impl From<rubato::audioadapter_buffers::SizeError> for GxxError {
    fn from(e: rubato::audioadapter_buffers::SizeError) -> Self {
        GxxError::BufferSize(e.to_string())
    }
}

impl From<anyhow::Error> for GxxError {
    fn from(e: anyhow::Error) -> Self {
        GxxError::Anyhow(e.to_string())
    }
}
