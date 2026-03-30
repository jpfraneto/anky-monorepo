#!/usr/bin/env python3
"""
Generate Anky image from real database reflection and post to Instagram as carousel.
"""
import sqlite3
import requests
import time
import json
import uuid
import subprocess
import sys
import os

# ============================================================
# Step 1: Fetch Real Data from Database
# ============================================================

print("Fetching real Anky data from database...")
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
    LIMIT 3
""")

rows = cursor.fetchall()
conn.close()

if len(rows) < 1:
    print("No ankys with reflections found in database")
    sys.exit(1)

print(f"Found {len(rows)} ankys for carousel")

# ============================================================
# Step 2: Generate Images via ComfyUI/Flux
# ============================================================

print("\nGenerating images via Flux/ComfyUI...")
comfyui_url = "http://127.0.0.1:8188"
generated_images = []

for idx, row in enumerate(rows):
    anky_id, kingdom_name, reflection, image_prompt = row
    print(f"\nGenerating image {idx+1}/{len(rows)} from {kingdom_name}...")
    
    # Use the actual image_prompt from the database
    prompt_text = f"anky, {image_prompt}"
    
    workflow = {
        "client_id": "anky-autonomous",
        "prompt": {
            "1": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "fp8_e4m3fn"}},
            "2": {"class_type": "VAELoader", "inputs": {"vae_name": "ae.safetensors"}},
            "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": "clip_l.safetensors", "clip_name2": "t5xxl_fp8_e4m3fn.safetensors", "type": "flux"}},
            "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": "anky_flux_lora_v2.safetensors", "strength_model": 0.85, "strength_clip": 0.85}},
            "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt_text}},
            "6": {"class_type": "EmptyLatentImage", "inputs": {"width": 1080, "height": 1080, "batch_size": 1}},
            "7": {"class_type": "KSampler", "inputs": {"seed": int(time.time() * 1000000) + idx, "steps": 20, "cfg": 3.5, "sampler_name": "euler", "scheduler": "normal", "denoise": 1, "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0], "latent_image": ["6", 0]}},
            "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
            "9": {"class_type": "PreviewImage", "inputs": {"images": ["8", 0]}}
        }
    }
    
    response = requests.post(f"{comfyui_url}/prompt", json=workflow)
    
    if response.status_code != 200:
        print(f"ComfyUI rejected prompt: {response.text}")
        continue
    
    prompt_id = response.json()["prompt_id"]
    print(f"Prompt queued: {prompt_id}")
    
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
                        
                        print(f"Image generated: {len(png_data):,} bytes")
                        break
                
                break
        
        except requests.RequestException as e:
            continue
    
    if image_url is None:
        print(f"ComfyUI generation timed out")
        continue
    
    # Convert to JPEG (Instagram requires JPEG for carousels)
    jpeg_data = None
    
    try:
        result = subprocess.run(
            ["ffmpeg", "-y", "-i", "/dev/stdin", "-f", "image2", "-"],
            input=png_data,
            capture_output=True,
            timeout=30
        )
        if result.returncode == 0:
            jpeg_data = result.stdout
            print(f"JPEG via ffmpeg: {len(jpeg_data):,} bytes")
    except (FileNotFoundError, subprocess.TimeoutExpired):
        print("JPEG conversion failed, using PNG")
        jpeg_data = png_data
    
    generated_images.append({
        "anky_id": anky_id,
        "kingdom_name": kingdom_name,
        "reflection": reflection,
        "image_data": jpeg_data
    })

if len(generated_images) < 1:
    print("No images generated successfully")
    sys.exit(1)

print(f"\nGenerated {len(generated_images)} images total")

# ============================================================
# Step 3: Create Carousel Caption
# ============================================================

print("\nCreating carousel caption...")

# Create compelling caption from reflections - NO hashtags or emojis
caption_parts = []
for idx, img_info in enumerate(generated_images, 1):
    reflection_preview = img_info["reflection"][:100].strip()
    if len(img_info["reflection"]) > 100:
        reflection_preview += "..."
    
    caption_parts.append(f"{idx}. {img_info['kingdom_name']}:\n   {reflection_preview}")

caption = "\n\n".join(caption_parts)
caption += "\n\nAnky reflections from the writing mirror"

print(f"Caption preview: {caption[:150]}...")

# ============================================================
# Step 4: Post Carousel to Instagram
# ============================================================

print("\nPosting carousel to Instagram...")

# Instagram credentials
INSTAGRAM_ACCESS_TOKEN = "EAAVWWRotLMMBQZBr6HoYsxke1XbZBlfEWG8vZCrFyPFZBqdHXdxHyYpecjqlc64hkp75wfPyCjT2biPHVmTmKZBBqECYHjqe4LnXPRg9WuYhc2LrEnQYiCm5tz1jnKeT4ZBnZAYn7Cbe5ZCe7nuF2kirZCm9DieKouNIQE7CB3C2ZCcy4Q40ZCHjNpuESGvW94IHMLAI0HUy4J8lsoy"
INSTAGRAM_USER_ID = "17841480674971908"

# Instagram Graph API endpoints
BASE_URL = "https://graph.instagram.com"

# Step 4a: Create media container for each image
media_ids = []

for idx, img_info in enumerate(generated_images):
    print(f"Uploading image {idx+1}/{len(generated_images)}...")
    
    # Save image to temp file
    import tempfile
    with tempfile.NamedTemporaryFile(suffix='.jpg', delete=False) as tmp_file:
        tmp_file.write(img_info["image_data"])
        tmp_path = tmp_file.name
    
    try:
        # Upload media to Instagram
        upload_url = f"{BASE_URL}/{INSTAGRAM_USER_ID}/media"
        
        params = {
            "access_token": INSTAGRAM_ACCESS_TOKEN,
            "media_type": "IMAGE",
            "caption": caption if idx == 0 else "",
            "is_carousel_child": "true"
        }
        
        files = {
            "file": open(tmp_path, 'rb')
        }
        
        response = requests.post(upload_url, params=params, files=files)
        
        files["file"].close()
        
        if response.status_code != 200:
            print(f"Upload failed: {response.text}")
            continue
        
        result = response.json()
        media_id = result.get("id")
        
        if media_id:
            media_ids.append(media_id)
            print(f"Media ID: {media_id[:20]}...")
    
    finally:
        import os
        if os.path.exists(tmp_path):
            os.remove(tmp_path)

if len(media_ids) < 1:
    print("No media uploaded successfully")
    sys.exit(1)

# Step 4b: Create carousel container
print("\nCreating carousel container...")

carousel_url = f"{BASE_URL}/{INSTAGRAM_USER_ID}/media"

payload = {
    "access_token": INSTAGRAM_ACCESS_TOKEN,
    "media_type": "CAROUSEL",
    "caption": caption,
    "children": ",".join(media_ids)
}

response = requests.post(carousel_url, json=payload)

if response.status_code != 200:
    print(f"Carousel creation failed: {response.text}")
    sys.exit(1)

carousel_result = response.json()
carousel_id = carousel_result.get("id")
print(f"Carousel ID: {carousel_id[:20]}...")

# Step 4c: Publish carousel
print("\nPublishing carousel...")

publish_url = f"{BASE_URL}/{INSTAGRAM_USER_ID}/media_publish"

publish_payload = {
    "access_token": INSTAGRAM_ACCESS_TOKEN,
    "creation_id": carousel_id
}

response = requests.post(publish_url, json=publish_payload)

if response.status_code != 200:
    print(f"Publish failed: {response.text}")
    sys.exit(1)

publish_result = response.json()
media_id = publish_result.get("id")

print()
print("="*60)
print("SUCCESS!")
print("="*60)
print(f"Instagram Media ID: {media_id}")
print(f"Carousel posted with {len(generated_images)} images")
print(f"Kingdoms: {', '.join([img['kingdom_name'] for img in generated_images])}")
