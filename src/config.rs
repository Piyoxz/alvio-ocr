use std::path::PathBuf;

/// Language presets for OCR text recognition.
///
/// Text detection (`ch_PP-OCRv4_det_infer.onnx`) is language-agnostic and detects
/// bounding boxes for any written script. The recognition model and dictionary determine
/// the recognized character set.
///
/// # Supported Presets
/// - [`OcrLanguage::Indonesian`] / [`OcrLanguage::English`]:
///   Uses the Latin alphabet PP-OCRv4 model (`en_PP-OCRv4_rec_infer.onnx` + `en_dict.txt`).
///   Fastest inference (~97 classes: A-Z, a-z, 0-9, standard symbols).
///   Ideal for Indonesian documents (KTP, SIM, NPWP, receipts, official letters) and English text.
///
/// - [`OcrLanguage::Multilingual`] / [`OcrLanguage::Chinese`]:
///   Uses the universal PP-OCRv4 model (`ch_PP-OCRv4_rec_infer.onnx` + `ppocr_keys_v1.txt`).
///   Recognizes 6,625+ characters including Chinese (Hanzi), Japanese (Kanji), Latin alphabet
///   (Indonesian/English), numbers, and mathematical symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrLanguage {
    /// Indonesian (Latin alphabet, fast, default).
    #[default]
    Indonesian,
    /// English (alias for Indonesian/Latin alphabet).
    English,
    /// Multilingual / Universal (6,625+ characters, Latin + Chinese + Kanji + symbols).
    Multilingual,
    /// Chinese (Traditional & Simplified, 6,625+ characters).
    Chinese,
}

impl OcrLanguage {
    /// Returns the recommended recognition ONNX model filename for this language preset.
    pub fn model_filename(&self) -> &'static str {
        match self {
            Self::Indonesian | Self::English => "en_PP-OCRv4_rec_infer.onnx",
            Self::Multilingual | Self::Chinese => "ch_PP-OCRv4_rec_infer.onnx",
        }
    }

    /// Returns the character dictionary filename for this language preset.
    pub fn dict_filename(&self) -> &'static str {
        match self {
            Self::Indonesian | Self::English => "en_dict.txt",
            Self::Multilingual | Self::Chinese => "ppocr_keys_v1.txt",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Indonesian => "Indonesian (Latin)",
            Self::English => "English (Latin)",
            Self::Multilingual => "Multilingual (Latin + CJK + Symbols)",
            Self::Chinese => "Chinese (Hanzi)",
        }
    }

    /// Parse a language code string (e.g. "id", "en", "multi", "ch", "zh").
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

/// Configuration for the OCR engine.
///
/// Use [`OcrConfig::default_with_model_dir`] for quick setup, or the builder
/// methods for fine-grained control.
///
/// # Example
/// ```no_run
/// use alvio_ocr::{OcrConfig, OcrLanguage};
///
/// // Quick setup — default to Indonesian/English
/// let config = OcrConfig::default_with_model_dir("./models/ocr");
///
/// // Quick setup — with Multilingual support
/// let config = OcrConfig::default_with_language("./models/ocr", OcrLanguage::Multilingual);
///
/// // Fine-tuned setup
/// let config = OcrConfig::default_with_model_dir("./models/ocr")
///     .det_max_side_len(1280)
///     .det_threshold(0.25)
///     .rec_threshold(0.4)
///     .enable_enhancement(true);
/// ```
#[derive(Debug, Clone)]
pub struct OcrConfig {
    // --- Model paths ---
    /// Selected language preset.
    pub language: OcrLanguage,
    /// Path to the DBNet detection ONNX model.
    pub detection_model_path: PathBuf,
    /// Path to the SVTR recognition ONNX model.
    pub recognition_model_path: PathBuf,
    /// Path to the character dictionary file.
    pub dictionary_path: PathBuf,

    // --- Detection parameters ---
    /// Binarization threshold for the detection probability map. Default: 0.3.
    pub det_threshold: f32,
    /// Minimum confidence for a detected bounding box. Default: 0.5.
    pub det_box_thresh: f32,
    /// Expansion ratio for detected text boxes (unclip). Default: 1.5.
    pub unclip_ratio: f32,
    /// Maximum side length for detection input resize. Higher = more accurate but slower.
    /// Default: **960** (optimized from 736).
    pub det_max_side_len: u32,

    // --- Recognition parameters ---
    /// Minimum confidence for a recognized text to be included. Default: 0.5.
    pub rec_threshold: f32,

    // --- NMS parameters ---
    /// IoU threshold for Non-Maximum Suppression. Default: 0.5.
    pub nms_iou_threshold: f32,

    // --- Enhancement ---
    /// Enable adaptive image enhancement (auto-contrast + sharpen) for low-quality inputs.
    /// Default: true.
    pub enhancement_enabled: bool,
    /// Variance threshold below which enhancement is applied. Default: 1500.0.
    pub enhancement_variance_threshold: f32,

    // --- Threading ---
    /// Number of intra-op threads for ONNX. 0 = auto (num_cpus / 2). Default: 0.
    pub intra_threads: usize,
    /// Number of inter-op threads for ONNX parallel execution. 0 = auto. Default: 0.
    pub inter_threads: usize,

    // --- Limits ---
    /// Maximum image width allowed. Default: 4096.
    pub max_image_width: u32,
    /// Maximum image height allowed. Default: 4096.
    pub max_image_height: u32,

    // --- PDF-specific (only with "pdf" feature) ---
    /// Path to the pdfium native library (optional, auto-detected if None).
    pub pdfium_path: Option<PathBuf>,
    /// Maximum number of PDF pages to process. Default: 50.
    pub max_pdf_pages: usize,
    /// Minimum area (w*h) for an embedded PDF image to be OCR'd. Default: 10000.
    pub pdf_embedded_img_min_area: u32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        let dir = crate::download::default_model_dir();
        Self::default_with_model_dir(dir)
    }
}

impl OcrConfig {
    /// Create a config with default parameters, pointing at a model directory.
    ///
    /// Expects the following files inside `model_dir`:
    /// - `ch_PP-OCRv4_det_infer.onnx` (detection model)
    /// - `en_PP-OCRv4_rec_infer.onnx` (recognition model)
    /// - `en_dict.txt` (character dictionary)
    pub fn default_with_model_dir(model_dir: impl Into<PathBuf>) -> Self {
        let dir = model_dir.into();
        Self {
            language: OcrLanguage::Indonesian,
            detection_model_path: dir.join("ch_PP-OCRv4_det_infer.onnx"),
            recognition_model_path: dir.join("en_PP-OCRv4_rec_infer.onnx"),
            dictionary_path: dir.join("en_dict.txt"),

            // Detection — A1: Higher resolution default
            det_threshold: 0.3,
            det_box_thresh: 0.5,
            unclip_ratio: 1.5,
            det_max_side_len: 960,

            // Recognition
            rec_threshold: 0.5,

            // NMS — A5
            nms_iou_threshold: 0.5,

            // Enhancement — A4
            enhancement_enabled: true,
            enhancement_variance_threshold: 1500.0,

            // Threading — S3: will be resolved to actual values at engine init
            intra_threads: 0,
            inter_threads: 0,

            // Limits
            max_image_width: 4096,
            max_image_height: 4096,

            // PDF
            pdfium_path: None,
            max_pdf_pages: 50,
            pdf_embedded_img_min_area: 10_000,
        }
    }

    /// Create a config with a specific language preset pointing at a model directory.
    ///
    /// # Example
    /// ```no_run
    /// use alvio_ocr::{OcrConfig, OcrLanguage};
    ///
    /// // Indonesian & English (fastest Latin model, default)
    /// let config = OcrConfig::default_with_language("./models/ocr", OcrLanguage::Indonesian);
    ///
    /// // Multilingual (Latin + Chinese + Kanji + symbols)
    /// let config = OcrConfig::default_with_language("./models/ocr", OcrLanguage::Multilingual);
    /// ```
    pub fn default_with_language(model_dir: impl Into<PathBuf>, lang: OcrLanguage) -> Self {
        let dir = model_dir.into();
        let mut config = Self::default_with_model_dir(&dir);
        config.language = lang;
        config.recognition_model_path = dir.join(lang.model_filename());
        config.dictionary_path = dir.join(lang.dict_filename());
        config
    }

    // --- Builder methods ---

    /// Switch language preset, updating `recognition_model_path` and `dictionary_path`
    /// relative to the current model directory.
    pub fn with_language(mut self, lang: OcrLanguage) -> Self {
        self.language = lang;
        if let Some(parent) = self.recognition_model_path.parent().map(|p| p.to_path_buf()) {
            self.recognition_model_path = parent.join(lang.model_filename());
            self.dictionary_path = parent.join(lang.dict_filename());
        }
        self
    }

    /// Set language preset explicitly.
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

    /// Resolve auto-detect values (threads = 0 → actual CPU count).
    pub(crate) fn resolve_threads(&self) -> (usize, usize) {
        let cpus = num_cpus::get().max(1);
        let intra = if self.intra_threads == 0 {
            (cpus / 2).max(1)
        } else {
            self.intra_threads
        };
        let inter = if self.inter_threads == 0 {
            cpus.max(1)
        } else {
            self.inter_threads
        };
        (intra, inter)
    }
}
