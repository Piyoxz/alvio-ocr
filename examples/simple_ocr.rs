//! Simple OCR example — recognize text from an image file.
//!
//! Usage:
//! ```bash
//! cargo run --example simple_ocr -- path/to/image.jpg
//! ```

use alvio_ocr::OcrEngine;
use std::env;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example simple_ocr -- <path-to-image>");
        eprintln!("Example: cargo run --example simple_ocr -- test.jpg");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let lang_str = env::var("OCR_LANG").ok().or_else(|| {
        args.iter().position(|a| a == "--lang")
            .and_then(|idx| args.get(idx + 1).cloned())
    }).unwrap_or_else(|| "id".to_string());

    let lang = alvio_ocr::OcrLanguage::from_code(&lang_str).unwrap_or(alvio_ocr::OcrLanguage::Indonesian);

    // Initialize engine (auto-downloads models if not yet present)
    println!("Initializing OCR engine [{}] (zero configuration) ...", lang.display_name());
    let start = Instant::now();
    let engine = if let Ok(custom_dir) = env::var("OCR_MODEL_DIR") {
        OcrEngine::with_language(&custom_dir, lang)?
    } else {
        OcrEngine::auto_with_language(lang)?
    };
    println!("Engine ready in {:.2?}\n", start.elapsed());

    // Run OCR
    println!("Processing '{}' ...", file_path);
    let start = Instant::now();
    let result = engine.recognize_file(file_path)?;
    let elapsed = start.elapsed();

    // Print results
    println!("=== OCR Result ===");
    println!("Text:\n{}\n", result.text);

    println!("--- Regions ({}) ---", result.regions.len());
    for (i, region) in result.regions.iter().enumerate() {
        let (x, y, x2, y2) = region.region.aabb();
        println!(
            "  [{}] conf={:.4} bbox=({:.0},{:.0})-({:.0},{:.0}) → \"{}\"",
            i + 1,
            region.confidence,
            x,
            y,
            x2,
            y2,
            region.text
        );
    }

    println!("\nProcessed in {:.2?}", elapsed);
    Ok(())
}
