use crate::{
    config::OcrConfig,
    error::OcrError,
    nms::apply_nms,
    preprocessing::preprocess_detection,
    types::{Point, TextRegion},
};
use image::{DynamicImage, GrayImage};
use imageproc::contours::{find_contours_with_threshold, BorderType};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use parking_lot::Mutex;
use tracing::info;

pub struct TextDetector {
    session: Mutex<Session>,
    det_threshold: f32,
    det_box_thresh: f32,
    unclip_ratio: f32,
    max_side_len: u32,
    nms_iou_threshold: f32,
}

impl TextDetector {
    pub fn new(config: &OcrConfig) -> Result<Self, OcrError> {
        let model_path = &config.detection_model_path;
        if !model_path.exists() {
            return Err(OcrError::ModelNotFound(format!(
                "Detection model not found at: {:?}. Run scripts/download_models.ps1 or .sh",
                model_path
            )));
        }

        info!(
            "Loading PP-OCRv6 text detection model from {:?} ...",
            model_path
        );

        let (intra, inter) = config.resolve_threads();

        let session = Session::builder()
            .map_err(|e| OcrError::ModelLoadFailed(format!("SessionBuilder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Optimization level: {}", e)))?
            .with_intra_threads(intra)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Intra threads: {}", e)))?
            .with_inter_threads(inter)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Inter threads: {}", e)))?
            .with_parallel_execution(true)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Parallel execution: {}", e)))?
            .commit_from_file(model_path)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Load detection model: {}", e)))?;

        info!(
            "PP-OCRv6 detection model loaded (intra_threads={}, inter_threads={}, parallel=true)",
            intra, inter
        );

        Ok(Self {
            session: Mutex::new(session),
            det_threshold: config.det_threshold,
            det_box_thresh: config.det_box_thresh,
            unclip_ratio: config.unclip_ratio,
            max_side_len: config.det_max_side_len,
            nms_iou_threshold: config.nms_iou_threshold,
        })
    }

    pub fn detect(&self, img: &DynamicImage) -> Result<Vec<TextRegion>, OcrError> {
        let (orig_w, orig_h) = (img.width(), img.height());
        let preprocessed = preprocess_detection(img, self.max_side_len);

        let input_tensor = Tensor::from_array(preprocessed.tensor)
            .map_err(|e| OcrError::InferenceFailed(format!("Input tensor: {}", e)))?;

        let target_h = preprocessed.target_h as usize;
        let target_w = preprocessed.target_w as usize;
        let pixel_count = target_h * target_w;
        let det_thresh = self.det_threshold;

        let (prob_vec, binary_img) = {
            let mut session_guard = self.session.lock();
            let outputs = session_guard
                .run(ort::inputs!["x" => input_tensor])
                .map_err(|e| OcrError::InferenceFailed(format!("Detection: {}", e)))?;

            let output_val = outputs
                .iter()
                .next()
                .map(|(_, v)| v)
                .ok_or_else(|| OcrError::InferenceFailed("Empty detection output".to_string()))?;

            let pred_view = output_val
                .try_extract_array::<f32>()
                .map_err(|e| OcrError::InferenceFailed(format!("Extract array: {}", e)))?;

            let raw_slice = pred_view.as_slice().unwrap_or(&[]);

            let prob_vec: Vec<f32> = if raw_slice.len() >= pixel_count {
                raw_slice[..pixel_count].to_vec()
            } else {
                raw_slice.to_vec()
            };

            let mut binary_img = GrayImage::new(target_w as u32, target_h as u32);
            let binary_raw = binary_img.as_mut();
            let limit = pixel_count.min(prob_vec.len()).min(binary_raw.len());
            for i in 0..limit {
                if prob_vec[i] >= det_thresh {
                    unsafe { *binary_raw.get_unchecked_mut(i) = 255; }
                }
            }

            (prob_vec, binary_img)
        };

        let contours = find_contours_with_threshold::<u32>(&binary_img, 128);
        let binary_raw = binary_img.as_ref();
        let mut regions = Vec::with_capacity(contours.len() / 2);

        for contour in contours {
            if contour.border_type != BorderType::Outer || contour.points.len() < 3 {
                continue;
            }

            let mut min_x = u32::MAX;
            let mut min_y = u32::MAX;
            let mut max_x = u32::MIN;
            let mut max_y = u32::MIN;

            for pt in &contour.points {
                if pt.x < min_x { min_x = pt.x; }
                if pt.x > max_x { max_x = pt.x; }
                if pt.y < min_y { min_y = pt.y; }
                if pt.y > max_y { max_y = pt.y; }
            }

            let box_w = (max_x.saturating_sub(min_x)) as f32;
            let box_h = (max_y.saturating_sub(min_y)) as f32;

            if box_w < 3.0 || box_h < 3.0 {
                continue;
            }

            let pts = &contour.points;
            let n = pts.len();
            let mut area = 0.0f32;
            let mut perimeter = 0.0f32;

            for i in 0..n {
                let j = (i + 1) % n;
                let xi = pts[i].x as f32;
                let yi = pts[i].y as f32;
                let xj = pts[j].x as f32;
                let yj = pts[j].y as f32;

                area += xi * yj - xj * yi;
                perimeter += ((xj - xi).powi(2) + (yj - yi).powi(2)).sqrt();
            }
            let area = (area.abs() * 0.5).max(1.0);
            let perimeter = perimeter.max(1.0);

            let clamped_max_y = max_y.min(target_h as u32 - 1);
            let clamped_max_x = max_x.min(target_w as u32 - 1);

            let mut score_sum = 0.0f32;
            let mut score_count = 0usize;
            for cy in min_y..=clamped_max_y {
                let row_base = cy as usize * target_w;
                for cx in min_x..=clamped_max_x {
                    let idx = row_base + cx as usize;
                    if idx < binary_raw.len() && unsafe { *binary_raw.get_unchecked(idx) } > 0 {
                        if idx < prob_vec.len() {
                            score_sum += unsafe { *prob_vec.get_unchecked(idx) };
                        }
                        score_count += 1;
                    }
                }
            }

            let confidence = if score_count > 0 {
                score_sum / score_count as f32
            } else {
                0.0
            };

            if confidence < self.det_box_thresh {
                continue;
            }

            let distance = (area * self.unclip_ratio) / perimeter;

            let exp_min_x = (min_x as f32 - distance).max(0.0);
            let exp_min_y = (min_y as f32 - distance).max(0.0);
            let exp_max_x = (max_x as f32 + distance).min(target_w as f32 - 1.0);
            let exp_max_y = (max_y as f32 + distance).min(target_h as f32 - 1.0);

            let orig_min_x = (exp_min_x * preprocessed.scale_x).clamp(0.0, orig_w as f32);
            let orig_min_y = (exp_min_y * preprocessed.scale_y).clamp(0.0, orig_h as f32);
            let orig_max_x = (exp_max_x * preprocessed.scale_x).clamp(0.0, orig_w as f32);
            let orig_max_y = (exp_max_y * preprocessed.scale_y).clamp(0.0, orig_h as f32);

            let polygon = vec![
                Point::new(orig_min_x, orig_min_y),
                Point::new(orig_max_x, orig_min_y),
                Point::new(orig_max_x, orig_max_y),
                Point::new(orig_min_x, orig_max_y),
            ];

            regions.push(TextRegion::new(polygon, confidence));
        }

        let regions = apply_nms(regions, self.nms_iou_threshold);

        Ok(regions)
    }
}
