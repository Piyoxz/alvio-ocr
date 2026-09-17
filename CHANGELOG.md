# Changelog

All notable changes to the `alvio-ocr` project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-09-17

### Added
- **Universal `recognize()`**: Automatic resolution of local file paths and remote URLs (`http://`, `https://`).
- **Web URL OCR**: Direct image download and recognition via `recognize_url()` and parallel `recognize_urls()`.
- **Parallel Multi-File Processing**: `recognize_files(&[paths])` powered by Rayon threadpool returning structured `BatchResult`.
- **Structured Response (`OcrResult`)**:
  - `confidence`: Overall aggregate confidence score across all detected regions.
  - `lines`: Grouped reading order text lines (`TextLine`) with line bounding boxes and line numbers.
  - `to_json()` & `to_json_pretty()`: Instant JSON serialization.
  - `to_tsv()`: Tab-separated values export for tabular analysis.
  - `filter_by_confidence()`: Filter low-confidence detections dynamically.
  - `search()`: Substring search across recognized regions.
- **Format Validation & Sniffing**:
  - `supported_formats()` API listing all supported formats (`JPEG`, `PNG`, `WebP`, `BMP`, `TIFF`, `PDF`).
  - Explicit rejection of unsupported formats (`OcrError::UnsupportedFormat`) with descriptive error messages.
- **Language & Model Introspection**:
  - `supported_languages()` returning detailed language information, character count, and underlying ONNX model names.
  - `OcrLanguage::all()` and `OcrLanguage::info()`.

### Changed
- Refactored internal preprocessing and recognition pipelines to emit structured line items.
- Enhanced PDF processor with full `PageResult` metadata.
- Cleaned codebase: Refactored internal functions and streamlined documentation for idiomatic Rust.

---

## [0.1.0] - 2026-09-17

### Added
- Initial release on crates.io.
- DBNet text detector + SVTR text recognizer powered by ONNX Runtime (`ort`).
- Zero-configuration automatic downloading of PP-OCRv4 ONNX models and native Pdfium libraries.
- SIMD-accelerated image resizing (`fast_image_resize` with AVX2 and NEON).
- Batch recognition inference and two-phase parallel PDF OCR pipeline.
- Adaptive contrast enhancement and IoU-based Non-Maximum Suppression (NMS).
- Built-in presets for Latin (Indonesian, English) and Multilingual (6,625+ characters).
