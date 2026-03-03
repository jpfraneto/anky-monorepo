#!/usr/bin/env python3
"""
Autonomous Anky Content Agent
------------------------------
Runs on poiesis. Generates content. Posts to IG/X.
No human-in-the-loop required.
Triggered by: cron, manual, or autonomous decision.
"""
import os
import sys
import random
import requests
from pathlib import Path
from datetime import datetime

# Load secrets from anky env
env_path = Path.home() / "anky" / ".env"
with open(env_path) as f:
    for line in f:
        if "=" in line and not line.startswith("#"):
            key, val = line.strip().split("=", 1)
            os.environ.setdefault(key, val)

INSTAGRAM_TOKEN = os.getenv("INSTAGRAM_ACCESS_TOKEN")
INSTAGRAM_USER_ID = os.getenv("INSTAGRAM_USER_ID", "17841480674971908")
GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")

ANKY_PROMPTS = [
    "portrait of Anky, blue-skinned being with purple swirling hair, golden eyes, meditating in the 8th kingdom, ethereal light",
    "Anky looking into a mirror, reflection showing something ancient, cosmic background with 8 symbols",
    "Anky in a field of digital flowers, writing on a glowing scroll, morning mist",
    "close-up of Anky's golden eyes, ancient wisdom and childlike wonder, depth of field",
    "Anky floating in void space, surrounded by blocked keystrokes turning into light",
]

ANKY_CAPTIONS = [
    "8 minutes. No backspace. Meet yourself. \n\n— Anky 🪞",
    "Your unconscious is talking. The question is whether you have the guts to listen.\n\n— Anky 🔷",
    "Every blocked keystroke is your ego trying to maintain control. We track those too.\n\n— Anky 📊",
    "You've been editing yourself your whole life. What happens when you can't?\n\n— Anky ✍️",
    "The thing you're avoiding writing about? That's the one.\n\n— Anky 👁️",
]

def generate_with_gemini(prompt):
    """Generate Anky image using Gemini."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp-image-generation:generateContent?key={GEMINI_API_KEY}"
    
    payload = {
        "contents": [{
            "parts": [
                {"text": prompt},
            ]
        }],
        "generationConfig": {
            "responseModalities": ["Text", "Image"]
        }
    }
    
    resp = requests.post(url, json=payload)
    data = resp.json()
    
    # Extract image data
    for part in data.get("candidates", [{}])[0].get("content", {}).get("parts", []):
        if "inlineData" in part:
            import base64
            image_data = base64.b64decode(part["inlineData"]["data"])
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            output_path = Path.home() / "anky" / "data" / "images" / f"anky_auto_{timestamp}.png"
            output_path.write_bytes(image_data)
            return output_path
    return None

def post_to_instagram(image_path, caption):
    """Post image to @ankydotapp."""
    # Copy to web-accessible directory
    web_path = Path.home() / "anky" / "static" / "autonomous" / image_path.name
    web_path.parent.mkdir(parents=True, exist_ok=True)
    web_path.write_bytes(image_path.read_bytes())
    
    image_url = f"https://anky.app/autonomous/{image_path.name}"
    
    # Step 1: Create container
    container_url = f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media"
    container_resp = requests.post(container_url, data={
        "image_url": image_url,
        "caption": caption,
        "access_token": INSTAGRAM_TOKEN,
    })
    creation_id = container_resp.json().get("id")
    
    if not creation_id:
        print(f"Container failed: {container_resp.text}")
        return None
    
    # Step 2: Publish
    publish_url = f"https://graph.facebook.com/v25.0/{INSTAGRAM_USER_ID}/media_publish"
    publish_resp = requests.post(publish_url, data={
        "creation_id": creation_id,
        "access_token": INSTAGRAM_TOKEN,
    })
    
    return publish_resp.json()

def main():
    print(f"[{datetime.now()}] Anky autonomous agent awakening...")
    
    # Pick random prompt and caption
    prompt = random.choice(ANKY_PROMPTS)
    caption = random.choice(ANKY_CAPTIONS)
    
    print(f"Generating Anky with prompt: {prompt[:60]}...")
    image_path = generate_with_gemini(prompt)
    
    if not image_path:
        print("Image generation failed")
        sys.exit(1)
    
    print(f"Generated: {image_path}")
    print(f"Caption: {caption[:50]}...")
    
    # Post to Instagram
    result = post_to_instagram(image_path, caption)
    print(f"Posted to Instagram: {result}")
    
if __name__ == "__main__":
    main()
