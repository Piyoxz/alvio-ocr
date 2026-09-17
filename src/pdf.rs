//! PDF processing with parallel page OCR.
//!
//! Optimization S5: Two-phase approach:
//! 1. Sequential: Extract text and render pages via pdfium (not thread-safe)
//! 2. Parallel: OCR all rendered page images via rayon

use crate::{
    config::OcrConfig,
    deduplication::deduplicate_ocr_items,
    error::OcrError,
    types::{OcrText, PageResult, TextLine, TextSource},
};
use image::DynamicImage;
use parking_lot::Mutex;
use pdfium_render::prelude::*;
use std::sync::Arc;
use tracing::{debug, info};

/// Internal representation of page data after pdfium extraction (phase 1).
struct ExtractedPage {
    page_number: usize,
    native_text: String,
    has_usable_text: bool,
    candidate_images: Vec<DynamicImage>,
    rendered_image: Option<DynamicImage>,
}

struct SafePdfium(Mutex<Pdfium>);
unsafe impl Send for SafePdfium {}
unsafe impl Sync for SafePdfium {}

static SHARED_PDFIUM: Mutex<Option<Arc<SafePdfium>>> = Mutex::new(None);

pub struct PdfProcessor {
    pdfium: Arc<SafePdfium>,
    config: Arc<OcrConfig>,
}

unsafe impl Send for PdfProcessor {}
unsafe impl Sync for PdfProcessor {}

impl PdfProcessor {
    pub fn new(config: Arc<OcrConfig>) -> Result<Self, OcrError> {
        let mut guard = SHARED_PDFIUM.lock();
        let pdfium = if let Some(existing) = &*guard {
            Arc::clone(existing)
        } else {
            let instance = Arc::new(SafePdfium(Mutex::new(Self::init_pdfium(&config)?)));
            *guard = Some(Arc::clone(&instance));
            instance
        };

        Ok(Self {
            pdfium,
            config,
        })
    }

    fn init_pdfium(config: &OcrConfig) -> Result<Pdfium, OcrError> {
        if let Some(path) = &config.pdfium_path {
            if path.exists() {
                info!("Binding Pdfium to library at: {:?}", path);
                let bindings = Pdfium::bind_to_library(path).map_err(|e| {
                    OcrError::InvalidPdf(format!("Failed to bind Pdfium at {:?}: {}", path, e))
                })?;
                return Ok(Pdfium::new(bindings));
            }
        }

        // Check PDFIUM_PATH env var
        if let Ok(env_path) = std::env::var("PDFIUM_PATH") {
            let path = std::path::PathBuf::from(env_path);
            if path.exists() {
                info!("Binding Pdfium from PDFIUM_PATH: {:?}", path);
                let bindings = Pdfium::bind_to_library(&path).map_err(|e| {
                    OcrError::InvalidPdf(format!("Failed to bind Pdfium at {:?}: {}", path, e))
                })?;
                return Ok(Pdfium::new(bindings));
            }
        }

        // Check common directories
        for dir in &["./libs", "../libs", "../ocr/libs", "./bin"] {
            let libs_path = Pdfium::pdfium_platform_library_name_at_path(dir);
            if libs_path.exists() {
                info!("Binding Pdfium to library at: {:?}", libs_path);
                let bindings = Pdfium::bind_to_library(&libs_path).map_err(|e| {
                    OcrError::InvalidPdf(format!("Failed to bind Pdfium at {:?}: {}", libs_path, e))
                })?;
                return Ok(Pdfium::new(bindings));
            }
        }

        info!("Attempting to bind Pdfium to system library...");
        if let Ok(bindings) = Pdfium::bind_to_system_library() {
            return Ok(Pdfium::new(bindings));
        }

        // Auto-download Pdfium if missing
        info!("Pdfium not found on system. Automatically downloading prebuilt Pdfium binary...");
        let libs_dir = crate::download::default_libs_dir();
        crate::download::ensure_pdfium(&libs_dir)?;

        let downloaded_path = Pdfium::pdfium_platform_library_name_at_path(&libs_dir);
        if downloaded_path.exists() {
            let bindings = Pdfium::bind_to_library(&downloaded_path).map_err(|e| {
                OcrError::InvalidPdf(format!("Failed to bind downloaded Pdfium: {}", e))
            })?;
            return Ok(Pdfium::new(bindings));
        }

        Err(OcrError::InvalidPdf(
            "Pdfium library not found. Please place pdfium.dll / libpdfium.so in './libs' or set PDFIUM_PATH.".to_string(),
        ))
    }

    /// Process a PDF document with parallel page OCR.
    ///
    /// Uses a two-phase approach for maximum speed:
    /// 1. **Phase 1 (sequential)**: Extract text and render page images via pdfium
    /// 2. **Phase 2 (parallel)**: OCR all rendered page images via rayon
    pub fn process_pdf<F>(
        &self,
        pdf_bytes: &[u8],
        ocr_fn: F,
    ) -> Result<Vec<PageResult>, OcrError>
    where
        F: Fn(&DynamicImage) -> Result<(String, f32, Vec<OcrText>, Vec<TextLine>, (u32, u32)), OcrError> + Send + Sync,
    {
        // === PHASE 1: Sequential extraction via pdfium ===
        let extracted_pages = {
            let pdfium_guard = self.pdfium.0.lock();

            let document = pdfium_guard
                .load_pdf_from_byte_slice(pdf_bytes, None)
                .map_err(|e| OcrError::InvalidPdf(format!("Failed to open PDF: {}", e)))?;

            let total_pages = document.pages().len() as usize;
            debug!("PDF loaded. Total pages: {}", total_pages);

            if total_pages == 0 {
                return Err(OcrError::InvalidPdf("PDF has 0 pages".to_string()));
            }

            if total_pages > self.config.max_pdf_pages {
                return Err(OcrError::PdfTooManyPages(format!(
                    "PDF has {} pages, max is {}",
                    total_pages, self.config.max_pdf_pages
                )));
            }

            let mut pages = Vec::with_capacity(total_pages);

            for page_idx in 0..total_pages {
                let page_num = page_idx + 1;
                let page = document
                    .pages()
                    .get(page_idx as u16)
                    .map_err(|e| OcrError::InvalidPdf(format!("Page {}: {}", page_num, e)))?;

                // Extract native text
                let native_text = match page.text() {
                    Ok(t) => t.all(),
                    Err(e) => {
                        debug!("Page {} text extraction failed: {}", page_num, e);
                        String::new()
                    }
                };

                let trimmed = native_text.trim().to_string();
                let has_usable_text =
                    !trimmed.is_empty() && trimmed.chars().any(|c| c.is_alphanumeric());

                // Extract embedded images
                let min_area = self.config.pdf_embedded_img_min_area;
                let mut candidate_images = Vec::new();
                for obj in page.objects().iter() {
                    if let Some(img_obj) = obj.as_image_object() {
                        if let Ok(dyn_img) = img_obj.get_raw_image() {
                            let area = dyn_img.width() * dyn_img.height();
                            if area >= min_area {
                                candidate_images.push(dyn_img);
                            }
                        }
                    }
                }

                // Render page if needed (no native text and no embedded images to OCR)
                let rendered_image = if !has_usable_text && candidate_images.is_empty() {
                    debug!("Page {} needs full render for OCR", page_num);
                    let render_config = PdfRenderConfig::new().set_target_width(1024);
                    let bitmap = page.render_with_config(&render_config).map_err(|e| {
                        OcrError::PdfRenderingFailed(format!("Page {}: {}", page_num, e))
                    })?;
                    Some(bitmap.as_image())
                } else {
                    None
                };

                pages.push(ExtractedPage {
                    page_number: page_num,
                    native_text: trimmed,
                    has_usable_text,
                    candidate_images,
                    rendered_image,
                });
            }

            pages
        }; // pdfium_guard dropped here

        // === PHASE 2: Parallel OCR via rayon ===
        let results: Result<Vec<PageResult>, OcrError> = {
            use rayon::prelude::*;

            extracted_pages
                .into_par_iter()
                .map(|page| {
                    let page_num = page.page_number;

                    if page.has_usable_text && page.candidate_images.is_empty() {
                        debug!("Page {} native text extraction", page_num);
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::NativeText,
                            text: page.native_text,
                            confidence: 1.0,
                            regions: Vec::new(),
                            lines: Vec::new(),
                            dimensions: (1024, 1448),
                        })
                    } else if page.has_usable_text && !page.candidate_images.is_empty() {
                        debug!("Page {} mixed content", page_num);

                        let mut embedded_regions = Vec::new();
                        let mut embedded_lines = Vec::new();
                        for img in &page.candidate_images {
                            if let Ok((_, _, regions, lines, _)) = ocr_fn(img) {
                                embedded_regions.extend(regions);
                                embedded_lines.extend(lines);
                            }
                        }

                        let (unique_ocr_items, appended_ocr_text) =
                            deduplicate_ocr_items(&page.native_text, embedded_regions);

                        if !unique_ocr_items.is_empty() {
                            let mut combined = page.native_text.clone();
                            combined.push('\n');
                            combined.push_str(&appended_ocr_text);

                            let conf = unique_ocr_items.iter().map(|r| r.confidence).sum::<f32>()
                                / unique_ocr_items.len() as f32;

                            Ok(PageResult {
                                page_number: page_num,
                                source: TextSource::Mixed,
                                text: combined,
                                confidence: conf,
                                regions: unique_ocr_items,
                                lines: embedded_lines,
                                dimensions: (1024, 1448),
                            })
                        } else {
                            Ok(PageResult {
                                page_number: page_num,
                                source: TextSource::NativeText,
                                text: page.native_text,
                                confidence: 1.0,
                                regions: Vec::new(),
                                lines: Vec::new(),
                                dimensions: (1024, 1448),
                            })
                        }
                    } else if let Some(rendered) = &page.rendered_image {
                        debug!("Page {} rendered image OCR", page_num);
                        let (text, conf, regions, lines, dims) = ocr_fn(rendered)?;
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::Ocr,
                            text,
                            confidence: conf,
                            regions,
                            lines,
                            dimensions: dims,
                        })
                    } else if !page.candidate_images.is_empty() {
                        debug!("Page {} embedded images OCR", page_num);
                        let mut all_text = String::new();
                        let mut all_regions = Vec::new();
                        let mut all_lines = Vec::new();
                        for img in &page.candidate_images {
                            if let Ok((text, _, regions, lines, _)) = ocr_fn(img) {
                                if !all_text.is_empty() && !text.is_empty() {
                                    all_text.push('\n');
                                }
                                all_text.push_str(&text);
                                all_regions.extend(regions);
                                all_lines.extend(lines);
                            }
                        }

                        let conf = if all_regions.is_empty() {
                            0.0
                        } else {
                            all_regions.iter().map(|r| r.confidence).sum::<f32>()
                                / all_regions.len() as f32
                        };

                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::Ocr,
                            text: all_text,
                            confidence: conf,
                            regions: all_regions,
                            lines: all_lines,
                            dimensions: (1024, 1448),
                        })
                    } else {
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::NativeText,
                            text: String::new(),
                            confidence: 1.0,
                            regions: Vec::new(),
                            lines: Vec::new(),
                            dimensions: (1024, 1448),
                        })
                    }
                })
                .collect()
        };

        let mut pages = results?;

        // Ensure pages are in order (rayon may reorder)
        pages.sort_by_key(|p| p.page_number);

        Ok(pages)
    }
}
