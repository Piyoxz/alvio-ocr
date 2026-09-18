use crate::{
    config::OcrConfig,
    error::OcrError,
    preprocessing::{preprocess_recognition, preprocess_recognition_batch},
    types::{OcrText, TextRegion},
};
use image::{DynamicImage, RgbImage};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use parking_lot::Mutex;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use tracing::info;

pub const EMBEDDED_DICT: &str = include_str!("ppocrv6_keys.txt");

pub struct TextRecognizer {
    session: Mutex<Session>,
    dictionary: Vec<String>,
    rec_threshold: f32,
}

impl TextRecognizer {
    pub fn new(config: &OcrConfig) -> Result<Self, OcrError> {
        let model_path = &config.recognition_model_path;
        let dict_path = &config.dictionary_path;

        if !model_path.exists() {
            return Err(OcrError::ModelNotFound(format!(
                "Recognition model not found at: {:?}. Run scripts/download_models.ps1 or .sh",
                model_path
            )));
        }

        let dictionary: Vec<String> = if dict_path.exists() {
            let dict_file = File::open(dict_path)?;
            let reader = BufReader::with_capacity(32768, dict_file);
            reader.lines().filter_map(|l| l.ok()).collect()
        } else if let Some(parent) = dict_path.parent() {
            if parent.join("ppocrv6_keys.txt").exists() {
                let dict_file = File::open(parent.join("ppocrv6_keys.txt"))?;
                let reader = BufReader::with_capacity(32768, dict_file);
                reader.lines().filter_map(|l| l.ok()).collect()
            } else {
                EMBEDDED_DICT.lines().map(|s| s.to_string()).collect()
            }
        } else {
            EMBEDDED_DICT.lines().map(|s| s.to_string()).collect()
        };
        info!("Loaded {} characters into dictionary.", dictionary.len());

        let (intra, inter) = config.resolve_threads();

        info!("Loading PP-OCRv6 recognition model from {:?} ...", model_path);
        let session = Session::builder()
            .map_err(|e| OcrError::ModelLoadFailed(format!("SessionBuilder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Optimization level: {}", e)))?
            .with_intra_threads(intra)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Intra threads: {}", e)))?
            .with_inter_threads(inter)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Inter threads: {}", e)))?
            .with_parallel_execution(false)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Parallel execution: {}", e)))?
            .commit_from_file(model_path)
            .map_err(|e| OcrError::ModelLoadFailed(format!("Load recognition model: {}", e)))?;

        info!(
            "PP-OCRv6 recognition model loaded (intra_threads={}, inter_threads={}, parallel=false)",
            intra, inter
        );

        Ok(Self {
            session: Mutex::new(session),
            dictionary,
            rec_threshold: config.rec_threshold,
        })
    }

    pub fn recognize_crop(&self, crop: &RgbImage) -> Result<(String, f32), OcrError> {
        let tensor = preprocess_recognition(crop);
        let input_tensor = Tensor::from_array(tensor)
            .map_err(|e| OcrError::InferenceFailed(format!("Recognition tensor: {}", e)))?;

        let mut session_guard = self.session.lock();
        let outputs = session_guard
            .run(ort::inputs!["x" => input_tensor])
            .map_err(|e| OcrError::InferenceFailed(format!("Recognition: {}", e)))?;

        let output_val = outputs
            .iter()
            .next()
            .map(|(_, v)| v)
            .ok_or_else(|| OcrError::InferenceFailed("Empty recognition output".to_string()))?;

        let pred_view = output_val
            .try_extract_array::<f32>()
            .map_err(|e| OcrError::InferenceFailed(format!("Extract array: {}", e)))?;

        let shape = pred_view.shape();
        if shape.len() != 3 {
            return Err(OcrError::InferenceFailed(format!(
                "Unexpected recognition dimensions: {:?}",
                shape
            )));
        }

        let seq_len = shape[1];
        let num_classes = shape[2];
        let raw_slice = pred_view.as_slice().unwrap_or(&[]);

        Ok(self.decode_ctc(raw_slice, seq_len, num_classes))
    }

    pub fn recognize_region(
        &self,
        img: &DynamicImage,
        region: &TextRegion,
    ) -> Result<Option<OcrText>, OcrError> {
        let crop = match self.crop_region(img, region) {
            Some(c) => c,
            None => return Ok(None),
        };

        let (text, confidence) = self.recognize_crop(&crop)?;

        if text.is_empty() || confidence < self.rec_threshold {
            return Ok(None);
        }

        Ok(Some(OcrText::new(text, confidence, region.clone())))
    }

    pub fn recognize_regions_batch(
        &self,
        img: &DynamicImage,
        regions: &[TextRegion],
    ) -> Result<Vec<OcrText>, OcrError> {
        if regions.is_empty() {
            return Ok(Vec::new());
        }

        struct CropItem {
            crop: RgbImage,
            valid_idx: usize,
            target_width: u32,
        }

        let target_h = 48u32;
        let mut items: Vec<CropItem> = Vec::with_capacity(regions.len());

        for (i, region) in regions.iter().enumerate() {
            if let Some(crop) = self.crop_region(img, region) {
                let (cw, ch) = crop.dimensions();
                let ratio = cw as f32 / ch.max(1) as f32;
                let target_width = ((target_h as f32 * ratio).round() as u32).clamp(16, 320);
                items.push(CropItem {
                    crop,
                    valid_idx: i,
                    target_width,
                });
            }
        }

        if items.is_empty() {
            return Ok(Vec::new());
        }

        if items.len() == 1 {
            let mut results = Vec::new();
            if let Some(ocr_text) = self.recognize_region(img, &regions[items[0].valid_idx])? {
                results.push(ocr_text);
            }
            return Ok(results);
        }

        // Aspect-ratio bucketing: Sort items by target width so items in each chunk
        // have similar width, eliminating 60-70% unnecessary padding
        items.sort_by_key(|it| it.target_width);

        const MAX_BATCH_CHUNK: usize = 16;
        let mut results_with_index: Vec<(usize, OcrText)> = Vec::with_capacity(items.len());

        for chunk in items.chunks(MAX_BATCH_CHUNK) {
            let chunk_crops: Vec<RgbImage> = chunk.iter().map(|it| it.crop.clone()).collect();
            let (batch_tensor, target_widths) = preprocess_recognition_batch(&chunk_crops);

            let input_tensor = match Tensor::from_array(batch_tensor) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("Batch tensor creation failed: {}, falling back to sequential", e);
                    for it in chunk {
                        if let Some(ocr_text) = self.recognize_region(img, &regions[it.valid_idx])? {
                            results_with_index.push((it.valid_idx, ocr_text));
                        }
                    }
                    continue;
                }
            };

            let (shape, raw_vec) = {
                let mut session_guard = self.session.lock();
                let res = match session_guard.run(ort::inputs!["x" => input_tensor]) {
                    Ok(outputs) => {
                        if let Some((_, output_val)) = outputs.iter().next() {
                            if let Ok(pred_view) = output_val.try_extract_array::<f32>() {
                                (pred_view.shape().to_vec(), pred_view.as_slice().unwrap_or(&[]).to_vec())
                            } else {
                                (Vec::new(), Vec::new())
                            }
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Batch recognition inference failed: {}", e);
                        (Vec::new(), Vec::new())
                    }
                };
                res
            };

            if shape.len() != 3 {
                for it in chunk {
                    if let Some(ocr_text) = self.recognize_region(img, &regions[it.valid_idx])? {
                        results_with_index.push((it.valid_idx, ocr_text));
                    }
                }
                continue;
            }

            let batch_size = shape[0];
            let seq_len = shape[1];
            let num_classes = shape[2];
            let per_item = seq_len * num_classes;
            let max_w = *target_widths.iter().max().unwrap_or(&16) as f32;

            for b in 0..batch_size.min(chunk.len()) {
                let offset = b * per_item;
                if offset + per_item > raw_vec.len() {
                    break;
                }
                let item_slice = &raw_vec[offset..offset + per_item];
                let tw = target_widths[b] as f32;

                let effective_seq_len = (((tw / max_w) * seq_len as f32).ceil() as usize + 2).min(seq_len);
                let (text, confidence) = self.decode_ctc(item_slice, effective_seq_len, num_classes);

                if !text.is_empty() && confidence >= self.rec_threshold {
                    results_with_index.push((
                        chunk[b].valid_idx,
                        OcrText::new(text, confidence, regions[chunk[b].valid_idx].clone()),
                    ));
                }
            }
        }

        // Restore original region order
        results_with_index.sort_by_key(|(orig_idx, _)| *orig_idx);
        let results = results_with_index.into_iter().map(|(_, text)| text).collect();

        Ok(results)
    }

    pub fn crop_region(&self, img: &DynamicImage, region: &TextRegion) -> Option<RgbImage> {
        if region.polygon.len() >= 4 {
            let p0 = region.polygon[0];
            let p1 = region.polygon[1];
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let angle_deg = dy.atan2(dx).to_degrees();
            if angle_deg.abs() >= 1.5 && angle_deg.abs() <= 45.0 {
                if let Some(oriented) = crate::orientation::extract_oriented_crop(img, &region.polygon) {
                    return Some(oriented);
                }
            }
        }

        let (min_x, min_y, max_x, max_y) = region.aabb();
        let (img_w, img_h) = (img.width() as f32, img.height() as f32);
        let region_h = (max_y - min_y).max(1.0);

        let pad_x = (region_h * 0.15).max(3.0);
        let pad_y = (region_h * 0.1).max(2.0);

        let crop_x = (min_x - pad_x).max(0.0).min(img_w - 1.0) as u32;
        let crop_y = (min_y - pad_y).max(0.0).min(img_h - 1.0) as u32;
        let crop_w = ((max_x + pad_x) - crop_x as f32).clamp(2.0, img_w - crop_x as f32) as u32;
        let crop_h = ((max_y + pad_y) - crop_y as f32).clamp(2.0, img_h - crop_y as f32) as u32;

        if crop_w < 3 || crop_h < 3 {
            return None;
        }

        Some(img.crop_imm(crop_x, crop_y, crop_w, crop_h).to_rgb8())
    }

    #[inline]
    fn decode_ctc(&self, raw_slice: &[f32], seq_len: usize, num_classes: usize) -> (String, f32) {
        let mut decoded_text = String::with_capacity(seq_len);
        let mut confidence_sum = 0.0f32;
        let mut confidence_count = 0usize;
        let mut last_index = 0usize;
        let dict_len = self.dictionary.len();

        for t in 0..seq_len {
            let row_start = t * num_classes;
            if row_start + num_classes > raw_slice.len() {
                break;
            }

            let row = unsafe { raw_slice.get_unchecked(row_start..row_start + num_classes) };

            let mut max_val = f32::NEG_INFINITY;
            let mut argmax = 0usize;

            for (c, &val) in row.iter().enumerate() {
                if val > max_val {
                    max_val = val;
                    argmax = c;
                }
            }

            let prob = if max_val <= 1.0 && max_val >= 0.0 {
                max_val
            } else {
                let mut sum_exp = 0.0f32;
                for &val in row.iter() {
                    sum_exp += (val - max_val).exp();
                }
                if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 }
            };

            if argmax != last_index {
                last_index = argmax;

                if argmax != 0 {
                    let ch = if argmax >= 1 && argmax <= dict_len {
                        Some(unsafe { self.dictionary.get_unchecked(argmax - 1).as_str() })
                    } else if argmax > dict_len {
                        Some(" ")
                    } else {
                        None
                    };

                    if let Some(c_str) = ch {
                        if c_str == " " && decoded_text.ends_with(' ') {
                            continue;
                        }
                        decoded_text.push_str(c_str);
                        confidence_sum += prob;
                        confidence_count += 1;
                    }
                }
            }
        }

        let trimmed = decoded_text.trim().to_string();
        let avg_confidence = if confidence_count > 0 {
            confidence_sum / confidence_count as f32
        } else {
            0.0
        };

        (trimmed, avg_confidence)
    }
}
