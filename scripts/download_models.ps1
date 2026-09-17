$ErrorActionPreference = "Stop"

$modelsDir = Join-Path $PSScriptRoot "..\models"
if (-not (Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
    Write-Host "Created directory: $modelsDir"
}

$files = @(
    @{
        Name = "PP-OCRv6_det_small.onnx"
        Url  = "https://huggingface.co/PaddlePaddle/PP-OCRv6_small_det_onnx/resolve/main/inference.onnx"
    },
    @{
        Name = "PP-OCRv6_rec_small.onnx"
        Url  = "https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx/resolve/main/inference.onnx"
    },
    @{
        Name = "ppocr_keys_v1.txt"
        Url  = "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/paddleocr/utils/ppocr_keys_v1.txt"
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

Write-Host "`nAll PP-OCRv6 models downloaded successfully."
Write-Host "Models directory: $modelsDir"
