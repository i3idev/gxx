//! Error type for gxx.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GxxError {
    #[error("{0}")]
    Msg(String),
    #[error("io: {0}")]
    Io(String),
    #[error("sqlite: {0}")]
    Sqlite(String),
    #[error("json: {0}")]
    Json(String),
    #[error("wav: {0}")]
    Wav(String),
    #[error("onnx: {0}")]
    Onnx(String),
    #[error("shape: {0}")]
    Shape(String),
    #[error("resample: {0}")]
    Resample(String),
    #[error("resample construction: {0}")]
    ResampleConstruction(String),
    #[error("buffer size: {0}")]
    BufferSize(String),
    #[error("anyhow: {0}")]
    Anyhow(String),
}

impl GxxError {
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
