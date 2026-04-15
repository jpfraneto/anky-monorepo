#!/usr/bin/env python3
"""Generate 22 Easter-themed Anky images via ComfyUI Flux pipeline."""

import json
import time
import random
import requests
import uuid
import sys
from pathlib import Path

COMFYUI_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = Path(__file__).parent.parent / "static" / "easter"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

FLUX_UNET = "flux1-dev.safetensors"
FLUX_VAE = "ae.safetensors"
FLUX_CLIP_L = "clip_l.safetensors"
FLUX_T5 = "t5xxl_fp8_e4m3fn.safetensors"
LORA = "anky_flux_lora_v2.safetensors"
LORA_STRENGTH = 0.85
STEPS = 20
GUIDANCE = 3.5

PROMPTS = [
    "anky sitting peacefully in a sunlit meadow, holding a single pastel egg gently in both hands, soft morning light, wildflowers",
    "anky discovering a small golden egg hidden beneath a moss-covered stone, forest clearing, dappled light filtering through trees",
    "anky walking through a garden at dawn, carrying a woven basket with a few colorful eggs, dewdrops on grass",
    "anky meditating cross-legged on a hilltop, one luminous egg floating above open palms, cherry blossoms falling",
    "anky painting delicate patterns on an egg with a tiny brush, cozy workshop, warm candlelight, focused expression",
    "anky cradling a cracked egg from which soft golden light spills, twilight sky, standing in a wheat field",
    "anky perched on a tree branch, nest beside it with three speckled eggs, spring canopy, birds singing",
    "anky in a rainy spring garden, sheltering a nest of eggs under a large leaf, puddles reflecting clouds",
    "anky offering an ornate egg to a small child, village square with flowering trees, gentle afternoon sun",
    "anky sleeping curled around a warm glowing egg, underground burrow, roots forming natural ceiling, cozy earth tones",
    "anky standing in a field of tulips, one egg balanced perfectly on its head, playful expression, blue sky",
    "anky underwater in a crystal-clear spring, finding a pearl-like egg on the sandy bottom, light rays from above",
    "anky in a misty bamboo forest, holding a jade-colored egg, serene expression, zen atmosphere",
    "anky on a rocky coast at sunrise, tide pool containing a luminescent egg, orange and pink sky",
    "anky in a cozy kitchen, carefully placing eggs in a pot of natural dye made from flowers, warm interior",
    "anky walking along a winding stone path, each stepping stone shaped like an egg, enchanted garden",
    "anky sitting by a campfire at night, an egg nestled in the embers glowing with inner warmth, starry sky",
    "anky in a sunflower field, a butterfly landing on a pastel egg held gently in its palm, summer warmth",
    "anky floating in space, cradling a cosmic egg with galaxies swirling inside, nebula background, ethereal",
    "anky in an ancient library, discovering a jeweled egg on a dusty shelf, rays of light through stained glass",
    "anky on a japanese bridge over a koi pond, reflecting on a porcelain egg, cherry blossoms, tranquil water",
    "anky in a lavender field at golden hour, basket of hand-painted eggs, butterflies, warm pastoral scene",
]


def build_workflow(prompt: str, client_id: str, seed: int) -> dict:
    if "anky" not in prompt.lower():
        prompt = f"anky, {prompt}"
    return {
        "client_id": client_id,
        "prompt": {
            "1": {"class_type": "UNETLoader", "inputs": {"unet_name": FLUX_UNET, "weight_dtype": "fp8_e4m3fn"}},
            "2": {"class_type": "VAELoader", "inputs": {"vae_name": FLUX_VAE}},
            "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": FLUX_CLIP_L, "clip_name2": FLUX_T5, "type": "flux"}},
            "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": LORA, "strength_model": LORA_STRENGTH, "strength_clip": LORA_STRENGTH}},
            "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt}},
            "6": {"class_type": "EmptyLatentImage", "inputs": {"width": 1024, "height": 1024, "batch_size": 1}},
            "7": {"class_type": "KSampler", "inputs": {"model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0], "latent_image": ["6", 0], "seed": seed, "steps": STEPS, "cfg": GUIDANCE, "sampler_name": "euler", "scheduler": "simple", "denoise": 1.0}},
            "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
            "9": {"class_type": "SaveImage", "inputs": {"images": ["8", 0], "filename_prefix": "anky_easter"}},
        },
    }


def generate_one(index: int, prompt: str) -> bool:
    client_id = str(uuid.uuid4())
    seed = random.randint(0, 2**32)
    workflow = build_workflow(prompt, client_id, seed)

    print(f"\n[{index+1}/22] Queuing: {prompt[:60]}...")

    try:
        resp = requests.post(f"{COMFYUI_URL}/prompt", json=workflow, timeout=30)
        if not resp.ok:
            print(f"  ERROR queuing: {resp.text[:200]}")
            return False
        prompt_id = resp.json()["prompt_id"]
    except Exception as e:
        print(f"  ERROR connecting to ComfyUI: {e}")
        return False

    # Poll for completion
    for attempt in range(120):
        time.sleep(2)
        try:
            hist = requests.get(f"{COMFYUI_URL}/history/{prompt_id}", timeout=10).json()
        except Exception:
            continue

        entry = hist.get(prompt_id)
        if not entry:
            continue

        # Check for errors
        status = entry.get("status", {})
        for msg in status.get("messages", []):
            if msg[0] == "execution_error":
                print(f"  ERROR: {msg[1].get('exception_message', 'unknown')}")
                return False

        # Find output image
        outputs = entry.get("outputs", {})
        for node_id, output in outputs.items():
            images = output.get("images", [])
            if images:
                filename = images[0]["filename"]
                # Download image
                img_resp = requests.get(
                    f"{COMFYUI_URL}/view",
                    params={"filename": filename, "type": "output"},
                    timeout=30,
                )
                if img_resp.ok:
                    out_path = OUTPUT_DIR / f"easter_{index+1:02d}.png"
                    out_path.write_bytes(img_resp.content)
                    print(f"  Saved: {out_path.name} ({len(img_resp.content)//1024}KB)")
                    return True

    print(f"  TIMEOUT after 240s")
    return False


def main():
    # Check ComfyUI is up
    try:
        r = requests.get(f"{COMFYUI_URL}/system_stats", timeout=5)
        if not r.ok:
            print("ComfyUI not responding")
            sys.exit(1)
    except Exception:
        print("Cannot reach ComfyUI at", COMFYUI_URL)
        sys.exit(1)

    print(f"ComfyUI is up. Generating 22 Easter Anky images...")
    print(f"Output: {OUTPUT_DIR}\n")

    # Check which ones already exist (for resume)
    existing = set()
    for f in OUTPUT_DIR.glob("easter_*.png"):
        try:
            num = int(f.stem.split("_")[1])
            if f.stat().st_size > 1000:  # skip corrupt
                existing.add(num)
        except (ValueError, IndexError):
            pass

    if existing:
        print(f"Resuming — already have: {sorted(existing)}")

    successes = len(existing)
    failures = 0

    for i, prompt in enumerate(PROMPTS):
        if (i + 1) in existing:
            print(f"[{i+1}/22] Already exists, skipping")
            continue
        if generate_one(i, prompt):
            successes += 1
        else:
            failures += 1

    print(f"\nDone! {successes} successes, {failures} failures")
    print(f"View at: https://anky.app/easter")


if __name__ == "__main__":
    main()
