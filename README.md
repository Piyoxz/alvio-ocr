# alvio-ocr

High-performance OCR library for Rust, powered by ONNX Runtime and PP-OCRv4. Built for extreme speed, high accuracy, and zero-configuration ergonomics.

## Key Features

- **Blazing Fast**: SIMD-accelerated resizing (AVX2/NEON), single-pass batch recognition inference, and parallel multi-page PDF processing.
- **Universal Input**: Recognize text directly from local file paths, web URLs (`http://`, `https://`), raw memory bytes, or decoded images.
- **Multi-File Batching**: Process dozens or hundreds of files in parallel across CPU threads with automatic workload balancing.
- **Zero Configuration**: Missing ONNX models and native runtime libraries (Pdfium) are automatically downloaded to user cache on first run.
- **Rich Structured Output**: Returns text, overall confidence, structured reading lines, word bounding boxes, TSV exports, and JSON serialization.
- **Multi-Language Support**: Pre-configured presets for Latin (Indonesian, English) and Universal Multilingual (6,625+ CJK characters and symbols).
- **Format Validation**: Automatic image format sniffing and explicit rejection of unsupported files with helpful error messages.

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
alvio-ocr = "0.1"
```

To enable PDF document OCR, enable the `pdf` feature:

```toml
[dependencies]
alvio-ocr = { version = "0.1", features = ["pdf"] }
```

## Supported File Formats

`alvio-ocr` automatically detects file formats via magic bytes and extensions:

| Format | File Extensions | MIME Type | Status |
|---|---|---|---|
| JPEG | `.jpg`, `.jpeg` | `image/jpeg` | Supported |
| PNG | `.png` | `image/png` | Supported |
| WebP | `.webp` | `image/webp` | Supported |
| BMP | `.bmp` | `image/bmp` | Supported |
| TIFF | `.tiff`, `.tif` | `image/tiff` | Supported |
| PDF | `.pdf` | `application/pdf` | Supported (with `features = ["pdf"]`) |

### Rejected Formats & Error Handling

Attempting to process unsupported file formats (such as `.docx`, `.xlsx`, `.txt`, `.mp4`, `.exe`) returns an explicit `OcrError::UnsupportedFormat`:

```rust
use alvio_ocr::OcrEngine;

fn main() {
    let engine = OcrEngine::auto().unwrap();
    let result = engine.recognize("archive.zip");

    match result {
        Ok(output) => println!("{}", output.text),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

You can inspect all supported formats programmatically at runtime:

```rust
let supported = alvio_ocr::supported_formats();
println!("Supported formats: {:?}", supported);
```

## Supported Languages & Models

`alvio-ocr` includes built-in presets covering major world languages:

| Language Preset | Code | Character Set | Character Count | Underlying ONNX Model | Auto-Download |
|---|---|---|---|---|---|
| `OcrLanguage::Indonesian` | `"id"` | Latin alphabet (A-Z, a-z), numbers, punctuation | 95 | `en_PP-OCRv4_rec_infer.onnx` | Yes |
| `OcrLanguage::English` | `"en"` | Latin alphabet (A-Z, a-z), numbers, punctuation | 95 | `en_PP-OCRv4_rec_infer.onnx` | Yes |
| `OcrLanguage::Multilingual` | `"multi"` | Universal: Latin, Chinese (Hanzi), Japanese (Kanji), symbols | 6,625+ | `ch_PP-OCRv4_rec_infer.onnx` | Yes |
| `OcrLanguage::Chinese` | `"zh"` | Simplified & Traditional Chinese, Latin, digits | 6,625+ | `ch_PP-OCRv4_rec_infer.onnx` | Yes |

You can query available languages and their details via code:

```rust
for lang in alvio_ocr::supported_languages() {
    println!("{}: {} ({} characters)", lang.id, lang.name, lang.character_count);
}
```

## Quick Start

### 1. Universal Recognition

The `recognize()` method automatically distinguishes between local file paths and remote URLs:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;

    let local_result = engine.recognize("invoice.png")?;
    println!("Local text:\n{}", local_result.text);

    let remote_result = engine.recognize("https://example.com/receipt.jpg")?;
    println!("Remote text:\n{}", remote_result.text);

    Ok(())
}
```

### 2. Recognize from URL

Fetch and process images directly from the web without saving them to disk first:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;
    let result = engine.recognize_url("https://example.com/document.webp")?;

    println!("Recognized: {}", result.text);
    println!("Confidence: {:.2}%", result.confidence * 100.0);
    Ok(())
}
```

### 3. Multiple Files Batch Processing

Process multiple files in parallel across available CPU cores using Rayon:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;

    let files = ["page1.png", "page2.jpg", "page3.webp"];
    let batch = engine.recognize_files(&files)?;

    println!("Total: {}, Succeeded: {}, Failed: {}", batch.total, batch.succeeded, batch.failed);
    println!("Elapsed: {} ms", batch.total_duration_ms);

    for item in batch.items {
        if let Some(result) = item.result {
            println!("{}: {} words (took {} ms)", item.source, result.words().len(), item.duration_ms);
        } else if let Some(err) = item.error {
            eprintln!("Failed {}: {}", item.source, err);
        }
    }

    Ok(())
}
```

### 4. Working with the Structured Output

`OcrResult` provides rich metadata and export formats:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;
    let result = engine.recognize("document.jpg")?;

    println!("Text:\n{}", result.text);
    println!("Average Confidence: {:.4}", result.confidence);
    println!("Image Dimensions: {}x{}", result.dimensions.0, result.dimensions.1);

    for line in &result.lines {
        println!("Line {} [conf={:.2}]: {}", line.line_number, line.confidence, line.text);
    }

    for region in &result.regions {
        let bbox = region.region.bbox();
        println!("Word '{}' at ({:.0}, {:.0})", region.text, bbox.min_x, bbox.min_y);
    }

    let json_string = result.to_json_pretty();
    let tsv_table = result.to_tsv();

    let matches = result.search("total");
    println!("Found {} occurrences of 'total'", matches.len());

    let high_conf = result.filter_by_confidence(0.9);

    Ok(())
}
```

### 5. PDF Documents

Process multi-page PDF files with parallel OCR extraction:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;
    let pdf_bytes = std::fs::read("statement.pdf")?;

    let pages = engine.recognize_pdf(&pdf_bytes)?;

    for page in &pages {
        println!("--- Page {} [{:?}] ---", page.page_number, page.source);
        println!("{}\n", page.text);
    }

    Ok(())
}
```

## Performance Comparison: Debug vs Release

Compile with the `--release` flag to enable SIMD loop vectorization and aggressive inlining:

| Operation | Debug Profile | Release Profile (`--release`) | Speedup |
|---|---|---|---|
| Engine Initialization | ~243 ms | **~190 ms** | 1.2x |
| Single Image OCR | ~490 ms | **~119 ms** | **4.1x faster** |
| Complex Document OCR | ~525 ms | **~126 ms** | **4.2x faster** |
| Scanned PDF OCR | ~1,320 ms | **~198 ms** | **6.7x faster** |
| Memory Footprint | ~180 MB | **~60-110 MB** | Significantly reduced |

## License

MIT License. Free for commercial and private use.
