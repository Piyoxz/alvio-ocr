//! Text deduplication between native PDF text and OCR results.

use crate::types::OcrText;

/// Normalize a string for comparison (lowercase, collapse whitespace).
pub fn normalize_string(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Remove OCR items that are duplicates of native PDF text.
///
/// Returns the unique OCR items and the appended text string.
pub fn deduplicate_ocr_items(native_text: &str, ocr_items: Vec<OcrText>) -> (Vec<OcrText>, String) {
    let native_lines: Vec<String> = native_text
        .lines()
        .map(|l| normalize_string(l))
        .filter(|l| !l.is_empty())
        .collect();

    let mut unique_items = Vec::new();
    let mut appended_text = String::new();

    for item in ocr_items {
        let norm_ocr = normalize_string(&item.text);
        if norm_ocr.is_empty() {
            continue;
        }

        let is_duplicate = native_lines.iter().any(|n_line| {
            if n_line == &norm_ocr {
                return true;
            }
            if n_line.len() > 5 && norm_ocr.len() > 5 {
                if n_line.contains(&norm_ocr) || norm_ocr.contains(n_line.as_str()) {
                    return true;
                }
            }
            false
        });

        if !is_duplicate {
            if !appended_text.is_empty() {
                appended_text.push('\n');
            }
            appended_text.push_str(&item.text);
            unique_items.push(item);
        }
    }

    (unique_items, appended_text)
}
