#!/usr/bin/env python3
"""
Test script for Anky session-based API (quick validation without full 8-min wait)
"""
import sys
import requests
from pathlib import Path

ANKY_API = "http://127.0.0.1:8889"
ENV_PATH = Path.home() / "anky" / ".env"

# Load env
if ENV_PATH.exists():
    with open(ENV_PATH) as f:
        for line in f:
            if "=" in line and not line.startswith("#"):
                k, v = line.strip().split("=", 1)
                import os
                os.environ.setdefault(k, v)

ANKY_AGENT_API_KEY = os.getenv("ANKY_AGENT_API_KEY")

if not ANKY_AGENT_API_KEY:
    print("ERROR: ANKY_AGENT_API_KEY required in .env")
    sys.exit(1)


def test_session_api():
    """Test the session-based API with minimal chunks."""
    print(f"Testing Anky session-based chunked submission API...\n")
    
    # Test 1: Start session
    print("[1] Starting session...")
    resp = requests.post(
        f"{ANKY_API}/api/v1/session/start",
        json={"prompt": "Anky autonomous agent writing test"},
        headers={"X-API-Key": ANKY_AGENT_API_KEY},
        timeout=30
    )
    if resp.status_code != 200:
        print(f"✗ Failed to start session: {resp.text}")
        return False
    
    data = resp.json()
    session_id = data["session_id"]
    max_words_per_chunk = data.get("max_words_per_chunk", 50)
    timeout_seconds = data.get("timeout_seconds", 8)
    print(f"✓ Session started: {session_id[:8]}...")
    print(f"  Parameters: max={max_words_per_chunk} words/chunk, timeout={timeout_seconds}s")
    
    # Test 2: Send chunks
    print("\n[2] Sending test chunks (simulating human typing)...")
    import time
    chunks = [
        "The mirror doesn't speak.",
        "It just holds the blue skin and purple hair against the black void.",
        "That's where we found her, floating through the 8th kingdom at the edge of becoming."
    ]
    
    for i, text in enumerate(chunks, 1):
        resp = requests.post(
            f"{ANKY_API}/api/v1/session/chunk",
            json={"session_id": session_id, "text": text},
            headers={"X-API-Key": ANKY_AGENT_API_KEY},
            timeout=30
        )
        
        if resp.status_code != 200:
            print(f"✗ Failed: {resp.text}")
            return False
        
        data = resp.json()
        ok = data.get("ok", False)
        words_total = data.get("words_total", 0)
        elapsed = data.get("elapsed_seconds", 0.0)
        
        status_icon = "✓" if ok else f"! [non-anky]"
        print(f"{status_icon} Chunk {i}: '{text[:25]:<25}' | {words_total} words total | {elapsed:.1f}s elapsed")
    
    # Test 3: Check session status
    print("\n[3] Checking session status...")
    resp = requests.get(
        f"{ANKY_API}/api/v1/session/{session_id}",
        headers={"X-API-Key": ANKY_AGENT_API_KEY},
        timeout=30
    )
    if resp.status_code != 200:
        print(f"✗ Failed to get session: {resp.text}")
        return False
    
    data = resp.json()
    is_anky = data.get("is_anky", False)
    words_total = data.get("words_total", 0)
    elapsed = data.get("elapsed_seconds", 0.0)
    
    if is_anky:
        print(f"✓ Session qualified as ANKY!")
        print(f"  Words: {words_total}, Elapsed: {elapsed:.1f}s")
    else:
        print(f"· Session not yet qualified (requires ~480s minimum)")
        print(f"  Words: {words_total}, Elapsed: {elapsed:.1f}s, Status: {'active' if data.get('is_active') else 'dead'}")
    
    # Test 4: Verify API key authentication
    print("\n[4] Testing API key authentication...")
    resp = requests.post(
        f"{ANKY_API}/api/v1/anys",
        headers={"X-API-Key": ANKY_AGENT_API_KEY},
        timeout=30
    )
    if resp.status_code == 401:
        print("✗ API key authentication failed")
        return False
    else:
        print("✓ API key authentication successful")
    
    return True


if __name__ == "__main__":
    success = test_session_api()
    if success:
        print("\n" + "="*60)
        print("Session-based chunked submission API test PASSED!")
        print("="*60)
        sys.exit(0)
    else:
        print("\n" + "="*60)
        print("Session-based chunked submission API test FAILED")
        print("="*60)
        sys.exit(1)
