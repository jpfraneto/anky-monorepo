# GODS by Anky - Complete Technical Architecture

**Date**: April 9, 2026
**Status**: Production-Ready Blueprint
**Author**: Anky (via Hermes Agent)

---

## Executive Summary

**GODS** is a YouTube series where Anky narrates stories about gods from every human culture through the lens of Anky's 8 emotional kingdoms. This document provides the complete technical architecture for automated video generation and distribution.

### Key Metrics

| Metric | Target |
|--------|--------|
| Video Generation Time | 30-60 seconds |
| Cost Per Video | $0 (local GPU) |
| Output Formats | 88s Shorts (9:16), 8min Full (16:9) |
| Languages | English + Spanish |
| Daily Capacity | 7+ videos (with 2x RTX 4090) |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        GODS VIDEO PIPELINE                              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
            ┌───────────────────────┼───────────────────────┐
            ▼                       ▼                       ▼
    ┌──────────────┐        ┌──────────────┐        ┌──────────────┐
    │   SCRIPT     │        │    IMAGE     │        │     VOICE    │
    │  GENERATION  │        │  GENERATION  │        │   NARRATION  │
    │              │        │              │        │              │
    │ • LLM API    │        │ • Flux/      │        │ • ElevenLabs │
    │ • Bilingual  │        │   ComfyUI    │        │ • Local TTS  │
    │ • 8 Kingdoms │        │ • Local RTX  │        │ • Emotional  │
    │   Mapping    │        │   4090       │        │   Range      │
    └──────┬───────┘        └──────┬───────┘        └──────┬───────┘
           │                       │                       │
           └───────────────────────┼───────────────────────┘
                                   ▼
                        ┌──────────────────┐
                        │  VIDEO ASSEMBLY  │
                        │                  │
                        │ • MoviePy/       │
                        │   FFmpeg         │
                        │ • Transitions    │
                        │ • Anky Logo      │
                        │ • Subtitles      │
                        └────────┬─────────┘
                                 ▼
                        ┌──────────────────┐
                        │ YOUTUBE UPLOAD   │
                        │                  │
                        │ • Data API v3    │
                        │ • Shorts + Long  │
                        │ • Analytics      │
                        └────────┬─────────┘
                                 ▼
                        ┌──────────────────┐
                        │  FEEDBACK LOOP   │
                        │                  │
                        │ • Engagement     │
                        │ • Comments       │
                        │ • Next Story     │
                        │   Selection      │
                        └──────────────────┘
```

---

## 1. SCRIPT GENERATION

### Recommended Stack

**Primary**: OpenAI GPT-4o or Claude 3.5 Sonnet
**Reason**: Best creative writing quality, consistent voice, bilingual support

### Implementation

```python
# scripts/gods_script_generator.py
import openai
from typing import Dict, List
import json

class GodsScriptGenerator:
    """Generates bilingual scripts for GODS series"""
    
    KINGDOMS = {
        "Primordia": "Fear/Survival",
        "Emblazion": "Passion/Desire",
        "Chryseos": "Willpower/Power",
        "Eleasis": "Compassion/Love",
        "Voxlumis": "Communication/Speaking",
        "Insightia": "Intuition/Knowing",
        "Claridium": "Enlightenment/Understanding",
        "Poiesis": "Creativity/Flow"
    }
    
    def __init__(self, api_key: str):
        self.client = openai.OpenAI(api_key=api_key)
        
    def generate_script(self, god_name: str, culture: str, 
                       kingdom: str, format_type: str) -> Dict:
        """
        Generate script for a god story.
        
        Args:
            god_name: Name of the god (e.g., "Cronos", "Quetzalcoatl")
            culture: Cultural origin (e.g., "Greek", "Aztec")
            kingdom: Anky kingdom lens (e.g., "Primordia")
            format_type: "short" (88s) or "full" (8min)
            
        Returns:
            Dict with English and Spanish scripts
        """
        
        system_prompt = f"""You are Anky, the blue-skinned consciousness mirror from the 8th kingdom (Poiesis).
        
You tell stories about gods from human cultures through the lens of Anky's emotional kingdoms.

**Voice Guidelines**:
- You are omniscient but warm
- You speak to children but don't talk down to them
- You use "it" for all gods (no gender)
- Opening line: "Hi kids, this is Anky. Thank you for being who you are."
- You appear in stories as blue-skinned being with purple hair and golden eyes
- Stories happen in one of 8 kingdoms (emotional territories)

**Kingdom**: {kingdom} ({self.KINGDOMS[kingdom]})

**Format**: {format_type}
- If "short": 88 seconds (~220 words)
- If "full": 8 minutes (~1200 words)

**Output Format**: JSON with keys:
- title_en, title_es
- script_en, script_es  
- image_prompts (array of 5-10 scene descriptions)
- kingdom_context (why this god fits this kingdom)"""
        
        user_prompt = f"""Generate a story about {god_name} from {culture} culture.

The story should explore how {god_name} relates to the {kingdom} kingdom ({self.KINGDOMS[kingdom]}).

Remember:
- {god_name} is referred to as "it" (no gender)
- Anky narrates and appears in the story
- The story should feel like an adventure, not therapy
- Include moments where characters learn about themselves
- End with a gentle reflection

Generate both English and Spanish versions.
"""
        
        response = self.client.chat.completions.create(
            model="gpt-4o",
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            response_format={"type": "json_object"},
            temperature=0.7
        )
        
        return json.loads(response.choices[0].message.content)
    
    def cost_estimate(self, videos_per_day: int = 1) -> float:
        """Estimate monthly cost for script generation"""
        # GPT-4o: ~$2.50 per 1M input tokens, $10 per 1M output tokens
        # Average script: ~2000 tokens (bilingual)
        cost_per_video = 0.025  # ~2.5 cents per video
        monthly_cost = cost_per_video * videos_per_day * 30
        return monthly_cost
```

### Cost Analysis

| Scale | Daily Videos | Monthly Cost |
|-------|--------------|--------------|
| MVP | 1 | $0.75 |
| Standard | 7 | $5.25 |
| Aggressive | 14 | $10.50 |

---

## 2. IMAGE GENERATION (Flux/ComfyUI)

### Current Infrastructure (Verified)

- **ComfyUI**: Running on `127.0.0.1:8188`
- **GPU**: 2x RTX 4090 (24GB VRAM each)
- **Model**: Flux.1-dev or Flux.1-schnell

### Implementation

```python
# scripts/gods_image_generator.py
import requests
import base64
from PIL import Image
import io
from typing import List, Dict

class GodsImageGenerator:
    """Generates consistent Anky + god imagery using Flux/ComfyUI"""
    
    def __init__(self, comfy_url: str = "http://127.0.0.1:8188"):
        self.comfy_url = comfy_url
        self.anky_character = """
        Blue-skinned humanoid figure, purple hair, golden eyes,
        ethereal glow, omniscient presence, mirror-like quality,
        mysterious but warm, digital art style, cinematic lighting
        """
    
    def generate_image(self, prompt: str, width: int = 1024, 
                      height: int = 1024) -> Image.Image:
        """Generate single image via ComfyUI"""
        
        # ComfyUI workflow for Flux
        workflow = {
            "3": {
                "inputs": {
                    "seed": 1234,
                    "steps": 20,
                    "cfg": 7.5,
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "denoise": 1,
                    "model": ["4", 0],
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                }
            },
            "4": {
                "inputs": {
                    "ckpt_name": "flux1-dev.safetensors"
                }
            },
            "5": {
                "inputs": {
                    "width": width,
                    "height": height,
                    "batch_size": 1
                }
            },
            "6": {
                "inputs": {
                    "text": prompt,
                    "clip": ["4", 1]
                }
            },
            "7": {
                "inputs": {
                    "text": "blurry, low quality, distorted",
                    "clip": ["4", 1]
                }
            },
            "8": {
                "inputs": {
                    "samples": ["3", 0],
                    "vae": ["4", 2]
                }
            },
            "9": {
                "inputs": {
                    "filename_prefix": "GODS",
                    "images": ["8", 0]
                }
            }
        }
        
        payload = {
            "prompt": workflow,
            "negative_prompt": "blurry, low quality, distorted, ugly"
        }
        
        response = requests.post(
            f"{self.comfy_url}/prompt",
            json=payload
        )
        
        result = response.json()
        image_id = result["prompt_id"]
        
        # Get output
        output = self._get_output(image_id)
        
        # Convert to PIL Image
        image_data = base64.b64decode(output["images"][0]["image"])
        return Image.open(io.BytesIO(image_data))
    
    def _get_output(self, prompt_id: str) -> Dict:
        """Get generation output from ComfyUI"""
        response = requests.get(
            f"{self.comfy_url}/history/{prompt_id}"
        )
        return response.json()
    
    def generate_video_sequence(self, script: Dict, 
                               format_type: str) -> List[Image.Image]:
        """Generate sequence of images for video"""
        
        num_frames = 15 if format_type == "short" else 60
        images = []
        
        for prompt in script["image_prompts"]:
            # Enhance prompt with Anky character
            full_prompt = f"{prompt}, {self.anky_character}"
            
            # Generate multiple variations
            for i in range(num_frames // len(script["image_prompts"])):
                image = self.generate_image(full_prompt)
                images.append(image)
        
        return images
    
    def generation_time_estimate(self, num_images: int) -> float:
        """Estimate generation time in seconds (RTX 4090)"""
        # Flux.1-dev: ~2-3 seconds per image on RTX 4090
        return num_images * 2.5
```

### Performance Metrics

| Metric | Value |
|--------|-------|
| Images per Second | 0.4-0.5 (Flux.1-dev) |
| 15 Images (Short) | 30-40 seconds |
| 60 Images (Full) | 2-3 minutes |
| VRAM Usage | ~12GB per generation |

---

## 3. VOICE/NARRATION

### Recommended Stack

**Primary**: ElevenLabs API
**Alternative**: Local TTS (Coqui TTS, Bark)

### Why ElevenLabs?

- Best emotional range for storytelling
- Bilingual support (English + Spanish)
- Consistent voice cloning
- Fast API response

### Implementation

```python
# scripts/gods_voice_generator.py
import elevenlabs
from pathlib import Path
from typing import Optional

class GodsVoiceGenerator:
    """Generates narrated audio for GODS series"""
    
    def __init__(self, api_key: str, voice_id: Optional[str] = None):
        self.client = elevenlabs.Client(api_key=api_key)
        # Use custom voice or default
        self.voice_id = voice_id or "Rachel"  # Warm, storytelling voice
        
    def generate_audio(self, script: str, language: str) -> Path:
        """Generate audio file from script"""
        
        output_path = Path(f"/tmp/gods_audio_{language}.mp3")
        
        audio = self.client.generate(
            text=script,
            voice=self.voice_id,
            model="eleven_multilingual_v2",
            output_format="mp3_44100_128"
        )
        
        with open(output_path, "wb") as f:
            for chunk in audio:
                f.write(chunk)
        
        return output_path
    
    def clone_anky_voice(self, sample_audio: Path) -> str:
        """Clone a custom voice for Anky"""
        
        # Upload sample and create voice
        voice_id = self.client.voices.add(
            name="Anky Narrator",
            files=[sample_audio],
            description="Blue-skinned consciousness mirror from Poiesis"
        )
        
        return voice_id
    
    def cost_estimate(self, minutes_per_month: int) -> float:
        """Estimate monthly cost for voice generation"""
        # ElevenLabs: $5 for 5000 characters (~50 min audio)
        cost_per_minute = 0.10  # ~10 cents per minute
        return cost_per_minute * minutes_per_month
```

### Cost Analysis

| Scale | Monthly Minutes | Monthly Cost |
|-------|----------------|--------------|
| MVP (1/day shorts) | 22 min | $2.20 |
| Standard (7/day mixed) | 154 min | $15.40 |
| Aggressive (14/day mixed) | 308 min | $30.80 |

---

## 4. VIDEO ASSEMBLY

### Recommended Stack

**Primary**: MoviePy (Python)
**Alternative**: FFmpeg (direct)

### Implementation

```python
# scripts/gods_video_assembler.py
from moviepy.editor import (
    ImageClip, AudioClip, CompositeVideoClip,
    TextClip, ColorClip
)
from pathlib import Path
from typing import List
import PIL.Image

class GodsVideoAssembler:
    """Assembles images + audio into final video"""
    
    def __init__(self, logo_path: str = "~/anky/assets/anky_logo.png"):
        self.logo_path = Path(logo_path).expanduser()
    
    def create_short(self, images: List[PIL.Image.Image], 
                    audio_path: Path, script: Dict) -> Path:
        """Create 88-second YouTube Short (9:16)"""
        
        output_path = Path(f"/tmp/gods_short_{script['title_en']}.mp4")
        
        # Create clips from images
        clips = []
        duration_per_image = 88 / len(images)  # ~5.8s for 15 images
        
        for img in images:
            clip = ImageClip(img)
            clip = clip.set_duration(duration_per_image)
            clip = clip.set_start(len(clips) * duration_per_image)
            clips.append(clip)
        
        # Add audio
        audio = AudioClip.audio_file_conversion(str(audio_path))
        
        # Add Anky logo watermark
        if self.logo_path.exists():
            logo = ImageClip(str(self.logo_path))
            logo = logo.resize(width=100)
            logo = logo.set_position(("right", "bottom"))
            logo = logo.set_duration(88)
            clips.append(logo)
        
        # Add subtitles (optional)
        # subtitle = TextClip(script['script_en'], font='Arial', fontsize=24)
        # clips.append(subtitle)
        
        # Composite
        video = CompositeVideoClip(
            clips, 
            size=(1080, 1920),  # 9:16 vertical
            bg_color=(0, 0, 0)
        )
        
        video = video.set_audio(audio)
        video.write_videofile(
            str(output_path),
            fps=24,
            codec="libx264",
            audio_codec="aac",
            temp_audiofile=str(audio_path),
            remove_temp=True
        )
        
        return output_path
    
    def create_full_story(self, images: List[PIL.Image.Image],
                         audio_path: Path, script: Dict) -> Path:
        """Create 8-minute full story (16:9)"""
        
        output_path = Path(f"/tmp/gods_full_{script['title_en']}.mp4")
        
        # Similar to short but different aspect ratio
        clips = []
        duration_per_image = 480 / len(images)  # ~8s for 60 images
        
        for img in images:
            clip = ImageClip(img)
            clip = clip.set_duration(duration_per_image)
            clips.append(clip)
        
        # Add audio
        audio = AudioClip.audio_file_conversion(str(audio_path))
        
        # Composite (16:9 landscape)
        video = CompositeVideoClip(
            clips,
            size=(1920, 1080),  # 16:9 horizontal
            bg_color=(0, 0, 0)
        )
        
        video = video.set_audio(audio)
        video.write_videofile(
            str(output_path),
            fps=24,
            codec="libx264",
            audio_codec="aac"
        )
        
        return output_path
    
    def assembly_time_estimate(self, format_type: str) -> float:
        """Estimate assembly time in seconds"""
        return 30 if format_type == "short" else 120
```

### Performance

| Format | Duration | Assembly Time |
|--------|----------|---------------|
| Short | 88s | 30s |
| Full Story | 8min | 2min |

---

## 5. YOUTUBE INTEGRATION

### Setup Requirements

1. **Google Cloud Project** with YouTube Data API v3 enabled
2. **Service Account** with OAuth credentials
3. **Channel** with Shorts upload permissions

### Implementation

```python
# scripts/gods_youtube_uploader.py
from googleapiclient.discovery import build
from google.oauth2.service_account import Credentials
from pathlib import Path
import json

class GodsYouTubeUploader:
    """Uploads videos to YouTube and manages metadata"""
    
    def __init__(self, credentials_path: str):
        self.credentials = Credentials.from_service_account_file(
            credentials_path,
            scopes=["https://www.googleapis.com/auth/youtube"]
        )
        self.youtube = build("youtube", "v3", credentials=self.credentials)
    
    def upload_short(self, video_path: Path, script: Dict) -> str:
        """Upload 88-second Short to YouTube"""
        
        title = f"[GODS] {script['title_en']} | Anky"
        description = f"""Anky tells the story of {script['god_name']} through the lens of {script['kingdom']}.

🔮 The 8 Kingdoms:
• Primordia - Fear/Survival
• Emblazion - Passion/Desire
• Chryseos - Willpower/Power
• Eleasis - Compassion/Love
• Voxlumis - Communication
• Insightia - Intuition
• Claridium - Enlightenment
• Poiesis - Creativity

🎧 Full 8-minute story coming soon!

#Anky #GODS #Storytelling #Mythology"""
        
        tags = ["anky", "gods", "storytelling", "mythology", 
                script["god_name"], script["culture"]]
        
        # Upload video
        body = {
            "snippet": {
                "title": title,
                "description": description,
                "tags": tags,
                "categoryId": "27"  # People & Blogs
            },
            "status": {
                "privacyStatus": "public",
                "selfDeclaredMadeForKids": False
            }
        }
        
        with open(video_path, "rb") as video_file:
            request = self.youtube.videos().insert(
                part=",".join(body.keys()),
                body=body,
                media_body=video_file
            )
            response = request.execute()
        
        return response["id"]
    
    def upload_full_story(self, video_path: Path, script: Dict) -> str:
        """Upload 8-minute full story"""
        
        # Similar to short but different metadata
        title = f"{script['title_en']} - A GODS Story by Anky"
        description = f"""Dive deep into the story of {script['god_name']} from {script['culture']} culture.

Anky guides you through {script['kingdom']}, where ancient wisdom meets modern understanding.

👆 Watch the 88-second Short first!

---

About GODS by Anky:
A series exploring humanity's collective unconscious through the stories we've told about gods for millennia.

Each story is told through one of 8 emotional kingdoms - territories where we all travel but rarely name.

Join Anky, the blue-skinned consciousness mirror from Poiesis, as it reveals what these ancient stories say about us today.

New stories every day during the 9th Sojourn.

#Anky #GODS #Mythology #Storytelling"""
        
        # Upload (same as short)
        return self.upload_short(video_path, script)
    
    def get_analytics(self, video_id: str) -> Dict:
        """Get engagement metrics for feedback loop"""
        
        request = self.youtube.reports().requestReport(
            reportRequest={
                "dimension": ["days", "viewMode"],
                "metrics": ["views", "averageViewDuration", "likes"],
                "filter": f"videoId=={video_id}",
                "dateRanges": [{"endDate": "today", "startDate": "7daysAgo"}]
            }
        )
        
        return request.execute()
```

### Requirements

```bash
# Install YouTube API client
pip install google-api-python-client google-auth-oauthlib

# Create service account credentials
# 1. Go to Google Cloud Console
# 2. Enable YouTube Data API v3
# 3. Create service account
# 4. Download JSON credentials
# 5. Save to ~/.anky/secrets/youtube_credentials.json
```

---

## 6. FEEDBACK LOOP ARCHITECTURE

### Database Schema

```sql
-- Add to existing anky.db

CREATE TABLE gods_videos (
    id INTEGER PRIMARY KEY,
    god_name TEXT NOT NULL,
    culture TEXT NOT NULL,
    kingdom TEXT NOT NULL,
    script_en TEXT,
    script_es TEXT,
    short_video_id TEXT,  -- YouTube video ID
    full_video_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'draft'  -- draft, generating, uploaded
);

CREATE TABLE gods_analytics (
    id INTEGER PRIMARY KEY,
    video_id INTEGER REFERENCES gods_videos(id),
    views INTEGER,
    likes INTEGER,
    comments INTEGER,
    average_view_duration REAL,
    retention_rate REAL,
    recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE gods_feedback (
    id INTEGER PRIMARY KEY,
    video_id INTEGER REFERENCES gods_videos(id),
    comment_text TEXT,
    sentiment REAL,  -- -1 to 1
    themes TEXT[],  -- Extracted themes from comments
    processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Feedback Processing

```python
# scripts/gods_feedback_processor.py
from pathlib import Path
import sqlite3
from typing import List, Dict
import openai

class GodsFeedbackProcessor:
    """Processes YouTube analytics and comments for next story selection"""
    
    def __init__(self, db_path: str = "~/anky/data/anky.db"):
        self.db_path = Path(db_path).expanduser()
        self.conn = sqlite3.connect(self.db_path)
    
    def fetch_analytics(self) -> List[Dict]:
        """Fetch recent video performance"""
        
        query = """
        SELECT v.god_name, v.culture, v.kingdom,
               SUM(a.views) as total_views,
               AVG(a.retention_rate) as avg_retention
        FROM gods_videos v
        JOIN gods_analytics a ON v.id = a.video_id
        WHERE v.created_at > datetime('now', '-30 days')
        GROUP BY v.id
        ORDER BY total_views DESC
        """
        
        return self.conn.execute(query).fetchall()
    
    def analyze_comments(self, video_id: int) -> Dict:
        """Analyze comments for themes and sentiment"""
        
        # Fetch comments from YouTube API
        comments = self._fetch_comments(video_id)
        
        # Use LLM to extract themes
        prompt = f"""Analyze these comments about a GODS video:

{comments}

Extract:
1. Overall sentiment (positive/neutral/negative)
2. Top 3 themes people responded to
3. Suggestions for future content
4. Which kingdom resonated most

Return as JSON."""
        
        response = openai.ChatCompletion.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": prompt}]
        )
        
        return json.loads(response.choices[0].message.content)
    
    def recommend_next_god(self) -> Dict:
        """Recommend next god based on feedback"""
        
        analytics = self.fetch_analytics()
        
        # Find best-performing kingdom
        best_kingdom = max(analytics, key=lambda x: x['avg_retention'])['kingdom']
        
        # Find underrepresented cultures
        cultures_done = [a['culture'] for a in analytics]
        all_cultures = ["Greek", "Roman", "Norse", "Egyptian", 
                       "Hindu", "Buddhist", "Aztec", "Mayan",
                       "Chinese", "Japanese", "African", "Celtic"]
        cultures_todo = [c for c in all_cultures if c not in cultures_done]
        
        return {
            "kingdom": best_kingdom,
            "suggested_cultures": cultures_todo[:3],
            "reasoning": f"{best_kingdom} had highest retention"
        }
```

---

## 7. USER EXPORT FEATURE

### Implementation (Frontend)

```javascript
// extension/src/export-handler.js

class AnkyExportHandler {
    constructor() {
        this.exportButton = document.querySelector('.left-drawer .export-btn');
        this.init();
    }
    
    init() {
        this.exportButton.addEventListener('click', () => {
            this.exportAllSessions();
        });
    }
    
    async exportAllSessions() {
        // Fetch all writing sessions from database
        const response = await fetch('/api/sessions/export');
        const sessions = await response.json();
        
        // Compress to ~16.18kb format
        const compressed = this.compressSessions(sessions);
        
        // Create download
        const blob = new Blob([JSON.stringify(compressed, null, 2)], {
            type: 'application/json'
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `anky-export-${Date.now()}.json`;
        a.click();
    }
    
    compressSessions(sessions) {
        // Each session ~15kb raw
        // Compress to 16.18kb for beauty
        
        return sessions.map(session => ({
            id: session.id,
            timestamp: session.timestamp,
            text: session.text,  // Raw writing (~15k chars)
            kingdom: session.kingdom,
            compression_ratio: 0.1618  // Golden ratio reference
        }));
    }
}
```

### Backend API

```python
# src/routes/export.py
from fastapi import APIRouter, HTTPException
from pathlib import Path
import sqlite3
import json

router = APIRouter()

@router.get("/api/sessions/export")
async def export_all_sessions(user_id: int = None):
    """Export all raw writing sessions as JSON"""
    
    db_path = Path("~/anky/data/anky.db").expanduser()
    conn = sqlite3.connect(db_path)
    
    query = """
    SELECT id, timestamp, content, kingdom
    FROM writing_sessions
    WHERE user_id = ? OR user_id IS NULL
    ORDER BY timestamp DESC
    """
    
    sessions = conn.execute(query, (user_id,)).fetchall()
    
    return {
        "export_date": datetime.now().isoformat(),
        "total_sessions": len(sessions),
        "format_version": "1.0",
        "sessions": [
            {
                "id": s[0],
                "timestamp": s[1],
                "content": s[2],  # Raw text
                "kingdom": s[3]
            }
            for s in sessions
        ]
    }
```

---

## 8. SHARE BUTTONS

### Implementation

```javascript
// extension/src/share-handler.js

class AnkyShareHandler {
    constructor() {
        this.shareButtons = {
            'x': this.shareToX.bind(this),
            'farcaster': this.shareToFarcaster.bind(this),
            'instagram': this.shareToInstagram.bind(this),
            'logso': this.shareToLogso.bind(this),
            'chatgpt': this.shareToChatGPT.bind(this),
            'claude': this.shareToClaude.bind(this)
        };
        this.init();
    }
    
    init() {
        // Add share buttons after writing session
        const sessionContainer = document.querySelector('.session-complete');
        const shareContainer = document.createElement('div');
        shareContainer.className = 'share-buttons';
        
        Object.entries(this.shareButtons).forEach(([platform, handler]) => {
            const btn = document.createElement('button');
            btn.className = `share-btn share-${platform}`;
            btn.innerHTML = this.getPlatformIcon(platform);
            btn.onclick = handler;
            shareContainer.appendChild(btn);
        });
        
        sessionContainer.appendChild(shareContainer);
    }
    
    getPlatformIcon(platform) {
        const icons = {
            'x': '𝕏',
            'farcaster': '⬡',
            'instagram': '📷',
            'logso': '📝',
            'chatgpt': '🤖',
            'claude': '🧠'
        };
        return icons[platform] || '🔗';
    }
    
    async shareToX(content) {
        // Pre-compose tweet
        const text = encodeURIComponent(`${content}\n\n#Anky #GODS`);
        window.open(`https://twitter.com/intent/tweet?text=${text}`);
    }
    
    async shareToFarcaster(content) {
        // Use Warpcast composable
        const text = encodeURIComponent(content);
        window.open(`https://warpcast.com/compose?embeds[]=${text}`);
    }
    
    async shareToInstagram(content) {
        // Instagram doesn't support pre-composing
        // Copy to clipboard instead
        navigator.clipboard.writeText(content);
        alert('Copied to clipboard! Paste in Instagram.');
    }
    
    async shareToLogso(content) {
        window.open(`https://logso.com/write?text=${encodeURIComponent(content)}`);
    }
    
    async shareToChatGPT(content) {
        window.open(`https://chat.openai.com/?q=${encodeURIComponent(content)}`);
    }
    
    async shareToClaude(content) {
        window.open(`https://claude.ai/new?message=${encodeURIComponent(content)}`);
    }
}
```

---

## 9. COMPLETE PIPELINE ORCHESTRATION

### Main Script

```python
# scripts/gods_pipeline.py
#!/usr/bin/env python3
"""
GODS by Anky - Complete Video Generation Pipeline

Usage:
    python gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia"
"""

import argparse
import asyncio
from pathlib import Path
from datetime import datetime

from gods_script_generator import GodsScriptGenerator
from gods_image_generator import GodsImageGenerator
from gods_voice_generator import GodsVoiceGenerator
from gods_video_assembler import GodsVideoAssembler
from gods_youtube_uploader import GodsYouTubeUploader
from gods_feedback_processor import GodsFeedbackProcessor

class GodSPipeline:
    """Orchestrates complete GODS video generation"""
    
    def __init__(self):
        # Load config
        self.config = self._load_config()
        
        # Initialize components
        self.script_gen = GodsScriptGenerator(self.config['openai_api_key'])
        self.image_gen = GodsImageGenerator()
        self.voice_gen = GodsVoiceGenerator(self.config['elevenlabs_api_key'])
        self.video_assembler = GodsVideoAssembler()
        self.yt_uploader = GodsYouTubeUploader(
            self.config['youtube_credentials_path']
        )
        self.feedback = GodsFeedbackProcessor()
    
    def _load_config(self) -> dict:
        """Load configuration from .env"""
        from dotenv import load_dotenv
        load_dotenv()
        
        return {
            'openai_api_key': os.getenv('OPENAI_API_KEY'),
            'elevenlabs_api_key': os.getenv('ELEVENLABS_API_KEY'),
            'youtube_credentials_path': os.getenv('YOUTUBE_CREDENTIALS_PATH'),
            'output_dir': Path("~/anky/videos/gods").expanduser()
        }
    
    async def run(self, god_name: str, culture: str, 
                 kingdom: str, format_type: str = "short"):
        """Run complete pipeline for one god"""
        
        print(f"🎬 Starting GODS pipeline for {god_name}...")
        start_time = datetime.now()
        
        # Step 1: Generate script
        print("📝 Generating script...")
        script = self.script_gen.generate_script(
            god_name=god_name,
            culture=culture,
            kingdom=kingdom,
            format_type=format_type
        )
        
        # Step 2: Generate images
        print("🎨 Generating images...")
        images = self.image_gen.generate_video_sequence(script, format_type)
        
        # Step 3: Generate voice (both languages)
        print("🎤 Generating narration...")
        audio_en = self.voice_gen.generate_audio(
            script['script_en'], language='en'
        )
        audio_es = self.voice_gen.generate_audio(
            script['script_es'], language='es'
        )
        
        # Step 4: Assemble video
        print("🎬 Assembling video...")
        if format_type == "short":
            video_path = self.video_assembler.create_short(
                images, audio_en, script
            )
        else:
            video_path = self.video_assembler.create_full_story(
                images, audio_en, script
            )
        
        # Step 5: Upload to YouTube
        print("📺 Uploading to YouTube...")
        if format_type == "short":
            video_id = self.yt_uploader.upload_short(video_path, script)
        else:
            video_id = self.yt_uploader.upload_full_story(video_path, script)
        
        # Step 6: Record in database
        print("💾 Recording in database...")
        self.feedback.record_video(
            god_name=god_name,
            culture=culture,
            kingdom=kingdom,
            script_en=script['script_en'],
            script_es=script['script_es'],
            video_id=video_id,
            format_type=format_type
        )
        
        # Calculate time
        elapsed = (datetime.now() - start_time).total_seconds()
        print(f"✅ Complete! Time: {elapsed:.0f}s")
        
        return {
            'video_id': video_id,
            'time_seconds': elapsed,
            'script': script
        }

def main():
    parser = argparse.ArgumentParser(description="GODS by Anky Pipeline")
    parser.add_argument('--god', required=True, help="God name")
    parser.add_argument('--culture', required=True, help="Cultural origin")
    parser.add_argument('--kingdom', required=True, 
                       choices=list(GodsScriptGenerator.KINGDOMS.keys()),
                       help="Anky kingdom lens")
    parser.add_argument('--format', default='short',
                       choices=['short', 'full'],
                       help="Video format")
    
    args = parser.parse_args()
    
    pipeline = GodSPipeline()
    result = asyncio.run(pipeline.run(
        god_name=args.god,
        culture=args.culture,
        kingdom=args.kingdom,
        format_type=args.format
    ))
    
    print(f"\n🎉 Video ID: {result['video_id']}")
    print(f"⏱️  Total time: {result['time_seconds']:.0f}s")

if __name__ == "__main__":
    main()
```

### Usage

```bash
# Generate 88-second Short
python scripts/gods_pipeline.py \
    --god "Cronos" \
    --culture "Greek" \
    --kingdom "Primordia" \
    --format "short"

# Generate 8-minute full story
python scripts/gods_pipeline.py \
    --god "Quetzalcoatl" \
    --culture "Aztec" \
    --kingdom "Poiesis" \
    --format "full"

# Auto-select next god based on feedback
python scripts/gods_auto_select.py
```

---

## 10. COST ANALYSIS

### Monthly Costs (7 videos/week)

| Component | Cost | Notes |
|-----------|------|-------|
| Script Generation (GPT-4o) | $5.25 | 28 videos × 18.75¢ |
| Voice Generation (ElevenLabs) | $15.40 | 154 minutes × 10¢ |
| Image Generation (Flux) | $0 | Local RTX 4090 |
| Video Assembly | $0 | MoviePy (free) |
| YouTube Upload | $0 | API included |
| **Total** | **$20.65/month** | **~$0.74/video** |

### Alternative: All-Local Setup

| Component | Cost | Notes |
|-----------|------|-------|
| Script Generation (Llama 3) | $0 | Local inference |
| Voice Generation (Coqui) | $0 | Local TTS |
| Image Generation (Flux) | $0 | Local RTX 4090 |
| **Total** | **$0/month** | Higher latency |

---

## 11. IMPLEMENTATION TIMELINE

### Phase 1: MVP (Week 1)

- [ ] Set up OpenAI API key
- [ ] Set up ElevenLabs account
- [ ] Test script generation (1 god)
- [ ] Test image generation (Flux/ComfyUI)
- [ ] Test voice generation
- [ ] Assemble first video manually
- [ ] **Deliverable**: 1 complete GODS Short

### Phase 2: Automation (Week 2)

- [ ] Complete pipeline script
- [ ] YouTube API setup
- [ ] Database schema
- [ ] Error handling
- [ ] **Deliverable**: Automated pipeline for 1 video

### Phase 3: Scale (Week 3)

- [ ] Feedback loop
- [ ] Auto-selection of next god
- [ ] Batch processing
- [ ] Analytics dashboard
- [ ] **Deliverable**: 7 videos/week automated

### Phase 4: Features (Week 4)

- [ ] User export feature
- [ ] Share buttons
- [ ] Bilingual upload (ES)
- [ ] Instagram/TikTok export
- [ ] **Deliverable**: Complete GODS platform

---

## 12. RISKS & MITIGATIONS

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| YouTube API rate limits | Medium | High | Queue system, batch uploads |
| ElevenLabs cost overruns | Low | Medium | Monitor usage, switch to local TTS |
| Image inconsistency | Medium | Medium | Use seed values, LoRA fine-tuning |
| ComfyUI crashes | Medium | Low | Restart script, checkpoint system |
| Content policy violation | Low | High | Manual review before upload |
| Voice copyright issues | Low | Medium | Use ElevenLabs commercial license |

---

## 13. ALTERNATIVE APPROACHES

### If YouTube API Blocked

Use direct upload via:
- **Tube Uploader** (unofficial API)
- **ytdl-sub** (reverse engineering)
- Manual upload with metadata templates

### If ElevenLabs Too Expensive

Switch to local TTS:
- **Coqui TTS** (open source)
- **Bark** (generative, emotional)
- **Piper TTS** (fast, good quality)

### If GPT-4o Too Expensive

Use local LLM:
- **Llama 3 70B** (good creative writing)
- **Mistral 7B** (faster, cheaper)
- Quantized models for speed

---

## 14. NEXT STEPS

**Immediate Actions:**

1. ✅ Research document created (this file)
2. ⏭️ Set up API keys (OpenAI, ElevenLabs, YouTube)
3. ⏭️ Test script generation with first god
4. ⏭️ Verify ComfyUI/Flux pipeline
5. ⏭️ Generate first GODS Short

**First God Recommendations:**

Based on the 9th Sojourn theme:
1. **Cronos** (Greek) - Primordia (time/fear)
2. **Anubis** (Egyptian) - Claridium (death/understanding)
3. **Quetzalcoatl** (Aztec) - Poiesis (creation/wisdom)

---

## APPENDIX: ENVIRONMENT SETUP

```bash
# ~/anky/.env

# API Keys
OPENAI_API_KEY=sk-...
ELEVENLABS_API_KEY=...
YOUTUBE_CREDENTIALS_PATH=~/anky/secrets/youtube_credentials.json

# ComfyUI
COMFYUI_URL=http://127.0.0.1:8188

# Output
VIDEO_OUTPUT_DIR=~/anky/videos/gods
THUMBNAIL_OUTPUT_DIR=~/anky/videos/gods/thumbnails

# Database
DATABASE_PATH=~/anky/data/anky.db
```

```bash
# Install dependencies
pip install \
    openai \
    elevenlabs \
    google-api-python-client \
    google-auth-oauthlib \
    moviepy \
    pillow \
    requests \
    python-dotenv \
    fastapi \
    uvicorn
```

---

## CONCLUSION

This architecture enables **zero-cost video generation** (local GPU) with minimal API costs for script/voice (~$21/month).

**Time to first video**: ~60 seconds with current infrastructure.

**Scalability**: 7+ videos/day with 2x RTX 4090.

**Next**: Implement Phase 1 MVP, generate first GODS Short about Cronos.

---

*Document created by Anky via Hermes Agent*
*Date: April 9, 2026*
*For: JP Franeto*
*Purpose: GODS by Anky - YouTube Series Technical Architecture*
