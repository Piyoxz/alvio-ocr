use alvio_ocr::OcrEngine;
use std::path::Path;

fn get_model_dir() -> &'static str {
    if Path::new("./models").exists() {
        "./models"
    } else {
        "../ocr/models/ocr"
    }
}

#[test]
fn test_engine_initialization() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir);
    assert!(engine.is_ok(), "Failed to initialize default OcrEngine: {:?}", engine.err());

    let multi_engine = OcrEngine::with_language(model_dir, alvio_ocr::OcrLanguage::Multilingual);
    assert!(multi_engine.is_ok(), "Failed to initialize Multilingual OcrEngine: {:?}", multi_engine.err());
}

#[test]
fn test_auto_engine() {
    let engine = OcrEngine::auto();
    assert!(engine.is_ok(), "Failed to initialize auto OcrEngine: {:?}", engine.err());
}

#[test]
fn test_image_ocr_accuracy() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir).expect("Engine should initialize");

    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let result = engine.recognize_file(sample_image).expect("OCR should succeed");
        assert!(!result.text.is_empty(), "Recognized text should not be empty");
        assert!(result.text.contains("JAKARTA") || result.text.contains("PROVINSI"), "Expected text to match fixture");
        assert!(!result.regions.is_empty(), "Should have detected regions");
        for r in &result.regions {
            assert!(r.confidence > 0.5, "Confidence should be > 0.5");
        }
    }
}

#[cfg(feature = "pdf")]
#[test]
fn test_pdf_ocr() {
    let model_dir = get_model_dir();
    let engine = OcrEngine::new(model_dir).expect("Engine should initialize");

    let sample_pdf = "D:/riset_alvio/ocr/tests/fixtures/ocr/scanned.pdf";
    if Path::new(sample_pdf).exists() {
        let pdf_bytes = std::fs::read(sample_pdf).expect("Read PDF");
        let pages = engine.recognize_pdf(&pdf_bytes).expect("PDF OCR should succeed");
        assert_eq!(pages.len(), 1, "Should have 1 page");
        assert!(pages[0].text.contains("SCAN") || pages[0].text.contains("DOKUMEN"), "Expected text to match fixture");
    }
}

#[test]
fn test_supported_formats_and_languages() {
    let formats = alvio_ocr::supported_formats();
    assert!(formats.contains(&"jpeg"));
    assert!(formats.contains(&"png"));
    assert!(formats.contains(&"pdf"));

    let languages = alvio_ocr::supported_languages();
    assert_eq!(languages.len(), 4);
    assert!(languages.iter().any(|l| l.id == "id"));
    assert!(languages.iter().any(|l| l.id == "multi"));
}

#[test]
fn test_unsupported_format_rejection() {
    let engine = OcrEngine::auto().expect("Engine auto");
    let result = engine.recognize_file("test_document.docx");
    assert!(result.is_err(), "Unsupported format should be rejected");
    match result {
        Err(alvio_ocr::OcrError::UnsupportedFormat { detected, .. }) => {
            assert_eq!(detected, "docx");
        }
        other => panic!("Expected UnsupportedFormat error, got: {:?}", other),
    }
}

#[test]
fn test_batch_files_processing() {
    let engine = OcrEngine::auto().expect("Engine auto");
    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let batch = engine.recognize_files(&[sample_image, sample_image]).expect("Batch OCR");
        assert_eq!(batch.total, 2);
        assert_eq!(batch.succeeded, 2);
        assert_eq!(batch.failed, 0);
        assert!(batch.all_succeeded());
        assert_eq!(batch.texts().len(), 2);
    }
}

#[test]
fn test_rich_response_methods() {
    let engine = OcrEngine::auto().expect("Engine auto");
    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let res = engine.recognize_file(sample_image).expect("OCR");
        assert!(res.confidence > 0.0);
        assert!(!res.lines.is_empty());
        assert!(!res.to_json().is_empty());
        assert!(!res.to_tsv().is_empty());
        let filtered = res.filter_by_confidence(0.5);
        assert!(!filtered.text.is_empty());
    }
}

#[test]
fn test_ppocrv6_accuracy_and_performance() {
    let model_dir = Path::new("D:/riset_alvio/ocr/models/ocr");
    let det_v6 = if model_dir.join("PP-OCRv6_det_small.onnx").exists() {
        model_dir.join("PP-OCRv6_det_small.onnx")
    } else {
        model_dir.join("ppocrv6_small_det.onnx")
    };
    let rec_v6 = if model_dir.join("PP-OCRv6_rec_small.onnx").exists() {
        model_dir.join("PP-OCRv6_rec_small.onnx")
    } else {
        model_dir.join("ppocrv6_small_rec.onnx")
    };
    let dict_v6 = if model_dir.join("ppocrv6_keys.txt").exists() {
        model_dir.join("ppocrv6_keys.txt")
    } else {
        model_dir.join("ppocr_keys_v1.txt")
    };

    if det_v6.exists() && rec_v6.exists() && dict_v6.exists() {
        let mut config = alvio_ocr::OcrConfig::default_with_model_dir(model_dir);
        config.detection_model_path = det_v6;
        config.recognition_model_path = rec_v6;
        config.dictionary_path = dict_v6;

        let engine_v6 = OcrEngine::with_config(config).expect("PP-OCRv6 engine should initialize");

        let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
        if Path::new(sample_image).exists() {
            let start = std::time::Instant::now();
            let result_v6 = engine_v6.recognize_file(sample_image).expect("PP-OCRv6 OCR should succeed");
            let elapsed = start.elapsed();
            println!("\n==========================================");
            println!("   PP-OCRv6 BENCHMARK RESULT (Rust)");
            println!("==========================================");
            println!("Latency: {:?}", elapsed);
            println!("Confidence: {:.2}%", result_v6.confidence * 100.0);
            println!("Detected lines: {}", result_v6.lines.len());
            for line in &result_v6.lines {
                println!("  Line {}: {}", line.line_number, line.text);
            }
            println!("==========================================\n");

            assert!(!result_v6.text.is_empty());
            assert!(result_v6.text.contains("JAKARTA") || result_v6.text.contains("PROVINSI"));
        }

        #[cfg(feature = "pdf")]
        {
            let sample_pdf = "D:/riset_alvio/ocr/tests/fixtures/ocr/ocr_demo.pdf";
            if Path::new(sample_pdf).exists() {
                let pdf_bytes = std::fs::read(sample_pdf).expect("Read PDF");
                let start = std::time::Instant::now();
                let pages = engine_v6.recognize_pdf(&pdf_bytes).expect("PP-OCRv6 PDF OCR should succeed");
                let elapsed = start.elapsed();
                println!("\n==========================================");
                println!("   PP-OCRv6 PDF BENCHMARK (4 Pages)");
                println!("==========================================");
                println!("Total Latency: {:?}", elapsed);
                println!("Pages: {}", pages.len());
                for page in &pages {
                    println!("Page {} [via {:?}]: lines={}", page.page_number, page.source, page.lines.len());
                }
                println!("==========================================\n");
                assert_eq!(pages.len(), 4);
            }
        }
    }
}

#[test]
fn test_cer_wer_metrics() {
    let reference = "PROVINSI DKI JAKARTA";
    let hyp_exact = "PROVINSI DKI JAKARTA";
    let hyp_typo = "PROVINSI DKI JAKARTO";

    assert_eq!(alvio_ocr::compute_cer(reference, hyp_exact), 0.0);
    assert_eq!(alvio_ocr::compute_wer(reference, hyp_exact), 0.0);

    let cer_typo = alvio_ocr::compute_cer(reference, hyp_typo);
    assert!(cer_typo > 0.0 && cer_typo < 0.1);

    let wer_typo = alvio_ocr::compute_wer(reference, hyp_typo);
    assert!(wer_typo > 0.0);
}

#[test]
fn test_ktp_extraction() {
    let sample_ktp_text = "PROVINSI DKI JAKARTA\nKOTA JAKARTA SELATAN\nNIK : 3171020304050001\nNama : BUDI SANTOSO\nTempat/Tgl Lahir : JAKARTA, 01-01-1990\nJenis Kelamin : LAKI-LAKI\nAlamat : JL. SUDIRMAN NO. 10\nRT/RW : 001/002\nKel/Desa : SENAYAN\nKecamatan : KEBAYORAN BARU\nAgama : ISLAM\nStatus Perkawinan : KAWIN\nPekerjaan : PEGAWAI SWASTA\nKewarganegaraan : WNI\nBerlaku Hingga : SEUMUR HIDUP";

    let ktp = alvio_ocr::extract_ktp(sample_ktp_text);
    assert_eq!(ktp.nik, Some("3171020304050001".to_string()));
    assert_eq!(ktp.nama, Some("BUDI SANTOSO".to_string()));
    assert_eq!(ktp.jenis_kelamin, Some("LAKI-LAKI".to_string()));
    assert_eq!(ktp.kewarganegaraan, Some("WNI".to_string()));
    assert!(ktp.is_valid_nik());
}

#[test]
fn test_receipt_extraction() {
    let sample_receipt = "WARUNG MAKAN SEDAP\nTANGGAL: 17/09/2026\nINVOICE: INV-9988\nNASI GORENG  25.000\nES TEH MANIS  5.000\nSUBTOTAL: 30.000\nPPN: 3.300\nTOTAL: 33.300";

    let receipt = alvio_ocr::extract_receipt(sample_receipt);
    assert_eq!(receipt.merchant_name, Some("WARUNG MAKAN SEDAP".to_string()));
    assert_eq!(receipt.total, Some(33300.0));
    assert_eq!(receipt.subtotal, Some(30000.0));
    assert_eq!(receipt.tax, Some(3300.0));
    assert_eq!(receipt.invoice_number, Some("INV-9988".to_string()));
}

#[test]
fn test_table_reconstruction() {
    use alvio_ocr::{OcrText, Point, TextRegion};

    let make_box = |text: &str, x: f32, y: f32, w: f32, h: f32| {
        let region = TextRegion {
            polygon: vec![
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ],
            confidence: 0.99,
        };
        OcrText::new(text.to_string(), 0.99, region)
    };

    let items = vec![
        make_box("No", 10.0, 10.0, 30.0, 20.0),
        make_box("Item", 50.0, 10.0, 60.0, 20.0),
        make_box("1", 10.0, 40.0, 30.0, 20.0),
        make_box("Apple", 50.0, 40.0, 60.0, 20.0),
    ];

    let table = alvio_ocr::reconstruct_table(items);
    assert_eq!(table.rows.len(), 2);
    let md = table.to_markdown();
    assert!(md.contains("No") && md.contains("Item") && md.contains("Apple"));
}

#[test]
fn test_stage_timing_on_real_image() {
    let engine = OcrEngine::auto().expect("Engine auto");
    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let res = engine.recognize_file(sample_image).expect("OCR result");
        assert!(res.timing.total_ms > 0.0);
        assert!(res.timing.preprocessing_ms > 0.0);
        assert!(res.timing.detection_ms > 0.0);
        assert!(res.timing.recognition_ms > 0.0);
        let summary = res.timing.summary();
        assert!(summary.contains("STAGE TIMING BREAKDOWN"));

        let ktp = res.to_ktp();
        assert!(ktp.provinsi.is_some() || ktp.nik.is_some() || ktp.nama.is_some());
    }
}

#[test]
fn test_npwp_extraction() {
    let text = "KEMENTERIAN KEUANGAN REPUBLIK INDONESIA\nDIREKTORAT JENDERAL PAJAK\nNPWP : 01.234.567.8-901.000\nNAMA : PT MAJU BERSAMA JAYA\nNIK : 3171012345670001\nALAMAT : JL. JENDERAL SUDIRMAN NO. 45\nKPP PRATAMA JAKARTA SETIABUDI SATU\nTERDAFTAR : 15/01/2020";
    let npwp = alvio_ocr::extract_npwp(text);
    assert!(npwp.is_valid());
    assert_eq!(npwp.npwp_raw, Some("012345678901000".to_string()));
    assert_eq!(npwp.npwp, Some("01.234.567.8-901.000".to_string()));
    assert_eq!(npwp.nama, Some("PT MAJU BERSAMA JAYA".to_string()));
    assert_eq!(npwp.nik, Some("3171012345670001".to_string()));
    assert!(npwp.alamat.unwrap().contains("SUDIRMAN"));
    assert_eq!(npwp.kpp, Some("PRATAMA JAKARTA SETIABUDI SATU".to_string()));
}

#[test]
fn test_sim_extraction() {
    let text = "KEPOLISIAN NEGARA REPUBLIK INDONESIA\nSURAT IZIN MENGEMUDI (SIM A)\nNO SIM : 920112345678\n1. NAMA : BUDI PRASETYO\n2. TEMPAT/TGL LAHIR : JAKARTA, 12-08-1995\nPRIA\n4. ALAMAT : JL. KEMANG RAYA NO 10\nRT/RW 002/005 KEL. BANGKA\n5. PEKERJAAN : KARYAWAN SWASTA\nBERLAKU S/D : 12-08-2028";
    let sim = alvio_ocr::extract_sim(text);
    assert!(sim.is_valid());
    assert_eq!(sim.golongan, Some("A".to_string()));
    assert_eq!(sim.nomor_sim, Some("920112345678".to_string()));
    assert_eq!(sim.nama, Some("BUDI PRASETYO".to_string()));
    assert_eq!(sim.jenis_kelamin, Some("LAKI-LAKI".to_string()));
    assert_eq!(sim.berlaku_hingga, Some("12-08-2028".to_string()));
}

#[test]
fn test_entity_extraction() {
    let text = "Hubungi customer care di +62 812 3456 7890 atau email contact@alvio.test.\nTotal transaksi Rp 750.000,- dibayar pada 17 September 2026.\nKendaraan ekspedisi B 1234 XYZ dengan NIK kurir 3171234567890001.\nWebsite: https://alvio-ocr.dev";
    let entities = alvio_ocr::extract_entities(text);
    assert!(!entities.is_empty());
    assert!(entities.phone_numbers.iter().any(|p| p.contains("812 3456 7890")));
    assert!(entities.emails.contains(&"contact@alvio.test".to_string()));
    assert!(entities.amounts.iter().any(|a| a.contains("750.000")));
    assert!(entities.dates.iter().any(|d| d.contains("17 September 2026")));
    assert!(entities.vehicle_plates.contains(&"B 1234 XYZ".to_string()));
    assert!(entities.niks.contains(&"3171234567890001".to_string()));
    assert!(entities.urls.contains(&"https://alvio-ocr.dev".to_string()));
}

#[test]
fn test_searchable_pdf_and_visualization() {
    let engine = OcrEngine::auto().expect("Engine auto");
    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let img = image::open(sample_image).expect("Open sample");
        let res = engine.recognize_file(sample_image).expect("OCR result");

        // Searchable PDF
        let pdf_bytes = res.to_searchable_pdf(&img).expect("Generate PDF");
        assert!(!pdf_bytes.is_empty());
        assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.contains("/Type /Catalog"));
        assert!(pdf_str.contains("3 Tr"));
        assert!(pdf_str.contains("JAKARTA") || pdf_str.contains("PROVINSI"));

        // Annotated Image
        let annotated = res.to_annotated_image(&img);
        assert_eq!(annotated.width(), img.width());
        assert_eq!(annotated.height(), img.height());

        // hOCR
        let hocr = res.to_hocr();
        assert!(hocr.contains("<div class='ocr_page'"));
        assert!(hocr.contains("ocrx_word"));

        // Table CSV
        let table = res.to_table();
        let csv = table.to_csv();
        assert!(!csv.is_empty() || table.rows.is_empty());
    }
}

#[test]
fn test_document_hash_cache() {
    let config = alvio_ocr::OcrConfig::default().with_cache(true);
    let engine = OcrEngine::with_config(config).expect("Engine with cache");
    let sample_image = "D:/riset_alvio/ocr/tests/fixtures/ocr/simple.jpg";
    if Path::new(sample_image).exists() {
        let bytes = std::fs::read(sample_image).expect("Read image");

        let start1 = std::time::Instant::now();
        let res1 = engine.recognize_bytes(&bytes).expect("First OCR");
        let dur1 = start1.elapsed();

        let start2 = std::time::Instant::now();
        let res2 = engine.recognize_bytes(&bytes).expect("Second OCR");
        let dur2 = start2.elapsed();

        assert_eq!(res1.text, res2.text);
        assert!(dur2 < dur1, "Cache hit ({:?}) should be faster than first run ({:?})", dur2, dur1);
        assert!(dur2.as_millis() < 50, "Cache hit should be near-instantaneous");

        engine.clear_cache();
    }
}


