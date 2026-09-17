# alvio-ocr

High-performance OCR library for Rust, powered by ONNX Runtime and PP-OCRv4.

## Features

- **Fast**: SIMD-accelerated image resize, batch recognition inference, and parallel PDF processing
- **Accurate**: Adaptive image enhancement, NMS deduplication, and proportional padding
- **Zero Configuration**: Models and native Pdfium binaries are automatically downloaded on first use
- **Multi-format**: JPEG, PNG, WebP, BMP, TIFF, and PDF (via optional feature)
- **Multi-language**: Built-in support for Latin scripts (Indonesian, English) and Multilingual (6,625+ characters)
- **Pure Rust API**: Thread-safe, memory-safe, and ergonomic design

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
alvio-ocr = "0.1"
```

For PDF document support, enable the `pdf` feature:

```toml
[dependencies]
alvio-ocr = { version = "0.1", features = ["pdf"] }
```

## Quick Start

Initialize the engine without parameters. Missing models and binaries are automatically downloaded to the user cache directory:

```rust
use alvio_ocr::OcrEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = OcrEngine::auto()?;

    let result = engine.recognize_file("document.jpg")?;
    println!("Recognized text:\n{}", result.text);

    let bytes = std::fs::read("scan.png")?;
    let result = engine.recognize_bytes(&bytes)?;

    #[cfg(feature = "pdf")]
    {
        let pdf_bytes = std::fs::read("document.pdf")?;
        let pages = engine.recognize_pdf(&pdf_bytes)?;
        for page in &pages {
            println!("Page {}: {}", page.page_number, page.text);
        }
    }

    Ok(())
}
```

## Language Presets

The engine provides language presets:

### Indonesian and English

The default preset uses a lightweight Latin alphabet model containing 97 characters. It delivers the fastest inference speed and high accuracy for documents written in Indonesian or English:

```rust
use alvio_ocr::{OcrEngine, OcrLanguage};

let engine = OcrEngine::auto()?;
let engine = OcrEngine::auto_with_language(OcrLanguage::Indonesian)?;
```

### Multilingual and CJK

For documents containing Chinese (Hanzi), Japanese (Kanji), or mathematical symbols, use the Multilingual preset covering 6,625+ characters:

```rust
use alvio_ocr::{OcrEngine, OcrLanguage};

let engine = OcrEngine::auto_with_language(OcrLanguage::Multilingual)?;
```

### Custom Models

You can supply custom ONNX models and dictionaries from PaddleOCR:

```rust
use alvio_ocr::{OcrConfig, OcrEngine};

let config = OcrConfig::default_with_model_dir("./models")
    .recognition_model_path("./models/custom_rec.onnx")
    .dictionary_path("./models/custom_dict.txt");

let engine = OcrEngine::with_config(config)?;
```

## Advanced Configuration

Fine-tune detection thresholds, image limits, and thread allocation:

```rust
use alvio_ocr::{OcrConfig, OcrEngine};

let config = OcrConfig::default()
    .det_max_side_len(1280)
    .det_threshold(0.25)
    .rec_threshold(0.4)
    .nms_iou_threshold(0.5)
    .enable_enhancement(true)
    .intra_threads(4)
    .max_image_width(8192)
    .max_image_height(8192);

let engine = OcrEngine::with_config(config)?;
```

## Performance Optimizations

| Optimization | Technique | Impact |
|---|---|---|
| SIMD Resize | `fast_image_resize` with AVX2 and NEON | Resize operations are 10x to 15x faster |
| Batch Recognition | Single batched ONNX tensor inference | Recognition inference runs 2x to 5x faster |
| Parallel Inference | ONNX Runtime parallel execution | Lowers inference latency across multiple cores |
| Zero-Copy Normalization | Direct Array4 buffer transformation | Eliminates intermediate memory allocations |
| Two-Phase PDF Pipeline | Sequential render with Rayon page OCR | Near-linear speedup across multiple CPU cores |
| NMS Filtering | Overlap suppression before recognition | Reduces recognition workload |

## Accuracy Optimizations

| Optimization | Technique | Impact |
|---|---|---|
| Higher Detection Size | Default input bound raised to 960px | Significantly sharpens small font detection |
| Bilinear Sampling | Sub-pixel anti-aliasing interpolation | Preserves text edge geometry during resize |
| Proportional Padding | Height-relative bounding box padding | Eliminates character boundary clipping |
| Adaptive Enhancement | Automated contrast and sharpen filters | Improves readability on degraded inputs |
| IoU-based NMS | Intersection over Union polygon deduplication | Prevents duplicate word detections |
| Spatial Reading Order | Column-aware sorting algorithm | Corrects column and paragraph ordering |

## Pretrained Models

| Model | Purpose | Default Preset | File Name |
|---|---|---|---|
| DBNet v4 | Universal Text Detection | All languages | `ch_PP-OCRv4_det_infer.onnx` |
| SVTR v4 Latin | Text Recognition (97 characters) | Indonesian, English | `en_PP-OCRv4_rec_infer.onnx` |
| SVTR v4 Universal | Text Recognition (6,625+ characters) | Multilingual, Chinese | `ch_PP-OCRv4_rec_infer.onnx` |

## License

MIT
