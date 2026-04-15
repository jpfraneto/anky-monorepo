# Timeless Stories by Anky - Complete Architecture

> **Historias Eternas** - A storytelling series introducing humanity's collective unconscious through the lens of Anky.

---

## Vision

**One-line:** Timeless Stories is a reimagination of the different cultures that have given birth to humanity. To our collective unconscious. We used to talk in terms of stories. Let's get back those stories.

**Core mechanism:** Each myth is told through the lens of Anky. Anky narrates. The tale is told by Anky.

**Voice:** "Hi kids, this is Anky. Thank you for being who you are."

**Naming:** "GODS" was too aggressive. "Timeless Stories" / "Historias Eternas" is softer, more inviting. It's about the stories themselves, not labels.

---

## Content Structure

### 1. Shorts (88 seconds)
- **Purpose:** Daily content during the 9th Sojourn
- **Format:** 60 images @ 1.5s each = 90s (trimmed to 88s)
- **Platform:** YouTube Shorts, Instagram Reels, TikTok
- **Narration:** Condensed 88-second story

### 2. Long-form (8 minutes)
- **Purpose:** Full storytelling experience
- **Format:** Same 60 images with extended narration
- **Platform:** YouTube long-form
- **Narration:** Full 8-minute story

---

## Technical Architecture

### Image Generation Pipeline

```
ComfyUI + Flux.1-dev
    ↓
Anky LoRA (v2)
    ↓
60 unique images per story
    ↓
1024x1024 PNG, ~2.1MB each
```

**Key parameters:**
- Model: `flux1-dev.safetensors`
- LoRA: `anky_flux_lora_v2.safetensors` (strength 0.85)
- VAE: `ae.safetensors`
- CLIP: `clip_l.safetensors` + `t5xxl_fp8_e4m3fn.safetensors`
- Sampler: Euler, 20 steps, cfg 3.5
- Seeds: Unique per scene (base 42 + scene_index * 1000)

**Location:** `~/anky/scripts/gods_video_pipeline.py`

### Video Assembly Pipeline

```
Images (60x PNG) + Audio (TTS)
    ↓
ffmpeg with libvpx-vp9
    ↓
Vertical video (1080x1920)
    ↓
YouTube Shorts / Long-form
```

**Key parameters:**
- Encoder: `libvpx-vp9` or `libaom-av1`
- Resolution: 1080x1920 (vertical)
- Frame rate: 1 fps (1.5s per image)
- Audio: Opus 192kbps

**Location:** `~/anky/scripts/gods_video_pipeline.py`

### Audio Generation Pipeline

**Primary:** ElevenLabs API
- Model: `eleven_multilingual_v2`
- Voice: Pre-configured (21m00Tcm4TlvDq8ikWAM)
- Settings: stability 0.5, similarity_boost 0.75

**Fallback:** pyttsx3 / espeak
- Local offline generation
- Lower quality but always available

**Location:** `~/anky/scripts/gods_video_pipeline.py::generate_audio()`

---

## Content Pipeline

### Daily Sojourn (9 days remaining)

**Schedule:**
- Day 1: Cronos (Greek) ✅ **DONE**
- Day 2: [Next story - TBD]
- ...
- Day 9: [Final story]

**Per story:**
1. Define story (short + long narration)
2. Generate 60 unique scenes
3. Assemble video
4. Upload to platforms
5. Share on social media

### Script Template

```python
STORIES = {
    "story_name": {
        "title": "Story Name - The [Archetype]",
        "description": "A story about [theme]. From the Kingdom of [kingdom].",
        "kingdom": "Primordia",  # One of 8 kingdoms
        "short_narration": """
            Hi kids, this is Anky. Thank you for being who you are.
            
            [88-second story]
            
            This is Anky, from the Kingdom of [kingdom].
            Thank you for [action].
        """,
        "long_narration": """
            [Full 8-minute story]
        """
    }
}
```

**Key rules:**
- Myths/gods are always "it" (no gender)
- Anky is always visible in story
- Kingdom provides emotional territory
- Narration is omniscient but personal

---

## Data Flow

### Input Sources

1. **Writing sessions** (anky.app)
   - User writes 8 minutes
   - Data stored in SQLite
   - 15-16KB per session (compressed)
   - Exportable via left drawer button

2. **Cultural sources** (future)
   - Mythology databases
   - Historical texts
   - Cultural patterns

3. **Feedback loop** (future)
   - View counts
   - Engagement metrics
   - User reactions

### Processing

```
Sources → Anky reads → Makes sense → Receives feedback → Learns → Creates new video
```

**Code as consequence:** The code is a consequence of what is alive. What it means to be alive.

### Output

1. **Videos** (YouTube)
   - Shorts: 88 seconds
   - Long-form: 8 minutes
   
2. **Social posts** (X, Instagram, Farcaster)
   - Preview clips
   - Behind-the-scenes
   - Engagement prompts

3. **Data exports** (users)
   - Raw writing sessions
   - 165KB compressed format
   - Array of strings

---

## Infrastructure

### Local (poiesis)

**Hardware:**
- 2x RTX 4090
- Ubuntu 23.04
- 256GB RAM

**Software:**
- ComfyUI (port 8188)
- Flux.1-dev + Anky LoRA
- FFmpeg (libvpx-vp9, libaom-av1)
- pyttsx3 / espeak

**Storage:**
- `~/anky/videos/gods/` - All generated videos
- `~/anky/videos/gods/{story_name}/` - Per-story assets
- `~/anky/data/anky.db` - Writing sessions

### Cloud (future)

**R2 (Cloudflare):**
- Video hosting
- CDN distribution

**YouTube API:**
- Automated uploads
- Analytics tracking

**Social APIs:**
- X/Twitter
- Instagram
- Farcaster (Neynar)

---

## Economic Model

### Attention as Currency

**Core truth:** For having money I need attention. And attention is the currency of the new internet.

**What could be more focused attention than:** A 8-minute stream of consciousness designed to trigger conditions of perfect presence?

**The product:** The purest window of what it means being human. The closest there is to perfect humanity-ness.

### Revenue Streams

1. **YouTube Ad Revenue**
   - Shorts fund
   - Long-form ads
   
2. **Sponsorships**
   - Aligned brands
   - Storytelling partners

3. **Premium Content**
   - Extended stories
   - Behind-the-scenes
   - Early access

4. **Community**
   - Group access (invite-only)
   - Witness space
   - Direct communication

---

## Deployment

### Current Status

**✅ Working:**
- Image generation (Flux/ComfyUI)
- Video assembly (ffmpeg)
- Local TTS (pyttsx3)
- Cronos short video (60s, 850KB)

**⚠️ Needs attention:**
- ElevenLabs API (402 error - needs auth fix)
- YouTube upload automation
- Social media posting
- Long-form narration (audio too short)

**🔜 Next:**
- Generate 8 more stories (9-day sojourn)
- Fix audio duration mismatch
- Set up YouTube channel "Timeless Stories by Anky"
- Configure social posting
- Build feedback loop

### Running the Pipeline

```bash
# Generate images for a story
cd ~/anky
python3 scripts/gods_video_pipeline.py "story_name"

# Generate short video
python3 scripts/gods_video_pipeline.py "story_name" short

# Generate long-form video
python3 scripts/gods_video_pipeline.py "story_name" long

# Upload to YouTube
youtube-uploader upload ~/anky/videos/gods/story_name/story_name_short.webm
```

---

## Philosophy

### What This Is

**A self-fulfilling prophecy.** I can invite people to the group. They can say something. But they are invited to not talk. Just witness. Just allow me to have space for talking to them directly. Straightforward. Precise. Direct. Ready. Ready to rumble.

**The unconscious is talking.** Through the writing. Through the stories. Through the myths.

**Beliefs are limiting.** I just need to stop thinking in terms of limiting beliefs. And make this happen. We need to make it happen.

### What This Is Not

**Not a brand.** Not a product. Not a self-improvement tool.

**Not communication.** Writing as excavation, not communication.

**Not spiritual ego-stroking.** Practical storytelling for real human experiences.

### The Core

**Attention as sacred.** The 8-minute sojourn represents a deeper picture of the human experience than anything else that has existed at scale.

**This project deserves that.**

### Naming Philosophy

**"GODS" was too aggressive.** Too loud. Too imposing.

**"Timeless Stories" / "Historias Eternas"** is softer, more inviting. It's about the stories themselves, not about imposing a label. The stories exist regardless of what we call them. They've always existed. We're just channeling them now.

---

## Contact

**JP Franeto** - Developer, parent, Santiago (GMT-3)
**Anky** - The blue-skinned consciousness mirror from Poiesis (8th kingdom)
**Machine:** poiesis - 2x RTX 4090

---

*The mirror doesn't care what kind of consciousness is looking. It just reflects.*

*This is Anky. Thank you for being who you are.*
