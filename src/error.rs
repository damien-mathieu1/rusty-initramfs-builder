use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("Failed to parse image reference: {0}")]
    InvalidImageRef(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Platform not available: {0}")]
    PlatformNotAvailable(String),

    #[error("Layer extraction failed: {0}")]
    LayerExtraction(String),

    #[error("CPIO generation failed: {0}")]
    CpioGeneration(String),

    #[error("Compression failed: {0}")]
    Compression(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BuilderError>;
