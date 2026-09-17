#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODELS_DIR="$SCRIPT_DIR/../models/ocr"

mkdir -p "$MODELS_DIR"

FILES=(
    "ch_PP-OCRv4_det_infer.onnx|https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_det_infer.onnx"
    "en_PP-OCRv4_rec_infer.onnx|https://huggingface.co/breezedeus/cnocr-ppocr-en_PP-OCRv4/resolve/main/en_PP-OCRv4_rec_infer.onnx"
    "en_dict.txt|https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/en_dict.txt"
    "ch_PP-OCRv4_rec_infer.onnx|https://huggingface.co/SWHL/RapidOCR/resolve/main/PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx"
    "ppocr_keys_v1.txt|https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/ppocr_keys_v1.txt"
)

for entry in "${FILES[@]}"; do
    NAME="${entry%%|*}"
    URL="${entry#*|}"
    TARGET="$MODELS_DIR/$NAME"

    if [ -f "$TARGET" ] && [ -s "$TARGET" ]; then
        echo "Already exists: $NAME ($(stat -f%z "$TARGET" 2>/dev/null || stat -c%s "$TARGET") bytes) - Skipping."
    else
        echo "Downloading $NAME ..."
        curl -L -o "$TARGET" "$URL"
        echo "Downloaded $NAME."
    fi
done

echo ""
echo "All OCR models downloaded successfully."
echo "Models directory: $MODELS_DIR"
