use alvio_ocr::OcrEngine;
use std::path::Path;

fn get_model_dir() -> &'static str {
    if Path::new("./models").exists() {
        "./models"
    } else {
        "../ocr/models/ocr"
    }
}

#[test]
fn test_engine_initialization() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir);
    assert!(engine.is_ok(), "Failed to initialize default OcrEngine: {:?}", engine.err());

    let multi_engine = OcrEngine::with_language(model_dir, alvio_ocr::OcrLanguage::Multilingual);
    assert!(multi_engine.is_ok(), "Failed to initialize Multilingual OcrEngine: {:?}", multi_engine.err());
}

#[test]
fn test_auto_engine() {
    let engine = OcrEngine::auto();
    assert!(engine.is_ok(), "Failed to initialize auto OcrEngine: {:?}", engine.err());
}

#[test]
fn test_image_ocr_accuracy() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir).expect("Engine should initialize");

    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let result = engine.recognize_file(sample_image).expect("OCR should succeed");
        assert!(!result.text.is_empty(), "Recognized text should not be empty");
        assert!(result.text.contains("JAKARTA") || result.text.contains("PROVINSI"), "Expected text to match fixture");
        assert!(!result.regions.is_empty(), "Should have detected regions");
        for r in &result.regions {
            assert!(r.confidence > 0.5, "Confidence should be > 0.5");
        }
    }
}

#[cfg(feature = "pdf")]
#[test]
fn test_pdf_ocr() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir).expect("Engine should initialize");

    let sample_pdf = "D:/riset_alvio/ocr/tests/fixtures/ocr/scanned.pdf";
    if Path::new(sample_pdf).exists() {
        let pdf_bytes = std::fs::read(sample_pdf).expect("Read PDF");
        let pages = engine.recognize_pdf(&pdf_bytes).expect("PDF OCR should succeed");
        assert_eq!(pages.len(), 1, "Should have 1 page");
        assert!(pages[0].text.contains("SCAN") || pages[0].text.contains("DOKUMEN"), "Expected text to match fixture");
    }
}
