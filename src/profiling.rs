use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct StageTiming {
    pub parsing_ms: f32,
    pub rendering_ms: f32,
    pub preprocessing_ms: f32,
    pub detection_ms: f32,
    pub orientation_ms: f32,
    pub recognition_ms: f32,
    pub second_pass_ms: f32,
    pub postprocessing_ms: f32,
    pub total_ms: f32,
}

impl StageTiming {
    pub fn summary(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("----------------------------------------\n");
        out.push_str("          STAGE TIMING BREAKDOWN        \n");
        out.push_str("----------------------------------------\n");
        if self.parsing_ms > 0.0 {
            out.push_str(&format!("Parsing:           {:>8.2} ms\n", self.parsing_ms));
        }
        if self.rendering_ms > 0.0 {
            out.push_str(&format!("Rendering:         {:>8.2} ms\n", self.rendering_ms));
        }
        out.push_str(&format!("Preprocessing:     {:>8.2} ms\n", self.preprocessing_ms));
        out.push_str(&format!("Detection:         {:>8.2} ms\n", self.detection_ms));
        if self.orientation_ms > 0.0 {
            out.push_str(&format!("Orientation:       {:>8.2} ms\n", self.orientation_ms));
        }
        out.push_str(&format!("Recognition:       {:>8.2} ms\n", self.recognition_ms));
        if self.second_pass_ms > 0.0 {
            out.push_str(&format!("Second Pass Re-OCR:{:>8.2} ms\n", self.second_pass_ms));
        }
        out.push_str(&format!("Postprocessing:    {:>8.2} ms\n", self.postprocessing_ms));
        out.push_str("----------------------------------------\n");
        out.push_str(&format!("TOTAL:             {:>8.2} ms\n", self.total_ms));
        out.push_str("----------------------------------------\n");
        out
    }
}

pub fn levenshtein_chars(a: &[char], b: &[char]) -> usize {
    let len_a = a.len();
    let len_b = b.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut prev_row: Vec<usize> = (0..=len_b).collect();
    let mut curr_row: Vec<usize> = vec![0; len_b + 1];

    for i in 1..=len_a {
        curr_row[0] = i;
        for j in 1..=len_b {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len_b]
}

pub fn compute_cer(reference: &str, hypothesis: &str) -> f32 {
    let ref_chars: Vec<char> = reference.chars().collect();
    let hyp_chars: Vec<char> = hypothesis.chars().collect();

    if ref_chars.is_empty() {
        return if hyp_chars.is_empty() { 0.0 } else { 1.0 };
    }

    let distance = levenshtein_chars(&ref_chars, &hyp_chars);
    (distance as f32 / ref_chars.len() as f32).min(1.0)
}

pub fn compute_wer(reference: &str, hypothesis: &str) -> f32 {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();

    let len_a = ref_words.len();
    let len_b = hyp_words.len();

    if len_a == 0 {
        return if len_b == 0 { 0.0 } else { 1.0 };
    }

    let mut prev_row: Vec<usize> = (0..=len_b).collect();
    let mut curr_row: Vec<usize> = vec![0; len_b + 1];

    for i in 1..=len_a {
        curr_row[0] = i;
        for j in 1..=len_b {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    (prev_row[len_b] as f32 / len_a as f32).min(1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PercentileStats {
    pub min: f32,
    pub p50: f32,
    pub p90: f32,
    pub p95: f32,
    pub p99: f32,
    pub max: f32,
    pub mean: f32,
}

impl PercentileStats {
    pub fn compute(mut values: Vec<f32>) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let len = values.len();
        let sum: f32 = values.iter().sum();
        let mean = sum / len as f32;

        let p = |pct: f32| -> f32 {
            let idx = ((pct / 100.0) * (len - 1) as f32).round() as usize;
            values[idx.min(len - 1)]
        };

        Some(Self {
            min: values[0],
            p50: p(50.0),
            p90: p(90.0),
            p95: p(95.0),
            p99: p(99.0),
            max: values[len - 1],
            mean,
        })
    }
}
