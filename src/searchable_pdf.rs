use crate::{
    error::OcrError,
    types::TextLine,
};
use image::DynamicImage;
use std::io::Write;

pub fn create_searchable_pdf(img: &DynamicImage, lines: &[TextLine]) -> Result<Vec<u8>, OcrError> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(OcrError::InvalidImage("Image has zero dimensions".to_string()));
    }

    let rgb = img.to_rgb8();
    let mut jpeg_bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
    encoder
        .encode(
            rgb.as_raw(),
            w,
            h,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| OcrError::InvalidImage(format!("JPEG encoding failed: {}", e)))?;

    // Build PDF content stream
    let mut text_stream = Vec::new();
    writeln!(text_stream, "q").unwrap();
    writeln!(text_stream, "{} 0 0 {} 0 0 cm", w, h).unwrap();
    writeln!(text_stream, "/Im0 Do").unwrap();
    writeln!(text_stream, "Q").unwrap();
    writeln!(text_stream, "BT").unwrap();
    // 3 Tr = invisible text (neither fill nor stroke), selectable & searchable
    writeln!(text_stream, "3 Tr").unwrap();

    for line in lines {
        if line.text.trim().is_empty() {
            continue;
        }

        let bx = line.bbox.min_x;
        let by = line.bbox.min_y;
        let _bw = line.bbox.width();
        let bh = line.bbox.height().max(6.0);

        // PDF coordinate origin is bottom-left
        let pdf_y = (h as f32) - by - bh;

        let font_size = bh * 0.9;
        let escaped_text = escape_pdf_string(&line.text);

        writeln!(text_stream, "/F1 {:.2} Tf", font_size).unwrap();
        // Position text
        writeln!(text_stream, "1 0 0 1 {:.2} {:.2} Tm", bx, pdf_y).unwrap();
        writeln!(text_stream, "({}) Tj", escaped_text).unwrap();
    }

    writeln!(text_stream, "ET").unwrap();

    // Assemble complete PDF document
    let mut pdf = Vec::new();
    let mut offsets = Vec::new();

    // Header
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    writeln!(pdf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj").unwrap();

    // Object 2: Pages
    offsets.push(pdf.len());
    writeln!(pdf, "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj").unwrap();

    // Object 3: Page
    offsets.push(pdf.len());
    writeln!(
        pdf,
        "3 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /MediaBox [0 0 {} {}]\n  /Resources <<\n    /ProcSet [/PDF /Text /ImageC]\n    /Font << /F1 4 0 R >>\n    /XObject << /Im0 5 0 R >>\n  >>\n  /Contents 6 0 R\n>>\nendobj",
        w, h
    ).unwrap();

    // Object 4: Font (Helvetica)
    offsets.push(pdf.len());
    writeln!(
        pdf,
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /StandardEncoding >>\nendobj"
    ).unwrap();

    // Object 5: Image XObject
    offsets.push(pdf.len());
    writeln!(
        pdf,
        "5 0 obj\n<<\n  /Type /XObject\n  /Subtype /Image\n  /Width {}\n  /Height {}\n  /ColorSpace /DeviceRGB\n  /BitsPerComponent 8\n  /Filter /DCTDecode\n  /Length {}\n>>\nstream",
        w, h, jpeg_bytes.len()
    ).unwrap();
    pdf.extend_from_slice(&jpeg_bytes);
    writeln!(pdf, "\nendstream\nendobj").unwrap();

    // Object 6: Contents stream
    offsets.push(pdf.len());
    writeln!(
        pdf,
        "6 0 obj\n<< /Length {} >>\nstream",
        text_stream.len()
    ).unwrap();
    pdf.extend_from_slice(&text_stream);
    writeln!(pdf, "endstream\nendobj").unwrap();

    // Cross-reference table
    let xref_offset = pdf.len();
    writeln!(pdf, "xref\n0 7\n0000000000 65535 f ").unwrap();
    for offset in &offsets {
        writeln!(pdf, "{:010} 00000 n ", offset).unwrap();
    }

    // Trailer
    writeln!(
        pdf,
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
        xref_offset
    ).unwrap();

    Ok(pdf)
}

fn escape_pdf_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            // Only Latin-1 / ASCII safe characters in StandardEncoding
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}
