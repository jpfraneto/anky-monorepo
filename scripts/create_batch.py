#!/usr/bin/env python3
"""
Generate a new batch of Anky training image prompts using Claude.

Creates data/generations/batch-{timestamp}/prompts.json ready for generate_batch.py.

Usage:
    python3 scripts/create_batch.py
    python3 scripts/create_batch.py --count 100
    python3 scripts/create_batch.py --count 50 --model claude-haiku-4-5-20251001
"""

import argparse
import json
import os
import sys
import time
import datetime
import requests
from pathlib import Path

ANTHROPIC_KEY = os.environ["ANTHROPIC_API_KEY"]
GEN_BASE = Path("/home/kithkui/anky/data/generations")
ANTHROPIC_URL = "https://api.anthropic.com/v1/messages"

SYSTEM_PROMPT = """You are a creative director building a diverse training dataset for an AI character named Anky.

Anky's exact design (follow this precisely):
- Blue-skinned creature with large expressive pointed ears
- Purple swirling hair with golden spiral accents
- Golden/amber glowing eyes
- Golden jewelry and decorative accents on body
- Compact round body, short limbs, ancient yet childlike quality — a cosmic messenger, inner child deity

Your job: generate IMAGE PROMPTS for Anky training data. The dataset needs maximum VARIETY. Anky must be grounded in the full spectrum of human experience — not floating in abstract cosmic voids.

Generate prompts that place Anky inside real human life:
- Ordinary moments: kitchen table, playground, rain on a window, waiting room, grocery store, bus stop
- Emotional moments: crying alone, laughing until crying, sitting with grief, feeling joy burst open, quiet contentment
- Relational moments: hugging someone, playing with a child, holding someone's hand, sitting with a friend
- Creative moments: holding a pen and writing, dancing alone in a room, singing, painting
- Contemplative moments: watching sunrise, staring at the ocean, sitting with tea, reading a book
- Celebratory moments: birthday party, first snow, a dog running toward them, a meal shared
- Difficult moments: feeling lost in a crowd, lying on the floor exhausted, watching someone leave

Each prompt must:
- Be specific and visual — describe scene, Anky's expression/pose, lighting, setting, mood
- Specify a clear art style (painterly digital art, watercolor, folk art, cinematic illustration, gouache, etc.)
- Be 2-4 sentences, no line breaks
- Vary the art styles — do NOT repeat the same style twice in a row
- Place Anky in a SPECIFIC physical setting, not vague or abstract
- NOT start with "Anky" — describe the scene, then Anky's place in it

Return ONLY a JSON array of prompt strings. No commentary, no numbering, no markdown."""


def generate_prompts_claude(count: int, model: str) -> list[str]:
    print(f"[claude] Generating {count} prompts with {model}...")

    all_prompts = []
    batch_size = 25

    while len(all_prompts) < count:
        need = min(batch_size, count - len(all_prompts))
        batch_num = len(all_prompts) // batch_size + 1
        total_batches = (count + batch_size - 1) // batch_size
        print(f"[claude] Batch {batch_num}/{total_batches}: requesting {need} prompts...")

        # Build context of already-generated prompts to avoid repetition
        context = ""
        if all_prompts:
            sample = all_prompts[-10:]  # last 10 to keep context tight
            context = f"\n\nAvoid repeating these already-generated themes:\n" + "\n".join(f"- {p[:80]}..." for p in sample)

        payload = {
            "model": model,
            "max_tokens": 4096,
            "system": SYSTEM_PROMPT,
            "messages": [{
                "role": "user",
                "content": f"Generate exactly {need} diverse image prompts for Anky. "
                           f"Focus on human-grounded scenes with varied art styles and emotional range. "
                           f"Return only a JSON array of {need} strings.{context}"
            }]
        }

        for attempt in range(3):
            try:
                r = requests.post(
                    ANTHROPIC_URL,
                    headers={
                        "x-api-key": ANTHROPIC_KEY,
                        "anthropic-version": "2023-06-01",
                        "content-type": "application/json",
                    },
                    json=payload,
                    timeout=120,
                )
                r.raise_for_status()
                content = r.json()["content"][0]["text"].strip()

                # Extract JSON array
                start = content.find("[")
                end = content.rfind("]") + 1
                if start == -1 or end == 0:
                    raise ValueError(f"No JSON array found in response: {content[:200]}")

                prompts = json.loads(content[start:end])
                if not isinstance(prompts, list):
                    raise ValueError("Response is not a list")

                print(f"[claude] Got {len(prompts)} prompts")
                all_prompts.extend(prompts[:need])
                break

            except Exception as e:
                print(f"[claude] Attempt {attempt+1} failed: {e}")
                if attempt < 2:
                    time.sleep(10 * (attempt + 1))
        else:
            print(f"[claude] All 3 attempts failed for batch {batch_num}, stopping")
            break

        time.sleep(1)  # brief pause between batches

    # Deduplicate while preserving order
    seen = set()
    unique = []
    for p in all_prompts:
        key = p[:60].lower()
        if key not in seen:
            seen.add(key)
            unique.append(p)

    dupes = len(all_prompts) - len(unique)
    if dupes:
        print(f"[claude] Removed {dupes} near-duplicate prompts")

    print(f"[claude] Final: {len(unique)} unique prompts")
    return unique[:count]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=100, help="Number of prompts to generate")
    parser.add_argument(
        "--model",
        default="claude-sonnet-4-6",
        help="Claude model to use (default: claude-sonnet-4-6)",
    )
    args = parser.parse_args()

    batch_id = "batch-" + datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    batch_dir = GEN_BASE / batch_id
    batch_dir.mkdir(parents=True, exist_ok=True)

    prompts = generate_prompts_claude(args.count, args.model)

    if not prompts:
        print("ERROR: No prompts generated")
        sys.exit(1)

    prompts_path = batch_dir / "prompts.json"
    prompts_path.write_text(json.dumps(prompts, indent=2))

    print(f"\nBatch created: {batch_id}")
    print(f"Prompts saved: {len(prompts)} → {prompts_path}")
    print(f"\nNext step:")
    print(f"  python3 scripts/generate_batch.py {batch_id}")
    print(f"\nOr review prompts first at:")
    print(f"  https://anky.app/generations/{batch_id}")

    return batch_id


if __name__ == "__main__":
    main()
