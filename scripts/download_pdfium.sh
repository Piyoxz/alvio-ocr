#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIBS_DIR="${SCRIPT_DIR}/../libs"

mkdir -p "${LIBS_DIR}"
TARGET_SO="${LIBS_DIR}/libpdfium.so"

if [ -f "${TARGET_SO}" ] && [ -s "${TARGET_SO}" ]; then
    echo "libpdfium.so already exists in ${LIBS_DIR} - Skipping."
    exit 0
fi

DOWNLOAD_URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-linux-x64.tgz"
TEMP_TGZ="${LIBS_DIR}/pdfium-linux-x64.tgz"

echo "Downloading Pdfium Linux x64 binary from ${DOWNLOAD_URL} ..."
curl -L -o "${TEMP_TGZ}" "${DOWNLOAD_URL}"

echo "Extracting libpdfium.so ..."
tar -xzf "${TEMP_TGZ}" -C "${LIBS_DIR}" lib/libpdfium.so
mv "${LIBS_DIR}/lib/libpdfium.so" "${TARGET_SO}"
rm -rf "${LIBS_DIR}/lib" "${TEMP_TGZ}"

if [ -f "${TARGET_SO}" ]; then
    echo "Successfully installed libpdfium.so to ${TARGET_SO}"
else
    echo "Error: Failed to install libpdfium.so"
    exit 1
fi
