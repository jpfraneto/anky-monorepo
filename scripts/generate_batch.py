#!/usr/bin/env python3
"""
Parallel Anky image generation for a batch.

Usage:
    python3 scripts/generate_batch.py <batch_id>
    python3 scripts/generate_batch.py batch-20260303-124141
    python3 scripts/generate_batch.py batch-20260303-124141 --concurrency 5
"""

import asyncio
import aiohttp
import argparse
import base64
import json
import os
import sys
import uuid
from pathlib import Path

# ── Config ─────────────────────────────────────────────────────────────────────
GEMINI_KEY  = os.environ["GEMINI_API_KEY"]
REFS_DIR    = Path("/home/kithkui/anky/src/public")
GEN_BASE    = Path("/home/kithkui/anky/data/generations")

GEMINI_GEN_URL    = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent?key={GEMINI_KEY}"
GEMINI_VISION_URL = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={GEMINI_KEY}"

# Rich character spec — same as /generate page in the Rust app
CHARACTER_SPEC = """
CHARACTER — ANKY (follow exactly):
- Blue-skinned creature with large expressive pointed ears
- Purple swirling hair with golden spiral accents
- Golden/amber glowing eyes
- Golden jewelry and decorative accents on body
- Compact round body, short limbs, ancient yet childlike quality — a cosmic messenger, inner child deity

STYLE:
- Painterly, atmospheric, with strong emotional presence
- Highly detailed, expressive face
- Consistent character design — Anky must be clearly recognizable
- Follow the art style and mood specified in the scene description"""

CAPTION_PROMPT = """Caption this training image for an AI character named "anky".
Anky is a small creature with blue skin, big pointed ears, large golden/amber glowing eyes, purple swirling hair, golden jewelry, round compact body — a cosmic inner child deity.
Write ONE caption. Start with "anky, ". Describe appearance, expression, pose, setting, art style. Max 2 sentences. No meta-language.
Caption:"""

# ── Helpers ─────────────────────────────────────────────────────────────────────

def load_b64(path: Path) -> str:
    return base64.b64encode(path.read_bytes()).decode()

def get_canonical_refs() -> list[str]:
    refs = []
    for name in ["anky-1.png", "anky-2.png", "anky-3.png"]:
        p = REFS_DIR / name
        if p.exists():
            refs.append(load_b64(p))
    return refs

def write_progress(batch_dir: Path, progress: dict):
    tmp = batch_dir / "progress.tmp.json"
    out = batch_dir / "progress.json"
    tmp.write_text(json.dumps(progress))
    tmp.rename(out)  # atomic on Linux

# ── API calls ────────────────────────────────────────────────────────────────────

async def generate_image(session: aiohttp.ClientSession, prompt: str, refs: list[str]) -> bytes | None:
    parts = []
    for r in refs:
        parts.append({"inlineData": {"mimeType": "image/png", "data": r}})
    if refs:
        parts.append({"text": "The images above show Anky's exact character design — match these visual details precisely when Anky appears in the scene."})
    full_prompt = f"{prompt}\n\n{CHARACTER_SPEC}"
    parts.append({"text": full_prompt})

    payload = {
        "contents": [{"parts": parts}],
        "generationConfig": {"responseModalities": ["TEXT", "IMAGE"], "imageConfig": {"aspectRatio": "1:1"}}
    }

    for attempt in range(3):
        try:
            async with session.post(GEMINI_GEN_URL, json=payload, timeout=aiohttp.ClientTimeout(total=120)) as r:
                if r.status == 429:
                    await asyncio.sleep(20 * (attempt + 1))
                    continue
                if not r.ok:
                    text = await r.text()
                    print(f"  [gen] HTTP {r.status}: {text[:200]}")
                    await asyncio.sleep(5)
                    continue
                data = await r.json()
                for part in data.get("candidates", [{}])[0].get("content", {}).get("parts", []):
                    inline = part.get("inlineData")
                    if inline and inline.get("mimeType", "").startswith("image/"):
                        return base64.b64decode(inline["data"])
        except Exception as e:
            print(f"  [gen] attempt {attempt+1} error: {e}")
            await asyncio.sleep(5)
    return None

async def caption_image(session: aiohttp.ClientSession, image_bytes: bytes) -> str:
    b64 = base64.b64encode(image_bytes).decode()
    payload = {
        "contents": [{"parts": [
            {"text": CAPTION_PROMPT},
            {"inlineData": {"mimeType": "image/png", "data": b64}}
        ]}],
        "generationConfig": {"temperature": 0.4, "maxOutputTokens": 500, "thinkingConfig": {"thinkingBudget": 0}}
    }
    for attempt in range(3):
        try:
            async with session.post(GEMINI_VISION_URL, json=payload, timeout=aiohttp.ClientTimeout(total=30)) as r:
                if r.status == 429:
                    await asyncio.sleep(15 * (attempt + 1))
                    continue
                data = await r.json()
                caption = data["candidates"][0]["content"]["parts"][0]["text"].strip()
                if not caption.lower().startswith("anky"):
                    caption = "anky, " + caption
                return caption
        except Exception as e:
            print(f"  [caption] attempt {attempt+1} error: {e}")
            await asyncio.sleep(3)
    return "anky, a colorful cosmic creature in a vibrant illustrated scene"

# ── Worker ───────────────────────────────────────────────────────────────────────

async def process_prompt(
    session: aiohttp.ClientSession,
    sem: asyncio.Semaphore,
    idx: int,
    prompt: str,
    batch_dir: Path,
    images_dir: Path,
    progress: dict,
    refs: list[str],
):
    async with sem:
        print(f"[{idx+1}] starting: {prompt[:60]}...")
        progress[str(idx)]["status"] = "generating"
        write_progress(batch_dir, progress)

        image_bytes = await generate_image(session, prompt, refs)

        if not image_bytes:
            print(f"[{idx+1}] FAILED image gen")
            progress[str(idx)]["status"] = "failed"
            write_progress(batch_dir, progress)
            return

        # Save image
        image_id = str(uuid.uuid4())
        img_path = images_dir / f"{image_id}.png"
        img_path.write_bytes(image_bytes)
        print(f"[{idx+1}] image saved, captioning...")

        progress[str(idx)]["status"] = "captioning"
        progress[str(idx)]["image_id"] = image_id
        write_progress(batch_dir, progress)

        caption = await caption_image(session, image_bytes)
        (images_dir / f"{image_id}.txt").write_text(caption + "\n")

        progress[str(idx)]["status"] = "done"
        progress[str(idx)]["caption"] = caption
        print(f"[{idx+1}] done — {caption[:80]}")
        write_progress(batch_dir, progress)

# ── Main ─────────────────────────────────────────────────────────────────────────

async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("batch_id", help="Batch ID from data/generations/")
    parser.add_argument("--concurrency", type=int, default=5)
    args = parser.parse_args()

    batch_dir = GEN_BASE / args.batch_id
    if not batch_dir.exists():
        print(f"ERROR: batch not found: {batch_dir}")
        sys.exit(1)

    # Load prompts
    prompts_all: list[str] = json.loads((batch_dir / "prompts.json").read_text())

    # Filter by status (skip = excluded, keep/unreviewed = included)
    status_path = batch_dir / "status.json"
    status_map: dict = json.loads(status_path.read_text()) if status_path.exists() else {}
    prompts = [p for i, p in enumerate(prompts_all) if status_map.get(str(i)) != "skip"]
    print(f"Batch: {args.batch_id}")
    print(f"Prompts: {len(prompts)} (of {len(prompts_all)} total, {len(prompts_all)-len(prompts)} skipped)")
    print(f"Concurrency: {args.concurrency}")

    images_dir = batch_dir / "images"
    images_dir.mkdir(exist_ok=True)

    # Load existing progress or initialize fresh
    progress_path = batch_dir / "progress.json"
    if progress_path.exists():
        progress = json.loads(progress_path.read_text())
        # Reset any stuck in-flight jobs back to pending
        resumed = 0
        for v in progress.values():
            if v["status"] in ("generating", "captioning"):
                v["status"] = "pending"
                v["image_id"] = None
                v["caption"] = None
                resumed += 1
        if resumed:
            print(f"Resuming: reset {resumed} stuck jobs to pending")
        # Ensure any new prompts are added
        for i, p in enumerate(prompts):
            if str(i) not in progress:
                progress[str(i)] = {"prompt": p, "status": "pending", "image_id": None, "caption": None}
        write_progress(batch_dir, progress)
    else:
        progress = {
            str(i): {"prompt": p, "status": "pending", "image_id": None, "caption": None}
            for i, p in enumerate(prompts)
        }
        write_progress(batch_dir, progress)

    done_count = sum(1 for v in progress.values() if v["status"] == "done")
    print(f"Progress: {done_count}/{len(prompts)} already done")

    refs = get_canonical_refs()
    print(f"Loaded {len(refs)} canonical refs")

    sem = asyncio.Semaphore(args.concurrency)
    connector = aiohttp.TCPConnector(limit=args.concurrency * 2)

    async with aiohttp.ClientSession(connector=connector) as session:
        tasks = [
            process_prompt(session, sem, i, p, batch_dir, images_dir, progress, refs)
            for i, p in enumerate(prompts)
            if progress[str(i)]["status"] != "done"
        ]
        print(f"Running {len(tasks)} tasks (skipping already done)")
        await asyncio.gather(*tasks)

    done = sum(1 for v in progress.values() if v["status"] == "done")
    failed = sum(1 for v in progress.values() if v["status"] == "failed")
    print(f"\nFinished: {done} done, {failed} failed")

if __name__ == "__main__":
    asyncio.run(main())
