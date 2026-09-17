# Changelog

All notable changes to the `alvio-ocr` project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-09-17

### Added
- **Embedded 18,708-Key PP-OCRv6 Dictionary**: Embedded `ppocrv6_keys.txt` directly into crate binary (`include_str!`). Eliminates all network failure risks and guarantees 100% dictionary availability on any fresh machine offline.
- **Production Security & Resource Guards**:
  - `max_file_size`: Configurable file/payload guard (default 50 MB) rejecting oversized inputs prior to memory allocation.
  - `max_url_download_size`: Capped HTTP streaming download (default 25 MB) preventing memory exhaustion / DOS attacks via malicious URLs.
  - `url_timeout_secs`: Enforced 15s connection and read timeouts.
- **Cross-Platform Hardening**:
  - Verified multi-platform native downloads for Windows x64/arm64, Linux x64/arm64, and macOS Intel/Apple Silicon.
  - Added `libgomp1` to production Dockerfile for OpenMP Linux threading support.
  - Documented CPU-offloading via `tokio::task::spawn_blocking` in Axum web service guide.
  - Added detailed reproducible benchmark environment specifications.

---

## [0.2.1] - 2026-09-17

### Optimized
- **Zero-Copy Image Borrowing**: `enhance_if_needed` now uses `Cow<'a, DynamicImage>`, eliminating unnecessary full-image clones for clean input images.
- **Zero-Copy FIR Resizing**: Converted `fast_resize_rgb_from_raw` to use `ImageRef` instead of cloning raw RGB bytes prior to convolution.
- **Zero-Copy Detection Slicing**: Eliminated 4MB `prob_vec` dynamic float buffer allocation per detection pass; uses direct contiguous tensor slice.
- **Threadpool Tuning**: Standardized `resolve_threads` to `intra=cpus.min(6)` and `inter=1`, eliminating multi-threaded context switching and barrier contention on CPUs.
- **Engine Warmup**: Added pre-warmed sessions during `OcrEngine::with_config` to eliminate cold-start allocator allocation on the initial request.
- **Optimal Variance Threshold**: Reduced default `enhancement_variance_threshold` to 500.0, avoiding expensive unsharp mask filtering on clear images.

---

## [0.2.0] - 2026-09-17

### Added
- **Document AI Intelligent Schemas**:
  - `KtpData` & `to_ktp()`: Automated Indonesian e-KTP field extraction (NIK, Nama, Tempat/Tgl Lahir, Alamat, Agama, Status, Pekerjaan, Kewarganegaraan) with specialized heuristic character typo sanitization (e.g. B->8, O->0 in NIK).
  - `ReceiptData` & `to_receipt()`: Automated receipt & invoice parser with itemized lines, prices, merchant name, transaction date, and currency total detection.
  - `TableData` & `to_table()`: Spatial 2D table & grid reconstructor with column/row alignment and direct `to_markdown()` generator.
  - `KeyValuePair` & `to_key_values()`: Generic key-value extractor for structured form inspection.
- **Orientation & Deskew Pipeline**:
  - Radon/Projection profile skew detector measuring text tilt from -45° to +45°.
  - Bilinear rotation deskew (`deskew_image`, `deskew_rgb`) and 90°/180°/270° orthogonal orientation correction.
  - Configurable via `OcrConfig::with_deskew(true)`.
- **Confidence-based 2nd-Pass Re-OCR**:
  - Region-level second chance recognition for low-confidence detections below customizable threshold.
  - Contrast stretching and sharpening preprocessing prior to re-inference.
  - Configurable via `OcrConfig::with_second_pass(true, 0.75)`.
- **Latency Profiler (`StageTiming`)**:
  - Granular nanosecond tracking of Preprocessing, Detection, Crop, Recognition, and Postprocessing stages.
  - Integrated into `OcrResult.timing` with formatted summary (`P50/P95/P99`).
- **Benchmark Evaluation Suite**:
  - Levenshtein distance metrics `compute_cer` (Character Error Rate) and `compute_wer` (Word Error Rate).
- **GPU Acceleration Support**:
  - Optional `cuda` and `directml` feature flags in `Cargo.toml`.

---

## [0.1.3] - 2026-09-17

### Changed
- **PP-OCRv6 Upgrade**: Migrated entire OCR pipeline from PP-OCRv4 to PP-OCRv6 Small (LCNetV4 backbone, RepLKFPN detection neck, EncoderWithLightSVTR recognition). +4.6% detection accuracy, +5.1% recognition accuracy vs PP-OCRv5.
- **Unified 50-Language Model**: All language presets (Indonesian, English, Multilingual, Chinese) now share a single PP-OCRv6 unified model supporting 50 languages without model switching.
- **Model Source**: Models now downloaded from official PaddlePaddle HuggingFace repos (`PaddlePaddle/PP-OCRv6_small_det_onnx`, `PaddlePaddle/PP-OCRv6_small_rec_onnx`).

### Optimized
- **LUT-based Auto Contrast**: Replaced per-pixel arithmetic with 256-entry lookup table for instant contrast mapping.
- **Separable Box Blur**: 2-pass horizontal+vertical blur with `u16` accumulation instead of single-pass `f32` (2x faster).
- **Unsafe Buffer Access**: All hot pixel loops use `get_unchecked()` to eliminate bounds checking in detection, recognition, and preprocessing.
- **Const Normalization**: All normalization constants (`INV_255`, `INV_127_5`, `DET_MEAN_*`, `DET_INV_STD_*`) computed at compile time.
- **SIMD-friendly Variance**: `chunks_exact(4)` accumulation with `u64` integer math for image variance computation.
- **Inline IoU**: NMS IoU function marked `#[inline(always)]`.
- **Release Profile**: Added `strip = "symbols"` for smaller binary size.
- **Tuned Thresholds**: Detection/recognition thresholds fine-tuned for PP-OCRv6's improved precision.

### Removed
- All code comments stripped from source files as requested.

---

## [0.1.2] - 2026-09-17

### Added
- **README Release Notes**: Added styled release callout box highlighting recent additions directly in crate documentation.
- **Dynamic User-Agent Versioning**: Network fetch requests now dynamically advertise `alvio-ocr/<version>`.
- **Project Documentation**: Integrated `CHANGELOG.md` following Keep a Changelog specifications.

---

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
