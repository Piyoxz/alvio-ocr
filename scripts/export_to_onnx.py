import argparse
import os
import subprocess
import sys

def parse_args():
    parser = argparse.ArgumentParser(description="Export PaddleOCR / PP-OCRv6 models to ONNX")
    parser.add_argument("--model_type", type=str, choices=["det", "rec"], required=True)
    parser.add_argument("--model_dir", type=str, required=True)
    parser.add_argument("--save_file", type=str, required=True)
    parser.add_argument("--opset_version", type=int, default=14)
    parser.add_argument("--enable_onnxsim", action="store_true", default=True)
    return parser.parse_args()

def export_det(model_dir: str, save_file: str, opset: int, enable_onnxsim: bool):
    os.makedirs(os.path.dirname(os.path.abspath(save_file)), exist_ok=True)
    cmd = [
        "paddle2onnx",
        "--model_dir", model_dir,
        "--model_filename", "inference.pdmodel",
        "--params_filename", "inference.pdiparams",
        "--save_file", save_file,
        "--opset_version", str(opset),
        "--enable_onnxchecker", "True",
        "--input_shape_dict", "{'x': [-1, 3, -1, -1]}",
    ]
    if enable_onnxsim:
        cmd.extend(["--enable_onnxsim", "True"])
    print(f"Exporting detection model to {save_file}...")
    subprocess.check_call(cmd)
    print("Export complete.")

def export_rec(model_dir: str, save_file: str, opset: int, enable_onnxsim: bool):
    os.makedirs(os.path.dirname(os.path.abspath(save_file)), exist_ok=True)
    cmd = [
        "paddle2onnx",
        "--model_dir", model_dir,
        "--model_filename", "inference.pdmodel",
        "--params_filename", "inference.pdiparams",
        "--save_file", save_file,
        "--opset_version", str(opset),
        "--enable_onnxchecker", "True",
        "--input_shape_dict", "{'x': [-1, 3, 48, -1]}",
    ]
    if enable_onnxsim:
        cmd.extend(["--enable_onnxsim", "True"])
    print(f"Exporting recognition model to {save_file}...")
    subprocess.check_call(cmd)
    print("Export complete.")

def main():
    args = parse_args()
    if args.model_type == "det":
        export_det(args.model_dir, args.save_file, args.opset_version, args.enable_onnxsim)
    elif args.model_type == "rec":
        export_rec(args.model_dir, args.save_file, args.opset_version, args.enable_onnxsim)

if __name__ == "__main__":
    main()
