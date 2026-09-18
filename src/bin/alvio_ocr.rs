use alvio_ocr::{
    supported_formats, supported_languages, OcrEngine,
};
use std::env;
use std::path::Path;
use std::time::Instant;

fn print_help() {
    println!("alvio-ocr v{} - High-Performance Native Document OCR Engine", env!("CARGO_PKG_VERSION"));
    println!("\nUSAGE:");
    println!("  alvio-ocr <INPUT_FILE_OR_URL> [OPTIONS]");
    println!("  alvio-ocr info\n");
    println!("OPTIONS:");
    println!("  --format <text|json|tsv|hocr>   Output format (default: text)");
    println!("  --schema <ktp|npwp|sim|receipt|table|entities>  Run specialized Document AI parser");
    println!("  --searchable-pdf <OUTPUT_FILE>  Generate searchable PDF with invisible text layer");
    println!("  --annotate <OUTPUT_FILE>        Save visual preview image with bounding boxes");
    println!("  --deskew                        Force document deskew preprocessing");
    println!("  --help, -h                      Show this help message");
    println!("\nEXAMPLES:");
    println!("  alvio-ocr document.jpg");
    println!("  alvio-ocr document.jpg --format json");
    println!("  alvio-ocr ktp_sample.jpg --schema ktp");
    println!("  alvio-ocr invoice.png --schema receipt");
    println!("  alvio-ocr scan.jpg --searchable-pdf output.pdf");
    println!("  alvio-ocr scan.jpg --annotate preview.png");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_help();
        return Ok(());
    }

    if args[1] == "info" {
        println!("Supported File Formats: {:?}", supported_formats());
        println!("Available Languages:");
        for lang in supported_languages() {
            println!("  [{}] {} ({} characters)", lang.id, lang.name, lang.character_count);
        }
        return Ok(());
    }

    let input_path = &args[1];
    let mut format = "text".to_string();
    let mut schema = None;
    let mut searchable_pdf_out = None;
    let mut annotate_out = None;
    let mut force_deskew = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                if i + 1 < args.len() {
                    format = args[i + 1].to_lowercase();
                    i += 1;
                }
            }
            "--schema" => {
                if i + 1 < args.len() {
                    schema = Some(args[i + 1].to_lowercase());
                    i += 1;
                }
            }
            "--searchable-pdf" => {
                if i + 1 < args.len() {
                    searchable_pdf_out = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--annotate" => {
                if i + 1 < args.len() {
                    annotate_out = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--deskew" => {
                force_deskew = true;
            }
            _ => {}
        }
        i += 1;
    }

    let start = Instant::now();
    let mut config = alvio_ocr::OcrConfig::default();
    if force_deskew {
        config.deskew_enabled = true;
    }

    let engine = OcrEngine::with_config(config)?;

    let is_pdf = Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);

    if is_pdf {
        #[cfg(feature = "pdf")]
        {
            let pdf_bytes = std::fs::read(input_path)?;
            let pages = engine.recognize_pdf(&pdf_bytes)?;
            println!("Processed PDF '{}' ({} pages) in {:.2?}:", input_path, pages.len(), start.elapsed());
            for page in &pages {
                println!("\n--- Page {} [source: {:?}] ---", page.page_number, page.source);
                println!("{}", page.text);
            }
            return Ok(());
        }
        #[cfg(not(feature = "pdf"))]
        {
            eprintln!("PDF recognition requires compiling alvio-ocr with `--features pdf`");
            return Ok(());
        }
    }

    let result = engine.recognize(input_path)?;

    // Handle searchable PDF export
    if let Some(pdf_out) = searchable_pdf_out {
        let img = image::open(input_path)?;
        let pdf_bytes = result.to_searchable_pdf(&img)?;
        std::fs::write(&pdf_out, pdf_bytes)?;
        eprintln!("[alvio-ocr] Saved searchable PDF to: {}", pdf_out);
    }

    // Handle annotation export
    if let Some(anno_out) = annotate_out {
        let img = image::open(input_path)?;
        let annotated = result.to_annotated_image(&img);
        annotated.save(&anno_out)?;
        eprintln!("[alvio-ocr] Saved annotated image to: {}", anno_out);
    }

    // Handle schema extraction
    if let Some(s) = schema {
        match s.as_str() {
            "ktp" => {
                let ktp = result.to_ktp();
                println!("{}", ktp.to_json());
            }
            "npwp" => {
                let npwp = result.to_npwp();
                println!("{}", npwp.to_json());
            }
            "sim" => {
                let sim = result.to_sim();
                println!("{}", sim.to_json());
            }
            "receipt" | "invoice" => {
                let receipt = result.to_receipt();
                println!("{}", receipt.to_json());
            }
            "table" => {
                let table = result.to_table();
                if format == "csv" {
                    println!("{}", table.to_csv());
                } else if format == "json" {
                    println!("{}", table.to_json());
                } else {
                    println!("{}", table.to_markdown());
                }
            }
            "entities" => {
                let entities = result.to_entities();
                println!("{}", entities.to_json());
            }
            other => {
                eprintln!("Unknown schema '{}'. Available: ktp, npwp, sim, receipt, table, entities", other);
            }
        }
        return Ok(());
    }

    // Handle output formatting
    match format.as_str() {
        "json" => println!("{}", result.to_json_pretty()),
        "tsv" => print!("{}", result.to_tsv()),
        "hocr" => println!("{}", result.to_hocr()),
        _ => {
            println!("{}", result.text);
            eprintln!("\n[Execution Time: {:.2?}, Confidence: {:.2}%]", start.elapsed(), result.confidence * 100.0);
        }
    }

    Ok(())
}
