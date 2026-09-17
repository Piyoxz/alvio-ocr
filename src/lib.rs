pub mod config;
pub mod deduplication;
pub mod detector;
pub mod document;
pub mod download;
pub mod engine;
pub mod enhancement;
pub mod error;
pub mod nms;
pub mod orientation;
pub mod postprocessing;
pub mod preprocessing;
pub mod profiling;
pub mod recognizer;
pub mod schema;
pub mod types;

#[cfg(feature = "pdf")]
pub mod pdf;

pub use config::{supported_languages, LanguageInfo, OcrConfig, OcrLanguage};
pub use document::{
    is_supported_extension, supported_formats, supported_formats_string, SUPPORTED_FORMATS,
};
pub use engine::OcrEngine;
pub use error::{OcrError, Result};
pub use orientation::{deskew_image, detect_skew_angle, PageOrientation};
pub use profiling::{compute_cer, compute_wer, PercentileStats, StageTiming};
pub use schema::{
    extract_key_values, extract_ktp, extract_receipt, reconstruct_table, KeyValuePair, KtpData,
    ReceiptData, ReceiptItem, TableCell, TableData,
};
pub use types::{
    BatchItem, BatchResult, BoundingBox, DocumentFormat, OcrResult, OcrText, PageResult, PdfResult,
    Point, RecognitionResult, TextLine, TextRegion, TextSource,
};
