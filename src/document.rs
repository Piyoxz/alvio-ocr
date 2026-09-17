//! Document format detection via magic bytes.

use crate::error::OcrError;

/// Supported image types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Tiff,
}

/// Detected document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Pdf,
    Image(ImageType),
}

/// Detect document format from file header magic bytes.
pub fn detect_format(bytes: &[u8]) -> Result<DocumentFormat, OcrError> {
    if bytes.len() < 4 {
        return Err(OcrError::UnsupportedFormat(
            "File too small to inspect signature".to_string(),
        ));
    }

    if bytes.starts_with(b"%PDF-") || (bytes.len() >= 4 && &bytes[0..4] == b"%PDF") {
        return Ok(DocumentFormat::Pdf);
    }

    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Ok(DocumentFormat::Image(ImageType::Png));
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(DocumentFormat::Image(ImageType::Jpeg));
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(DocumentFormat::Image(ImageType::Webp));
    }

    if bytes.starts_with(b"BM") {
        return Ok(DocumentFormat::Image(ImageType::Bmp));
    }

    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return Ok(DocumentFormat::Image(ImageType::Tiff));
    }

    Err(OcrError::UnsupportedFormat(
        "Unsupported format. Expected PDF, JPEG, PNG, WebP, BMP, or TIFF.".to_string(),
    ))
}
