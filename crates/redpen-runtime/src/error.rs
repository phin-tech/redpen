use redpen_core::sidecar::SidecarError;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<SidecarError> for RuntimeError {
    fn from(err: SidecarError) -> Self {
        match err {
            SidecarError::Io(e) => RuntimeError::Io(e),
            SidecarError::Json(e) => RuntimeError::Json(e),
        }
    }
}
