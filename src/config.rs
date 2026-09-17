use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub model_name: &'static str,
    pub character_count: usize,
    pub description: &'static str,
    pub auto_download: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrLanguage {
    #[default]
    Indonesian,
    English,
    Multilingual,
    Chinese,
}

impl OcrLanguage {
    pub fn all() -> &'static [OcrLanguage] {
        &[
            Self::Indonesian,
            Self::English,
            Self::Multilingual,
            Self::Chinese,
        ]
    }

    pub fn model_filename(&self) -> &'static str {
        "PP-OCRv6_rec_small.onnx"
    }

    pub fn dict_filename(&self) -> &'static str {
        "ppocrv6_keys.txt"
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Indonesian => "Indonesian (Latin)",
            Self::English => "English (Latin)",
            Self::Multilingual => "Multilingual (50 Languages)",
            Self::Chinese => "Chinese (Hanzi)",
        }
    }

    pub fn info(&self) -> LanguageInfo {
        match self {
            Self::Indonesian => LanguageInfo {
                id: "id",
                name: "Indonesian",
                model_name: "PP-OCRv6_rec_small.onnx",
                character_count: 18708,
                description: "Unified PP-OCRv6 model supporting 50 languages. Optimized for Indonesian documents (KTP, SIM, NPWP, receipts, invoices).",
                auto_download: true,
            },
            Self::English => LanguageInfo {
                id: "en",
                name: "English",
                model_name: "PP-OCRv6_rec_small.onnx",
                character_count: 18708,
                description: "Unified PP-OCRv6 model supporting 50 languages. Full Latin alphabet, numbers, and punctuation support.",
                auto_download: true,
            },
            Self::Multilingual => LanguageInfo {
                id: "multi",
                name: "Multilingual",
                model_name: "PP-OCRv6_rec_small.onnx",
                character_count: 18708,
                description: "Unified PP-OCRv6 model supporting 50 languages with native script recognition.",
                auto_download: true,
            },
            Self::Chinese => LanguageInfo {
                id: "ch",
                name: "Chinese",
                model_name: "PP-OCRv6_rec_small.onnx",
                character_count: 18708,
                description: "Unified PP-OCRv6 model supporting Simplified and Traditional Chinese.",
                auto_download: true,
            },
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_lowercase().as_str() {
            "id" | "ind" | "indonesia" | "indonesian" => Some(Self::Indonesian),
            "en" | "eng" | "english" => Some(Self::English),
            "multi" | "multilingual" | "all" => Some(Self::Multilingual),
            "ch" | "zh" | "chinese" | "mandarin" => Some(Self::Chinese),
            _ => None,
        }
    }
}

pub fn supported_languages() -> Vec<LanguageInfo> {
    OcrLanguage::all().iter().map(|lang| lang.info()).collect()
}

#[derive(Debug, Clone)]
pub struct OcrConfig {
    pub language: OcrLanguage,
    pub detection_model_path: PathBuf,
    pub recognition_model_path: PathBuf,
    pub dictionary_path: PathBuf,

    pub det_threshold: f32,
    pub det_box_thresh: f32,
    pub unclip_ratio: f32,
    pub det_max_side_len: u32,

    pub rec_threshold: f32,

    pub nms_iou_threshold: f32,

    pub enhancement_enabled: bool,
    pub enhancement_variance_threshold: f32,

    pub deskew_enabled: bool,
    pub second_pass_enabled: bool,
    pub second_pass_threshold: f32,

    pub intra_threads: usize,
    pub inter_threads: usize,

    pub max_image_width: u32,
    pub max_image_height: u32,

    pub max_file_size: u64,
    pub max_url_download_size: u64,
    pub url_timeout_secs: u64,

    pub pdfium_path: Option<PathBuf>,
    pub max_pdf_pages: usize,
    pub pdf_embedded_img_min_area: u32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        let dir = crate::download::default_model_dir();
        Self::default_with_model_dir(dir)
    }
}

impl OcrConfig {
    pub fn default_with_model_dir(model_dir: impl Into<PathBuf>) -> Self {
        let dir = model_dir.into();
        let detection_model_path = if dir.join("PP-OCRv6_det_small.onnx").exists() {
            dir.join("PP-OCRv6_det_small.onnx")
        } else if dir.join("ppocrv6_small_det.onnx").exists() {
            dir.join("ppocrv6_small_det.onnx")
        } else {
            dir.join("PP-OCRv6_det_small.onnx")
        };
        let recognition_model_path = if dir.join("PP-OCRv6_rec_small.onnx").exists() {
            dir.join("PP-OCRv6_rec_small.onnx")
        } else if dir.join("ppocrv6_small_rec.onnx").exists() {
            dir.join("ppocrv6_small_rec.onnx")
        } else {
            dir.join("PP-OCRv6_rec_small.onnx")
        };
        let dictionary_path = if dir.join("ppocrv6_keys.txt").exists() {
            dir.join("ppocrv6_keys.txt")
        } else {
            dir.join("ppocr_keys_v1.txt")
        };
        Self {
            language: OcrLanguage::Indonesian,
            detection_model_path,
            recognition_model_path,
            dictionary_path,

            det_threshold: 0.25,
            det_box_thresh: 0.4,
            unclip_ratio: 1.6,
            det_max_side_len: 960,

            rec_threshold: 0.4,

            nms_iou_threshold: 0.4,

            enhancement_enabled: true,
            enhancement_variance_threshold: 500.0,

            deskew_enabled: true,
            second_pass_enabled: true,
            second_pass_threshold: 0.65,

            intra_threads: 0,
            inter_threads: 0,

            max_image_width: 4096,
            max_image_height: 4096,

            max_file_size: 50 * 1024 * 1024,
            max_url_download_size: 25 * 1024 * 1024,
            url_timeout_secs: 15,

            pdfium_path: None,
            max_pdf_pages: 50,
            pdf_embedded_img_min_area: 10_000,
        }
    }

    pub fn default_with_language(model_dir: impl Into<PathBuf>, lang: OcrLanguage) -> Self {
        let dir = model_dir.into();
        let mut config = Self::default_with_model_dir(&dir);
        config.language = lang;
        config.recognition_model_path = dir.join(lang.model_filename());
        config.dictionary_path = dir.join(lang.dict_filename());
        config
    }

    pub fn with_language(mut self, lang: OcrLanguage) -> Self {
        self.language = lang;
        if let Some(parent) = self.recognition_model_path.parent().map(|p| p.to_path_buf()) {
            self.recognition_model_path = parent.join(lang.model_filename());
            self.dictionary_path = parent.join(lang.dict_filename());
        }
        self
    }

    pub fn language(self, lang: OcrLanguage) -> Self {
        self.with_language(lang)
    }

    pub fn detection_model_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.detection_model_path = path.into();
        self
    }

    pub fn recognition_model_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.recognition_model_path = path.into();
        self
    }

    pub fn dictionary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.dictionary_path = path.into();
        self
    }

    pub fn det_threshold(mut self, val: f32) -> Self {
        self.det_threshold = val;
        self
    }

    pub fn det_box_thresh(mut self, val: f32) -> Self {
        self.det_box_thresh = val;
        self
    }

    pub fn unclip_ratio(mut self, val: f32) -> Self {
        self.unclip_ratio = val;
        self
    }

    pub fn det_max_side_len(mut self, val: u32) -> Self {
        self.det_max_side_len = val;
        self
    }

    pub fn rec_threshold(mut self, val: f32) -> Self {
        self.rec_threshold = val;
        self
    }

    pub fn nms_iou_threshold(mut self, val: f32) -> Self {
        self.nms_iou_threshold = val;
        self
    }

    pub fn enable_enhancement(mut self, enabled: bool) -> Self {
        self.enhancement_enabled = enabled;
        self
    }

    pub fn enhancement_variance_threshold(mut self, val: f32) -> Self {
        self.enhancement_variance_threshold = val;
        self
    }

    pub fn deskew_enabled(mut self, enabled: bool) -> Self {
        self.deskew_enabled = enabled;
        self
    }

    pub fn second_pass_enabled(mut self, enabled: bool) -> Self {
        self.second_pass_enabled = enabled;
        self
    }

    pub fn second_pass_threshold(mut self, val: f32) -> Self {
        self.second_pass_threshold = val;
        self
    }

    pub fn intra_threads(mut self, n: usize) -> Self {
        self.intra_threads = n;
        self
    }

    pub fn inter_threads(mut self, n: usize) -> Self {
        self.inter_threads = n;
        self
    }

    pub fn max_image_width(mut self, val: u32) -> Self {
        self.max_image_width = val;
        self
    }

    pub fn max_image_height(mut self, val: u32) -> Self {
        self.max_image_height = val;
        self
    }

    pub fn max_file_size(mut self, bytes: u64) -> Self {
        self.max_file_size = bytes;
        self
    }

    pub fn max_url_download_size(mut self, bytes: u64) -> Self {
        self.max_url_download_size = bytes;
        self
    }

    pub fn url_timeout_secs(mut self, secs: u64) -> Self {
        self.url_timeout_secs = secs;
        self
    }

    pub fn pdfium_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.pdfium_path = Some(path.into());
        self
    }

    pub fn max_pdf_pages(mut self, val: usize) -> Self {
        self.max_pdf_pages = val;
        self
    }

    pub fn pdf_embedded_img_min_area(mut self, val: u32) -> Self {
        self.pdf_embedded_img_min_area = val;
        self
    }

    pub(crate) fn resolve_threads(&self) -> (usize, usize) {
        let cpus = num_cpus::get().max(1);
        let intra = if self.intra_threads == 0 {
            cpus.min(6).max(1)
        } else {
            self.intra_threads
        };
        let inter = if self.inter_threads == 0 {
            1
        } else {
            self.inter_threads
        };
        (intra, inter)
    }
}
