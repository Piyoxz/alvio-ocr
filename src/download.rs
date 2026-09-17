use crate::{config::OcrLanguage, error::OcrError};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{info, warn};

pub const DET_MODEL_URL: &str =
    "https://huggingface.co/PaddlePaddle/PP-OCRv6_small_det_onnx/resolve/main/inference.onnx";

pub const REC_MODEL_URL: &str =
    "https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx/resolve/main/inference.onnx";

pub const DICT_URL: &str =
    "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/paddleocr/utils/ppocr_keys_v1.txt";

pub fn cache_dir() -> PathBuf {
    if let Ok(val) = std::env::var("ALVIO_OCR_CACHE") {
        return PathBuf::from(val);
    }
    if let Ok(val) = std::env::var("ALVIO_CACHE_DIR") {
        return PathBuf::from(val);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(appdata).join("alvio-ocr");
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join(".alvio-ocr");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".cache").join("alvio-ocr");
        }
    }

    std::env::temp_dir().join("alvio-ocr")
}

pub fn default_model_dir() -> PathBuf {
    if let Ok(val) = std::env::var("OCR_MODEL_DIR") {
        let p = PathBuf::from(val);
        if p.exists() {
            return p;
        }
    }

    for candidate in &["./models", "./models/ocr", "../ocr/models/ocr", "../models"] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }

    cache_dir().join("models")
}

pub fn default_libs_dir() -> PathBuf {
    if let Ok(val) = std::env::var("PDFIUM_PATH") {
        let p = PathBuf::from(val);
        if let Some(parent) = p.parent() {
            if parent.exists() {
                return parent.to_path_buf();
            }
        }
    }

    for candidate in &["./libs", "../libs", "../ocr/libs", "./bin"] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }

    cache_dir().join("libs")
}

pub fn download_file(url: &str, target_path: &Path) -> Result<(), OcrError> {
    if target_path.exists() && fs::metadata(target_path).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = target_path.with_extension(format!("download_{}", std::process::id()));
    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    info!("Downloading {} from {} ...", file_name, url);
    eprintln!("[alvio-ocr] Downloading {} from remote repository...", file_name);

    let curl_status = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("-o")
        .arg(&tmp_path)
        .arg(url)
        .status();

    let mut download_succeeded = false;

    if let Ok(status) = curl_status {
        if status.success() && tmp_path.exists() && fs::metadata(&tmp_path).map(|m| m.len() > 0).unwrap_or(false) {
            download_succeeded = true;
        }
    }

    #[cfg(target_os = "windows")]
    if !download_succeeded {
        warn!("curl failed or not found, falling back to PowerShell Invoke-WebRequest...");
        let ps_cmd = format!(
            "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url,
            tmp_path.display()
        );
        let ps_status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&ps_cmd)
            .status();

        if let Ok(status) = ps_status {
            if status.success() && tmp_path.exists() && fs::metadata(&tmp_path).map(|m| m.len() > 0).unwrap_or(false) {
                download_succeeded = true;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    if !download_succeeded {
        warn!("curl failed, falling back to wget...");
        let wget_status = Command::new("wget")
            .arg("-q")
            .arg("-O")
            .arg(&tmp_path)
            .arg(url)
            .status();

        if let Ok(status) = wget_status {
            if status.success() && tmp_path.exists() && fs::metadata(&tmp_path).map(|m| m.len() > 0).unwrap_or(false) {
                download_succeeded = true;
            }
        }
    }

    if download_succeeded {
        fs::rename(&tmp_path, target_path)?;
        info!("Successfully downloaded {} to {:?}", file_name, target_path);
        eprintln!("[alvio-ocr] Finished downloading {}.", file_name);
        Ok(())
    } else {
        let _ = fs::remove_file(&tmp_path);
        Err(OcrError::DownloadFailed(format!(
            "Failed to download '{}' from '{}'. Please check internet connection or manually place files in {:?}",
            file_name, url, target_path
        )))
    }
}

pub fn ensure_models(model_dir: &Path, _lang: OcrLanguage) -> Result<(), OcrError> {
    if !model_dir.exists() {
        fs::create_dir_all(model_dir)?;
    }

    let det_path = model_dir.join("PP-OCRv6_det_small.onnx");
    if !det_path.exists() {
        download_file(DET_MODEL_URL, &det_path)?;
    }

    let rec_path = model_dir.join("PP-OCRv6_rec_small.onnx");
    if !rec_path.exists() {
        download_file(REC_MODEL_URL, &rec_path)?;
    }

    let dict_path = model_dir.join("ppocr_keys_v1.txt");
    if !dict_path.exists() {
        download_file(DICT_URL, &dict_path)?;
    }

    Ok(())
}

pub fn pdfium_lib_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    return "pdfium.dll";
    #[cfg(target_os = "linux")]
    return "libpdfium.so";
    #[cfg(target_os = "macos")]
    return "libpdfium.dylib";
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return "pdfium";
}

pub fn ensure_pdfium(libs_dir: &Path) -> Result<PathBuf, OcrError> {
    let lib_name = pdfium_lib_filename();
    let target_dll = libs_dir.join(lib_name);

    if target_dll.exists() && fs::metadata(&target_dll).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(target_dll);
    }

    if !libs_dir.exists() {
        fs::create_dir_all(libs_dir)?;
    }

    let (archive_name, internal_lib_path) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("pdfium-win-x64.tgz", "bin/pdfium.dll"),
        ("windows", "aarch64") => ("pdfium-win-arm64.tgz", "bin/pdfium.dll"),
        ("linux", "x86_64") => ("pdfium-linux-x64.tgz", "lib/libpdfium.so"),
        ("linux", "aarch64") => ("pdfium-linux-arm64.tgz", "lib/libpdfium.so"),
        ("macos", "x86_64") => ("pdfium-mac-x64.tgz", "lib/libpdfium.dylib"),
        ("macos", "aarch64") => ("pdfium-mac-arm64.tgz", "lib/libpdfium.dylib"),
        (os, arch) => {
            return Err(OcrError::DownloadFailed(format!(
                "Unsupported platform for automated Pdfium download: {}-{}. Please install Pdfium manually.",
                os, arch
            )));
        }
    };

    let download_url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/{}",
        archive_name
    );

    let temp_tgz = libs_dir.join(archive_name);
    download_file(&download_url, &temp_tgz)?;

    info!("Extracting {} from {:?} ...", lib_name, temp_tgz);
    eprintln!("[alvio-ocr] Extracting {} ...", lib_name);

    let tar_status = Command::new("tar")
        .arg("-xzf")
        .arg(&temp_tgz)
        .arg("-C")
        .arg(libs_dir)
        .arg(internal_lib_path)
        .status();

    let _ = fs::remove_file(&temp_tgz);

    if let Ok(status) = tar_status {
        if status.success() {
            let extracted_path = libs_dir.join(internal_lib_path);
            if extracted_path.exists() {
                if extracted_path != target_dll {
                    let _ = fs::rename(&extracted_path, &target_dll);
                }
                if let Some(parent) = extracted_path.parent() {
                    if parent != libs_dir {
                        let _ = fs::remove_dir_all(parent);
                    }
                }
            }
        }
    }

    if target_dll.exists() && fs::metadata(&target_dll).map(|m| m.len() > 0).unwrap_or(false) {
        info!("Pdfium successfully installed to {:?}", target_dll);
        eprintln!("[alvio-ocr] Pdfium successfully configured at {:?}.", target_dll);
        Ok(target_dll)
    } else {
        Err(OcrError::DownloadFailed(format!(
            "Failed to extract Pdfium dynamic library to {:?}",
            target_dll
        )))
    }
}
