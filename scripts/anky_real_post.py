#!/usr/bin/env python3
"""
Generate Anky image from real database reflection and post to social media.
"""
import sqlite3
import requests
import time
import json
import uuid
import boto3
import subprocess
import sys
import os

# ============================================================
# Step 1: Fetch Real Data from Database
# ============================================================

print("📊 Fetching real Anky data from database...")
db_path = os.path.expanduser('~/anky/data/anky.db')
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Get latest ankys with reflections and image prompts
cursor.execute("""
    SELECT id, kingdom_name, reflection, image_prompt 
    FROM ankys 
    WHERE kingdom_id IS NOT NULL 
      AND reflection IS NOT NULL 
      AND image_prompt IS NOT NULL
    ORDER BY created_at DESC 
    LIMIT 1
""")

row = cursor.fetchone()
conn.close()

if row is None:
    print("✗ No ankys with reflections found in database")
    sys.exit(1)

anky_id, kingdom_name, reflection, image_prompt = row

print(f"✓ Anky ID: {anky_id}")
print(f"✓ Kingdom: {kingdom_name}")
print(f"✓ Reflection preview: {reflection[:100]}...")
print(f"✓ Image prompt: {image_prompt[:100]}...")

# ============================================================
# Step 2: Generate Image via ComfyUI/Flux
# ============================================================

print("\n🎨 Generating image via Flux/ComfyUI...")
comfyui_url = "http://127.0.0.1:8188"

# Use the actual image_prompt from the database, prepend "anky, " for LoRA trigger
prompt_text = f"anky, {image_prompt}"

workflow = {
    "client_id": "anky-autonomous",
    "prompt": {
        "1": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "fp8_e4m3fn"}},
        "2": {"class_type": "VAELoader", "inputs": {"vae_name": "ae.safetensors"}},
        "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": "clip_l.safetensors", "clip_name2": "t5xxl_fp8_e4m3fn.safetensors", "type": "flux"}},
        "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": "anky_flux_lora_v2.safetensors", "strength_model": 0.85, "strength_clip": 0.85}},
        "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt_text}},
        "6": {"class_type": "EmptyLatentImage", "inputs": {"width": 1024, "height": 1024, "batch_size": 1}},
        "7": {"class_type": "KSampler", "inputs": {"seed": int(time.time() * 1000000), "steps": 20, "cfg": 3.5, "sampler_name": "euler", "scheduler": "normal", "denoise": 1, "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0], "latent_image": ["6", 0]}},
        "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
        "9": {"class_type": "PreviewImage", "inputs": {"images": ["8", 0]}}
    }
}

response = requests.post(f"{comfyui_url}/prompt", json=workflow)

if response.status_code != 200:
    print(f"✗ ComfyUI rejected prompt: {response.text}")
    sys.exit(1)

prompt_id = response.json()["prompt_id"]
print(f"✓ Prompt queued: {prompt_id}")

# Poll for completion
image_url = None
for attempt in range(120):
    time.sleep(2)
    
    try:
        history = requests.get(f"{comfyui_url}/history/{prompt_id}").json()
        
        if prompt_id in history:
            outputs = history[prompt_id].get("outputs", {})
            
            for node_id, output in outputs.items():
                if "images" in output and output["images"]:
                    filename = output["images"][0]["filename"]
                    subfolder = output["images"][0].get("subfolder", "")
                    type_ = output["images"][0].get("type", "output")
                    
                    if subfolder:
                        image_url = f"{comfyui_url}/view?filename={filename}&subfolder={subfolder}&type={type_}"
                    else:
                        image_url = f"{comfyui_url}/view?filename={filename}&type={type_}"
                    
                    image_response = requests.get(image_url)
                    image_response.raise_for_status()
                    png_data = image_response.content
                    
                    print(f"✓ Image generated: {len(png_data):,} bytes")
                    break
            
            break
    
    except requests.RequestException as e:
        continue

if image_url is None:
    print(f"✗ ComfyUI generation timed out after 240s")
    sys.exit(1)

# ============================================================
# Step 3: Convert to WebP & Upload to R2
# ============================================================

print("\n☁️  Converting to WebP and uploading to R2...")

# Convert to WebP
webp_data = None

try:
    result = subprocess.run(
        ["cwebp", "-q", "85"],
        input=png_data,
        capture_output=True,
        timeout=30
    )
    if result.returncode == 0:
        webp_data = result.stdout
        print(f"✓ WebP via cwebp: {len(webp_data):,} bytes ({100-len(webp_data)/len(png_data)*100:.1f}% compression)")
except (FileNotFoundError, subprocess.TimeoutExpired):
    pass

if webp_data is None:
    try:
        result = subprocess.run(
            ["ffmpeg", "-y", "-f", "pngpipe", "-i", "/dev/stdin", 
             "-f", "webp", "-qscale:v", "15", "-"],
            input=png_data,
            capture_output=True,
            timeout=30
        )
        if result.returncode == 0:
            webp_data = result.stdout
            print(f"✓ WebP via ffmpeg: {len(webp_data):,} bytes")
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

if webp_data is None:
    print("⚠️  WebP conversion failed, using PNG")
    webp_data = png_data

# Upload to R2 (credentials from environment)
r2_account_id = os.environ.get("R2_ACCOUNT_ID", "")
r2_bucket = os.environ.get("R2_BUCKET_NAME", "anky")
r2_access_key = os.environ.get("R2_ACCESS_KEY_ID", "")
r2_secret_key = os.environ.get("R2_SECRET_ACCESS_KEY", "")
r2_public_url = os.environ.get("R2_PUBLIC_URL", "https://storage.anky.app")

if not r2_access_key or not r2_secret_key or not r2_account_id:
    print("✗ Missing R2 credentials. Set R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY env vars.")
    sys.exit(1)

s3_client = boto3.client(
    's3',
    aws_access_key_id=r2_access_key,
    aws_secret_access_key=r2_secret_key,
    endpoint_url=f'https://{r2_account_id}.r2.cloudflarestorage.com'
)

object_key = f"stories/{anky_id}/page-0.webp"

s3_client.put_object(
    Bucket=r2_bucket,
    Key=object_key,
    Body=webp_data,
    ContentType='image/webp',
    CacheControl='public, max-age=31536000, immutable'
)

cdn_url = f"{r2_public_url}/{object_key}"
print(f"✓ Uploaded to R2")
print(f"✓ CDN URL: {cdn_url}")

# ============================================================
# Step 4: Create Post Text from Reflection
# ============================================================

print("\nCreating post text from reflection...")

# Extract key insight from reflection
reflection_preview = reflection[:180].strip()
if len(reflection) > 180:
    reflection_preview += "..."

# Create compelling post text - NO hashtags or emojis
post_text = f"{reflection_preview}\n\nAnky from {kingdom_name}"

print(f"Post text: {post_text[:100]}...")

# ============================================================
# Step 5: Post to X/Twitter
# ============================================================

print("\n📡 Posting to X...")

try:
    from requests_oauthlib import OAuth1
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "requests_oauthlib", "-q"])
    from requests_oauthlib import OAuth1

x_consumer_key = os.environ.get("X_CONSUMER_KEY", "")
x_consumer_secret = os.environ.get("X_CONSUMER_SECRET", "")
x_access_token = os.environ.get("X_ACCESS_TOKEN", "")
x_access_token_secret = os.environ.get("X_ACCESS_TOKEN_SECRET", "")

if not all([x_consumer_key, x_consumer_secret, x_access_token, x_access_token_secret]):
    print("✗ Missing X/Twitter credentials. Set X_CONSUMER_KEY, X_CONSUMER_SECRET, X_ACCESS_TOKEN, X_ACCESS_TOKEN_SECRET env vars.")
    sys.exit(1)

auth = OAuth1(x_consumer_key, x_consumer_secret, x_access_token, x_access_token_secret)

# Download image with proper headers
req = requests.Request(
    'GET',
    cdn_url,
    headers={
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
        'Accept': 'image/webp,image/*,*/*;q=0.8',
        'Referer': 'https://anky.app/'
    }
)
prepared = req.prepare()
session = requests.Session()
image_response = session.send(prepared)
image_response.raise_for_status()
image_data = image_response.content

# Upload media
upload_url = "https://upload.twitter.com/1.1/media/upload.json"
files = {"media": image_data}

upload_response = requests.post(upload_url, auth=auth, files=files)
upload_response.raise_for_status()
media_id = upload_response.json()["media_id_string"]
print(f"✓ Media ID: {media_id}")

# Post tweet
tweet_url = "https://api.twitter.com/2/tweets"
payload = {"text": post_text, "media": {"media_ids": [media_id]}}

response = requests.post(tweet_url, auth=auth, json=payload)
response.raise_for_status()

result = response.json()
tweet_id = result.get('data', {}).get('id')

print()
print("="*60)
print("✅ SUCCESS!")
print("="*60)
print(f"Anky ID: {anky_id}")
print(f"Kingdom: {kingdom_name}")
print(f"Tweet: https://x.com/ankydotapp/status/{tweet_id}")
print(f"Image: {cdn_url}")
