use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KtpData {
    pub provinsi: Option<String>,
    pub kota_kabupaten: Option<String>,
    pub nik: Option<String>,
    pub nama: Option<String>,
    pub tempat_tgl_lahir: Option<String>,
    pub jenis_kelamin: Option<String>,
    pub golongan_darah: Option<String>,
    pub alamat: Option<String>,
    pub rt_rw: Option<String>,
    pub kel_desa: Option<String>,
    pub kecamatan: Option<String>,
    pub agama: Option<String>,
    pub status_perkawinan: Option<String>,
    pub pekerjaan: Option<String>,
    pub kewarganegaraan: Option<String>,
    pub berlaku_hingga: Option<String>,
}

impl KtpData {
    pub fn is_valid_nik(&self) -> bool {
        if let Some(nik) = &self.nik {
            nik.len() == 16 && nik.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn clean_nik_digits(s: &str) -> String {
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

pub fn extract_ktp_from_lines(lines: &[TextLine]) -> KtpData {
    let text_lines: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    extract_ktp_from_strings(&text_lines)
}

pub fn extract_ktp(text: &str) -> KtpData {
    let raw_lines: Vec<&str> = text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    extract_ktp_from_strings(&raw_lines)
}

fn extract_ktp_from_strings(lines: &[&str]) -> KtpData {
    let mut data = KtpData::default();

    for (idx, &line) in lines.iter().enumerate() {
        let upper = line.to_uppercase();

        if upper.contains("PROVINSI") {
            data.provinsi = Some(clean_value(line, "PROVINSI"));
            continue;
        }

        if upper.contains("KOTA") || upper.contains("KABUPATEN") {
            if upper.contains("KOTA") {
                data.kota_kabupaten = Some(clean_value(line, "KOTA"));
            } else {
                data.kota_kabupaten = Some(clean_value(line, "KABUPATEN"));
            }
            continue;
        }

        if upper.contains("NIK") {
            let raw_nik = clean_value(line, "NIK");
            let cleaned = clean_nik_digits(&raw_nik);
            if cleaned.len() >= 16 {
                data.nik = Some(cleaned[..16].to_string());
            } else if !cleaned.is_empty() {
                data.nik = Some(cleaned);
            }
            continue;
        }

        if upper.contains("NAMA") {
            data.nama = Some(clean_value(line, "NAMA"));
            continue;
        }

        if upper.contains("TEMPAT") || upper.contains("TGL LAHIR") || upper.contains("LAHIR") {
            data.tempat_tgl_lahir = Some(clean_value(line, "LAHIR"));
            continue;
        }

        if upper.contains("JENIS KELAMIN") || upper.contains("KELAMIN") {
            let val = clean_value(line, "KELAMIN");
            let upper_val = val.to_uppercase();
            if upper_val.contains("LAKI") {
                data.jenis_kelamin = Some("LAKI-LAKI".to_string());
            } else if upper_val.contains("PEREMPUAN") {
                data.jenis_kelamin = Some("PEREMPUAN".to_string());
            } else {
                data.jenis_kelamin = Some(val);
            }

            if upper.contains("DARAH") || upper.contains("GOL") {
                if let Some(pos) = upper.find("DARAH") {
                    let blood_part = &line[pos + 5..];
                    data.golongan_darah = Some(blood_part.trim_matches(|c: char| c == ':' || c == '-' || c.is_whitespace()).to_string());
                }
            }
            continue;
        }

        if upper.contains("ALAMAT") {
            data.alamat = Some(clean_value(line, "ALAMAT"));
            continue;
        }

        if upper.contains("RT") || upper.contains("RW") {
            data.rt_rw = Some(clean_value(line, "RW"));
            continue;
        }

        if upper.contains("KEL") || upper.contains("DESA") {
            data.kel_desa = Some(clean_value(line, "DESA"));
            continue;
        }

        if upper.contains("KECAMATAN") {
            data.kecamatan = Some(clean_value(line, "KECAMATAN"));
            continue;
        }

        if upper.contains("AGAMA") {
            data.agama = Some(clean_value(line, "AGAMA"));
            continue;
        }

        if upper.contains("STATUS") || upper.contains("PERKAWINAN") {
            data.status_perkawinan = Some(clean_value(line, "PERKAWINAN"));
            continue;
        }

        if upper.contains("PEKERJAAN") {
            data.pekerjaan = Some(clean_value(line, "PEKERJAAN"));
            continue;
        }

        if upper.contains("KEWARGANEGARAAN") {
            let val = clean_value(line, "KEWARGANEGARAAN");
            if val.to_uppercase().contains("WNI") {
                data.kewarganegaraan = Some("WNI".to_string());
            } else {
                data.kewarganegaraan = Some(val);
            }
            continue;
        }

        if upper.contains("BERLAKU") {
            data.berlaku_hingga = Some(clean_value(line, "BERLAKU HINGGA"));
            continue;
        }

        if data.nik.is_none() && idx <= 3 {
            let digits = clean_nik_digits(line);
            if digits.len() == 16 {
                data.nik = Some(digits);
            }
        }
    }

    data
}

fn clean_value(line: &str, key: &str) -> String {
    let upper = line.to_uppercase();
    if let Some(pos) = upper.find(key) {
        let after_key = &line[pos + key.len()..];
        let trimmed = after_key.trim_start_matches(|c: char| c == ':' || c == '-' || c == '=' || c.is_whitespace());
        trimmed.trim().to_string()
    } else if let Some(colon_pos) = line.find(':') {
        line[colon_pos + 1..].trim().to_string()
    } else {
        line.trim().to_string()
    }
}
