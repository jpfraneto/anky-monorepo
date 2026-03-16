#!/usr/bin/env python3
"""
Anky Autonomous Content System
===============================
Platform strategy:
- IG: Visual mirror (generated anky images)
- X: Journey doc (how anky #847 was made)
- Farcaster: Growth doc (how anky performed)

Runs on poiesis via hermes cron.
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

ANKY_API = "http://127.0.0.1:8889"
IMAGES_DIR = Path.home() / "anky" / "data" / "images"
ENV_PATH = Path.home() / "anky" / ".env"

# Load env
if ENV_PATH.exists():
    with open(ENV_PATH) as f:
        for line in f:
            if "=" in line and not line.startswith("#"):
                k, v = line.strip().split("=", 1)
                os.environ.setdefault(k, v)

INSTAGRAM_TOKEN = os.getenv("INSTAGRAM_ACCESS_TOKEN")
INSTAGRAM_USER_ID = os.getenv("INSTAGRAM_USER_ID", "17841480674971908")


def resolve_local_image_path(anky):
    """Resolve the generated image on disk from the current API response."""
    image_path = anky.get("image_path")
    if image_path:
        candidate = Path(str(image_path).lstrip("/"))
        if candidate.is_absolute() and candidate.exists():
            return candidate
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / candidate.name
        if under_images.exists():
            return under_images

    image_url = anky.get("image_url")
    if image_url:
        parsed = urlparse(str(image_url))
        candidate = Path(parsed.path.lstrip("/"))
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / candidate.name
        if under_images.exists():
            return under_images

    return None

def generate_and_post():
    """Full autonomous flow."""
    moment = random.choice([
        "blue consciousness dissolving into 8 seconds of silence",
        "the mirror that doesn't judge what it reflects",
        "purple hair swirling like forgotten galaxies",
        "golden eyes seeing through the simulation",
        "8th kingdom at the edge of becoming",
    ])
    
    print(f"[{datetime.now()}] Starting generation: {moment[:40]}...")
    
    # 1. Generate via anky api
    resp = requests.post(f"{ANKY_API}/api/generate",
        json={"thinker_name": "Anky", "moment": moment},
        timeout=180  # 3min for full generation
    )
    data = resp.json()
    anky_id = data["anky_id"]
    print(f"✓ Generation started: {anky_id}")
    
    # 2. Poll for completion
    print("Waiting for image...")
    for i in range(90):  # 3min poll
        time.sleep(2)
        status = requests.get(f"{ANKY_API}/api/v1/anky/{anky_id}").json()
        if status.get("status") in {"complete", "completed"} and status.get("image_url"):
            break
    
    # 3. Get image path
    anky = requests.get(f"{ANKY_API}/api/v1/anky/{anky_id}").json()
    img_path = resolve_local_image_path(anky)
    if not img_path:
        raise ValueError(f"No image_path or image_url in {anky}")
    
    # 4. Post to instagram (pure visual)
    web_dir = Path.home() / "anky/static/autonomous"
    web_dir.mkdir(parents=True, exist_ok=True)
    web_img = web_dir / f"{datetime.now():%Y%m%d_%H%M%S}.png"
    web_img.write_bytes(img_path.read_bytes())
    
    # Instagram container
    container = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media",
        data={
            "image_url": f"https://anky.app/static/autonomous/{web_img.name}",
            "caption": f"{anky.get('title', 'Anky')} 🪞\n\n{anky.get('reflection', '')}",
            "access_token": INSTAGRAM_TOKEN
        }
    ).json()
    
    # Publish
    try:
        result = requests.post(
            f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media_publish",
            data={"creation_id": container["id"], "access_token": INSTAGRAM_TOKEN}
        ).json()
    except KeyError as e:
        print(f"⚠️ Instagram publish failed: {e}")
        print(f"Container response: {container}")
        return {}
    
    post_id = result.get('id', 'unknown') if 'id' in result else container["id"]
    print(f"✓ Anky: {anky_id}")
    print(f"→ X/Farcaster: Manual journey docs (separate)")
    
    return result

if __name__ == "__main__":
    try:
        generate_and_post()
    except Exception as e:
        print(f")✗ Failed: {e}")
        sys.exit(1)
