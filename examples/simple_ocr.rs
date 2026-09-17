use alvio_ocr::OcrEngine;
use std::env;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("Initializing OCR engine [{}] (PP-OCRv6) ...", lang.display_name());
    let start = Instant::now();
    let engine = if let Ok(custom_dir) = env::var("OCR_MODEL_DIR") {
        OcrEngine::with_language(&custom_dir, lang)?
    } else {
        OcrEngine::auto_with_language(lang)?
    };
    println!("Engine ready in {:.2?}\n", start.elapsed());

    println!("Processing '{}' ...", file_path);
    let start = Instant::now();
    let result = engine.recognize_file(file_path)?;
    let elapsed = start.elapsed();

    println!("=== OCR Result ===");
    println!("Text:\n{}\n", result.text);

    println!("--- Regions ({}) ---", result.regions.len());
    for (i, region) in result.regions.iter().enumerate() {
        let (x, y, x2, y2) = region.region.aabb();
        println!(
            "  [{}] conf={:.4} bbox=({:.0},{:.0})-({:.0},{:.0}) -> \"{}\"",
            i + 1,
            region.confidence,
            x,
            y,
            x2,
            y2,
            region.text
        );
    }

    println!("\n--- Lines ({}) ---", result.lines.len());
    for line in &result.lines {
        println!("  Line {}: \"{}\"", line.line_number, line.text);
    }

    println!("\n{}", result.timing.summary());

    let ktp = result.to_ktp();
    if ktp.nik.is_some() || ktp.nama.is_some() {
        println!("\n--- Document AI: KTP Extractor ---");
        if let Some(nik) = &ktp.nik {
            println!("  NIK:      {}", nik);
        }
        if let Some(nama) = &ktp.nama {
            println!("  Nama:     {}", nama);
        }
        if let Some(ttl) = &ktp.tempat_tgl_lahir {
            println!("  TTL:      {}", ttl);
        }
        if let Some(alamat) = &ktp.alamat {
            println!("  Alamat:   {}", alamat);
        }
    }

    let receipt = result.to_receipt();
    if receipt.total.is_some() || !receipt.items.is_empty() {
        println!("\n--- Document AI: Receipt / Invoice ---");
        if let Some(merchant) = &receipt.merchant_name {
            println!("  Merchant: {}", merchant);
        }
        if let Some(date) = &receipt.date {
            println!("  Date:     {}", date);
        }
        if !receipt.items.is_empty() {
            println!("  Items ({}):", receipt.items.len());
            for item in &receipt.items {
                let price_str = item.price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "-".to_string());
                println!("    - {} : {}", item.name, price_str);
            }
        }
        if let Some(total) = receipt.total {
            println!("  Total:    {:.2}", total);
        }
    }

    let table = result.to_table();
    if table.rows.len() > 1 && !table.rows[0].is_empty() {
        println!("\n--- Document AI: Table Reconstructor ---");
        println!("{}", table.to_markdown());
    }

    println!("\nTotal processed in {:.2?} (confidence: {:.1}%)", elapsed, result.confidence * 100.0);
    Ok(())
}
