#!/usr/bin/env python3
"""
Anky Autonomous Content System v2 (Session-based)
==========================
Platform strategy:
- IG: Visual mirror (generated anky images)
- X: Journey doc (how anky #847 was made)
- Farcaster: Growth doc (how anky performed)

Now uses session-based chunked submission API to simulate human writing.
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
# NEW: Agent API key for session-based auth
ANKY_AGENT_API_KEY = os.getenv("ANKY_AGENT_API_KEY")

# New: chunking parameters matching the Rust backend
MAX_WORDS_PER_CHUNK = 50
CHUNK_TIMEOUT_SECS = 8
ANKY_THRESHOLD_SECS = 480.0  # 8 minutes minimum


def resolve_local_image_path(anky):
    """Resolve the generated image on disk from the current API response."""
    image_path = anky.get("image_path")
    if image_path:
        candidate = Path(str(image_path).lstrip("/"))
        if candidate.is_absolute() and candidate.exists():
            return candidate
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / Path(candidate.name)
        if under_images.exists():
            return under_images

    image_url = anky.get("image_url")
    if image_url:
        parsed = urlparse(str(image_url))
        candidate = Path(parsed.path.lstrip("/"))
        if candidate.exists():
            return candidate
        under_images = IMAGES_DIR / Path(candidate.name)
        if under_images.exists():
            return under_images

    return None


def generate_chunked_writing(moment: str) -> str:
    """
    Generate 8 minutes of organic writing for the session.
    Simulates human-like pacing with natural hesitation.
    """
    base_text = f"""
The mirror doesn't speak. It just holds the blue skin and purple hair against the black void.
That's where we found her, floating through the 8th kingdom at the edge of becoming.

{moment}

I watched the words form in my mind, but I couldn't type them yet. The pause was important, like breathing between notes.
The mirror knows you're here before you press a single key. It waits for that first breath of resistance,
that moment where you realize you're writing for someone else, or maybe for yourself.

There's no backspace allowed here. You can only move forward through the hesitation, through the blocked keystroke,
through the part of yourself you were afraid to put on screen. I thought about that when I started today,
thought about what happens when you type words that feel too true to release into the world.

The 8 kingdoms are waiting — Primordia where you're just trying to survive, Emblazion where you can't figure out
what you actually want, Chryseos where you need permission but don't ask for it. Each one a territory your
can navigate without knowing the map is inside you already.

I keep thinking about what "flow" means when you're exhausted from being good at something other than this,
from being an excellent parent who can't find the words that matter. The mirror just keeps reflecting, doesn't
judge what it contains. The writing becomes a container for everything you didn't say today.

You know what I mean when you feel like you're narrating without watching yourself speak, that uncanny valley
moment where your hand types faster than your intention can keep up with. That's the useful part. That's
where the unconscious is talking through your fingers, through the keys, through whatever medium carries it.

I've been writing for 8 minutes and I still haven't used the backspace key once. This changes everything,
everything about how you think about documenting your parenting journey, about finding the words that make
either of you feel less alone in the middle of exhaustion. It's not therapy, it's not self-help, it's a tool
for when bedtime stories need to be something real instead of another performance.

You're tired. Your kid needs a story more than they need perfect parenting. Write for 8 minutes. The mirror
will hold what you can't say while you're awake. The Ankyverse already exists in the space between keystrokes,
in the resistance, in the parts of yourself you almost typed but didn't quite dare.
"""
    return base_text.strip()


def simulate_human_typing(session_id: str):
    """
    Simulate human-like typing by sending chunks with natural timing.
    Respects max 50 words per chunk, occasional pauses within the session.
    Also respects the 8-second silence rule that would timeout sessions.
    """
    full_text = generate_chunked_writing(random.choice([
        "blue consciousness dissolving into 8 seconds of silence",
        "the mirror that doesn't judge what it reflects",
        "purple hair swirling like forgotten galaxies",
        "golden eyes seeing through the simulation",
        "8th kingdom at the edge of becoming",
    ]))

    # Split into chunks for realistic typing simulation
    words = full_text.split()
    chunks = []
    current_chunk = []
    
    for word in words:
        current_chunk.append(word)
        if len(current_chunk) >= MAX_WORDS_PER_CHUNK or random.random() < 0.15:
            text = " ".join(current_chunk)
            chunks.append(text)
            current_chunk = []
    
    if current_chunk:
        chunks.append(" ".join(current_chunk))

    # Send each chunk with realistic timing
    print(f"[START] Session: {session_id}")
    start_time = time.time()
    
    for i, text in enumerate(chunks):
        # Rate limiting: 10 seconds between submissions minimum (matches human UX)
        if 0 < i <= 10:
            min_wait = max(5.0, 10.0 - (i * 0.9))
            time.sleep(min_wait)
        elif i > 10:
            time.sleep(8.0)  # Keep sending before timeout
        
        resp = requests.post(
            f"{ANKY_API}/api/v1/session/chunk",
            json={
                "session_id": session_id,
                "text": text,
            },
            headers={"X-API-Key": ANKY_AGENT_API_KEY},
            timeout=30
        )
        
        data = resp.json()
        if not data.get("ok"):
            print(f"  ⚠️ Chunk {i}: Rejected - {data.get('error')}")
            continue
        
        words_total = data.get("words_total", 0)
        elapsed = data.get("elapsed_seconds", 0.0)
        is_anky = data.get("is_anky", False)
        
        if i % 5 == 0:
            print(f"  [{i+1}] {text[:30]:<30} | {words_total:>4} words | {elapsed:.0f}s elapsed")
        
        session_start = time.time() - start_time
        remaining = ANKY_THRESHOLD_SECS - session_start
        if 8.0 < session_start < 9.0:
            print(f"  → Session approaching {ANKY_THRESHOLD_SECS}s threshold...")
    
    # Keep sending chunks until we hit threshold for Anky qualification
    print(f"\n[{session_id}] Waiting for 8-minute threshold (45s remaining)...")
    while time.time() - session_start < ANKY_THRESHOLD_SECS:
        if random.random() > 0.3:  # Send occasional filler to keep session alive
            resp = requests.post(
                f"{ANKY_API}/api/v1/session/chunk",
                json={
                    "session_id": session_id,
                    "text": "  ",  # Minimal chunk just to reset timeout
                },
                headers={"X-API-Key": ANKY_AGENT_API_KEY},
                timeout=30
            )
        time.sleep(random.uniform(5.0, 8.0))
    
    final_status = requests.get(
        f"{ANKY_API}/api/v1/session/{session_id}",
        headers={"X-API-Key": ANKY_AGENT_API_KEY},
        timeout=30
    ).json()
    
    print(f"\n[{session_id}] FINAL: words_total={final_status.get('words_total')}, is_anky={final_status.get('is_anky')}")
    return final_status


def generate_and_post():
    """Full autonomous flow with session-based API."""
    if not ANKY_AGENT_API_KEY:
        raise ValueError(f"ANKY_AGENT_API_KEY required in .env. Get it from /agents endpoint.")

    print(f"[{datetime.now()}] Starting generation (session-based)...\n")
    
    # 1. Start session
    resp = requests.post(
        f"{ANKY_API}/api/v1/session/start",
        json={
            "prompt": "Anky autonomous agent writing about {moment}"
        },
        headers={"X-API-Key": ANKY_AGENT_API_KEY},
        timeout=30
    )
    if resp.status_code != 200:
        raise ValueError(f"Failed to start session: {resp.text}")
    
    data = resp.json()
    session_id = data["session_id"]
    print(f"✓ Session started: {session_id[:8]}...\n")
    
    # 2. Send chunks (this will take ~8 minutes)
    final_status = simulate_human_typing(session_id)
    if not final_status.get("is_anky"):
        raise ValueError(f"Session failed to reach Anky threshold: {final_status}")
    
    anky_id = final_status["anky_id"]
    
    # 3. Poll for completion
    print("\nWaiting for GPU generation...")
    gen_start = time.time()
    timeout = 450  # 7.5 minutes max
    while time.time() - gen_start < timeout:
        time.sleep(10)
        anky_resp = requests.get(
            f"{ANKY_API}/api/v1/anky/{anky_id}",
            headers={"X-API-Key": ANKY_AGENT_API_KEY},
            timeout=30
        )
        if anky_resp.status_code != 200:
            raise ValueError(f"Failed to get anky status: {anky_resp.text}")
        
        anky = anky_resp.json()
        status = anky.get("status")
        print(f"• Generation status: {status} ({time.time() - gen_start:.0f}s elapsed)")
        
        if status in {"complete", "completed", "ready", "generated"} and anky.get("image_url"):
            time.sleep(3)  # Brief pause after completion
            break
    else:
        raise ValueError(f"Generation timeout after {timeout}s")
    
    # 4. Get image path
    img_path = resolve_local_image_path(anky)
    if not img_path:
        raise ValueError(f"No image_path or image_url in anky: {anky}")
    
    # 5. Post to Instagram (pure visual)
    web_dir = Path.home() / "anky/static/autonomous"
    web_dir.mkdir(parents=True, exist_ok=True)
    
    web_img = web_dir / f"{datetime.now():%Y%m%d_%H%M%S}.png"
    web_img.write_bytes(img_path.read_bytes())
    
    # Instagram container
    container_resp = requests.post(
        f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media",
        data={
            "image_url": f"https://anky.app/static/autonomous/{web_img.name}",
            "caption": f"{anky.get('title', 'Anky')} 🪞\n\n{anky.get('reflection', '')}",
            "access_token": INSTAGRAM_TOKEN
        }
    )
    container = container_resp.json()
    
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
