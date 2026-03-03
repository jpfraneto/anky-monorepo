#!/usr/bin/env python3
"""
Generate 100 new Anky training images using:
  1. Grok (xAI) — generates 100 diverse prompts spanning human experience
  2. Gemini image gen — generates each image using 1 random training seed + 3 canonical refs
  3. Gemini Vision — recaptions each generated image with a descriptive training caption

Output: data/generated_training/{uuid}.png + {uuid}.txt

Usage:
    python3 scripts/generate_training_images.py
    python3 scripts/generate_training_images.py --count 20   # generate fewer
    python3 scripts/generate_training_images.py --skip-prompts prompts.json  # reuse saved prompts
"""

import os
import sys
import json
import time
import base64
import random
import argparse
import requests
import uuid
from pathlib import Path

# ── Config ────────────────────────────────────────────────────────────────────

DATASET_DIR  = Path("/home/kithkui/Desktop/code/z-image-turbo/files/anky_lora_training/dataset")
REFS_DIR     = Path("/home/kithkui/anky/src/public")
OUTPUT_DIR   = Path("/home/kithkui/anky/data/generated_training")
PROMPTS_LOG  = Path("/home/kithkui/anky/data/generated_training/prompts.json")

GEMINI_KEY   = os.environ["GEMINI_API_KEY"]
XAI_KEY      = os.environ["XAI_API_KEY"]

GEMINI_GEN_URL     = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent?key={GEMINI_KEY}"
GEMINI_VISION_URL  = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={GEMINI_KEY}"
XAI_CHAT_URL       = "https://api.x.ai/v1/chat/completions"

# ── Prompt generation with Grok ───────────────────────────────────────────────

GROK_SYSTEM = """You are a creative director designing training images for an AI character named Anky.

Anky is a small, chubby, otherworldly being — a cosmic goblin embodying the inner child, joy, grief, god, beauty, and the raw energy underneath all human experience. Anky has:
- A round compact body with short limbs and big pointed ears
- Large expressive glowing eyes
- Blue or richly colored skin depending on art style
- An ancient yet childlike presence — somewhere between a deity and a goblin

Your job: generate IMAGE PROMPTS for Anky training data. The dataset needs VARIETY. Anky must be grounded in human experience — not just floating in cosmic voids.

Generate prompts that place Anky inside the full spectrum of human life:
- Ordinary moments: kitchen table, playground, rain on a window, waiting room, grocery store
- Emotional moments: crying alone, laughing until crying, sitting with grief, feeling joy burst open
- Relational moments: hugging someone, playing with a child, holding someone's hand
- Creative moments: holding a pen and writing, dancing alone in a room, singing
- Contemplative moments: watching the sunrise, staring at the ocean, sitting with a cup of tea
- Celebratory moments: birthday party, first snow, a dog running toward them
- Difficult moments: feeling lost in a crowd, lying on the floor exhausted, watching someone leave

Each prompt should:
- Be vivid and specific — describe the scene, Anky's expression/pose, the lighting, the setting
- Specify an art style (painterly, digital illustration, folk art, watercolor, cinematic, etc.)
- Be 2-4 sentences
- NOT be generic cosmic imagery — Anky can be cosmic in spirit but human in setting

Return ONLY a JSON array of prompt strings, no commentary."""

def generate_prompts_with_grok(count: int) -> list[str]:
    print(f"[grok] Generating {count} prompts via xAI Grok...")

    # Generate in batches of 25 to stay under token limits
    all_prompts = []
    batch_size = 25
    batches = (count + batch_size - 1) // batch_size

    for i in range(batches):
        n = min(batch_size, count - len(all_prompts))
        print(f"[grok] Batch {i+1}/{batches}: requesting {n} prompts...")

        payload = {
            "model": "grok-3",
            "messages": [
                {"role": "system", "content": GROK_SYSTEM},
                {"role": "user", "content": f"Generate exactly {n} diverse image prompts for Anky. Focus on human-grounded scenes. Return only a JSON array of {n} strings."}
            ],
            "temperature": 0.9,
            "max_tokens": 4000,
        }

        for attempt in range(3):
            try:
                r = requests.post(
                    XAI_CHAT_URL,
                    headers={"Authorization": f"Bearer {XAI_KEY}", "Content-Type": "application/json"},
                    json=payload,
                    timeout=60,
                )
                r.raise_for_status()
                content = r.json()["choices"][0]["message"]["content"].strip()

                # Parse JSON array from response
                start = content.find("[")
                end = content.rfind("]") + 1
                if start == -1 or end == 0:
                    raise ValueError("No JSON array found in response")

                prompts = json.loads(content[start:end])
                print(f"[grok] Got {len(prompts)} prompts")
                all_prompts.extend(prompts[:n])
                break
            except Exception as e:
                print(f"[grok] Attempt {attempt+1} failed: {e}")
                if attempt < 2:
                    time.sleep(5)

        time.sleep(2)  # rate limit pause between batches

    print(f"[grok] Total prompts generated: {len(all_prompts)}")
    return all_prompts[:count]


# ── Image generation with Gemini ──────────────────────────────────────────────

def load_image_b64(path: Path) -> str:
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode()

def get_random_training_seed() -> tuple[str, str]:
    """Pick a random PNG from the training dataset. Returns (b64, mime_type)."""
    pngs = list(DATASET_DIR.glob("*.png"))
    chosen = random.choice(pngs)
    return load_image_b64(chosen), "image/png"

def get_canonical_refs() -> list[tuple[str, str]]:
    """Load canonical anky-1, anky-2, anky-3 reference images."""
    refs = []
    for name in ["anky-1.png", "anky-2.png", "anky-3.png"]:
        path = REFS_DIR / name
        if path.exists():
            refs.append((load_image_b64(path), "image/png"))
    return refs

def generate_image_gemini(prompt: str, seed_b64: str, seed_mime: str, canonical_refs: list) -> bytes | None:
    """Generate an Anky image. Returns raw PNG bytes or None on failure."""

    parts = []

    # Add canonical refs first (character consistency)
    for b64, mime in canonical_refs:
        parts.append({"inlineData": {"mimeType": mime, "data": b64}})

    # Add random training seed (visual variety)
    parts.append({"inlineData": {"mimeType": seed_mime, "data": seed_b64}})

    parts.append({"text": (
        "The images above show Anky — use them as visual character references. "
        "Generate a new image based on this prompt:\n\n" + prompt
    )})

    payload = {
        "contents": [{"parts": parts}],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"],
            "imageConfig": {"aspectRatio": "1:1"},
        }
    }

    for attempt in range(3):
        try:
            r = requests.post(GEMINI_GEN_URL, json=payload, timeout=120)
            if r.status_code == 429:
                wait = 20 * (attempt + 1)
                print(f"  [rate limit] waiting {wait}s...")
                time.sleep(wait)
                continue
            r.raise_for_status()

            data = r.json()
            candidates = data.get("candidates", [])
            if not candidates:
                print(f"  [gemini-gen] No candidates in response")
                continue

            parts_resp = candidates[0].get("content", {}).get("parts", [])
            for part in parts_resp:
                inline = part.get("inlineData")
                if inline and inline.get("mimeType", "").startswith("image/"):
                    return base64.b64decode(inline["data"])

            print(f"  [gemini-gen] No image part in response")
        except Exception as e:
            print(f"  [gemini-gen] Attempt {attempt+1} failed: {e}")
            time.sleep(5)

    return None


# ── Captioning with Gemini Vision ─────────────────────────────────────────────

CAPTION_PROMPT = """Caption this training image for an AI character named "anky".

Anky is a small chubby creature with big pointed ears, large glowing eyes, round compact body, blue or colorful skin, ancient yet childlike — like a cosmic goblin or inner child deity.

Write ONE caption. Start with "anky, ". Describe: appearance, expression, pose, setting, art style. Max 2 sentences. No meta-language.

Caption:"""

def caption_image_gemini(image_bytes: bytes) -> str:
    b64 = base64.b64encode(image_bytes).decode()
    payload = {
        "contents": [{"parts": [
            {"text": CAPTION_PROMPT},
            {"inlineData": {"mimeType": "image/png", "data": b64}}
        ]}],
        "generationConfig": {
            "temperature": 0.4,
            "maxOutputTokens": 500,
            "thinkingConfig": {"thinkingBudget": 0},
        }
    }
    for attempt in range(3):
        try:
            r = requests.post(GEMINI_VISION_URL, json=payload, timeout=30)
            if r.status_code == 429:
                time.sleep(15 * (attempt + 1))
                continue
            r.raise_for_status()
            caption = r.json()["candidates"][0]["content"]["parts"][0]["text"].strip()
            if not caption.lower().startswith("anky"):
                caption = "anky, " + caption
            return caption
        except Exception as e:
            print(f"  [caption] Attempt {attempt+1} failed: {e}")
            time.sleep(3)
    return "anky, a colorful cosmic creature in a vibrant illustrated scene"


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=100)
    parser.add_argument("--skip-prompts", type=str, help="Path to existing prompts JSON to reuse")
    args = parser.parse_args()

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Step 1: Get prompts
    if args.skip_prompts and Path(args.skip_prompts).exists():
        print(f"[prompts] Loading from {args.skip_prompts}")
        with open(args.skip_prompts) as f:
            prompts = json.load(f)[:args.count]
    else:
        prompts = generate_prompts_with_grok(args.count)
        PROMPTS_LOG.parent.mkdir(parents=True, exist_ok=True)
        with open(PROMPTS_LOG, "w") as f:
            json.dump(prompts, f, indent=2)
        print(f"[prompts] Saved {len(prompts)} prompts to {PROMPTS_LOG}")

    # Step 2: Load canonical refs once
    canonical_refs = get_canonical_refs()
    print(f"[refs] Loaded {len(canonical_refs)} canonical reference images")

    # Step 3: Generate images
    success = 0
    failed = 0

    for i, prompt in enumerate(prompts):
        print(f"\n[{i+1}/{len(prompts)}] {prompt[:80]}...")

        # Pick a random seed from training dataset
        seed_b64, seed_mime = get_random_training_seed()

        # Generate image
        print(f"  [gemini-gen] generating...")
        image_bytes = generate_image_gemini(prompt, seed_b64, seed_mime, canonical_refs)

        if not image_bytes:
            print(f"  FAILED — skipping")
            failed += 1
            continue

        # Save image
        image_id = str(uuid.uuid4())
        img_path = OUTPUT_DIR / f"{image_id}.png"
        img_path.write_bytes(image_bytes)

        # Caption it
        print(f"  [caption] captioning...")
        caption = caption_image_gemini(image_bytes)
        txt_path = OUTPUT_DIR / f"{image_id}.txt"
        txt_path.write_text(caption + "\n")

        print(f"  saved: {image_id[:8]}...")
        print(f"  caption: {caption[:100]}{'...' if len(caption) > 100 else ''}")
        success += 1

        # Rate limit: ~2s between images
        time.sleep(2)

    print(f"\n{'='*50}")
    print(f"Done! Generated: {success}, Failed: {failed}")
    print(f"Images saved to: {OUTPUT_DIR}")
    print(f"\nTo add to training dataset, copy to:")
    print(f"  {DATASET_DIR}")


if __name__ == "__main__":
    main()
