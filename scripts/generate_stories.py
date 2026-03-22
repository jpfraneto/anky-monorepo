#!/usr/bin/env python3
"""Generate cuentacuentos stories from existing ankys via Ollama.

Picks ankys that don't have stories yet, generates stories via the local
Ollama model, inserts them into SQLite with image job rows and translations.
The server's background retry loop will pick up pending image jobs automatically.

Usage:
    python3 scripts/generate_stories.py [--count N] [--skip-translations]
"""

import sqlite3
import uuid
import json
import re
import sys
import time
import requests
import argparse
from pathlib import Path

DB_PATH = Path(__file__).parent.parent / "data" / "anky.db"
OLLAMA_URL = "http://localhost:11434"
OLLAMA_MODEL = "qwen3.5:35b"
SYSTEM_PROMPT_PATH = Path(__file__).parent.parent / "prompts" / "cuentacuentos_system.md"

def get_system_prompt():
    return SYSTEM_PROMPT_PATH.read_text()

def call_ollama(system: str, user: str, timeout: int = 600) -> str:
    resp = requests.post(
        f"{OLLAMA_URL}/api/chat",
        json={
            "model": OLLAMA_MODEL,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "stream": False,
            "think": False,
            "options": {"num_predict": 4096},
        },
        timeout=timeout,
    )
    resp.raise_for_status()
    return resp.json()["message"]["content"]

def call_ollama_simple(prompt: str, timeout: int = 600) -> str:
    resp = requests.post(
        f"{OLLAMA_URL}/api/chat",
        json={
            "model": OLLAMA_MODEL,
            "messages": [{"role": "user", "content": prompt}],
            "stream": False,
            "think": False,
            "options": {"num_predict": 4096},
        },
        timeout=timeout,
    )
    resp.raise_for_status()
    return resp.json()["message"]["content"]

def build_user_prompt(writing: str) -> str:
    return f"""Parent writing:

---
{writing[:4000]}
---

Return ONLY valid JSON with this exact shape:
{{
  "chakra": <number 1-8>,
  "kingdom": "<kingdom name>",
  "city": "<city name from that kingdom>",
  "title": "A short evocative title",
  "content": "The full story in the same language as the parent's writing, 400-600 words, with paragraph breaks as double newlines. Set in the named city, narrated by Anky from inside one character."
}}"""

def strip_fences(raw: str) -> str:
    raw = raw.strip()
    # Remove thinking tags if present
    raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
    if raw.startswith("```json"):
        raw = raw[7:]
    elif raw.startswith("```"):
        raw = raw[3:]
    if raw.endswith("```"):
        raw = raw[:-3]
    return raw.strip()

def parse_story(raw: str) -> dict:
    clean = strip_fences(raw)
    # Try to find JSON object in the text
    # Sometimes models output extra text before/after the JSON
    match = re.search(r'\{[\s\S]*\}', clean)
    if match:
        try:
            return json.loads(match.group())
        except json.JSONDecodeError:
            pass
    return json.loads(clean)

def story_paragraphs(content: str) -> list[str]:
    return [p.strip() for p in content.split("\n\n") if p.strip()]

def estimate_duration(paragraph: str) -> int:
    words = len(paragraph.split())
    return max(12, min(90, round((words / 130) * 60)))

def build_guidance_phases(content: str) -> str:
    paragraphs = story_paragraphs(content)
    phases = []
    for i, p in enumerate(paragraphs):
        phases.append({
            "name": f"Parte {i+1}",
            "phase_type": "narration",
            "duration_seconds": estimate_duration(p),
            "narration": p,
            "inhale_seconds": None,
            "exhale_seconds": None,
            "hold_seconds": None,
            "reps": None,
        })
    return json.dumps(phases)

def generate_image_prompt(title: str, paragraph: str, kingdom: str | None) -> str:
    kingdom_ctx = f" in the kingdom of {kingdom}" if kingdom else ""
    seed = f"Children's story scene{kingdom_ctx} from the tale \"{title}\": {paragraph}"
    prompt = f"""Convert this scene description into a concise image generation prompt for a children's story illustration. Focus on visual elements, mood, lighting, and composition. Keep it under 100 words. Return ONLY the prompt, no explanation.

Scene: {seed}"""
    try:
        result = call_ollama_simple(prompt, timeout=120)
        # Strip thinking tags
        result = re.sub(r'<think>.*?</think>', '', result, flags=re.DOTALL).strip()
        if result and len(result) > 20:
            return result[:500]
    except Exception as e:
        print(f"  [warn] image prompt generation failed: {e}")
    return seed[:500]

def translate_story(title: str, content: str) -> dict:
    """Translate story to ES/ZH/HI/AR. Returns dict of {code: full_text}."""
    paragraphs = story_paragraphs(content)
    numbered = "\n\n".join(f"[{i+1}] {p}" for i, p in enumerate(paragraphs))

    languages = [
        ("es", "Spanish"),
        ("zh", "Mandarin Chinese"),
        ("hi", "Hindi"),
        ("ar", "Arabic"),
    ]

    translations = {}
    for code, name in languages:
        prompt = f"""Translate this children's story into {name}. Maintain the same paragraph numbering. Each paragraph is marked with [N]. Return ONLY the translated paragraphs in the same [N] format, nothing else.

Title: {title}

{numbered}

Return the translation preserving [N] markers. Do not add any explanation."""

        try:
            raw = call_ollama_simple(prompt, timeout=420)
            # Strip thinking tags
            raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
            parsed = parse_numbered_paragraphs(raw, len(paragraphs))
            translations[code] = parsed
            print(f"    translated to {name} ({len(parsed)} paragraphs)")
        except Exception as e:
            print(f"    [warn] translation to {name} failed: {e}")

    return translations

def parse_numbered_paragraphs(raw: str, expected: int) -> list[str]:
    result = []
    current_idx = None
    current_text = ""

    for line in raw.split("\n"):
        trimmed = line.strip()
        m = re.match(r'^\[(\d+)\]\s*(.*)', trimmed)
        if m:
            if current_idx is not None:
                result.append((current_idx, current_text.strip()))
            current_idx = int(m.group(1))
            current_text = m.group(2)
        elif current_idx is not None:
            if trimmed:
                if current_text:
                    current_text += " "
                current_text += trimmed

    if current_idx is not None:
        result.append((current_idx, current_text.strip()))

    # Sort by index and return texts
    result.sort(key=lambda x: x[0])
    texts = [t for _, t in result]

    # Pad if needed
    while len(texts) < expected:
        texts.append("")

    return texts[:expected]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--count", type=int, default=8)
    parser.add_argument("--skip-translations", action="store_true")
    parser.add_argument("--skip-images", action="store_true")
    args = parser.parse_args()

    if not DB_PATH.exists():
        print(f"Database not found at {DB_PATH}")
        sys.exit(1)

    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row

    # Find ankys without stories, with real content (exclude agent-written)
    ankys = conn.execute("""
        SELECT ws.id, ws.user_id, ws.content, ws.word_count
        FROM writing_sessions ws
        LEFT JOIN cuentacuentos c ON c.writing_id = ws.id
        WHERE ws.is_anky = 1
          AND c.id IS NULL
          AND ws.content IS NOT NULL
          AND length(ws.content) > 200
          AND ws.word_count > 400
          AND ws.user_id NOT LIKE 'agent:%'
          AND ws.user_id != 'api-user'
        ORDER BY ws.word_count DESC
        LIMIT ?
    """, (args.count * 3,)).fetchall()  # fetch extra in case some fail

    if not ankys:
        print("No eligible ankys found")
        sys.exit(0)

    # Deduplicate by content prefix (some writings are duplicated)
    seen_prefixes = set()
    unique_ankys = []
    for a in ankys:
        prefix = a["content"][:200]
        if prefix not in seen_prefixes:
            seen_prefixes.add(prefix)
            unique_ankys.append(a)
    ankys = unique_ankys[:args.count]

    print(f"Generating {len(ankys)} stories from ankys...\n")
    system_prompt = get_system_prompt()

    # Find the wallet address to use as parent — use the one from the existing story
    parent_wallet = conn.execute(
        "SELECT parent_wallet_address FROM cuentacuentos LIMIT 1"
    ).fetchone()
    default_wallet = parent_wallet["parent_wallet_address"] if parent_wallet else "0x0000000000000000000000000000000000000000"

    # Map user_ids to wallet addresses
    def get_wallet_for_user(user_id: str) -> str:
        row = conn.execute(
            "SELECT wallet_address FROM users WHERE id = ?", (user_id,)
        ).fetchone()
        if row and row["wallet_address"]:
            return row["wallet_address"]
        return default_wallet

    generated = 0
    for i, anky in enumerate(ankys):
        writing_id = anky["id"]
        content = anky["content"]
        word_count = anky["word_count"]
        user_id = anky["user_id"]
        wallet = get_wallet_for_user(user_id)

        print(f"[{i+1}/{len(ankys)}] Writing {writing_id[:12]}... ({word_count} words)")
        print(f"  Preview: {content[:80]}...")

        # Generate story via Ollama
        user_msg = build_user_prompt(content)
        try:
            raw = call_ollama(system_prompt, user_msg, timeout=300)
            parsed = parse_story(raw)
        except Exception as e:
            print(f"  [ERROR] Generation failed: {e}")
            # Retry once
            try:
                print("  Retrying...")
                raw = call_ollama(system_prompt, user_msg, timeout=300)
                parsed = parse_story(raw)
            except Exception as e2:
                print(f"  [ERROR] Retry also failed: {e2}")
                continue

        title = parsed.get("title", "Cuentacuentos").strip()
        story_content = parsed.get("content", "").strip()
        chakra = parsed.get("chakra")
        kingdom = parsed.get("kingdom")
        city = parsed.get("city")

        if not story_content or len(story_content) < 100:
            print(f"  [SKIP] Story too short ({len(story_content)} chars)")
            continue

        paragraphs = story_paragraphs(story_content)
        guidance_phases = build_guidance_phases(story_content)
        story_id = str(uuid.uuid4())

        print(f"  Title: {title}")
        print(f"  Kingdom: {kingdom} / City: {city} / Chakra: {chakra}")
        print(f"  Content: {len(story_content)} chars, {len(paragraphs)} paragraphs")

        # Insert story
        conn.execute("""
            INSERT INTO cuentacuentos
            (id, writing_id, parent_wallet_address, title, content, guidance_phases,
             played, generated_at, chakra, kingdom, city)
            VALUES (?, ?, ?, ?, ?, ?, 0, datetime('now'), ?, ?, ?)
        """, (
            story_id, writing_id, wallet, title, story_content,
            guidance_phases, chakra, kingdom, city,
        ))

        # Insert image jobs
        if not args.skip_images:
            print(f"  Generating image prompts...")
            for idx, paragraph in enumerate(paragraphs):
                image_id = str(uuid.uuid4())
                image_prompt = generate_image_prompt(title, paragraph, kingdom)
                conn.execute("""
                    INSERT INTO cuentacuentos_images
                    (id, cuentacuentos_id, phase_index, image_prompt, status, attempts, created_at)
                    VALUES (?, ?, ?, ?, 'pending', 0, datetime('now'))
                """, (image_id, story_id, idx, image_prompt))
            print(f"  Queued {len(paragraphs)} image jobs")

        conn.commit()

        # Translate
        if not args.skip_translations:
            print(f"  Translating...")
            translations = translate_story(title, story_content)

            if translations:
                # Build enriched guidance phases with translations
                phases = json.loads(guidance_phases)
                for idx, phase in enumerate(phases):
                    for code, paragraphs_translated in translations.items():
                        key = f"narration_{code}"
                        text = paragraphs_translated[idx] if idx < len(paragraphs_translated) else ""
                        phase[key] = text

                enriched_json = json.dumps(phases)
                content_es = "\n\n".join(translations.get("es", []))
                content_zh = "\n\n".join(translations.get("zh", []))
                content_hi = "\n\n".join(translations.get("hi", []))
                content_ar = "\n\n".join(translations.get("ar", []))

                conn.execute("""
                    UPDATE cuentacuentos
                    SET content_es = ?, content_zh = ?, content_hi = ?, content_ar = ?,
                        guidance_phases = ?
                    WHERE id = ?
                """, (
                    content_es or None, content_zh or None,
                    content_hi or None, content_ar or None,
                    enriched_json, story_id,
                ))
                conn.commit()

        generated += 1
        print(f"  Done! Story {story_id[:8]}\n")

    conn.close()
    print(f"\n=== Generated {generated} stories ===")
    print("Image jobs are pending — the server's background loop will process them via ComfyUI.")

if __name__ == "__main__":
    main()
