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
        if status.get("image_url"):
            break
    
    # 3. Get image path
    anky = requests.get(f"{ANKY_API}/api/v1/anky/{anky_id}").json()
    img_path = IMAGES_DIR / anky["image_path"]
    
    # 4. Post to instagram (pure visual)
    web_dir = Path.home() / "anky/static/autonomous"
    web_dir.mkdir(parents=True, exist_ok=True)
    web_img = web_dir / f"{datetime.now():%Y%m%d_%H%M%S}.png"
    web_img.write_bytes(img_path.read_bytes())
    
    # Instagram container
    container = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media",
        data={
            "image_url": f"https://anky.app/autonomous/{web_img.name}",
            "caption": f"{anky.get('title', 'Anky')} 🪞\n\n{anky.get('reflection', '')}",
            "access_token": INSTAGRAM_TOKEN
        }
    ).json()
    
    # Publish
    result = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media_publish",
        data={"creation_id": container["id"], "access_token": INSTAGRAM_TOKEN}
    ).json()
    
    print(f"✓ Posted to Instagram: {result.get('id')}")
    print(f"✓ Anky: {anky_id}")
    print(f"→ X/Farcaster: Manual journey docs (separate)")
    
    return result

if __name__ == "__main__":
    try:
        generate_and_post()
    except Exception as e:
        print(f")✗ Failed: {e}")
        sys.exit(1)
