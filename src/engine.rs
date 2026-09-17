//! The main OCR engine — single entry point for all OCR operations.
//!
//! # Quick Start
//! ```no_run
//! use alvio_ocr::OcrEngine;
//!
//! let engine = OcrEngine::new("./models/ocr").unwrap();
//!
//! // OCR from a file
//! let result = engine.recognize_file("document.png").unwrap();
//! println!("{}", result.text);
//!
//! // OCR from bytes
//! let bytes = std::fs::read("photo.jpg").unwrap();
//! let result = engine.recognize_bytes(&bytes).unwrap();
//! ```

use crate::{
    config::OcrConfig,
    detector::TextDetector,
    document::{detect_format, DocumentFormat},
    enhancement::enhance_if_needed,
    error::OcrError,
    postprocessing::{reconstruct_text, sort_reading_order},
    recognizer::TextRecognizer,
    types::RecognitionResult,
};
use image::{DynamicImage, GenericImageView, ImageReader};
use std::{io::Cursor, path::Path, sync::Arc};
use tracing::debug;

#[cfg(feature = "pdf")]
use crate::{pdf::PdfProcessor, types::PageResult};

/// High-performance OCR engine.
///
/// Internally holds the loaded ONNX models (detection + recognition) and
/// configuration. Thread-safe — can be shared across threads via `Arc`.
///
/// # Example
/// ```no_run
/// use alvio_ocr::{OcrEngine, OcrConfig};
///
/// // Simple: just point at the model directory
/// let engine = OcrEngine::new("./models/ocr").unwrap();
///
/// // Advanced: full config control
/// let config = OcrConfig::default_with_model_dir("./models/ocr")
///     .det_max_side_len(1280)
///     .rec_threshold(0.4)
///     .enable_enhancement(true);
/// let engine = OcrEngine::with_config(config).unwrap();
/// ```
pub struct OcrEngine {
    detector: Arc<TextDetector>,
    recognizer: Arc<TextRecognizer>,
    config: Arc<OcrConfig>,
    #[cfg(feature = "pdf")]
    pdf_processor: Option<Arc<PdfProcessor>>,
}

impl OcrEngine {
    /// Create an OCR engine with automatic model downloading and default language (Indonesian/Latin).
    ///
    /// Truly zero configuration — models are searched locally or downloaded
    /// automatically into the standard cache directory (`~/.alvio-ocr/models`).
    ///
    /// # Example
    /// ```no_run
    /// use alvio_ocr::OcrEngine;
    ///
    /// let engine = OcrEngine::auto().unwrap();
    /// let result = engine.recognize_file("photo.jpg").unwrap();
    /// ```
    pub fn auto() -> Result<Self, OcrError> {
        Self::auto_with_language(crate::config::OcrLanguage::Indonesian)
    }

    /// Create an OCR engine with automatic model downloading for a specific language preset.
    ///
    /// # Example
    /// ```no_run
    /// use alvio_ocr::{OcrEngine, OcrLanguage};
    ///
    /// let engine = OcrEngine::auto_with_language(OcrLanguage::Multilingual).unwrap();
    /// ```
    pub fn auto_with_language(lang: crate::config::OcrLanguage) -> Result<Self, OcrError> {
        let dir = crate::download::default_model_dir();
        crate::download::ensure_models(&dir, lang)?;
        let config = OcrConfig::default_with_language(&dir, lang);
        Self::with_config(config)
    }

    /// Create an OCR engine with default configuration, pointing at a model directory.
    ///
    /// If required models are missing in `model_dir`, they will be automatically downloaded.
    pub fn new(model_dir: impl Into<std::path::PathBuf>) -> Result<Self, OcrError> {
        let dir = model_dir.into();
        let _ = crate::download::ensure_models(&dir, crate::config::OcrLanguage::Indonesian);
        let config = OcrConfig::default_with_model_dir(dir);
        Self::with_config(config)
    }

    /// Create an OCR engine with a specific language preset.
    ///
    /// If required models are missing in `model_dir`, they will be automatically downloaded.
    pub fn with_language(
        model_dir: impl Into<std::path::PathBuf>,
        lang: crate::config::OcrLanguage,
    ) -> Result<Self, OcrError> {
        let dir = model_dir.into();
        let _ = crate::download::ensure_models(&dir, lang);
        let config = OcrConfig::default_with_language(dir, lang);
        Self::with_config(config)
    }

    /// Create an OCR engine with custom configuration.
    pub fn with_config(config: OcrConfig) -> Result<Self, OcrError> {
        // Auto-download missing models if possible
        if !config.detection_model_path.exists() {
            if let Some(parent) = config.detection_model_path.parent() {
                let _ = crate::download::ensure_models(parent, config.language);
            }
        }

        let config = Arc::new(config);

        let detector = Arc::new(TextDetector::new(&config)?);
        let recognizer = Arc::new(TextRecognizer::new(&config)?);

        #[cfg(feature = "pdf")]
        let pdf_processor = match PdfProcessor::new(Arc::clone(&config)) {
            Ok(p) => Some(Arc::new(p)),
            Err(e) => {
                tracing::warn!("PDF processor init failed (PDF OCR disabled): {}", e);
                None
            }
        };

        Ok(Self {
            detector,
            recognizer,
            config,
            #[cfg(feature = "pdf")]
            pdf_processor,
        })
    }

    /// OCR a `DynamicImage` directly.
    ///
    /// This is the core method. All other `recognize_*` methods eventually call this.
    ///
    /// Applies the full optimized pipeline:
    /// 1. **A4**: Adaptive enhancement (if enabled and image quality is low)
    /// 2. **S1/A2**: SIMD detection preprocessing with bilinear interpolation
    /// 3. **A5/S6**: Detection + NMS
    /// 4. **S2/A3**: Batch recognition with proportional padding
    /// 5. **A6**: Reading order sort + text reconstruction
    pub fn recognize_image(&self, img: &DynamicImage) -> Result<RecognitionResult, OcrError> {
        let (w, h) = img.dimensions();
        debug!("OCR on image ({}x{})...", w, h);

        // A4: Adaptive image enhancement
        let processed_img = if self.config.enhancement_enabled {
            enhance_if_needed(img, self.config.enhancement_variance_threshold)
        } else {
            img.clone()
        };

        // Detection (S1, A1, A2, A5, S6 — all baked in)
        let regions = self.detector.detect(&processed_img)?;
        debug!("Detected {} text regions (post-NMS)", regions.len());

        if regions.is_empty() {
            return Ok(RecognitionResult::default());
        }

        // S2: Batch recognition (A3 proportional padding baked in)
        let recognized_items = self
            .recognizer
            .recognize_regions_batch(&processed_img, &regions)?;
        debug!("Recognized {} text items", recognized_items.len());

        // A6: Reading order sort + text reconstruction
        let sorted_items = sort_reading_order(recognized_items);
        let text = reconstruct_text(&sorted_items);

        Ok(RecognitionResult {
            text,
            regions: sorted_items,
        })
    }

    /// OCR from raw file bytes. Auto-detects format (image or PDF).
    ///
    /// For PDF files, this will only work if the `pdf` feature is enabled.
    /// If you specifically want PDF results with page-level data, use [`recognize_pdf`].
    pub fn recognize_bytes(&self, bytes: &[u8]) -> Result<RecognitionResult, OcrError> {
        let format = detect_format(bytes)?;

        match format {
            DocumentFormat::Pdf => {
                #[cfg(feature = "pdf")]
                {
                    let pages = self.recognize_pdf(bytes)?;
                    let combined_text = pages
                        .iter()
                        .map(|p| p.text.as_str())
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    let all_regions: Vec<_> = pages
                        .into_iter()
                        .flat_map(|p| p.regions)
                        .collect();

                    Ok(RecognitionResult {
                        text: combined_text,
                        regions: all_regions,
                    })
                }

                #[cfg(not(feature = "pdf"))]
                Err(OcrError::UnsupportedFormat(
                    "PDF support requires the 'pdf' feature. Add `features = [\"pdf\"]` to your Cargo.toml.".to_string(),
                ))
            }
            DocumentFormat::Image(_) => {
                let reader = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .map_err(|e| OcrError::InvalidImage(format!("Parse format: {}", e)))?;

                let img = reader
                    .decode()
                    .map_err(|e| OcrError::InvalidImage(format!("Decode: {}", e)))?;

                let (w, h) = img.dimensions();
                if w > self.config.max_image_width || h > self.config.max_image_height {
                    return Err(OcrError::ImageTooLarge(format!(
                        "{}x{} exceeds max {}x{}",
                        w, h, self.config.max_image_width, self.config.max_image_height
                    )));
                }

                self.recognize_image(&img)
            }
        }
    }

    /// OCR from a file path. Auto-detects format.
    pub fn recognize_file(&self, path: impl AsRef<Path>) -> Result<RecognitionResult, OcrError> {
        let bytes = std::fs::read(path.as_ref())?;
        self.recognize_bytes(&bytes)
    }

    /// OCR a PDF document, returning per-page results.
    ///
    /// Uses parallel page processing (S5) via rayon for multi-page PDFs.
    ///
    /// # Example
    /// ```no_run
    /// # use alvio_ocr::OcrEngine;
    /// let engine = OcrEngine::new("./models/ocr").unwrap();
    /// let pdf_bytes = std::fs::read("document.pdf").unwrap();
    /// let pages = engine.recognize_pdf(&pdf_bytes).unwrap();
    /// for page in &pages {
    ///     println!("Page {}: {}", page.page_number, page.text);
    /// }
    /// ```
    #[cfg(feature = "pdf")]
    pub fn recognize_pdf(&self, pdf_bytes: &[u8]) -> Result<Vec<PageResult>, OcrError> {
        let pdf_processor = self.pdf_processor.as_ref().ok_or_else(|| {
            OcrError::InvalidPdf("PDF processor not initialized. Check pdfium library.".to_string())
        })?;

        let detector = Arc::clone(&self.detector);
        let recognizer = Arc::clone(&self.recognizer);
        let config = Arc::clone(&self.config);

        pdf_processor.process_pdf(pdf_bytes, |img| {
            // Run the full OCR pipeline on a page image
            let processed_img = if config.enhancement_enabled {
                enhance_if_needed(img, config.enhancement_variance_threshold)
            } else {
                img.clone()
            };

            let regions = detector.detect(&processed_img)?;
            if regions.is_empty() {
                return Ok((String::new(), Vec::new()));
            }

            let recognized = recognizer.recognize_regions_batch(&processed_img, &regions)?;
            let sorted = sort_reading_order(recognized);
            let text = reconstruct_text(&sorted);

            Ok((text, sorted))
        })
    }
}

