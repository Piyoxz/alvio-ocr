$ErrorActionPreference = "Stop"

$modelsDir = Join-Path $PSScriptRoot "..\models\ocr"
if (-not (Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
    Write-Host "Created directory: $modelsDir"
}

$files = @(
    @{
        Name = "ch_PP-OCRv4_det_infer.onnx"
        Url  = "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_det_infer.onnx"
    },
    @{
        Name = "en_PP-OCRv4_rec_infer.onnx"
        Url  = "https://huggingface.co/breezedeus/cnocr-ppocr-en_PP-OCRv4/resolve/main/en_PP-OCRv4_rec_infer.onnx"
    },
    @{
        Name = "en_dict.txt"
        Url  = "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/en_dict.txt"
    },
    @{
        Name = "ch_PP-OCRv4_rec_infer.onnx"
        Url  = "https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx"
    },
    @{
        Name = "ppocr_keys_v1.txt"
        Url  = "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/ppocr_keys_v1.txt"
    }
)

foreach ($f in $files) {
    $targetPath = Join-Path $modelsDir $f.Name
    if (Test-Path $targetPath) {
        $size = (Get-Item $targetPath).Length
        Write-Host "Already exists: $($f.Name) ($size bytes) - Skipping."
    } else {
        Write-Host "Downloading $($f.Name) ..."
        curl.exe -L -o $targetPath $f.Url
        if ((Test-Path $targetPath) -and ((Get-Item $targetPath).Length -gt 0)) {
            Write-Host "Downloaded $($f.Name) ($((Get-Item $targetPath).Length) bytes)."
        } else {
            Write-Error "Failed to download $($f.Name)."
        }
    }
}

Write-Host "`nAll OCR models downloaded successfully."
Write-Host "Models directory: $modelsDir"
