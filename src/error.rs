use thiserror::Error;

/// All errors that can occur during OCR operations.
#[derive(Debug, Error)]
pub enum OcrError {
    /// OCR model file was not found at the specified path.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Failed to load or initialize an ONNX model.
    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    /// ONNX inference encountered an error.
    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    /// The provided image data is invalid or cannot be decoded.
    #[error("Invalid image: {0}")]
    InvalidImage(String),

    /// The provided PDF document is invalid or cannot be parsed.
    #[error("Invalid PDF: {0}")]
    InvalidPdf(String),

    /// PDF rendering failed.
    #[error("PDF rendering failed: {0}")]
    PdfRenderingFailed(String),

    /// The file format is not supported.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Image dimensions exceed the configured maximum.
    #[error("Image too large: {0}")]
    ImageTooLarge(String),

    /// PDF contains too many pages.
    #[error("PDF too many pages: {0}")]
    PdfTooManyPages(String),

    /// Failed to download a required model or library.
    #[error("Download failed: {0}")]
    DownloadFailed(String),
}

/// Convenience type alias for results with [`OcrError`].
pub type OcrResult<T> = std::result::Result<T, OcrError>;
