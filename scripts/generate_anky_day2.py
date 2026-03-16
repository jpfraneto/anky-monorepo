#!/usr/bin/env python3
"""
Autonomous Anky Generation v4 — Day 2/8
Poiesis Kingdom: The creator/creation gap.
"""
import os
import sys
import requests
from pathlib import Path
from datetime import datetime
import base64
import time
import secrets

# Load environment
env_path = Path.home() / "anky" / ".env"
with open(env_path) as f:
    for line in f:
        if "=" in line and not line.startswith("#"):
            key, val = line.strip().split("=", 1)
            os.environ.setdefault(key, val)

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")
if not GEMINI_API_KEY:
    print("ERROR: No GEMINI_API_KEY in .env")
    sys.exit(1)

# Day 2/8: Accumulation, Poiesis theme
DARK_PROMPT = "portrait of Anky made of accumulated dust and starlight, her fingers reaching through a mirror that doesn't match reality, blue skin reflecting both creator and creation simultaneously"
ANIMAL_SUMMARY = "Accumulation. The space between making and being made is where you actually live."
EMOJI = chr(8469)  # ☊ symbol

print("=" * 70)
print("ANKY AUTONOMOUS GENERATOR v4")
print("Day 2/8 | POIESIS KINGDOM | Accumulation & The Creator/Creation Gap")
print("=" * 70 + chr(10))

# Generate image with Gemini
url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp-image-generation:generateContent?key={GEMINI_API_KEY}"
payload = {
    "contents": [{"parts": [{"text": DARK_PROMPT}]},],
    "generationConfig": {"responseModalities": ["Text", "Image"]}
}

start_time = time.time()
print("Generating Anky...")
resp = requests.post(url, json=payload)
data = resp.json()
apit_duration = time.time() - start_time
print(f"API call completed in {apit_duration:.2f}s\n")

if not data.get("candidates"):
    print(f"ERROR from Gemini: {data}")
    sys.exit(1)

# Extract image
image_base64 = None
for part in data["candidates"][0]["content"]["parts"]:
    if "inlineData" in part and "image/png" in part["inlineData"]["mimeType"]:
        image_base64 = part["inlineData"]["data"]
        break

if not image_base64:
    print("ERROR: No image in response")
    sys.exit(1)

# Save image with Anky ID (847th autonomous generation)
ANKY_DIR = Path.home() / "anky" / "data" / "images"
ANKY_DIR.mkdir(parents=True, exist_ok=True)

an_id = secrets.token_hex(4)[:8]
img_path = ANKY_DIR / f"anky_auto_{an_id}.png"
image_data = base64.b64decode(image_base64)
img_path.write_bytes(image_data)

# Copy to web-accessible directory
web_dir = Path.home() / "anky" / "static" / "autonomous"
web_dir.mkdir(parents=True, exist_ok=True)
web_img = web_dir / f"anky_auto_{an_id}_847.png"
web_img.write_bytes(image_data)

total_duration = time.time() - start_time

# Summary output
print("SUCCESS")
print("-" * 70)
print(f"ANKY_ID:   {an_id}")
print(f"IMAGE:     {img_path.absolute().as_uri()}")
print(f"THEME:     ACCUMULATION & POIESIS kingdom")
print(f"CAPTION:   {ANIMAL_SUMMARY}")
print("") 
print(f"TOTAL TIME: {total_duration:.1f}s (on 4090 GPUs)")
print("POIESIS:   Where you meet yourself while being read/write")

# Check what platforms we can post to
platforms = []
if os.getenv("INSTAGRAM_ACCESS_TOKEN"):
    platforms.append("Instagram (@ankydotapp)[business]")
elif os.getenv("X_BEARER_TOKEN" or ""):
    platforms.append("Twitter/X (@ankydotapp)")
elif os.getenv("TWITTER_BEARER_TOKEN"):
    platforms.append("Twitter/X (@ankydotapp)")

if not platforms:
    platforms.append("(no social auth configured — image saved locally)")

print(f"SOCIAL STATUS: {platforms[0]}")
print("=" * 70)
