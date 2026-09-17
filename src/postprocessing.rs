use crate::types::{BoundingBox, OcrText, TextLine};

pub fn sort_reading_order(items: Vec<OcrText>) -> Vec<OcrText> {
    if items.len() <= 1 {
        return items;
    }

    let mut min_x_total = f32::MAX;
    let mut max_x_total = f32::MIN;
    for item in &items {
        let (min_x, _, max_x, _) = item.region.aabb();
        if min_x < min_x_total { min_x_total = min_x; }
        if max_x > max_x_total { max_x_total = max_x; }
    }
    let total_width = (max_x_total - min_x_total).max(1.0);
    let center_x = min_x_total + total_width / 2.0;

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

fn sort_lines(mut items: Vec<OcrText>) -> Vec<OcrText> {
    if items.is_empty() {
        return items;
    }

    items.sort_by(|a, b| {
        let (_, a_min_y, _, _) = a.region.aabb();
        let (_, b_min_y, _, _) = b.region.aabb();
        a_min_y
            .partial_cmp(&b_min_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

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

    lines.sort_by(|a, b| {
        let a_y = a.first().map(|i| i.region.center_y()).unwrap_or(0.0);
        let b_y = b.first().map(|i| i.region.center_y()).unwrap_or(0.0);
        a_y.partial_cmp(&b_y).unwrap_or(std::cmp::Ordering::Equal)
    });

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

pub fn group_into_lines(sorted_items: Vec<OcrText>) -> (Vec<OcrText>, Vec<TextLine>) {
    if sorted_items.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut lines_raw: Vec<Vec<OcrText>> = Vec::new();
    let mut current_line: Vec<OcrText> = Vec::new();
    let mut last_y = f32::MIN;

    for item in sorted_items {
        let (_, min_y, _, max_y) = item.region.aabb();
        let h = (max_y - min_y).max(1.0);

        if last_y == f32::MIN {
            current_line.push(item);
        } else {
            let delta_y = (min_y - last_y).abs();
            if delta_y > h * 0.5 {
                if !current_line.is_empty() {
                    lines_raw.push(std::mem::take(&mut current_line));
                }
            }
            current_line.push(item);
        }
        last_y = min_y;
    }

    if !current_line.is_empty() {
        lines_raw.push(current_line);
    }

    let mut flat_items = Vec::new();
    let mut structured_lines = Vec::new();

    for (line_idx, raw_line) in lines_raw.into_iter().enumerate() {
        let line_number = line_idx + 1;
        let line_text = raw_line
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let line_conf = if raw_line.is_empty() {
            0.0
        } else {
            raw_line.iter().map(|w| w.confidence).sum::<f32>() / raw_line.len() as f32
        };

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        let mut updated_words = Vec::new();
        for mut word in raw_line {
            word.line_number = line_number;
            let (bx1, by1, bx2, by2) = word.region.aabb();
            if bx1 < min_x { min_x = bx1; }
            if by1 < min_y { min_y = by1; }
            if bx2 > max_x { max_x = bx2; }
            if by2 > max_y { max_y = by2; }
            word.bbox = BoundingBox::new(bx1, by1, bx2, by2);
            flat_items.push(word.clone());
            updated_words.push(word);
        }

        let line_bbox = if min_x == f32::MAX {
            BoundingBox::default()
        } else {
            BoundingBox::new(min_x, min_y, max_x, max_y)
        };

        structured_lines.push(TextLine {
            line_number,
            text: line_text,
            confidence: line_conf,
            bbox: line_bbox,
            words: updated_words,
        });
    }

    (flat_items, structured_lines)
}

pub fn reconstruct_text_from_lines(lines: &[TextLine]) -> String {
    lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
