use crate::{
    config::OcrConfig,
    detector::TextDetector,
    document::{detect_format, validate_file_path},
    enhancement::enhance_if_needed,
    error::OcrError,
    postprocessing::{group_into_lines, reconstruct_text_from_lines, sort_reading_order},
    recognizer::TextRecognizer,
    types::{BatchItem, BatchResult, DocumentFormat, OcrResult},
};

#[cfg(feature = "pdf")]
use crate::types::{PageResult, PdfResult};
use image::{DynamicImage, GenericImageView, ImageReader};
use rayon::prelude::*;
use std::{
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
    time::Instant,
};
use tracing::debug;

#[cfg(feature = "pdf")]
use crate::pdf::PdfProcessor;

pub struct OcrEngine {
    detector: Arc<TextDetector>,
    recognizer: Arc<TextRecognizer>,
    config: Arc<OcrConfig>,
    #[cfg(feature = "pdf")]
    pdf_processor: Option<Arc<PdfProcessor>>,
}

impl OcrEngine {
    pub fn auto() -> Result<Self, OcrError> {
        Self::auto_with_language(crate::config::OcrLanguage::Indonesian)
    }

    pub fn auto_with_language(lang: crate::config::OcrLanguage) -> Result<Self, OcrError> {
        let dir = crate::download::default_model_dir();
        crate::download::ensure_models(&dir, lang)?;
        let config = OcrConfig::default_with_language(&dir, lang);
        Self::with_config(config)
    }

    pub fn new(model_dir: impl Into<std::path::PathBuf>) -> Result<Self, OcrError> {
        let dir = model_dir.into();
        let _ = crate::download::ensure_models(&dir, crate::config::OcrLanguage::Indonesian);
        let config = OcrConfig::default_with_model_dir(dir);
        Self::with_config(config)
    }

    pub fn with_language(
        model_dir: impl Into<std::path::PathBuf>,
        lang: crate::config::OcrLanguage,
    ) -> Result<Self, OcrError> {
        let dir = model_dir.into();
        let _ = crate::download::ensure_models(&dir, lang);
        let config = OcrConfig::default_with_language(dir, lang);
        Self::with_config(config)
    }

    pub fn with_config(config: OcrConfig) -> Result<Self, OcrError> {
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
                tracing::warn!("PDF processor initialization failed: {}", e);
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

    pub fn recognize(&self, input: &str) -> Result<OcrResult, OcrError> {
        let trimmed = input.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            self.recognize_url(trimmed)
        } else {
            self.recognize_file(trimmed)
        }
    }

    pub fn recognize_url(&self, url: &str) -> Result<OcrResult, OcrError> {
        debug!("Fetching OCR image from URL: {}", url);
        let resp = ureq::get(url)
            .set("User-Agent", concat!("alvio-ocr/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .call()
            .map_err(|e| OcrError::Network(format!("Failed to fetch '{}': {}", url, e)))?;

        let mut bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| OcrError::Network(format!("Failed to read body from '{}': {}", url, e)))?;

        self.recognize_bytes(&bytes)
    }

    pub fn recognize_file(&self, path: impl AsRef<Path>) -> Result<OcrResult, OcrError> {
        let path_ref = path.as_ref();
        validate_file_path(path_ref)?;
        let bytes = std::fs::read(path_ref)?;
        self.recognize_bytes(&bytes)
    }

    pub fn recognize_files<P: AsRef<Path> + Sync>(&self, paths: &[P]) -> Result<BatchResult, OcrError> {
        let start = Instant::now();
        let items: Vec<BatchItem> = paths
            .par_iter()
            .map(|p| {
                let path_str = p.as_ref().to_string_lossy().to_string();
                let item_start = Instant::now();
                match self.recognize_file(p) {
                    Ok(result) => BatchItem {
                        source: path_str,
                        success: true,
                        result: Some(result),
                        error: None,
                        duration_ms: item_start.elapsed().as_millis() as u64,
                    },
                    Err(err) => BatchItem {
                        source: path_str,
                        success: false,
                        result: None,
                        error: Some(err.to_string()),
                        duration_ms: item_start.elapsed().as_millis() as u64,
                    },
                }
            })
            .collect();

        let succeeded = items.iter().filter(|i| i.success).count();
        let failed = items.len() - succeeded;

        Ok(BatchResult {
            items,
            total: paths.len(),
            succeeded,
            failed,
            total_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn recognize_urls(&self, urls: &[&str]) -> Result<BatchResult, OcrError> {
        let start = Instant::now();
        let items: Vec<BatchItem> = urls
            .par_iter()
            .map(|url| {
                let item_start = Instant::now();
                match self.recognize_url(url) {
                    Ok(result) => BatchItem {
                        source: url.to_string(),
                        success: true,
                        result: Some(result),
                        error: None,
                        duration_ms: item_start.elapsed().as_millis() as u64,
                    },
                    Err(err) => BatchItem {
                        source: url.to_string(),
                        success: false,
                        result: None,
                        error: Some(err.to_string()),
                        duration_ms: item_start.elapsed().as_millis() as u64,
                    },
                }
            })
            .collect();

        let succeeded = items.iter().filter(|i| i.success).count();
        let failed = items.len() - succeeded;

        Ok(BatchResult {
            items,
            total: urls.len(),
            succeeded,
            failed,
            total_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn recognize_bytes(&self, bytes: &[u8]) -> Result<OcrResult, OcrError> {
        let format = detect_format(bytes)?;

        match format {
            DocumentFormat::Pdf => {
                #[cfg(feature = "pdf")]
                {
                    let start = Instant::now();
                    let pages = self.recognize_pdf(bytes)?;
                    let mut combined_lines = Vec::new();
                    let mut combined_regions = Vec::new();

                    for page in &pages {
                        combined_lines.extend(page.lines.clone());
                        combined_regions.extend(page.regions.clone());
                    }

                    let full_text = reconstruct_text_from_lines(&combined_lines);
                    let conf = if combined_regions.is_empty() {
                        0.0
                    } else {
                        combined_regions.iter().map(|r| r.confidence).sum::<f32>()
                            / combined_regions.len() as f32
                    };

                    let first_dims = pages.first().map(|p| p.dimensions).unwrap_or((0, 0));

                    Ok(OcrResult {
                        text: full_text,
                        confidence: conf,
                        lines: combined_lines,
                        regions: combined_regions,
                        dimensions: first_dims,
                        format: DocumentFormat::Pdf,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }

                #[cfg(not(feature = "pdf"))]
                Err(OcrError::UnsupportedFormat {
                    detected: "pdf".to_string(),
                    supported: "jpeg, png, webp, bmp, tiff (PDF requires `features = [\"pdf\"]`)".to_string(),
                })
            }
            _ => {
                let reader = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .map_err(|e| OcrError::InvalidImage(format!("Parse image format: {}", e)))?;

                let img = reader
                    .decode()
                    .map_err(|e| OcrError::InvalidImage(format!("Decode image: {}", e)))?;

                let (w, h) = img.dimensions();
                if w == 0 || h == 0 {
                    return Err(OcrError::InvalidImage("Image has zero width or height".to_string()));
                }
                if w > self.config.max_image_width || h > self.config.max_image_height {
                    return Err(OcrError::ImageTooLarge(format!(
                        "{}x{} exceeds max {}x{}",
                        w, h, self.config.max_image_width, self.config.max_image_height
                    )));
                }

                let mut res = self.recognize_image(&img)?;
                res.format = format;
                Ok(res)
            }
        }
    }

    pub fn recognize_image(&self, img: &DynamicImage) -> Result<OcrResult, OcrError> {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err(OcrError::InvalidImage("Image has zero width or height".to_string()));
        }
        let start = Instant::now();
        debug!("OCR on image ({}x{})...", w, h);

        let processed_img = if self.config.enhancement_enabled {
            enhance_if_needed(img, self.config.enhancement_variance_threshold)
        } else {
            img.clone()
        };

        let regions = self.detector.detect(&processed_img)?;
        debug!("Detected {} text regions", regions.len());

        if regions.is_empty() {
            return Ok(OcrResult {
                text: String::new(),
                confidence: 0.0,
                lines: Vec::new(),
                regions: Vec::new(),
                dimensions: (w, h),
                format: DocumentFormat::Unknown,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let recognized_items = self
            .recognizer
            .recognize_regions_batch(&processed_img, &regions)?;
        debug!("Recognized {} text items", recognized_items.len());

        let sorted_items = sort_reading_order(recognized_items);
        let (flat_regions, lines) = group_into_lines(sorted_items);
        let text = reconstruct_text_from_lines(&lines);

        let conf = if flat_regions.is_empty() {
            0.0
        } else {
            flat_regions.iter().map(|r| r.confidence).sum::<f32>() / flat_regions.len() as f32
        };

        Ok(OcrResult {
            text,
            confidence: conf,
            lines,
            regions: flat_regions,
            dimensions: (w, h),
            format: DocumentFormat::Unknown,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    #[cfg(feature = "pdf")]
    pub fn recognize_pdf(&self, pdf_bytes: &[u8]) -> Result<Vec<PageResult>, OcrError> {
        let pdf_processor = self.pdf_processor.as_ref().ok_or_else(|| {
            OcrError::InvalidPdf("PDF processor not initialized. Check pdfium library.".to_string())
        })?;

        let detector = Arc::clone(&self.detector);
        let recognizer = Arc::clone(&self.recognizer);
        let config = Arc::clone(&self.config);

        pdf_processor.process_pdf(pdf_bytes, move |img| {
            let (w, h) = img.dimensions();
            let processed_img = if config.enhancement_enabled {
                enhance_if_needed(img, config.enhancement_variance_threshold)
            } else {
                img.clone()
            };

            let regions = detector.detect(&processed_img)?;
            if regions.is_empty() {
                return Ok((String::new(), 0.0, Vec::new(), Vec::new(), (w, h)));
            }

            let recognized = recognizer.recognize_regions_batch(&processed_img, &regions)?;
            let sorted = sort_reading_order(recognized);
            let (flat_regions, lines) = group_into_lines(sorted);
            let text = reconstruct_text_from_lines(&lines);

            let conf = if flat_regions.is_empty() {
                0.0
            } else {
                flat_regions.iter().map(|r| r.confidence).sum::<f32>() / flat_regions.len() as f32
            };

            Ok((text, conf, flat_regions, lines, (w, h)))
        })
    }

    #[cfg(feature = "pdf")]
    pub fn recognize_pdf_document(&self, pdf_bytes: &[u8]) -> Result<PdfResult, OcrError> {
        let start = Instant::now();
        let pages = self.recognize_pdf(pdf_bytes)?;
        let total_pages = pages.len();
        let text = pages
            .iter()
            .map(|p| p.text.as_str())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        let conf = if pages.is_empty() {
            0.0
        } else {
            pages.iter().map(|p| p.confidence).sum::<f32>() / pages.len() as f32
        };

        Ok(PdfResult {
            pages,
            total_pages,
            text,
            confidence: conf,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
