use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct BoundingBox {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl BoundingBox {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    pub fn width(&self) -> f32 {
        (self.max_x - self.min_x).max(0.0)
    }

    pub fn height(&self) -> f32 {
        (self.max_y - self.min_y).max(0.0)
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn center(&self) -> (f32, f32) {
        ((self.min_x + self.max_x) / 2.0, (self.min_y + self.max_y) / 2.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRegion {
    pub polygon: Vec<Point>,
    pub confidence: f32,
}

impl TextRegion {
    pub fn new(polygon: Vec<Point>, confidence: f32) -> Self {
        Self { polygon, confidence }
    }

    pub fn aabb(&self) -> (f32, f32, f32, f32) {
        if self.polygon.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for pt in &self.polygon {
            if pt.x < min_x { min_x = pt.x; }
            if pt.x > max_x { max_x = pt.x; }
            if pt.y < min_y { min_y = pt.y; }
            if pt.y > max_y { max_y = pt.y; }
        }

        (min_x, min_y, max_x, max_y)
    }

    pub fn bbox(&self) -> BoundingBox {
        let (min_x, min_y, max_x, max_y) = self.aabb();
        BoundingBox::new(min_x, min_y, max_x, max_y)
    }

    pub fn center_y(&self) -> f32 {
        let (_, min_y, _, max_y) = self.aabb();
        (min_y + max_y) / 2.0
    }

    pub fn height(&self) -> f32 {
        let (_, min_y, _, max_y) = self.aabb();
        (max_y - min_y).max(1.0)
    }

    pub fn width(&self) -> f32 {
        let (min_x, _, max_x, _) = self.aabb();
        (max_x - min_x).max(1.0)
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrText {
    pub text: String,
    pub confidence: f32,
    pub region: TextRegion,
    #[serde(default)]
    pub line_number: usize,
    #[serde(default)]
    pub bbox: BoundingBox,
}

impl OcrText {
    pub fn new(text: String, confidence: f32, region: TextRegion) -> Self {
        let bbox = region.bbox();
        Self {
            text,
            confidence,
            region,
            line_number: 1,
            bbox,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    pub line_number: usize,
    pub text: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub words: Vec<OcrText>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    Jpeg,
    Png,
    WebP,
    Bmp,
    Tiff,
    Pdf,
    #[default]
    Unknown,
}

impl DocumentFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Pdf => "pdf",
            Self::Unknown => "unknown",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Pdf => "application/pdf",
            Self::Unknown => "application/octet-stream",
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub lines: Vec<TextLine>,
    pub regions: Vec<OcrText>,
    pub dimensions: (u32, u32),
    pub format: DocumentFormat,
    pub duration_ms: u64,
}

impl OcrResult {
    pub fn new(
        text: String,
        confidence: f32,
        lines: Vec<TextLine>,
        regions: Vec<OcrText>,
        dimensions: (u32, u32),
        format: DocumentFormat,
        duration_ms: u64,
    ) -> Self {
        Self {
            text,
            confidence,
            lines,
            regions,
            dimensions,
            format,
            duration_ms,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    pub fn lines(&self) -> &[TextLine] {
        &self.lines
    }

    pub fn words(&self) -> Vec<&str> {
        self.regions.iter().map(|r| r.text.as_str()).collect()
    }

    pub fn filter_by_confidence(&self, min_conf: f32) -> Self {
        let filtered_regions: Vec<OcrText> = self
            .regions
            .iter()
            .filter(|r| r.confidence >= min_conf)
            .cloned()
            .collect();

        let filtered_lines: Vec<TextLine> = self
            .lines
            .iter()
            .filter(|l| l.confidence >= min_conf)
            .cloned()
            .collect();

        let text = filtered_lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        let conf = if filtered_regions.is_empty() {
            0.0
        } else {
            filtered_regions.iter().map(|r| r.confidence).sum::<f32>() / filtered_regions.len() as f32
        };

        Self {
            text,
            confidence: conf,
            lines: filtered_lines,
            regions: filtered_regions,
            dimensions: self.dimensions,
            format: self.format,
            duration_ms: self.duration_ms,
        }
    }

    pub fn search(&self, query: &str) -> Vec<&OcrText> {
        let query_lower = query.to_lowercase();
        self.regions
            .iter()
            .filter(|r| r.text.to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn to_tsv(&self) -> String {
        let mut out = String::from("line\tconfidence\tmin_x\tmin_y\tmax_x\tmax_y\ttext\n");
        for r in &self.regions {
            let bbox = r.region.bbox();
            out.push_str(&format!(
                "{}\t{:.4}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{}\n",
                r.line_number, r.confidence, bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y, r.text
            ));
        }
        out
    }
}

impl fmt::Display for OcrResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

pub type RecognitionResult = OcrResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub source: String,
    pub success: bool,
    pub result: Option<OcrResult>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchResult {
    pub items: Vec<BatchItem>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
}

impl BatchResult {
    pub fn texts(&self) -> Vec<String> {
        self.items
            .iter()
            .filter_map(|item| item.result.as_ref().map(|r| r.text.clone()))
            .collect()
    }

    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSource {
    NativeText,
    Ocr,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub page_number: usize,
    pub source: TextSource,
    pub text: String,
    pub confidence: f32,
    pub regions: Vec<OcrText>,
    pub lines: Vec<TextLine>,
    pub dimensions: (u32, u32),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PdfResult {
    pub pages: Vec<PageResult>,
    pub total_pages: usize,
    pub text: String,
    pub confidence: f32,
    pub duration_ms: u64,
}

impl PdfResult {
    pub fn page(&self, num: usize) -> Option<&PageResult> {
        self.pages.iter().find(|p| p.page_number == num)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}
