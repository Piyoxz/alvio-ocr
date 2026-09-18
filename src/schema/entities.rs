use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub phone_numbers: Vec<String>,
    pub emails: Vec<String>,
    pub dates: Vec<String>,
    pub amounts: Vec<String>,
    pub niks: Vec<String>,
    pub vehicle_plates: Vec<String>,
    pub urls: Vec<String>,
}

impl ExtractedEntities {
    pub fn is_empty(&self) -> bool {
        self.phone_numbers.is_empty()
            && self.emails.is_empty()
            && self.dates.is_empty()
            && self.amounts.is_empty()
            && self.niks.is_empty()
            && self.vehicle_plates.is_empty()
            && self.urls.is_empty()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn extract_entities_from_lines(lines: &[TextLine]) -> ExtractedEntities {
    let full_text = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
    extract_entities(&full_text)
}

pub fn extract_entities(text: &str) -> ExtractedEntities {
    let mut entities = ExtractedEntities::default();

    for token in text.split_whitespace() {
        let cleaned_token = token.trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':' || c == '(' || c == ')' || c == '[' || c == ']');

        // Email
        if cleaned_token.contains('@') && cleaned_token.contains('.') {
            let parts: Vec<&str> = cleaned_token.split('@').collect();
            if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') && parts[1].len() >= 3 {
                if !entities.emails.contains(&cleaned_token.to_string()) {
                    entities.emails.push(cleaned_token.to_string());
                }
            }
        }

        // URL
        if cleaned_token.starts_with("http://") || cleaned_token.starts_with("https://") || cleaned_token.starts_with("www.") {
            if !entities.urls.contains(&cleaned_token.to_string()) {
                entities.urls.push(cleaned_token.to_string());
            }
        }
    }

    // Line-based extraction
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Phone numbers (+62..., 08...)
        extract_phone_numbers(line, &mut entities.phone_numbers);

        // NIK (16 digits)
        extract_niks(line, &mut entities.niks);

        // Amounts (Rp, IDR, $)
        extract_amounts(line, &mut entities.amounts);

        // Dates (DD/MM/YYYY, DD-MM-YYYY)
        extract_dates(line, &mut entities.dates);

        // Vehicle license plates
        extract_license_plates(line, &mut entities.vehicle_plates);
    }

    entities
}

fn extract_phone_numbers(line: &str, out: &mut Vec<String>) {
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let w = words[i].trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':' || c == '(' || c == ')');
        if w.starts_with("+62") || w.starts_with("08") || w.starts_with("628") || w.starts_with("(021)") || w.starts_with("021") {
            let mut full_phone = w.to_string();
            let mut j = i + 1;
            while j < words.len() {
                let next = words[j].trim_matches(|c: char| c == ',' || c == '.' || c == ';');
                if next.chars().all(|c| c.is_ascii_digit() || c == '-') && next.len() >= 3 && next.len() <= 6 {
                    full_phone.push(' ');
                    full_phone.push_str(next);
                    j += 1;
                } else {
                    break;
                }
            }
            let digits_count = full_phone.chars().filter(|c| c.is_ascii_digit()).count();
            if digits_count >= 10 && digits_count <= 15 {
                if !out.contains(&full_phone) {
                    out.push(full_phone);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

fn extract_niks(line: &str, out: &mut Vec<String>) {
    for token in line.split_whitespace() {
        let clean = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let digits: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 16 && (clean.len() == 16 || clean.contains('-') || clean.contains('.')) {
            if !out.contains(&digits) {
                out.push(digits);
            }
        }
    }
}

fn extract_amounts(line: &str, out: &mut Vec<String>) {
    let upper = line.to_uppercase();
    if upper.contains("RP") || upper.contains("IDR") || line.contains('$') {
        let words: Vec<&str> = line.split_whitespace().collect();
        for (i, &w) in words.iter().enumerate() {
            let up = w.to_uppercase();
            if up == "RP" || up == "RP." || up == "IDR" || up == "$" {
                if let Some(&next) = words.get(i + 1) {
                    let cleaned = next.trim_matches(|c: char| !c.is_ascii_digit());
                    if !cleaned.is_empty() {
                        let amount_str = format!("{} {}", w, next);
                        if !out.contains(&amount_str) {
                            out.push(amount_str);
                        }
                    }
                }
            } else if up.starts_with("RP") || up.starts_with("IDR") || up.starts_with('$') {
                let cleaned: String = w.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
                if cleaned.chars().any(|c| c.is_ascii_digit()) {
                    let amount_str = w.to_string();
                    if !out.contains(&amount_str) {
                        out.push(amount_str);
                    }
                }
            }
        }
    }
}

fn extract_dates(line: &str, out: &mut Vec<String>) {
    for token in line.split_whitespace() {
        let clean = token.trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':' || c == '(' || c == ')');
        if clean.contains('/') || clean.contains('-') {
            let sep = if clean.contains('/') { '/' } else { '-' };
            let parts: Vec<&str> = clean.split(sep).collect();
            if parts.len() == 3 {
                let p0_ok = parts[0].chars().all(|c| c.is_ascii_digit());
                let p1_ok = parts[1].chars().all(|c| c.is_ascii_digit());
                let p2_ok = parts[2].chars().all(|c| c.is_ascii_digit());
                if p0_ok && p1_ok && p2_ok {
                    let l0 = parts[0].len();
                    let l1 = parts[1].len();
                    let l2 = parts[2].len();
                    if (l0 == 2 || l0 == 4) && (l1 == 2) && (l2 == 2 || l2 == 4) {
                        let date_str = clean.to_string();
                        if !out.contains(&date_str) {
                            out.push(date_str);
                        }
                    }
                }
            }
        }
    }

    // Indonesian text dates: e.g. "17 September 2026"
    let months = [
        "JANUARI", "FEBRUARI", "MARET", "APRIL", "MEI", "JUNI",
        "JULI", "AGUSTUS", "SEPTEMBER", "OKTOBER", "NOVEMBER", "DESEMBER",
        "JAN", "FEB", "MAR", "APR", "JUN", "JUL", "AUG", "AGU", "SEP", "OKT", "NOV", "DES",
    ];

    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() >= 3 {
        for i in 0..words.len() - 2 {
            let day = words[i].trim_matches(|c: char| !c.is_ascii_digit());
            let month = words[i + 1].to_uppercase();
            let year = words[i + 2].trim_matches(|c: char| !c.is_ascii_digit());

            if let Ok(d) = day.parse::<u32>() {
                if d >= 1 && d <= 31 && year.len() == 4 && months.iter().any(|&m| month.contains(m)) {
                    let date_str = format!("{} {} {}", words[i], words[i + 1], words[i + 2]);
                    if !out.contains(&date_str) {
                        out.push(date_str);
                    }
                }
            }
        }
    }
}

fn extract_license_plates(line: &str, out: &mut Vec<String>) {
    let prefixes = [
        "BL", "BB", "BK", "BA", "BM", "BH", "BD", "BP", "BG", "BN", "BE", // Sumatra
        "A", "B", "D", "E", "F", "T", "Z", // West Java / Jakarta / Banten
        "G", "H", "K", "R", "AA", "AD", // Central Java
        "AB", // Yogyakarta
        "L", "M", "N", "P", "S", "W", "AE", "AG", // East Java
        "DK", "DR", "EA", "DH", "EB", "ED", // Bali / Nusa Tenggara
        "KB", "DA", "KH", "KT", "KU", // Kalimantan
        "DB", "DL", "DM", "DN", "DT", "DD", "DC", "DP", // Sulawesi
        "DE", "DG", "PA", "PB", // Maluku & Papua
    ];

    let words: Vec<&str> = line.split_whitespace().collect();
    for i in 0..words.len() {
        let code = words[i].to_uppercase();
        if prefixes.contains(&code.as_str()) {
            if i + 1 < words.len() {
                let next = words[i + 1];
                let num_digits = next.chars().filter(|c| c.is_ascii_digit()).count();
                if num_digits >= 1 && num_digits <= 4 {
                    if i + 2 < words.len() {
                        let suffix = words[i + 2];
                        let alpha_count = suffix.chars().filter(|c| c.is_ascii_alphabetic()).count();
                        if alpha_count >= 1 && alpha_count <= 3 && suffix.len() <= 4 {
                            let plate = format!("{} {} {}", code, next, suffix);
                            if !out.contains(&plate) {
                                out.push(plate);
                            }
                        }
                    }
                }
            }
        }
    }
}
