//! Error type for gxx.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GxxError {
    #[error("{0}")]
    Msg(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl GxxError {
    pub fn msg<S: Into<String>>(s: S) -> Self {
        GxxError::Msg(s.into())
    }
}
