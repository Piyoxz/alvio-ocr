use thiserror::Error;

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("Invalid image: {0}")]
    InvalidImage(String),

    #[error("Invalid PDF: {0}")]
    InvalidPdf(String),

    #[error("PDF rendering failed: {0}")]
    PdfRenderingFailed(String),

    #[error("Unsupported file format '{detected}'. Supported formats: {supported}")]
    UnsupportedFormat {
        detected: String,
        supported: String,
    },

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image too large: {0}")]
    ImageTooLarge(String),

    #[error("PDF too many pages: {0}")]
    PdfTooManyPages(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),
}

pub type Result<T> = std::result::Result<T, OcrError>;
