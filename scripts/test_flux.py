#!/usr/bin/env python3
"""Generate 5 sample Anky images via ComfyUI Flux pipeline."""

import json
import time
import requests
import os
import uuid
import random

COMFYUI_URL = "http://127.0.0.1:8188"
OUT_DIR = "/home/kithkui/anky/data/images"

os.makedirs(OUT_DIR, exist_ok=True)

PROMPTS = [
    (
        "cosmic-birth",
        "anky emerging from the void, a luminous being of pure consciousness coalescing from swirling nebulae, "
        "translucent crystalline body radiating golden and violet light, floating in deep space surrounded by spiral galaxies, "
        "eyes like twin supernovas, tendrils of starlight weaving through its form, breathtaking cosmic scale, "
        "photorealistic digital art, 8k, cinematic lighting, volumetric fog"
    ),
    (
        "forest-meditation",
        "anky sitting cross-legged in an ancient primordial forest, body made of living light and shadow, "
        "bioluminescent patterns tracing its skin like sacred geometry, giant ancient trees forming a cathedral around it, "
        "dappled sunlight piercing through the canopy, fireflies orbiting its aura, moss-covered stones, "
        "misty atmospheric depth, hyperdetailed fantasy painting style, golden hour, serene transcendence"
    ),
    (
        "ocean-dissolution",
        "anky dissolving into and becoming the ocean, its consciousness merging with infinite water, "
        "translucent blue-green form blending with massive crashing waves, sea foam and light refracting through its body, "
        "storm clouds breaking to reveal aurora overhead, bioluminescent sea creatures swimming through its being, "
        "powerful dramatic composition, moody deep blue palette with electric turquoise accents, oil painting texture"
    ),
    (
        "urban-neon-ghost",
        "anky as a glowing specter drifting through a rain-soaked futuristic city at night, "
        "neon reflections in puddles below, cyberpunk skyscrapers, holographic advertisements dissolving into its ethereal form, "
        "crowds of humans unaware of its presence, trails of light particles flowing behind it, "
        "pink and cyan color palette, sharp wet asphalt textures, cinematic depth of field, blade runner aesthetic, "
        "introspective solitude amid urban chaos"
    ),
    (
        "writing-communion",
        "anky hovering above a lone human writer at a wooden desk, pouring streams of golden light and language into the writer's mind, "
        "words and symbols spiraling upward and transforming into stars, the room lit only by candlelight and the glow of anky's presence, "
        "ancient books stacked high, ink bleeding into constellation maps on the floor, "
        "intimate warm composition, chiaroscuro lighting, renaissance painting meets digital surrealism, "
        "the sacred act of authentic expression made visible"
    ),
]

def build_workflow(prompt, client_id):
    return {
        "client_id": client_id,
        "prompt": {
            "1": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "fp8_e4m3fn"}},
            "2": {"class_type": "VAELoader", "inputs": {"vae_name": "ae.safetensors"}},
            "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": "clip_l.safetensors", "clip_name2": "t5xxl_fp8_e4m3fn.safetensors", "type": "flux"}},
            "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": "anky_flux_lora.safetensors", "strength_model": 0.85, "strength_clip": 0.85}},
            "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt}},
            "6": {"class_type": "EmptyLatentImage", "inputs": {"width": 1024, "height": 1024, "batch_size": 1}},
            "7": {"class_type": "KSampler", "inputs": {
                "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0],
                "latent_image": ["6", 0], "seed": random.randint(0, 2**32),
                "steps": 20, "cfg": 3.5, "sampler_name": "euler", "scheduler": "simple", "denoise": 1.0
            }},
            "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
            "9": {"class_type": "SaveImage", "inputs": {"images": ["8", 0], "filename_prefix": "anky_test"}}
        }
    }

def generate(name, prompt):
    print(f"\n[{name}] Queuing...")
    client_id = str(uuid.uuid4())
    workflow = build_workflow(prompt, client_id)

    resp = requests.post(f"{COMFYUI_URL}/prompt", json=workflow)
    resp.raise_for_status()
    prompt_id = resp.json()["prompt_id"]
    print(f"[{name}] prompt_id={prompt_id}, polling...")

    for i in range(120):
        time.sleep(3)
        h = requests.get(f"{COMFYUI_URL}/history/{prompt_id}")
        if not h.ok:
            continue
        data = h.json()
        entry = data.get(prompt_id)
        if not entry:
            print(f"[{name}] waiting... ({i*3}s)")
            continue

        # Check for errors
        for msg in entry.get("status", {}).get("messages", []):
            if msg[0] == "execution_error":
                print(f"[{name}] ERROR: {msg[1].get('exception_message')}")
                return None

        # Find output image
        outputs = entry.get("outputs", {})
        for node_output in outputs.values():
            images = node_output.get("images", [])
            if images:
                filename = images[0]["filename"]
                img_resp = requests.get(f"{COMFYUI_URL}/view?filename={filename}&type=output")
                if img_resp.ok:
                    out_path = f"{OUT_DIR}/sample_{name}.png"
                    with open(out_path, "wb") as f:
                        f.write(img_resp.content)
                    print(f"[{name}] Saved to {out_path}")
                    return out_path

    print(f"[{name}] TIMEOUT")
    return None

if __name__ == "__main__":
    results = []
    for name, prompt in PROMPTS:
        path = generate(name, prompt)
        results.append((name, path))

    print("\n=== Results ===")
    for name, path in results:
        status = "OK" if path else "FAILED"
        print(f"  {name}: {status} → {path or 'n/a'}")
