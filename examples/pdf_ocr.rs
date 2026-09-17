//! PDF OCR example — recognize text from a PDF document.
//!
//! Requires the `pdf` feature:
//! ```bash
//! cargo run --example pdf_ocr --features pdf -- path/to/document.pdf
//! ```

use alvio_ocr::OcrEngine;
use std::env;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example pdf_ocr --features pdf -- <path-to-pdf>");
        std::process::exit(1);
    }

    let file_path = &args[1];

    let lang_str = env::var("OCR_LANG").ok().or_else(|| {
        args.iter().position(|a| a == "--lang")
            .and_then(|idx| args.get(idx + 1).cloned())
    }).unwrap_or_else(|| "id".to_string());

    let lang = alvio_ocr::OcrLanguage::from_code(&lang_str).unwrap_or(alvio_ocr::OcrLanguage::Indonesian);

    println!("Initializing PDF OCR engine [{}] (zero configuration) ...", lang.display_name());
    let start = Instant::now();
    let engine = if let Ok(custom_dir) = env::var("OCR_MODEL_DIR") {
        OcrEngine::with_language(&custom_dir, lang)?
    } else {
        OcrEngine::auto_with_language(lang)?
    };
    println!("Engine ready in {:.2?}\n", start.elapsed());

    println!("Processing PDF '{}' ...", file_path);
    let pdf_bytes = std::fs::read(file_path)?;

    let start = Instant::now();
    let pages = engine.recognize_pdf(&pdf_bytes)?;
    let elapsed = start.elapsed();

    println!("=== PDF OCR Result ({} pages) ===\n", pages.len());

    for page in &pages {
        println!("--- Page {} [source: {:?}] ---", page.page_number, page.source);
        println!("{}\n", page.text);

        if !page.regions.is_empty() {
            println!("  Regions: {}", page.regions.len());
            for (i, r) in page.regions.iter().enumerate() {
                println!("    [{}] conf={:.4} → \"{}\"", i + 1, r.confidence, r.text);
            }
            println!();
        }
    }

    println!("Total processed in {:.2?}", elapsed);
    Ok(())
}
