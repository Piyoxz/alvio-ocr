$ErrorActionPreference = "Stop"

$libsDir = Join-Path $PSScriptRoot "..\libs"
if (-not (Test-Path $libsDir)) {
    New-Item -ItemType Directory -Path $libsDir -Force | Out-Null
}

$targetDll = Join-Path $libsDir "pdfium.dll"
if (Test-Path $targetDll) {
    Write-Host "pdfium.dll already exists in $libsDir - Skipping."
    exit 0
}

$tempTgz = Join-Path $libsDir "pdfium-win-x64.tgz"
$downloadUrl = "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz"

Write-Host "Downloading Pdfium Windows x64 binary from $downloadUrl ..."
curl.exe -L -o $tempTgz $downloadUrl

Write-Host "Extracting pdfium.dll ..."
tar.exe -xzf $tempTgz -C $libsDir bin/pdfium.dll
Move-Item -Path (Join-Path $libsDir "bin\pdfium.dll") -Destination $targetDll -Force
Remove-Item -Path (Join-Path $libsDir "bin") -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path $tempTgz -Force -ErrorAction SilentlyContinue

if (Test-Path $targetDll) {
    Write-Host "Successfully installed pdfium.dll to $targetDll"
} else {
    Write-Error "Failed to install pdfium.dll"
}
