//! PDF processing with parallel page OCR.
//!
//! Optimization S5: Two-phase approach:
//! 1. Sequential: Extract text and render pages via pdfium (not thread-safe)
//! 2. Parallel: OCR all rendered page images via rayon

use crate::{
    config::OcrConfig,
    deduplication::deduplicate_ocr_items,
    error::OcrError,
    types::{OcrText, PageResult, TextSource},
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
        let lib_path = crate::download::ensure_pdfium(&libs_dir)?;
        info!("Binding to automatically downloaded Pdfium at: {:?}", lib_path);
        let bindings = Pdfium::bind_to_library(&lib_path).map_err(|e| {
            OcrError::InvalidPdf(format!("Failed to bind downloaded Pdfium at {:?}: {}", lib_path, e))
        })?;
        Ok(Pdfium::new(bindings))
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
        F: Fn(&DynamicImage) -> Result<(String, Vec<OcrText>), OcrError> + Send + Sync,
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
                        // Pure native text — no OCR needed
                        debug!("Page {} → native text (no OCR)", page_num);
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::NativeText,
                            text: page.native_text,
                            regions: Vec::new(),
                        })
                    } else if page.has_usable_text && !page.candidate_images.is_empty() {
                        // Mixed: native text + embedded images
                        debug!(
                            "Page {} → mixed ({} embedded images)",
                            page_num,
                            page.candidate_images.len()
                        );

                        let mut embedded_regions = Vec::new();
                        for img in &page.candidate_images {
                            if let Ok((_, regions)) = ocr_fn(img) {
                                embedded_regions.extend(regions);
                            }
                        }

                        let (unique_ocr_items, appended_ocr_text) =
                            deduplicate_ocr_items(&page.native_text, embedded_regions);

                        if !unique_ocr_items.is_empty() {
                            let mut combined = page.native_text.clone();
                            combined.push('\n');
                            combined.push_str(&appended_ocr_text);

                            Ok(PageResult {
                                page_number: page_num,
                                source: TextSource::Mixed,
                                text: combined,
                                regions: unique_ocr_items,
                            })
                        } else {
                            Ok(PageResult {
                                page_number: page_num,
                                source: TextSource::NativeText,
                                text: page.native_text,
                                regions: Vec::new(),
                            })
                        }
                    } else if let Some(rendered) = &page.rendered_image {
                        // Full OCR on rendered page
                        debug!("Page {} → full OCR on rendered image", page_num);
                        let (text, regions) = ocr_fn(rendered)?;
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::Ocr,
                            text,
                            regions,
                        })
                    } else if !page.candidate_images.is_empty() {
                        // No native text but has embedded images
                        debug!(
                            "Page {} → OCR on {} embedded images",
                            page_num,
                            page.candidate_images.len()
                        );
                        let mut all_text = String::new();
                        let mut all_regions = Vec::new();
                        for img in &page.candidate_images {
                            if let Ok((text, regions)) = ocr_fn(img) {
                                if !all_text.is_empty() && !text.is_empty() {
                                    all_text.push('\n');
                                }
                                all_text.push_str(&text);
                                all_regions.extend(regions);
                            }
                        }
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::Ocr,
                            text: all_text,
                            regions: all_regions,
                        })
                    } else {
                        // Empty page
                        Ok(PageResult {
                            page_number: page_num,
                            source: TextSource::NativeText,
                            text: String::new(),
                            regions: Vec::new(),
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
