use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimData {
    pub nomor_sim: Option<String>,
    pub golongan: Option<String>,
    pub nama: Option<String>,
    pub tempat_tgl_lahir: Option<String>,
    pub jenis_kelamin: Option<String>,
    pub alamat: Option<String>,
    pub pekerjaan: Option<String>,
    pub berlaku_hingga: Option<String>,
}

impl SimData {
    pub fn is_valid(&self) -> bool {
        if let Some(no) = &self.nomor_sim {
            no.len() >= 8 && no.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn clean_sim_digits(s: &str) -> String {
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

pub fn extract_sim_from_lines(lines: &[TextLine]) -> SimData {
    let text_lines: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    extract_sim_from_strings(&text_lines)
}

pub fn extract_sim(text: &str) -> SimData {
    let raw_lines: Vec<&str> = text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    extract_sim_from_strings(&raw_lines)
}

fn clean_value(line: &str, keyword: &str) -> String {
    let mut val = line;
    if let Some(pos) = line.to_uppercase().find(keyword) {
        val = &line[pos + keyword.len()..];
    }
    val = val.trim_start_matches(|c: char| c == ':' || c == '=' || c == '-' || c.is_whitespace());
    val.trim().to_string()
}

fn extract_sim_from_strings(lines: &[&str]) -> SimData {
    let mut data = SimData::default();

    for (idx, &line) in lines.iter().enumerate() {
        let upper = line.to_uppercase();

        if upper.contains("SURAT IZIN MENGEMUDI") || upper.contains("SIM ") || upper.starts_with("SIM") {
            for candidate in &["C II", "C I", "B II", "B I", "A", "C", "D"] {
                if upper.contains(candidate) {
                    data.golongan = Some(candidate.to_string());
                    break;
                }
            }
        }

        if upper.contains("NO") && (upper.contains("SIM") || upper.contains("NOMOR")) {
            let val = clean_value(line, "SIM");
            let d = clean_sim_digits(&val);
            if d.len() >= 8 {
                data.nomor_sim = Some(d);
            }
            continue;
        }

        if data.nomor_sim.is_none() {
            let d = clean_sim_digits(line);
            if d.len() >= 12 && d.len() <= 16 {
                data.nomor_sim = Some(d);
            }
        }

        if upper.starts_with("1.") || upper.starts_with("1 ") || (upper.contains("NAMA") && !upper.contains("KEPALA")) {
            let val = if upper.contains("NAMA") { clean_value(line, "NAMA") } else { line[2..].trim().to_string() };
            if !val.is_empty() {
                data.nama = Some(val);
            }
            continue;
        }

        if upper.starts_with("2.") || upper.starts_with("2 ") || upper.contains("LAHIR") {
            let val = if upper.contains("LAHIR") { clean_value(line, "LAHIR") } else { line[2..].trim().to_string() };
            if !val.is_empty() {
                data.tempat_tgl_lahir = Some(val);
            }
            continue;
        }

        if upper.contains("PRIA") || upper.contains("LAKI") {
            data.jenis_kelamin = Some("LAKI-LAKI".to_string());
        } else if upper.contains("WANITA") || upper.contains("PEREMPUAN") {
            data.jenis_kelamin = Some("PEREMPUAN".to_string());
        }

        if upper.starts_with("4.") || upper.starts_with("4 ") || upper.contains("ALAMAT") {
            let mut addr = if upper.contains("ALAMAT") { clean_value(line, "ALAMAT") } else { line[2..].trim().to_string() };
            if let Some(&next) = lines.get(idx + 1) {
                let next_up = next.to_uppercase();
                if !next_up.starts_with("5.") && !next_up.contains("PEKERJAAN") && !next_up.contains("BERLAKU") {
                    addr.push(' ');
                    addr.push_str(next.trim());
                }
            }
            data.alamat = Some(addr);
            continue;
        }

        if upper.starts_with("5.") || upper.starts_with("5 ") || upper.contains("PEKERJAAN") {
            let val = if upper.contains("PEKERJAAN") { clean_value(line, "PEKERJAAN") } else { line[2..].trim().to_string() };
            if !val.is_empty() {
                data.pekerjaan = Some(val);
            }
            continue;
        }

        if upper.contains("BERLAKU") || upper.contains("S/D") || upper.contains("HINGGA") {
            let mut val = clean_value(line, "BERLAKU");
            if let Some(pos) = val.to_uppercase().find("S/D") {
                val = val[pos + 3..].trim().to_string();
            } else if let Some(pos) = val.to_uppercase().find("HINGGA") {
                val = val[pos + 6..].trim().to_string();
            }
            let cleaned = val.trim_start_matches(|c: char| c == ':' || c == '=' || c == '-' || c.is_whitespace());
            if !cleaned.is_empty() {
                data.berlaku_hingga = Some(cleaned.trim().to_string());
            }
            continue;
        }
    }

    data
}
