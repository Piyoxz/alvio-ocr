use serde::{Deserialize, Serialize};

/// A 2D point with floating-point coordinates.
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

/// A detected text region defined by a polygon and a detection confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRegion {
    /// Polygon vertices (typically 4 points for a quadrilateral).
    pub polygon: Vec<Point>,
    /// Detection confidence score (0.0–1.0).
    pub confidence: f32,
}

impl TextRegion {
    pub fn new(polygon: Vec<Point>, confidence: f32) -> Self {
        Self { polygon, confidence }
    }

    /// Compute axis-aligned bounding box: (min_x, min_y, max_x, max_y).
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

    /// Vertical center of the bounding box.
    pub fn center_y(&self) -> f32 {
        let (_, min_y, _, max_y) = self.aabb();
        (min_y + max_y) / 2.0
    }

    /// Height of the bounding box (minimum 1.0).
    pub fn height(&self) -> f32 {
        let (_, min_y, _, max_y) = self.aabb();
        (max_y - min_y).max(1.0)
    }

    /// Width of the bounding box (minimum 1.0).
    pub fn width(&self) -> f32 {
        let (min_x, _, max_x, _) = self.aabb();
        (max_x - min_x).max(1.0)
    }

    /// Area of the bounding box.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

/// A recognized text item: the text string, its confidence, and the region it came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrText {
    /// The recognized text content.
    pub text: String,
    /// Recognition confidence score (0.0–1.0).
    pub confidence: f32,
    /// The text region where this text was found.
    pub region: TextRegion,
}

/// Result of OCR on a single image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecognitionResult {
    /// The full combined text, lines separated by newlines.
    pub text: String,
    /// Individual recognized text regions with positions and confidence.
    pub regions: Vec<OcrText>,
}

/// Source of text extraction for a PDF page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSource {
    /// Text was extracted from the PDF's native text layer.
    NativeText,
    /// Text was obtained via OCR.
    Ocr,
    /// Combination of native text and OCR results.
    Mixed,
}

/// OCR result for a single PDF page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    /// 1-based page number.
    pub page_number: usize,
    /// How the text was obtained.
    pub source: TextSource,
    /// The extracted/recognized text.
    pub text: String,
    /// Individual OCR text regions (empty if source is NativeText).
    pub regions: Vec<OcrText>,
}
