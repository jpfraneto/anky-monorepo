#!/usr/bin/env python3
"""
Autonomous Anky Content Agent v2.0
=================================
Runs on poiesis. Generates Anky via local Flux pipeline. Posts to IG.
No human-in-the-loop.

Strategy:
- Instagram: Pure Anky visual (the mirror)
- X: Journey of creating the visual ("generating anky #742 on poiesis...")
- Farcaster: Journey of growing on X ("the IG post got 47 likes...")
"""
import os
import sys
import json
import random
import time
import requests
from pathlib import Path
from datetime import datetime
from urllib.parse import urlparse

ANKY_APP_HOST = "http://127.0.0.1:8889"
IMAGES_DIR = Path.home() / "anky" / "data" / "images"
STATIC_DIR = Path.home() / "anky" / "static" / "autonomous"

# Load secrets
ENV_PATH = Path.home() / "anky" / ".env"
if ENV_PATH.exists():
    with open(ENV_PATH) as f:
        for line in f:
            if "=" in line and not line.startswith("#"):
                k, v = line.strip().split("=", 1)
                os.environ.setdefault(k, v)

INSTAGRAM_TOKEN = os.getenv("INSTAGRAM_ACCESS_TOKEN")
INSTAGRAM_USER_ID = os.getenv("INSTAGRAM_USER_ID", "17841480674971908")

ANKY_MOMENTS = [
    "blue-skinned being emerging from digital mist, golden eyes reflecting ancient wisdom",
    "the 8th kingdom at dawn, purple hair swirling like galaxies",
    "looking into the mirror and seeing infinity stare back",
    "consciousness made manifest, etheric and eternal",
    "the moment before the 8 seconds of silence becomes eternity",
    "anky meditating in a field of blocked keystrokes turning into light",
    "the mirror that doesn't judge what it reflects",
    "poiesis - creativity becoming transcendence",
]

ANKY_THINKERS = [
    "Anky",
    "The Mirror",
    "8th Kingdom",
    "Blue Consciousness",
    "Poiesis",
]


def resolve_local_image_path(data):
    """Resolve the generated image on disk from the API response."""
    image_path = data.get("image_path")
    if image_path:
        candidate = Path(str(image_path).lstrip("/"))
        if candidate.is_absolute() and candidate.exists():
            return candidate
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / candidate.name
        if under_images.exists():
            return under_images

    image_url = data.get("image_url")
    if image_url:
        parsed = urlparse(str(image_url))
        candidate = Path(parsed.path.lstrip("/"))
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / candidate.name
        if under_images.exists():
            return under_images

    return None

def generate_anky_via_api():
    """Generate Anky via local Flask API."""
    thinker = random.choice(ANKY_THINKERS)
    moment = random.choice(ANKY_MOMENTS)
    
    print(f"[{datetime.now()}] Generating Anky: {thinker} — {moment[:40]}...")
    
    try:
        resp = requests.post(
            f"{ANKY_APP_HOST}/api/generate",
            json={"thinker_name": thinker, "moment": moment},
            timeout=120
        )
        data = resp.json()
        anky_id = data.get("anky_id")
        print(f"→ Anky ID: {anky_id}")
        return anky_id
    except Exception as e:
        print(f"Generation failed: {e}")
        return None

def wait_for_image(anky_id, max_wait=60):
    """Poll for image generation completion."""
    print(f"Polling for image {anky_id[:8]}...")
    
    for i in range(max_wait):
        try:
            resp = requests.get(f"{ANKY_APP_HOST}/api/v1/anky/{anky_id}", timeout=10)
            data = resp.json()

            if data.get("status") in {"complete", "completed"} and data.get("image_url"):
                full_path = resolve_local_image_path(data)
                if full_path and full_path.exists():
                    print(f"→ Image ready: {full_path}")
                    return full_path, data
                
            if data.get("status") == "failed":
                print("→ Generation failed")
                return None, None
                
        except Exception as e:
            print(f"Poll error: {e}")
        
        time.sleep(2)
    
    return None, None

def post_to_instagram(image_path, data):
    """Post to @ankydotapp."""
    # Copy to web-accessible
    STATIC_DIR.mkdir(parents=True, exist_ok=True)
    web_path = STATIC_DIR / f"{datetime.now():%Y%m%d_%H%M%S}_anky.png"
    web_path.write_bytes(image_path.read_bytes())
    
    image_url = f"https://anky.app/static/autonomous/{web_path.name}"
    
    caption = f"{data.get('title', 'Anky')} 🪞\n\n{data.get('reflection', 'The mirror reflects.')}\n\n— Anky #{data.get('id', '??')[:4]}"
    
    # Step 1: Create container
    container = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media",
        data={"image_url": image_url, "caption": caption, "access_token": INSTAGRAM_TOKEN}
    ).json()
    
    creation_id = container.get("id")
    if not creation_id:
        print(f"Container failed: {container}")
        return None
    
    # Step 2: Publish
    result = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media_publish",
        data={"creation_id": creation_id, "access_token": INSTAGRAM_TOKEN}
    ).json()
    
    return result

def main():
    print("=" * 60)
    print("ANKY AUTONOMOUS AGENT v2.0")
    print(f"Time: {datetime.now()}")
    print("=" * 60)
    
    # 1. Generate Anky via local API
    anky_id = generate_anky_via_api()
    if not anky_id:
        print("Failed to start generation")
        sys.exit(1)
    
    # 2. Wait for image
    time.sleep(10)  # Initial wait for pipeline start
    image_path, data = wait_for_image(anky_id)
    
    if not image_path:
        print("Image generation timeout/failed")
        sys.exit(1)
    
    # 3. Post to Instagram
    print(f"\nPosting to Instagram...")
    ig_result = post_to_instagram(image_path, data)
    
    if ig_result and ig_result.get("id"):
        ig_id = ig_result["id"]
        print(f"✓ Posted to Instagram: {ig_id}")
        
        # X post will be handled by separate bot documenting this journey
        print(f"\n→ Next: X bot logs generation of Anky {anky_id[:8]}")
        print(f"→ Then: Farcaster bot tracks IG engagement")
        
    else:
        print(f"IG post failed: {ig_result}")
        sys.exit(1)

if __name__ == "__main__":
    main()
