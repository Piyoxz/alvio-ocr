#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODELS_DIR="$SCRIPT_DIR/../models"

mkdir -p "$MODELS_DIR"

FILES=(
    "PP-OCRv6_det_small.onnx|https://huggingface.co/PaddlePaddle/PP-OCRv6_small_det_onnx/resolve/main/inference.onnx"
    "PP-OCRv6_rec_small.onnx|https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx/resolve/main/inference.onnx"
    "ppocr_keys_v1.txt|https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/release/2.7/ppocr/utils/ppocr_keys_v1.txt"
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
echo "All PP-OCRv6 models downloaded successfully."
echo "Models directory: $MODELS_DIR"
