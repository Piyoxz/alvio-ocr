//! Reading order sorting and text reconstruction.
//!
//! Optimization A6: Improved column detection with height-adaptive line clustering.

use crate::types::OcrText;

/// Sort recognized text items into natural reading order (top-to-bottom, left-to-right).
///
/// Automatically detects two-column layouts and sorts each column separately.
pub fn sort_reading_order(items: Vec<OcrText>) -> Vec<OcrText> {
    if items.len() <= 1 {
        return items;
    }

    // Compute page bounds
    let mut min_x_total = f32::MAX;
    let mut max_x_total = f32::MIN;
    for item in &items {
        let (min_x, _, max_x, _) = item.region.aabb();
        if min_x < min_x_total { min_x_total = min_x; }
        if max_x > max_x_total { max_x_total = max_x; }
    }
    let total_width = (max_x_total - min_x_total).max(1.0);
    let center_x = min_x_total + total_width / 2.0;

    // A6: Height-adaptive gutter margin
    let avg_height: f32 = items.iter().map(|i| i.region.height()).sum::<f32>() / items.len() as f32;
    let gutter_margin = (total_width * 0.05).max(avg_height * 0.5);

    let mut left_column = Vec::new();
    let mut right_column = Vec::new();
    let mut spans_center = 0usize;

    for item in &items {
        let (min_x, _, max_x, _) = item.region.aabb();
        if max_x < center_x - gutter_margin {
            left_column.push(item.clone());
        } else if min_x > center_x + gutter_margin {
            right_column.push(item.clone());
        } else {
            spans_center += 1;
        }
    }

    let is_two_column = left_column.len() >= 3
        && right_column.len() >= 3
        && (spans_center as f32 / items.len() as f32) < 0.20;

    if is_two_column {
        let mut sorted_left = sort_lines(left_column);
        let sorted_right = sort_lines(right_column);
        sorted_left.extend(sorted_right);
        sorted_left
    } else {
        sort_lines(items)
    }
}

/// Group items into lines based on vertical overlap, then sort lines top-to-bottom
/// and items within each line left-to-right.
fn sort_lines(mut items: Vec<OcrText>) -> Vec<OcrText> {
    if items.is_empty() {
        return items;
    }

    // Sort by top Y coordinate first
    items.sort_by(|a, b| {
        let (_, a_min_y, _, _) = a.region.aabb();
        let (_, b_min_y, _, _) = b.region.aabb();
        a_min_y
            .partial_cmp(&b_min_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Group into lines using vertical overlap
    let mut lines: Vec<Vec<OcrText>> = Vec::new();

    for item in items {
        let (_, item_min_y, _, item_max_y) = item.region.aabb();
        let item_h = (item_max_y - item_min_y).max(1.0);

        let mut found_line_idx = None;
        for (idx, line) in lines.iter().enumerate() {
            if let Some(first) = line.first() {
                let (_, line_min_y, _, line_max_y) = first.region.aabb();
                let line_h = (line_max_y - line_min_y).max(1.0);

                let overlap_top = item_min_y.max(line_min_y);
                let overlap_bottom = item_max_y.min(line_max_y);
                let overlap = (overlap_bottom - overlap_top).max(0.0);

                let min_h = item_h.min(line_h);
                if overlap / min_h >= 0.5 {
                    found_line_idx = Some(idx);
                    break;
                }
            }
        }

        if let Some(idx) = found_line_idx {
            lines[idx].push(item);
        } else {
            lines.push(vec![item]);
        }
    }

    // Sort lines by vertical center
    lines.sort_by(|a, b| {
        let a_y = a.first().map(|i| i.region.center_y()).unwrap_or(0.0);
        let b_y = b.first().map(|i| i.region.center_y()).unwrap_or(0.0);
        a_y.partial_cmp(&b_y).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Sort items within each line left-to-right
    let mut result = Vec::new();
    for mut line in lines {
        line.sort_by(|a, b| {
            let (a_min_x, _, _, _) = a.region.aabb();
            let (b_min_x, _, _, _) = b.region.aabb();
            a_min_x
                .partial_cmp(&b_min_x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result.extend(line);
    }

    result
}

/// Reconstruct a full text string from sorted text items,
/// inserting newlines between lines and spaces within lines.
pub fn reconstruct_text(items: &[OcrText]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut full_text = String::new();
    let mut last_y = f32::MIN;

    for item in items {
        let (_, min_y, _, max_y) = item.region.aabb();
        let h = (max_y - min_y).max(1.0);

        if last_y == f32::MIN {
            full_text.push_str(&item.text);
        } else {
            let delta_y = (min_y - last_y).abs();
            if delta_y > h * 0.5 {
                full_text.push('\n');
            } else {
                full_text.push(' ');
            }
            full_text.push_str(&item.text);
        }

        last_y = min_y;
    }

    full_text
}
