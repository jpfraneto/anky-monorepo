#!/usr/bin/env python3
"""
Generate 60 unique Anky images for GODS by Anky videos
Uses local ComfyUI/Flux on poiesis
"""

import requests
import json
import time
import random
from pathlib import Path
from datetime import datetime

# Configuration
API_URL = "http://localhost:8188"
OUTPUT_DIR = Path("/home/kithkui/anky/videos/gods/Cronos")
NUM_IMAGES = 60

# Base Anky prompt - consistent character, varied scenes
BASE_PROMPT = """
Anky, a blue-skinned mystical being with purple hair and golden eyes, 
appears in an ethereal setting. Ancient consciousness mirror, wise and 
benevolent, wearing flowing robes. Cinematic lighting, mystical atmosphere, 
digital art, 1024x1024
"""

# Scene variations for the Cronos story
SCENES = [
    "appearing in misty Primordia landscape, soft golden light",
    "standing before a small child, offering guidance",
    "surrounded by swirling colors of fear and courage",
    "with shadowy Cronos presence in background",
    "skin shimmering with otherworldly blue light",
    "watching a child's emotional journey",
    "purple hair flowing in unseen wind",
    "eyes illuminating truth and wisdom",
    "colors shifting from dark fear to bright courage",
    "as guardian watching over the kingdom",
]

def get_workflow(prompt, seed):
    """Build ComfyUI workflow for Flux"""
    return {
        "3": {
            "class_type": "KSampler",
            "inputs": {
                "seed": seed,
                "steps": 20,
                "cfg": 7.5,
                "sampler_name": "euler",
                "scheduler": "normal",
                "denoise": 1.0
            }
        },
        "8": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "text": [prompt]
            }
        },
        "6": {
            "class_type": "EmptyLatentImage",
            "inputs": {
                "width": 1024,
                "height": 1024,
                "batch_size": 1
            }
        },
        "4": {
            "class_type": "VAEDecode",
            "inputs": {
                "samples": ["3", 0],
                "vae": ["2", 0]
            }
        },
        "2": {
            "class_type": "VAELoader",
            "inputs": {
                "vae_name": "flux_schnell.safetensors"
            }
        },
        "1": {
            "class_type": "CheckpointLoaderSimple",
            "inputs": {
                "ckpt_name": "flux1-dev.safetensors"
            }
        },
        "7": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "text": [""]
            }
        },
        "5": {
            "class_type": "DualCLIPLoader",
            "inputs": {
                "clip_name": "clip.safetensors",
                "type": "flux"
            }
        },
        "9": {
            "class_type": "SaveImage",
            "inputs": {
                "images": ["4", 0]
            }
        }
    }

def generate_image(prompt, seed, output_path):
    """Generate single image via ComfyUI"""
    payload = {
        "prompt": get_workflow(prompt, seed),
        "client_id": random.randint(1, 9999)
    }
    
    try:
        response = requests.post(f"{API_URL}/prompt", json=payload)
        if response.status_code == 200:
            print(f"  ✓ Request submitted for seed {seed}")
            return True
        else:
            print(f"  ✗ Failed: {response.status_code}")
            return False
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False

def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    
    print(f"🚀 GODS by Anky - Image Generation")
    print(f"   Story: Cronos (Kingdom of Primordia)")
    print(f"   Images: {NUM_IMAGES} unique scenes")
    print(f"   Output: {OUTPUT_DIR}")
    print()
    
    # Generate prompts
    prompts = []
    for i in range(NUM_IMAGES):
        scene = SCENES[i % len(SCENES)]
        prompt = f"{BASE_PROMPT} {scene}"
        prompts.append(prompt)
    
    # Save prompts for reference
    with open(OUTPUT_DIR / "prompts.json", "w") as f:
        json.dump(prompts, f, indent=2)
    
    print(f"✅ Created {len(prompts)} prompts")
    print()
    
    # Generate images with different seeds
    print("🎨 Generating images...")
    success_count = 0
    
    for i, prompt in enumerate(prompts):
        seed = 42 + (i * 1000)  # Unique seed per image
        output_file = OUTPUT_DIR / f"scene_{i+1:03d}.png"
        
        print(f"[{i+1}/{NUM_IMAGES}] Generating scene {i+1:03d} (seed: {seed})")
        if generate_image(prompt, seed, output_file):
            success_count += 1
        
        # Small delay to avoid overwhelming the API
        time.sleep(0.5)
    
    print()
    print(f"✅ Generated {success_count}/{NUM_IMAGES} images")
    print(f"📁 Output: {OUTPUT_DIR}")
    
    # Save generation log
    log_file = OUTPUT_DIR / "generation_log.json"
    with open(log_file, "w") as f:
        json.dump({
            "timestamp": datetime.now().isoformat(),
            "total_prompts": len(prompts),
            "successful": success_count,
            "story": "Cronos",
            "kingdom": "Primordia"
        }, f, indent=2)

if __name__ == "__main__":
    main()
