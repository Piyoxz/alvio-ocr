use crate::types::OcrText;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub text: String,
    pub confidence: f32,
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableData {
    pub rows: Vec<Vec<TableCell>>,
}

impl TableData {
    pub fn to_matrix(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(|cell| cell.text.clone()).collect())
            .collect()
    }

    pub fn to_markdown(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        let matrix = self.to_matrix();
        let col_count = matrix.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return String::new();
        }

        let mut col_widths = vec![3usize; col_count];
        for row in &matrix {
            for (c, cell) in row.iter().enumerate() {
                col_widths[c] = col_widths[c].max(cell.len());
            }
        }

        let mut out = String::new();

        if let Some(header) = matrix.first() {
            out.push('|');
            for c in 0..col_count {
                let val = header.get(c).map(|s| s.as_str()).unwrap_or("");
                out.push_str(&format!(" {:<width$} |", val, width = col_widths[c]));
            }
            out.push('\n');

            out.push('|');
            for c in 0..col_count {
                out.push_str(&format!("-{:-<width$}-|", "", width = col_widths[c]));
            }
            out.push('\n');
        }

        for row in matrix.iter().skip(1) {
            out.push('|');
            for c in 0..col_count {
                let val = row.get(c).map(|s| s.as_str()).unwrap_or("");
                out.push_str(&format!(" {:<width$} |", val, width = col_widths[c]));
            }
            out.push('\n');
        }

        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn reconstruct_table(mut items: Vec<OcrText>) -> TableData {
    if items.is_empty() {
        return TableData::default();
    }

    items.sort_by(|a, b| {
        a.region
            .center_y()
            .partial_cmp(&b.region.center_y())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let avg_height: f32 = items.iter().map(|i| i.region.height()).sum::<f32>() / items.len() as f32;
    let y_tolerance = (avg_height * 0.6).max(8.0);

    let mut raw_rows: Vec<Vec<OcrText>> = Vec::new();
    for item in items {
        let cy = item.region.center_y();
        let mut placed = false;

        for row in raw_rows.iter_mut() {
            if let Some(first) = row.first() {
                if (first.region.center_y() - cy).abs() < y_tolerance {
                    row.push(item.clone());
                    placed = true;
                    break;
                }
            }
        }

        if !placed {
            raw_rows.push(vec![item]);
        }
    }

    let mut rows = Vec::with_capacity(raw_rows.len());

    for (r_idx, mut row_items) in raw_rows.into_iter().enumerate() {
        row_items.sort_by(|a, b| {
            a.region
                .center_x()
                .partial_cmp(&b.region.center_x())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut row_cells = Vec::with_capacity(row_items.len());
        for (c_idx, item) in row_items.into_iter().enumerate() {
            row_cells.push(TableCell {
                text: item.text,
                confidence: item.confidence,
                row: r_idx,
                col: c_idx,
            });
        }
        rows.push(row_cells);
    }

    TableData { rows }
}
