use crate::types::TextLine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub confidence: f32,
}

pub fn extract_key_values(lines: &[TextLine]) -> Vec<KeyValuePair> {
    let mut pairs = Vec::new();

    for line in lines {
        let text = line.text.trim();

        if let Some(pos) = text.find(':') {
            let key = text[..pos].trim();
            let val = text[pos + 1..].trim();

            if !key.is_empty() && !val.is_empty() {
                pairs.push(KeyValuePair {
                    key: key.to_string(),
                    value: val.to_string(),
                    confidence: line.confidence,
                });
                continue;
            }
        }

        if let Some(pos) = text.find('=') {
            let key = text[..pos].trim();
            let val = text[pos + 1..].trim();

            if !key.is_empty() && !val.is_empty() {
                pairs.push(KeyValuePair {
                    key: key.to_string(),
                    value: val.to_string(),
                    confidence: line.confidence,
                });
            }
        }
    }

    pairs
}
