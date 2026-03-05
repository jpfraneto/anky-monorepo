#!/usr/bin/env python3
"""Caption gallery images in data/images that are missing recoverable prompts.

This writes `.txt` files next to `data/images/*.png` so those images can be
included in the final round-two training export.
"""

from __future__ import annotations

import argparse
import base64
import os
import sqlite3
import time
from pathlib import Path

import requests


ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "data"
IMAGES_DIR = DATA_DIR / "images"
DB_PATH = DATA_DIR / "anky.db"

GEMINI_API_KEY = os.environ["GEMINI_API_KEY"]
GEMINI_URL = (
    "https://generativelanguage.googleapis.com/v1beta/models/"
    f"gemini-2.5-flash:generateContent?key={GEMINI_API_KEY}"
)

SYSTEM_PROMPT = """You are captioning training images for a FLUX LoRA fine-tune of the character "anky".

Write one training caption for the provided image.

Rules:
- ALWAYS begin with "anky, "
- Describe the visible character, pose, expression, environment, and art style
- Be concrete and visual, not poetic
- Keep it to one sentence if possible, at most two short sentences
- Do NOT mention LoRA, training, prompt engineering, or metadata
- Do NOT say "an image of" or "a picture of"
- If the image is a close duplicate frame from a video, still caption only what is visible in this exact frame
"""


def load_known_captions() -> set[str]:
    names: set[str] = set()
    if DB_PATH.exists():
        conn = sqlite3.connect(DB_PATH)
        try:
            cur = conn.cursor()
            for (image_path,) in cur.execute(
                """
                SELECT image_path
                FROM ankys
                WHERE image_path IS NOT NULL
                  AND TRIM(image_path) <> ''
                  AND image_prompt IS NOT NULL
                  AND TRIM(image_prompt) <> ''
                """
            ):
                names.add(image_path)
            for (image_path,) in cur.execute(
                """
                SELECT image_path
                FROM prompts
                WHERE image_path IS NOT NULL
                  AND TRIM(image_path) <> ''
                  AND prompt_text IS NOT NULL
                  AND TRIM(prompt_text) <> ''
                """
            ):
                names.add(image_path)
        finally:
            conn.close()

    for caption_path in IMAGES_DIR.glob("*.txt"):
        names.add(caption_path.with_suffix(".png").name)
    return names


def caption_image(image_path: Path) -> str:
    image_data = base64.b64encode(image_path.read_bytes()).decode()
    payload = {
        "contents": [{
            "parts": [
                {"text": SYSTEM_PROMPT},
                {"inlineData": {"mimeType": "image/png", "data": image_data}},
            ]
        }],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": 220,
            "thinkingConfig": {"thinkingBudget": 0},
        },
    }

    for attempt in range(4):
        try:
            resp = requests.post(GEMINI_URL, json=payload, timeout=60)
            if resp.status_code == 429:
                wait = 20 * (attempt + 1)
                print(f"rate limited, waiting {wait}s")
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
                if not text.lower().startswith("anky"):
                    text = "anky, " + text
                return text
        except Exception as exc:
            print(f"error on {image_path.name} attempt {attempt + 1}: {exc}")
            time.sleep(4 * (attempt + 1))
    return ""


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--limit", type=int, default=0, help="Only caption the first N missing images")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be captioned without writing")
    args = parser.parse_args()

    known = load_known_captions()
    missing = [
        path for path in sorted(IMAGES_DIR.glob("*.png"))
        if path.name not in known and not path.with_suffix(".txt").exists()
    ]
    if args.limit > 0:
        missing = missing[:args.limit]

    print(f"Missing gallery images to caption: {len(missing)}")

    updated = 0
    failed = 0

    for index, image_path in enumerate(missing, start=1):
        print(f"[{index}/{len(missing)}] {image_path.name}")
        caption = caption_image(image_path)
        if not caption:
            print("  failed")
            failed += 1
            continue
        if args.dry_run:
            print(f"  {caption}")
            updated += 1
            time.sleep(0.2)
            continue

        image_path.with_suffix(".txt").write_text(caption.strip() + "\n")
        print(f"  wrote {image_path.with_suffix('.txt').name}")
        updated += 1
        time.sleep(1.2)

    print(f"Done. Updated: {updated}, Failed: {failed}")


if __name__ == "__main__":
    main()
