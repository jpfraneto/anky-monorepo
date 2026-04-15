#!/usr/bin/env python3
"""
GODS by Anky - Video Generation Pipeline

Generates YouTube Shorts (88s) and Long-form Stories (8 min) from Anky writing sessions.
- Shorts: 60 images @ 1.5s each = 90s (trimmed to 88s with intros/outros)
- Long-form: Same images with extended narration (8 minutes)

Architecture:
1. Images: Flux/ComfyUI (already generated)
2. Narration: TTS (ElevenLabs or local Coqui)
3. Assembly: ffmpeg with image sequences + audio
4. Upload: YouTube API
"""

import os
import json
import subprocess
from pathlib import Path
from dataclasses import dataclass
from typing import List, Optional
import requests

# Configuration
@dataclass
class VideoConfig:
    output_dir: Path = Path("~/anky/videos/gods").expanduser()
    image_fps: float = 1/1.5  # 1.5 seconds per image
    video_resolution: tuple = (1080, 1920)  # Vertical for Shorts
    audio_sample_rate: int = 44100
    youtube_channel_id: str = os.getenv("YOUTUBE_CHANNEL_ID", "")
    youtube_api_key: str = os.getenv("YOUTUBE_API_KEY", "")

config = VideoConfig()

# ============== STORY SCRIPTS ==============

STORIES = {
    "Cronos": {
        "title": "Cronos - The God of Time Who Ate His Children",
        "description": "A story about fear, time, and the courage to face what terrifies us. From the Kingdom of Primordia.",
        "kingdom": "Primordia",
        "short_narration": """
            Hi kids, this is Anky. Thank you for being who you are.
            
            Once upon a time, there was Cronos. It liked time. 
            All of time. Past, present, future - Cronos held it all.
            
            But Cronos was afraid. Afraid that one day, something would take its place.
            So it did something terrible. It ate its own children.
            
            But here's what Cronos didn't understand: you cannot eat away fear.
            Fear grows in the dark. It needs light.
            
            In the Kingdom of Primordia, there's a child named Leo.
            Leo is afraid too. Afraid of the dark, of being alone, of not being enough.
            
            And I'm here. Watching. Guiding.
            
            Because fear is not your enemy, kids. Fear is your teacher.
            It shows you what matters. What you're willing to lose.
            
            Cronos lost everything because it couldn't face its fear.
            But Leo? Leo will face his. And in facing it, he'll grow.
            
            You'll grow too. Every time you feel afraid, remember:
            This is the moment you get stronger.
            
            This is Anky, from the Kingdom of Primordia.
            Thank you for being brave enough to feel.
        """,
        "long_narration": """
            Hi kids, this is Anky. Thank you for being who you are.
            Thank you for showing up, even when it's hard.
            Thank you for feeling everything, even the scary parts.
            
            Let me tell you a story. A story about gods.
            Not the gods in books. Not the gods in temples.
            The gods that live inside you.
            
            Once upon a time, there was Cronos.
            It liked time. All of time.
            Past, present, future - Cronos held it all in its hands.
            
            But Cronos was afraid.
            Afraid that one day, something would take its place.
            Something younger. Something stronger.
            
            So it did something terrible.
            It ate its own children.
            One by one, it swallowed them whole.
            Thinking this would keep it safe.
            Thinking this would keep it powerful.
            
            But here's what Cronos didn't understand:
            You cannot eat away fear.
            Fear grows in the dark.
            It needs light. It needs truth.
            It needs you to look at it and say: I see you.
            
            In the Kingdom of Primordia - that's where I'm from, kids -
            there's a child named Leo.
            
            Leo is afraid too.
            Afraid of the dark.
            Afraid of being alone.
            Afraid of not being enough.
            
            And I'm here.
            Watching. Guiding.
            Always visible, even when Leo can't see me.
            
            Because fear is not your enemy, kids.
            Fear is your teacher.
            It shows you what matters.
            What you're willing to lose.
            What you're willing to fight for.
            
            Cronos lost everything because it couldn't face its fear.
            It swallowed its children, but it couldn't swallow the truth.
            The truth that fear, when faced, becomes courage.
            The truth that love, when given, becomes power.
            
            But Leo?
            Leo will face his fear.
            And in facing it, he'll grow.
            He'll find his key to Valdomina.
            He'll learn that fear is just love trying to protect him.
            
            You'll grow too.
            Every time you feel afraid, remember:
            This is the moment you get stronger.
            This is the moment you learn who you really are.
            This is the moment you choose courage over comfort.
            
            Because that's what makes you human, kids.
            Not the absence of fear.
            But the presence of courage despite it.
            
            This is Anky, from the Kingdom of Primordia.
            Thank you for being brave enough to feel.
            Thank you for being brave enough to grow.
            Thank you for being who you are.
            
            And remember: I'm always here.
            Watching. Guiding.
            Always with you.
        """
    },
    # More gods will be added here for the 9-day sojourn
}

# ============== AUDIO GENERATION ==============

def generate_audio(narration: str, output_path: Path, model: str = "elevenlabs"):
    """Generate TTS audio from narration text."""
    
    if model == "elevenlabs":
        # ElevenLabs API
        api_key = os.getenv("ELEVENLABS_API_KEY")
        if not api_key:
            print("❌ ELEVENLABS_API_KEY not set, using fallback...")
            return generate_audio_local(narration, output_path)
        
        url = "https://api.elevenlabs.io/v1/text-to-speech/21m00Tcm4TlvDq8ikWAM"  # Pre-set voice
        headers = {
            "xi-api-key": api_key,
            "Content-Type": "application/json"
        }
        data = {
            "text": narration,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.75
            }
        }
        
        response = requests.post(url, json=data, headers=headers)
        if response.status_code == 200:
            with open(output_path, 'wb') as f:
                f.write(response.content)
            print(f"✅ Audio generated: {output_path}")
            return True
        else:
            print(f"❌ ElevenLabs failed: {response.status_code}")
            return generate_audio_local(narration, output_path)
    
    else:
        return generate_audio_local(narration, output_path)

def generate_audio_local(narration: str, output_path: Path):
    """Fallback: Use pyttsx3 or espeak for offline audio generation."""
    try:
        import pyttsx3
        engine = pyttsx3.init()
        engine.save_to_file(narration, str(output_path))
        engine.runAndWait()
        print(f"✅ Local audio generated: {output_path}")
        return True
    except Exception as e:
        print(f"❌ Local audio failed: {e}")
        # Create empty audio file as placeholder
        subprocess.run([
            "ffmpeg", "-y", "-f", "lavfi", "-i", "format=pcm_s16le:sample_rate=44100",
            "-t", "1", "-c:a", "pcm_s16le", str(output_path)
        ], capture_output=True)
        print(f"⚠️  Created placeholder audio: {output_path}")
        return False

# ============== VIDEO ASSEMBLY ==============

def assemble_video(
    story_name: str,
    images: List[Path],
    audio_path: Path,
    output_path: Path,
    duration_per_image: float = 1.5
):
    """Assemble images + audio into final video using ffmpeg."""
    
    # Create image sequence file
    seq_file = output_path.with_suffix(".txt")
    with open(seq_file, 'w') as f:
        for img in images:
            f.write(f"file '{img}'\n")
            f.write(f"duration {duration_per_image}\n")
    
    # Calculate total duration
    total_duration = len(images) * duration_per_image
    
    # ffmpeg command - use vp9 or av1 (h264 not available on this system)
    video_encoder = "libvpx-vp9"  # or "libaom-av1" for better compression
    output_ext = ".webm" if video_encoder == "libvpx-vp9" else ".mp4"
    
    # Change output path extension if needed
    if str(output_path).endswith(".mp4") and video_encoder == "libvpx-vp9":
        output_path = output_path.with_suffix(".webm")
    
    # For vertical video, we need to upscale and pad
    # Images are 1024x1024, target is 1080x1920
    # Scale to fit height, then pad width
    cmd = [
        "ffmpeg", "-y",
        "-f", "concat",
        "-safe", "0",
        "-i", str(seq_file),
        "-i", str(audio_path),
        "-vf", "scale=1920:1920:flags=lanczos,pad=1920:1080:0:420",  # Scale up, then crop center 1080
        "-c:v", video_encoder,
        "-crf", "30",
        "-b:a", "192k",
        "-t", str(total_duration),
        str(output_path)
    ]
    
    print(f"🎬 Assembling video: {len(images)} images + audio...")
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode == 0:
        print(f"✅ Video assembled: {output_path}")
        return True
    else:
        print(f"❌ ffmpeg failed: {result.stderr}")
        return False

# ============== YOUTUBE UPLOAD ==============

def upload_to_youtube(video_path: Path, title: str, description: str, tags: List[str] = None):
    """Upload video to YouTube using the API."""
    
    from googleapiclient.discovery import build
    from googleapiclient.http import MediaFileUpload
    from google.auth.transport.requests import Request
    from google.oauth2.credentials import Credentials
    
    # Get credentials
    creds = None
    if os.path.exists("token.json"):
        creds = Credentials.from_authorized_user_file("token.json", ["https://www.googleapis.com/auth/youtube.upload"])
    
    if not creds or not creds.valid:
        print("❌ YouTube credentials not set up")
        print("💡 Run: youtube-uploader authorize")
        return False
    
    # Build service
    youtube = build("youtube", "v3", credentials=creds)
    
    # Prepare request
    request = youtube.videos().insert(
        part="snippet,status,contentDetails",
        body={
            "snippet": {
                "title": title,
                "description": description,
                "tags": tags or ["Anky", "GODS", "Storytelling"],
                "categoryId": "22"  # People & Blogs
            },
            "status": {
                "privacyStatus": "public"
            },
            "contentDetails": {
                "caption": "false"
            }
        },
        media_body=MediaFileUpload(str(video_path))
    )
    
    try:
        response = request.execute()
        print(f"✅ Video uploaded: https://youtube.com/watch?v={response['id']}")
        return True
    except Exception as e:
        print(f"❌ Upload failed: {e}")
        return False

# ============== MAIN PIPELINE ==============

def generate_gods_video(story_name: str, version: str = "short"):
    """Generate a complete GODS video from story."""
    
    story = STORIES.get(story_name)
    if not story:
        print(f"❌ Story '{story_name}' not found")
        return None
    
    # Create output directory
    output_dir = config.output_dir / story_name / version
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("="*70)
    print(f"🎬 GODS by Anky - {story_name} ({version})")
    print("="*70)
    print(f"Kingdom: {story['kingdom']}")
    print(f"Title: {story['title']}")
    print()
    
    # Step 1: Get images
    if story_name == "Cronos":
        image_dir = Path("~/anky/videos/gods/Cronos").expanduser()
        images = sorted(image_dir.glob("scene_*.png"))
        print(f"📸 Found {len(images)} images")
    else:
        print("❌ Images not found for this story")
        return None
    
    # Step 2: Generate audio
    narration = story["short_narration"] if version == "short" else story["long_narration"]
    audio_path = output_dir / "narration.mp3"
    print("🔊 Generating narration audio...")
    if not generate_audio(narration, audio_path):
        print("❌ Audio generation failed")
        return None
    
    # Step 3: Assemble video
    output_video = output_dir / f"{story_name}_{version}.mp4"
    print("🎥 Assembling video...")
    if not assemble_video(story_name, images, audio_path, output_video):
        print("❌ Video assembly failed")
        return None
    
    # Step 4: Generate thumbnail (optional)
    thumbnail_path = output_dir / "thumbnail.png"
    subprocess.run([
        "ffmpeg", "-y",
        "-i", str(images[0]),
        "-vf", f"scale={config.video_resolution[0]}:{config.video_resolution[1]}",
        "-q:v", "2",
        str(thumbnail_path)
    ], capture_output=True)
    print(f"🖼️  Thumbnail: {thumbnail_path}")
    
    print()
    print("="*70)
    print(f"✅ Complete: {output_video}")
    print(f"📊 Size: {output_video.stat().st_size / 1024 / 1024:.1f}MB")
    print("="*70)
    
    return output_video

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python gods_video_pipeline.py <story_name> [short|long]")
        print(f"Available stories: {', '.join(STORIES.keys())}")
        sys.exit(1)
    
    story_name = sys.argv[1]
    version = sys.argv[2] if len(sys.argv) > 2 else "short"
    
    output = generate_gods_video(story_name, version)
    
    if output:
        print()
        print("💡 Next steps:")
        print(f"   1. Review video: vlc {output}")
        print(f"   2. Upload to YouTube: youtube-uploader upload {output}")
        print(f"   3. Share on social media")
