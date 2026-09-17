use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub name: String,
    pub quantity: Option<f32>,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReceiptData {
    pub merchant_name: Option<String>,
    pub date: Option<String>,
    pub invoice_number: Option<String>,
    pub subtotal: Option<f64>,
    pub tax: Option<f64>,
    pub discount: Option<f64>,
    pub total: Option<f64>,
    pub currency: Option<String>,
    pub items: Vec<ReceiptItem>,
}

impl ReceiptData {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub fn parse_currency_amount(s: &str) -> Option<f64> {
    let mut cleaned = String::with_capacity(s.len());
    let mut has_comma = false;
    let mut has_dot = false;

    for c in s.chars() {
        if c.is_ascii_digit() {
            cleaned.push(c);
        } else if c == '.' {
            has_dot = true;
            cleaned.push('.');
        } else if c == ',' {
            has_comma = true;
            cleaned.push(',');
        }
    }

    if cleaned.is_empty() {
        return None;
    }

    let normalized = if has_dot && has_comma {
        let last_dot = cleaned.rfind('.').unwrap_or(0);
        let last_comma = cleaned.rfind(',').unwrap_or(0);
        if last_comma > last_dot {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if has_comma {
        if cleaned.len() - cleaned.rfind(',').unwrap_or(0) == 3 {
            cleaned.replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if has_dot {
        if cleaned.len() - cleaned.rfind('.').unwrap_or(0) > 3 {
            cleaned.replace('.', "")
        } else {
            cleaned
        }
    } else {
        cleaned
    };

    normalized.parse::<f64>().ok()
}

pub fn extract_receipt_from_lines(lines: &[TextLine]) -> ReceiptData {
    let text_lines: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    extract_receipt_from_strings(&text_lines)
}

pub fn extract_receipt(text: &str) -> ReceiptData {
    let raw_lines: Vec<&str> = text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    extract_receipt_from_strings(&raw_lines)
}

fn extract_receipt_from_strings(lines: &[&str]) -> ReceiptData {
    let mut data = ReceiptData::default();

    if let Some(&first) = lines.first() {
        let upper = first.to_uppercase();
        if !upper.contains("TOTAL") && !upper.contains("INVOICE") && !upper.contains("RECEIPT") {
            data.merchant_name = Some(first.trim().to_string());
        }
    }

    for line in lines {
        let upper = line.to_uppercase();

        if upper.contains("RP") || upper.contains("IDR") {
            data.currency = Some("IDR".to_string());
        } else if upper.contains('$') || upper.contains("USD") {
            data.currency = Some("USD".to_string());
        }

        if upper.contains("TOTAL") && !upper.contains("SUB") {
            if let Some(amt) = parse_currency_amount(line) {
                data.total = Some(amt);
            }
        } else if upper.contains("SUBTOTAL") || upper.contains("SUB TOTAL") {
            if let Some(amt) = parse_currency_amount(line) {
                data.subtotal = Some(amt);
            }
        } else if upper.contains("TAX") || upper.contains("PPN") || upper.contains("PAJAK") {
            if let Some(amt) = parse_currency_amount(line) {
                data.tax = Some(amt);
            }
        } else if upper.contains("DISKON") || upper.contains("DISCOUNT") {
            if let Some(amt) = parse_currency_amount(line) {
                data.discount = Some(amt);
            }
        } else if upper.contains("INVOICE") || upper.contains("NO:") || upper.contains("STRUK:") {
            if let Some(pos) = line.find(':') {
                data.invoice_number = Some(line[pos + 1..].trim().to_string());
            }
        } else if upper.contains("TANGGAL") || upper.contains("DATE") {
            if let Some(pos) = line.find(':') {
                data.date = Some(line[pos + 1..].trim().to_string());
            }
        }
    }

    data
}
