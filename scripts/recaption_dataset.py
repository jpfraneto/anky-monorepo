#!/usr/bin/env python3
"""
Recaption the Anky LoRA training dataset using Gemini Vision.

For each image in the dataset dir, calls Gemini to generate a rich descriptive
caption with "anky" as the trigger word, replacing the generic "a photo of anky".

Usage:
    python3 scripts/recaption_dataset.py
    python3 scripts/recaption_dataset.py --dry-run       # preview without writing
    python3 scripts/recaption_dataset.py --skip-existing # skip already-captioned files
"""

import os
import sys
import time
import base64
import argparse
import requests
from pathlib import Path

DATASET_DIR = Path("/home/kithkui/Desktop/code/z-image-turbo/files/anky_lora_training/dataset")
GEMINI_API_KEY = os.environ["GEMINI_API_KEY"]
GEMINI_URL = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={GEMINI_API_KEY}"

SYSTEM_PROMPT = """You are captioning training images for a LoRA fine-tune of the character "anky".

Anky is a small, chubby, otherworldly creature — a cosmic being embodying the inner child, joy, god, and the raw energy underneath human experience. Anky has:
- A round, compact body with short limbs
- Large, expressive eyes (often glowing)
- Big pointed ears
- Blue, colorful, or psychedelic skin tones depending on the art style
- An ancient yet childlike presence — somewhere between a deity and a goblin

Your job: write a single descriptive caption for this image that will be used as a LoRA training caption.

Rules:
- ALWAYS start with "anky, " — this is the trigger word
- Describe Anky's physical appearance, expression, and pose as they appear in THIS image
- Describe the setting, environment, and mood
- Describe the art style (e.g. psychedelic folk art, digital illustration, painterly, etc.)
- Be specific and visual — not poetic or abstract
- Keep it to 1-3 sentences, no line breaks
- Do NOT mention "training image", "LoRA", or any meta-language
- Do NOT use generic filler like "a photo of" or "an image showing"

Example good captions:
- "anky, a small blue creature with large glowing orange eyes and big pointed ears, seated beneath a massive golden sun with concentric circles, arms raised in wonder, vibrant psychedelic folk art with deep purples and warm oranges"
- "anky, a chubby otherworldly being with expressive wide eyes and a joyful grin, standing in a lush green meadow at golden hour, soft painterly digital art style with warm ambient light"
- "anky, a round childlike cosmic entity with shimmering blue skin, crying a single tear while sitting alone in the rain on a city sidewalk, moody cinematic illustration with cool blue and grey tones"

Now caption this image:"""


def caption_image(image_path: Path) -> str:
    with open(image_path, "rb") as f:
        image_data = base64.b64encode(f.read()).decode()

    # Detect mime type
    ext = image_path.suffix.lower()
    mime = {"png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".webp": "image/webp"}.get(ext, "image/png")

    payload = {
        "contents": [{
            "parts": [
                {"text": SYSTEM_PROMPT},
                {"inlineData": {"mimeType": mime, "data": image_data}}
            ]
        }],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": 500,
            "thinkingConfig": {"thinkingBudget": 0},
        }
    }

    for attempt in range(3):
        try:
            resp = requests.post(GEMINI_URL, json=payload, timeout=30)
            if resp.status_code == 429:
                wait = 15 * (attempt + 1)
                print(f"  Rate limited, waiting {wait}s...")
                time.sleep(wait)
                continue
            resp.raise_for_status()
            data = resp.json()
            text = (
                data.get("candidates", [{}])[0]
                .get("content", {})
                .get("parts", [{}])[0]
                .get("text", "")
                .strip()
            )
            if text:
                return text
            print(f"  Empty response, retrying...")
            time.sleep(2)
        except Exception as e:
            print(f"  Error on attempt {attempt+1}: {e}")
            time.sleep(3)

    return ""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true", help="Print captions without writing files")
    parser.add_argument("--skip-existing", action="store_true", help="Skip images that already have non-generic captions")
    args = parser.parse_args()

    # Find all image files that have a paired .txt
    images = []
    for ext in ["*.png", "*.jpg", "*.jpeg", "*.webp"]:
        for img in DATASET_DIR.glob(ext):
            txt = img.with_suffix(".txt")
            if txt.exists():
                images.append((img, txt))

    images.sort(key=lambda x: x[0].name)
    print(f"Found {len(images)} image-caption pairs in {DATASET_DIR}")

    generic = {"a photo of anky", "a photo of anky\n"}
    skipped = 0
    updated = 0
    failed = 0

    for i, (img_path, txt_path) in enumerate(images):
        current = txt_path.read_text().strip()

        if args.skip_existing and current not in generic:
            skipped += 1
            continue

        print(f"[{i+1}/{len(images)}] {img_path.name}")

        if args.dry_run:
            print(f"  current: {current!r}")
            caption = caption_image(img_path)
            print(f"  new:     {caption!r}")
            print()
            updated += 1
            time.sleep(0.5)
            continue

        caption = caption_image(img_path)

        if not caption:
            print(f"  FAILED — keeping original")
            failed += 1
            continue

        # Ensure it starts with "anky, " or "anky "
        if not caption.lower().startswith("anky"):
            caption = "anky, " + caption

        txt_path.write_text(caption + "\n")
        print(f"  → {caption[:100]}{'...' if len(caption) > 100 else ''}")
        updated += 1

        # Small delay to stay under rate limits (~60 req/min for free tier)
        time.sleep(1.2)

    print(f"\nDone. Updated: {updated}, Skipped: {skipped}, Failed: {failed}")


if __name__ == "__main__":
    main()
