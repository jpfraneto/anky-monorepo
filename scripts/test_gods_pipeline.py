#!/usr/bin/env python3
"""
Test GODS Pipeline - Verify all components work
"""

import os
import sys
from pathlib import Path

# Test 1: Check environment variables
print("=" * 70)
print("TEST 1: Environment Variables")
print("=" * 70)

from dotenv import load_dotenv
load_dotenv()

required_vars = [
    'GROK_API_KEY',
    'ELEVENLABS_API_KEY',
    'COMFYUI_URL'
]

for var in required_vars:
    value = os.getenv(var, '')
    if value and not value.startswith('your-'):
        print(f"✅ {var}: {value[:20]}...")
    else:
        print(f"❌ {var}: NOT SET or placeholder")

# Test 2: Check ComfyUI
print("\n" + "=" * 70)
print("TEST 2: ComfyUI Connection")
print("=" * 70)

import requests

comfy_url = os.getenv('COMFYUI_URL', 'http://127.0.0.1:8188')
try:
    response = requests.get(f"{comfy_url}/object_info/CLIPTextEncode", timeout=5)
    if response.status_code == 200:
        print(f"✅ ComfyUI running at {comfy_url}")
    else:
        print(f"❌ ComfyUI returned {response.status_code}")
except Exception as e:
    print(f"❌ ComfyUI not reachable: {e}")
    print("   Start it with: cd ~/ComfyUI && python main.py --listen 127.0.0.1 --port 8188")

# Test 3: Check output directories
print("\n" + "=" * 70)
print("TEST 3: Output Directories")
print("=" * 70)

output_dir = Path("~/anky/videos/gods").expanduser()
output_dir.mkdir(parents=True, exist_ok=True)
print(f"✅ Output directory: {output_dir}")

db_path = Path("~/anky/data/anky.db").expanduser()
db_path.parent.mkdir(parents=True, exist_ok=True)
print(f"✅ Database path: {db_path}")

# Test 4: Check dependencies
print("\n" + "=" * 70)
print("TEST 4: Python Dependencies")
print("=" * 70)

dependencies = [
    'requests',
    'PIL',
    'dotenv',
    'elevenlabs',
]

for dep in dependencies:
    try:
        if dep == 'PIL':
            import PIL
        else:
            __import__(dep)
        print(f"✅ {dep}")
    except ImportError:
        print(f"❌ {dep} - install with: pip install {dep}")

# Test 5: Check moviepy
print("\n" + "=" * 70)
print("TEST 5: MoviePy (Video Assembly)")
print("=" * 70)

try:
    from moviepy.editor import ImageClip
    print("✅ MoviePy available")
except ImportError:
    print("❌ MoviePy not installed")
    print("   Install: pip install moviepy")

# Test 6: Test Grok API (optional)
print("\n" + "=" * 70)
print("TEST 6: Grok API (Optional)")
print("=" * 70)

grok_key = os.getenv('GROK_API_KEY', '')
if grok_key and not grok_key.startswith('sk-'):
    print(f"⚠️  GROK_API_KEY might be placeholder")
    print("   Get key from: https://x.ai/api")
else:
    print("✅ Grok key set (not tested)")

# Summary
print("\n" + "=" * 70)
print("SUMMARY")
print("=" * 70)

print("""
Next steps:

1. Set all environment variables in ~/anky/.env
2. Start ComfyUI if not running
3. Install missing dependencies
4. Run the pipeline:

   cd ~/anky/scripts
   python gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia"

Or auto mode:

   python gods_pipeline.py --auto

Expected output:
- gods_full_Cronos.mp4 (8 minutes)
- gods_short_Cronos.mp4 (88 seconds)
- ~180 generated images

Cost: ~$0.35 (API only, images are free on local GPU)
Time: ~10-15 minutes
""")
