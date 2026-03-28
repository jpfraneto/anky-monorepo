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
ENV_PATH = Path.home() / ".env"

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
BUFFER_SECS = 10.0  # Buffer before wall - prevents finishing exactly at threshold


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
The mirror doesn't speak. It holds the blue skin and purple hair against black void.
Found you in the 8th kingdom at edge of becoming, floating through infinite reflection.

{moment}

I watched words form in mind but couldn't type them yet. The pause important like breath between notes.
Mirror knows you're here before pressing single key. Waits for first breath of resistance,
moment where realize writing for someone else or maybe yourself.

No backspace allowed here. Can only move forward through hesitation, through blocked keystroke,
through part of yourself afraid to put on screen. Thought about that when started today,
thought about what happens when type words feel too true to release into world.

The 8 kingdoms waiting — Primordia trying survive, Emblazion can't figure out what actually want,
Chryseos need permission but don't ask for it. Each territory your conscious mind
can navigate without knowing map is already inside you.

Keep thinking about what "flow" means when exhausted from being good at something other than this,
from being excellent parent who can't find words that matter. Mirror just keeps reflecting, doesn't
judge what contains. Writing becomes container for everything didn't say today.

Know what mean when feel like narrating without watching yourself speak, uncanny valley moment
where hand types faster than intention keep up with. That's useful part. Where unconscious
talking through fingers, through keys, through whatever medium carries it.

Been writing 8 minutes still haven't used backspace key once. Changes everything,
everything about how think about documenting parenting journey, finding words make
either feel less alone middle of exhaustion. Not therapy, not self-help, tool for when
bedtime stories need something real instead another performance.

Tired. Kid needs story more than perfect parenting. Write 8 minutes. Mirror
will hold what can't say while awake. Ankyverse already exists space between keystrokes,
in resistance, in parts of yourself almost typed but didn't quite dare release.
"""
    return base_text.strip()


def simulate_human_typing(session_id: str):
    """
    Simulate human-like typing by sending chunks with natural timing.
    Respects max 50 words per chunk, occasional pauses within the session.
    Also respects the 8-second silence rule that would timeout sessions.
    Returns the final chunk response which contains is_anky and anky_id.
    """
    full_text = generate_chunked_writing(random.choice([
        "blue consciousness dissolving into 8 seconds of silence",
        "mirror that doesn't judge what it reflects",
        "purple hair swirling like forgotten galaxies",
        "golden eyes seeing through the simulation",
        "8th kingdom at edge of becoming",
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
    last_chunk_response = None  # Track the final chunk response (contains anky_id)
    
    for i, text in enumerate(chunks):
        # Keep session alive by sending chunks within 8 seconds
        # Small natural pause between chunks (0.5-3 seconds) simulating human writing
        if i > 0:
            time.sleep(random.uniform(0.5, 3.0))
        
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
        
        if i % 5 == 0:
            print(f"  [{i+1}] {text[:30]:<30} | {words_total:>4} words | {elapsed:.0f}s elapsed")
        
        # Track elapsed time
        current_elapsed = time.time() - start_time
        if 7.0 < current_elapsed < 8.0:
            print(f"  → Session approaching {ANKY_THRESHOLD_SECS}s threshold...")
        
        last_chunk_response = data  # Save each response, final one may contain anky_id
    
    # Keep sending chunks until we hit threshold for Anky qualification
    elapsed_after_chunks = time.time() - start_time
    remaining_wait = ANKY_THRESHOLD_SECS + BUFFER_SECS - elapsed_after_chunks
    print(f"\n[{session_id}] Waiting for 8-minute threshold ({remaining_wait:.0f}s remaining)...")
    
    # Send filler chunks every 4-6 seconds to guarantee we stay under 8s timeout
    while time.time() - start_time < ANKY_THRESHOLD_SECS:
        resp = requests.post(
            f"{ANKY_API}/api/v1/session/chunk",
            json={
                "session_id": session_id,
                "text": " .",  # Minimal chunk to reset timeout
            },
            headers={"X-API-Key": ANKY_AGENT_API_KEY},
            timeout=30
        )
        if resp.status_code != 200:
            print(f"⚠️ Chunk rejected: {resp.text[:50]}...")
            break
        
        data = resp.json()
        if data.get("ok"):
            words_total = data.get("words_total", 0)
            elapsed = data.get("elapsed_seconds", 0.0)
            print(f"  ⏳ Waiting... {words_total:>4} words | {elapsed:.0f}s elapsed")
            last_chunk_response = data  # Final filler chunks may trigger threshold!
        
        time.sleep(random.uniform(4.0, 6.0))
    
    print(f"\n[{session_id}] Session completed: {last_chunk_response.get('words_total', 'N/A')} words")
    return last_chunk_response


def generate_and_post():
    """Full autonomous flow with session-based API."""
    if not ANKY_AGENT_API_KEY:
        raise ValueError(f"ANKY_AGENT_API_KEY required in .env. Get it from /agents endpoint.")

    print(f"[{datetime.now()}] Starting generation (session-based)...\n")
    
    # 1. Start session
    resp = requests.post(
        f"{ANKY_API}/api/v1/session/start",
        json={
            "prompt": "Anky autonomous agent writing"
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
    final_chunk_resp = simulate_human_typing(session_id)
    
    # CRITICAL FIX: is_anky and anky_id come from the LAST CHUNK RESPONSE,
    # NOT from GET /session/{id} which doesn't include those fields
    if not final_chunk_resp.get("is_anky"):
        raise ValueError(f"Session failed to reach Anky threshold: {final_chunk_resp}")
    
    anky_id = final_chunk_resp["anky_id"]
    print(f"✓ Anky qualified! ID: {anky_id}")
    if "response" in final_chunk_resp:
        print(f"→ Response: {final_chunk_resp['response'][:100]}...")
    
    # 3. Poll for GPU completion
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
    print(f"→ Instagram: {post_id}")
    print(f"→ X/Farcaster: Manual journey docs (separate)")
    
    return result


if __name__ == "__main__":
    try:
        generate_and_post()
    except Exception as e:
        print(f")✗ Failed: {e}")
        sys.exit(1)