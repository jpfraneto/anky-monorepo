#!/usr/bin/env python3
"""
Simple inference server for the trained Anky LoRA.
Runs after training completes on RunPod.
Serves POST /generate → returns base64 PNG.

Usage (auto-started by train_anky_setup.sh after training):
    python3 /workspace/inference_server.py
"""

import base64
import glob
import io
import os
from pathlib import Path

import torch
from diffusers import FluxPipeline
from flask import Flask, jsonify, request

OUTPUT_DIR = Path("/workspace/output/anky_flux_lora_v2")
BASE_MODEL  = "black-forest-labs/FLUX.1-dev"
PORT        = int(os.environ.get("INFERENCE_PORT", 8000))

app = Flask(__name__)
pipe = None


def load_pipeline():
    global pipe
    # Find latest checkpoint
    checkpoints = sorted(OUTPUT_DIR.glob("*.safetensors"))
    if not checkpoints:
        raise FileNotFoundError(f"No .safetensors found in {OUTPUT_DIR}")

    lora_path = str(checkpoints[-1])
    print(f"[inference] Loading LoRA: {lora_path}")

    pipe = FluxPipeline.from_pretrained(
        BASE_MODEL,
        torch_dtype=torch.bfloat16,
    ).to("cuda")

    pipe.load_lora_weights(lora_path)
    pipe.fuse_lora(lora_scale=0.85)
    print("[inference] Pipeline ready")


@app.route("/health", methods=["GET"])
def health():
    return jsonify({"ok": True, "model": "anky_flux_lora_v2"})


@app.route("/generate", methods=["POST"])
def generate():
    data = request.get_json(force=True)
    prompt = data.get("prompt", "anky, a blue creature with golden eyes in a vibrant scene")
    steps  = int(data.get("steps", 28))
    scale  = float(data.get("guidance_scale", 3.5))
    width  = int(data.get("width", 1024))
    height = int(data.get("height", 1024))

    print(f"[inference] generating: {prompt[:80]}...")

    with torch.inference_mode():
        result = pipe(
            prompt=prompt,
            num_inference_steps=steps,
            guidance_scale=scale,
            width=width,
            height=height,
        )

    img = result.images[0]
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    b64 = base64.b64encode(buf.getvalue()).decode()

    return jsonify({"image": b64, "prompt": prompt})


if __name__ == "__main__":
    load_pipeline()
    print(f"[inference] server running on port {PORT}")
    app.run(host="0.0.0.0", port=PORT, threaded=False)
