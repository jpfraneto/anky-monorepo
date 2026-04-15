#!/usr/bin/env python3
"""
Generate pitch deck images for Anky via ComfyUI (Flux.1-dev + anky LoRA).
Each image takes ~20-30s on the local GPU.
Output: data/pitch-deck/
"""

import json
import time
import uuid
import random
import requests
import sys
from pathlib import Path

COMFYUI_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = Path("data/pitch-deck")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Flux model config (matches src/services/comfyui.rs)
FLUX_UNET = "flux1-dev.safetensors"
FLUX_VAE = "ae.safetensors"
FLUX_CLIP_L = "clip_l.safetensors"
FLUX_T5 = "t5xxl_fp8_e4m3fn.safetensors"
LORA_NAME = "anky_flux_lora_v2.safetensors"
LORA_STRENGTH = 0.85
STEPS = 20
GUIDANCE = 3.5

# ── Pitch deck images to generate ──────────────────────────────────
# Each entry: (filename, prompt, width, height)

IMAGES = [
    # SLIDE 1: The meme wall — 9 diverse ankys showing the visual range
    # We'll generate 9 individual images and composite them
    ("meme_wall_01", "anky, a luminous blue creature with enormous curious eyes sitting in a field of bioluminescent flowers, digital art, vibrant colors, whimsical", 1024, 1024),
    ("meme_wall_02", "anky, a golden celestial being meditating on a floating crystal platform above clouds, cosmic rays, sacred geometry, ethereal", 1024, 1024),
    ("meme_wall_03", "anky, a small fiery red creature writing in a leather journal by candlelight in a medieval library, warm tones, cozy, detailed", 1024, 1024),
    ("meme_wall_04", "anky, an emerald green forest spirit with leaf-like wings perched on a giant mushroom, enchanted forest, mystical fog, nature", 1024, 1024),
    ("meme_wall_05", "anky, a cosmic purple being floating in deep space surrounded by nebulae and stardust, third eye glowing, transcendent", 1024, 1024),
    ("meme_wall_06", "anky, a playful orange creature surfing on a wave of liquid light, dynamic pose, sunset colors, joyful energy", 1024, 1024),
    ("meme_wall_07", "anky, a wise ancient silver being sitting cross-legged on a mountain peak, snow falling, serene, crown chakra activated", 1024, 1024),
    ("meme_wall_08", "anky, a vibrant pink creature dancing in a garden of giant roses, butterflies everywhere, whimsical, heart chakra energy", 1024, 1024),
    ("meme_wall_09", "anky, a deep indigo being emerging from an ocean of consciousness, waves of thought visible, introspective, profound", 1024, 1024),

    # SLIDE 5: Kingdom showcase — one per kingdom with distinct aesthetic
    ("kingdom_primordia", "anky, a primal earthy creature emerging from volcanic rock, roots growing from its body, raw power, root chakra, red and brown tones, primordial landscape", 1024, 1024),
    ("kingdom_emblazion", "anky, a fluid sensual creature made of flowing water and fire, dancing with creative energy, sacral chakra, orange and coral tones, passionate movement", 1024, 1024),
    ("kingdom_chryseos", "anky, a radiant golden being standing in sunlight, solar plexus blazing, confident stance, yellow and gold, power and willpower emanating", 1024, 1024),
    ("kingdom_eleasis", "anky, a gentle glowing green being in a meadow of wildflowers, heart wide open, compassion visible as green light, soft and tender", 1024, 1024),
    ("kingdom_voxlumis", "anky, a crystalline blue being singing, sound waves visible as geometric patterns, throat chakra activated, truth and expression, cyan and blue", 1024, 1024),
    ("kingdom_insightia", "anky, a deep indigo being with a third eye radiating light, seeing through dimensions, psychedelic fractal visions, wisdom, purple and indigo", 1024, 1024),
    ("kingdom_claridium", "anky, a pure white luminous being floating in vast empty space, crown chakra open to the cosmos, violet and white, transcendence and unity", 1024, 1024),
    ("kingdom_poiesis", "anky, a being of pure creative light transcending all form, every color of the rainbow flowing through its body, beyond chakras, universal creation, cosmic art", 1024, 1024),

    # SLIDE 2: The writing moment — person writing in flow state
    ("writing_flow", "anky, a small glowing creature sitting in deep concentration, streams of colorful light flowing from its mind onto a glowing page, dark peaceful background, focused, meditative, flow state", 1024, 1024),

    # SLIDE 4: The alchemy — transformation visual
    ("alchemy_transform", "anky, raw chaotic text and words dissolving and transforming into a beautiful luminous creature, the process of creation visible, metamorphosis, digital alchemy, abstract to concrete", 1024, 1024),

    # SLIDE 6: The flywheel — sharing/virality energy
    ("flywheel_viral", "anky, multiple small glowing creatures of different colors connected by threads of light forming a network, each one unique, spreading outward in a spiral pattern, connection, community, growth", 1024, 1024),

    # SLIDE 7: The sojourn — sacred limited entry
    ("sojourn_gate", "anky, a massive ornate golden gate with the number 3456 inscribed, a single glowing creature standing before it, other creatures visible through the gate in a lush paradise, scarcity, sacred threshold", 1024, 1024),

    # Title/hero image — the definitive anky
    ("hero_anky", "anky, the most beautiful and iconic version of the creature, perfectly centered, glowing with inner light, enormous expressive eyes full of wisdom and curiosity, simple dark background, portrait, masterpiece, detailed", 1024, 1024),

    # WIDESCREEN versions for slide backgrounds (16:9)
    ("bg_dark_cosmos", "anky, vast dark cosmic space with subtle nebula clouds and distant stars, minimal, deep blue and black, cinematic, empty center for text overlay", 1536, 864),
    ("bg_kingdom_wheel", "anky, eight different colored orbs arranged in a circle, each glowing with a different chakra color from red to violet to white, mandala pattern, sacred geometry, dark background", 1536, 864),
]


def build_workflow(prompt_text, client_id, width=1024, height=1024):
    """Build ComfyUI workflow JSON for Flux + anky LoRA."""
    if "anky" not in prompt_text.lower():
        prompt_text = f"anky, {prompt_text}"

    return {
        "client_id": client_id,
        "prompt": {
            "1": {"class_type": "UNETLoader", "inputs": {"unet_name": FLUX_UNET, "weight_dtype": "fp8_e4m3fn"}},
            "2": {"class_type": "VAELoader", "inputs": {"vae_name": FLUX_VAE}},
            "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": FLUX_CLIP_L, "clip_name2": FLUX_T5, "type": "flux"}},
            "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": LORA_NAME, "strength_model": LORA_STRENGTH, "strength_clip": LORA_STRENGTH}},
            "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt_text}},
            "6": {"class_type": "EmptyLatentImage", "inputs": {"width": width, "height": height, "batch_size": 1}},
            "7": {"class_type": "KSampler", "inputs": {
                "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0],
                "latent_image": ["6", 0], "seed": random.randint(0, 2**63),
                "steps": STEPS, "cfg": GUIDANCE, "sampler_name": "euler",
                "scheduler": "simple", "denoise": 1.0
            }},
            "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
            "9": {"class_type": "SaveImage", "inputs": {"images": ["8", 0], "filename_prefix": "pitch_deck"}},
        }
    }


def generate_one(name, prompt, width, height):
    """Queue one image, poll until done, save to disk."""
    client_id = str(uuid.uuid4())
    workflow = build_workflow(prompt, client_id, width, height)

    # Queue
    resp = requests.post(f"{COMFYUI_URL}/prompt", json=workflow, timeout=30)
    resp.raise_for_status()
    prompt_id = resp.json()["prompt_id"]

    print(f"  Queued: {name} (prompt_id={prompt_id[:8]}...) [{width}x{height}]")

    # Poll
    for i in range(120):
        time.sleep(2)
        try:
            hist = requests.get(f"{COMFYUI_URL}/history/{prompt_id}", timeout=10).json()
        except Exception:
            continue

        entry = hist.get(prompt_id)
        if not entry:
            continue

        # Check errors
        status = entry.get("status", {})
        for msg in status.get("messages", []):
            if msg[0] == "execution_error":
                err = msg[1].get("exception_message", "unknown")
                print(f"  ERROR: {name} — {err}")
                return None

        # Find output image
        outputs = entry.get("outputs", {})
        for node_id, output in outputs.items():
            images = output.get("images", [])
            if images:
                filename = images[0]["filename"]
                # Download
                img_resp = requests.get(
                    f"{COMFYUI_URL}/view",
                    params={"filename": filename, "type": "output"},
                    timeout=30
                )
                if img_resp.ok:
                    out_path = OUTPUT_DIR / f"{name}.png"
                    out_path.write_bytes(img_resp.content)
                    print(f"  Done:   {name} -> {out_path} ({len(img_resp.content)//1024}KB)")
                    return str(out_path)

    print(f"  TIMEOUT: {name}")
    return None


def make_grid(image_paths, grid_path, cols=3):
    """Composite multiple images into a grid using PIL."""
    try:
        from PIL import Image
    except ImportError:
        print("  PIL not available, skipping grid composite")
        return

    imgs = [Image.open(p) for p in image_paths if p and Path(p).exists()]
    if not imgs:
        return

    w, h = imgs[0].size
    rows = (len(imgs) + cols - 1) // cols
    grid = Image.new('RGB', (w * cols, h * rows), (0, 0, 0))

    for i, img in enumerate(imgs):
        r, c = divmod(i, cols)
        grid.paste(img.resize((w, h)), (c * w, r * h))

    grid.save(grid_path)
    print(f"  Grid:   {grid_path} ({grid.size[0]}x{grid.size[1]})")


def main():
    # Check if we should start from a specific image
    start_from = 0
    if len(sys.argv) > 1:
        try:
            start_from = int(sys.argv[1])
        except ValueError:
            pass

    print(f"\nGenerating {len(IMAGES) - start_from} pitch deck images via ComfyUI...")
    print(f"Output: {OUTPUT_DIR.resolve()}\n")

    # Check ComfyUI
    try:
        requests.get(f"{COMFYUI_URL}/system_stats", timeout=5).raise_for_status()
    except Exception:
        print("ERROR: ComfyUI not reachable at", COMFYUI_URL)
        return

    results = {}
    for i, (name, prompt, w, h) in enumerate(IMAGES):
        if i < start_from:
            # Check if already generated
            existing = OUTPUT_DIR / f"{name}.png"
            if existing.exists():
                results[name] = str(existing)
            continue

        print(f"\n[{i+1}/{len(IMAGES)}] {name}")
        path = generate_one(name, prompt, w, h)
        results[name] = path

    # Build the 3x3 meme wall grid
    print("\n\nCompositing meme wall grid...")
    wall_paths = [results.get(f"meme_wall_{i:02d}") for i in range(1, 10)]
    make_grid(wall_paths, OUTPUT_DIR / "slide1_meme_wall_3x3.png", cols=3)

    # Build the 2x4 kingdom grid
    print("Compositing kingdom grid...")
    kingdom_names = ["primordia", "emblazion", "chryseos", "eleasis", "voxlumis", "insightia", "claridium", "poiesis"]
    kingdom_paths = [results.get(f"kingdom_{k}") for k in kingdom_names]
    make_grid(kingdom_paths, OUTPUT_DIR / "slide5_kingdoms_2x4.png", cols=4)

    print(f"\n\nDone! {sum(1 for v in results.values() if v)} images generated.")
    print(f"Output directory: {OUTPUT_DIR.resolve()}")


if __name__ == "__main__":
    main()
