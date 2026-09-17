//! # alvio-ocr
//!
//! High-performance OCR library powered by ONNX Runtime and PP-OCRv4.
//!
//! ## Features
//! - **Fast**: SIMD-accelerated image resize, batch recognition inference, parallel PDF processing
//! - **Accurate**: Adaptive image enhancement, NMS deduplication, proportional padding
//! - **Simple**: One struct, three methods — that's all you need
//!
//! ## Quick Start
//! ```no_run
//! use alvio_ocr::OcrEngine;
//!
//! // Initialize with model directory
//! let engine = OcrEngine::new("./models/ocr").unwrap();
//!
//! // OCR an image file
//! let result = engine.recognize_file("photo.jpg").unwrap();
//! println!("{}", result.text);
//!
//! // OCR from bytes
//! let bytes = std::fs::read("scan.png").unwrap();
//! let result = engine.recognize_bytes(&bytes).unwrap();
//!
//! // OCR a DynamicImage directly
//! let img = image::open("receipt.png").unwrap();
//! let result = engine.recognize_image(&img).unwrap();
//! ```
//!
//! ## PDF Support
//! Enable the `pdf` feature for PDF document processing:
//! ```toml
//! [dependencies]
//! alvio-ocr = { path = "../alvio-ocr", features = ["pdf"] }
//! ```
//!
//! ```no_run
//! # #[cfg(feature = "pdf")]
//! # {
//! use alvio_ocr::OcrEngine;
//!
//! let engine = OcrEngine::new("./models/ocr").unwrap();
//! let pdf_bytes = std::fs::read("document.pdf").unwrap();
//! let pages = engine.recognize_pdf(&pdf_bytes).unwrap();
//! for page in &pages {
//!     println!("Page {}: {}", page.page_number, page.text);
//! }
//! # }
//! ```

pub mod config;
pub mod deduplication;
pub mod detector;
pub mod document;
pub mod download;
pub mod engine;
pub mod enhancement;
pub mod error;
pub mod nms;
pub mod postprocessing;
pub mod preprocessing;
pub mod recognizer;
pub mod types;

#[cfg(feature = "pdf")]
pub mod pdf;

// Re-export public API
pub use config::{OcrConfig, OcrLanguage};
pub use engine::OcrEngine;
pub use error::OcrError;
pub use types::{OcrText, PageResult, Point, RecognitionResult, TextRegion, TextSource};
