//! Non-Maximum Suppression (NMS) for text detection regions.
//!
//! Removes overlapping bounding boxes, keeping only the most confident ones.
//! This both improves accuracy (no duplicate text) and speed (fewer recognition calls).

use crate::types::TextRegion;

/// Compute Intersection-over-Union (IoU) between two axis-aligned bounding boxes.
fn iou(a: &TextRegion, b: &TextRegion) -> f32 {
    let (a_x1, a_y1, a_x2, a_y2) = a.aabb();
    let (b_x1, b_y1, b_x2, b_y2) = b.aabb();

    let inter_x1 = a_x1.max(b_x1);
    let inter_y1 = a_y1.max(b_y1);
    let inter_x2 = a_x2.min(b_x2);
    let inter_y2 = a_y2.min(b_y2);

    let inter_w = (inter_x2 - inter_x1).max(0.0);
    let inter_h = (inter_y2 - inter_y1).max(0.0);
    let inter_area = inter_w * inter_h;

    let a_area = a.area();
    let b_area = b.area();
    let union_area = a_area + b_area - inter_area;

    if union_area <= 0.0 {
        0.0
    } else {
        inter_area / union_area
    }
}

/// Apply greedy Non-Maximum Suppression to a list of text regions.
///
/// Regions are sorted by confidence (descending). For each region, all remaining
/// regions with IoU above `iou_threshold` are suppressed (removed).
///
/// # Arguments
/// * `regions` — detected text regions
/// * `iou_threshold` — IoU threshold above which a region is suppressed (typical: 0.5)
pub fn apply_nms(mut regions: Vec<TextRegion>, iou_threshold: f32) -> Vec<TextRegion> {
    if regions.len() <= 1 {
        return regions;
    }

    // Sort by confidence descending
    regions.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = vec![true; regions.len()];
    let mut result = Vec::with_capacity(regions.len());

    for i in 0..regions.len() {
        if !keep[i] {
            continue;
        }

        for j in (i + 1)..regions.len() {
            if !keep[j] {
                continue;
            }
            if iou(&regions[i], &regions[j]) > iou_threshold {
                keep[j] = false;
            }
        }
    }

    for (i, region) in regions.into_iter().enumerate() {
        if keep[i] {
            result.push(region);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Point;

    fn rect(x1: f32, y1: f32, x2: f32, y2: f32, conf: f32) -> TextRegion {
        TextRegion::new(
            vec![
                Point::new(x1, y1),
                Point::new(x2, y1),
                Point::new(x2, y2),
                Point::new(x1, y2),
            ],
            conf,
        )
    }

    #[test]
    fn test_nms_removes_overlapping() {
        let regions = vec![
            rect(0.0, 0.0, 100.0, 50.0, 0.9),
            rect(5.0, 2.0, 105.0, 52.0, 0.7), // heavily overlaps with first
            rect(200.0, 200.0, 300.0, 250.0, 0.8), // no overlap
        ];
        let result = apply_nms(regions, 0.5);
        assert_eq!(result.len(), 2);
        assert!((result[0].confidence - 0.9).abs() < 0.01);
        assert!((result[1].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_nms_keeps_non_overlapping() {
        let regions = vec![
            rect(0.0, 0.0, 50.0, 50.0, 0.9),
            rect(100.0, 100.0, 150.0, 150.0, 0.8),
            rect(200.0, 200.0, 250.0, 250.0, 0.7),
        ];
        let result = apply_nms(regions, 0.5);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_nms_empty() {
        let result = apply_nms(Vec::new(), 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_nms_single() {
        let regions = vec![rect(0.0, 0.0, 100.0, 50.0, 0.9)];
        let result = apply_nms(regions, 0.5);
        assert_eq!(result.len(), 1);
    }
}
