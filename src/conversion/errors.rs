use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("image codec error: {0}")]
    Image(#[from] image::ImageError),
    #[error("conversion was cancelled")]
    Cancelled,
    #[error("invalid configured output directory: {0}")]
    InvalidOutputDirectory(String),
    #[error("HEIC/HEIF decoder error: {0}")]
    Heif(String),
    #[error("WebP encoder error: {0}")]
    WebP(String),
    #[error("animated images are not supported")]
    AnimatedUnsupported,
    #[error("source and destination resolve to the same file")]
    WouldOverwriteSource,
}
