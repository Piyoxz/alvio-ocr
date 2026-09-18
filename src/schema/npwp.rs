use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NpwpData {
    pub npwp: Option<String>,
    pub npwp_raw: Option<String>,
    pub nama: Option<String>,
    pub nik: Option<String>,
    pub alamat: Option<String>,
    pub kpp: Option<String>,
    pub terdaftar: Option<String>,
}

impl NpwpData {
    pub fn is_valid(&self) -> bool {
        if let Some(raw) = &self.npwp_raw {
            (raw.len() == 15 || raw.len() == 16) && raw.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn clean_npwp_digits(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '0'..='9' => out.push(c),
            'O' | 'o' | 'D' => out.push('0'),
            'I' | 'l' | 'i' | '|' => out.push('1'),
            'Z' | 'z' => out.push('2'),
            'A' => out.push('4'),
            'S' | 's' => out.push('5'),
            'b' | 'G' => out.push('6'),
            'B' => out.push('8'),
            'g' | 'q' => out.push('9'),
            _ => {}
        }
    }
    out
}

pub fn format_npwp_15(digits: &str) -> String {
    if digits.len() == 15 {
        format!(
            "{}.{}.{}.{}-{}.{}",
            &digits[0..2],
            &digits[2..5],
            &digits[5..8],
            &digits[8..9],
            &digits[9..12],
            &digits[12..15]
        )
    } else {
        digits.to_string()
    }
}

pub fn extract_npwp_from_lines(lines: &[TextLine]) -> NpwpData {
    let text_lines: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    extract_npwp_from_strings(&text_lines)
}

pub fn extract_npwp(text: &str) -> NpwpData {
    let raw_lines: Vec<&str> = text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    extract_npwp_from_strings(&raw_lines)
}

fn clean_value(line: &str, keyword: &str) -> String {
    let mut val = line;
    if let Some(pos) = line.to_uppercase().find(keyword) {
        val = &line[pos + keyword.len()..];
    }
    val = val.trim_start_matches(|c: char| c == ':' || c == '=' || c == '-' || c.is_whitespace());
    val.trim().to_string()
}

fn extract_npwp_from_strings(lines: &[&str]) -> NpwpData {
    let mut data = NpwpData::default();

    for (idx, &line) in lines.iter().enumerate() {
        let upper = line.to_uppercase();

        if upper.contains("NPWP") || upper.contains("NOMOR POKOK") {
            let val = clean_value(line, "NPWP");
            let digits = clean_npwp_digits(&val);
            if digits.len() >= 15 {
                let clean_15_or_16 = &digits[..digits.len().min(16)];
                data.npwp_raw = Some(clean_15_or_16.to_string());
                data.npwp = Some(format_npwp_15(clean_15_or_16));
            } else if !digits.is_empty() {
                data.npwp_raw = Some(digits.clone());
                data.npwp = Some(digits);
            }
            continue;
        }

        let digits = clean_npwp_digits(line);
        if data.npwp.is_none() && (digits.len() == 15 || digits.len() == 16) {
            data.npwp_raw = Some(digits.clone());
            data.npwp = Some(format_npwp_15(&digits));
            continue;
        }

        if upper.contains("NAMA") {
            data.nama = Some(clean_value(line, "NAMA"));
            continue;
        }

        if upper.contains("NIK") {
            let val = clean_value(line, "NIK");
            let d = clean_npwp_digits(&val);
            if d.len() >= 16 {
                data.nik = Some(d[..16].to_string());
            } else if !d.is_empty() {
                data.nik = Some(d);
            }
            continue;
        }

        if upper.contains("ALAMAT") {
            let mut addr = clean_value(line, "ALAMAT");
            if let Some(&next) = lines.get(idx + 1) {
                let next_up = next.to_uppercase();
                if !next_up.contains("KPP") && !next_up.contains("TERDAFTAR") && !next_up.contains("NPWP") && !next_up.contains("NIK") {
                    addr.push(' ');
                    addr.push_str(next.trim());
                }
            }
            data.alamat = Some(addr);
            continue;
        }

        if upper.contains("KPP") {
            data.kpp = Some(clean_value(line, "KPP"));
            continue;
        }

        if upper.contains("TERDAFTAR") {
            data.terdaftar = Some(clean_value(line, "TERDAFTAR"));
            continue;
        }
    }

    data
}
