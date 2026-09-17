use crate::{error::OcrError, types::DocumentFormat};
use std::path::Path;

pub const SUPPORTED_FORMATS: &[&str] = &[
    "jpeg", "jpg", "png", "webp", "bmp", "tiff", "tif", "pdf",
];

pub fn supported_formats() -> &'static [&'static str] {
    SUPPORTED_FORMATS
}

pub fn supported_formats_string() -> String {
    SUPPORTED_FORMATS.join(", ")
}

pub fn is_supported_extension(ext: &str) -> bool {
    let clean = ext.trim_start_matches('.').to_lowercase();
    SUPPORTED_FORMATS.contains(&clean.as_str())
}

pub fn detect_format(bytes: &[u8]) -> Result<DocumentFormat, OcrError> {
    if bytes.len() < 4 {
        return Err(OcrError::UnsupportedFormat {
            detected: "empty_or_too_small".to_string(),
            supported: supported_formats_string(),
        });
    }

    if bytes.starts_with(b"%PDF-") || (bytes.len() >= 4 && &bytes[0..4] == b"%PDF") {
        return Ok(DocumentFormat::Pdf);
    }

    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Ok(DocumentFormat::Png);
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(DocumentFormat::Jpeg);
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(DocumentFormat::WebP);
    }

    if bytes.starts_with(b"BM") {
        return Ok(DocumentFormat::Bmp);
    }

    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return Ok(DocumentFormat::Tiff);
    }

    let detected = if bytes.len() >= 4 {
        format!("magic_bytes({:02X}{:02X}{:02X}{:02X})", bytes[0], bytes[1], bytes[2], bytes[3])
    } else {
        "unknown".to_string()
    };

    Err(OcrError::UnsupportedFormat {
        detected,
        supported: supported_formats_string(),
    })
}

pub fn validate_file_path(path: &Path) -> Result<(), OcrError> {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if !is_supported_extension(ext) {
            return Err(OcrError::UnsupportedFormat {
                detected: ext.to_lowercase(),
                supported: supported_formats_string(),
            });
        }
    }
    Ok(())
}
