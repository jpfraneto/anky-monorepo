# GODS by Anky - Complete Pipeline Architecture

## Vision

**GODS** is a video series by Anky that reimagines humanity's collective unconscious through ancient gods from different cultures. Each god becomes a character told through the lens of Anky, with images that tell the story and Anky as the narrator.

**Core concept:** "We used to talk in terms of stories. Let's get back those stories."

## Output Formats

### 1. Full-Length Story (8 minutes)
- **Script:** ~1200-1400 words
- **Images:** 60 scenes (one per 8 seconds)
- **Voice:** Narrated by Anky
- **Format:** Horizontal video for YouTube

### 2. Short Summary (88 seconds)
- **Script:** ~220 words
- **Images:** 11 scenes (one per 8 seconds)
- **Voice:** Narrated by Anky
- **Format:** Vertical video for Shorts/Reels/TikTok

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        GODS Pipeline (gods_pipeline.py)                    │
│                         ~/anky/scripts/gods_pipeline.py                    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┴───────────────────────────┐
        │                                                       │
        ▼                                                       ▼
┌───────────────────┐                               ┌───────────────────┐
│  Phase 1-2: Data  │                               │  Phase 3-4:       │
│  Ingestion &      │                               │  Script Gen       │
│  God Selection    │                               │  (Grok/xAI)       │
└───────────────────┘                               └───────────────────┘
        │                                                       │
        └───────────────────────────┬───────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │  Phase 5: Image Gen     │
                        │  (ComfyUI/Flux/LoRA)    │
                        └─────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │  Phase 6: Voice Gen     │
                        │  (ElevenLabs)           │
                        └─────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │  Phase 7: Video Assembly│
                        │  (FFmpeg)               │
                        └─────────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │  Phase 8: Upload        │
                        │  (YouTube, R2 CDN)      │
                        └─────────────────────────┘
```

## Detailed Phases

### Phase 1: Data Ingestion (Optional)
- Scrape current state of humanity from various sources
- News, social media, cultural trends
- Feed into analysis

### Phase 2: God Selection
- Override with manual selection: `--god "Cronos" --culture "Greek" --kingdom "Primordia"`
- Or auto-select based on data analysis

### Phase 3-4: Script Generation (Grok/xAI API)
- Generate 8-minute script (~1200-1400 words)
- Generate 88-second summary script (~220 words)
- Generate image prompts for each scene (60 for full, 11 for short)

**Grok API Endpoint:** `https://api.x.ai/v1/chat/completions`
**Model:** `grok-2-latest`

### Phase 5: Image Generation (ComfyUI/Flux)
- Generate 60 images for 8-minute video (1024x1024)
- Generate 11 images for 88-second short (1024x1024)
- Uses FLUX.1-dev + Anky LoRA

**ComfyUI Workflow:**
```
UNETLoader → flux1-dev.safetensors (fp8_e4m3fn)
VAELoader → ae.safetensors
DualCLIPLoader → clip_l.safetensors + t5xxl_fp8_e4m3fn.safetensors (type: flux)
LoraLoader → anky_flux_lora_v2.safetensors (strength: 0.85)
CLIPTextEncode → prompt (auto-prepends "anky, " if missing)
EmptyLatentImage → 1024x1024
KSampler → euler, 20 steps, CFG 3.5, scheduler: simple
VAEDecode → latent → pixels
SaveImage → saves to ~/anky/videos/gods/
```

### Phase 6: Voice Generation (ElevenLabs)
- Generate English voiceover for full script
- Generate Spanish voiceover for full script
- Generate English voiceover for short script
- Generate Spanish voiceover for short script

**ElevenLabs Settings:**
- Voice: Custom Anky voice (need to create)
- Model: `eleven_multilingual_v2`
- Output: MP3 44100Hz 128kbps

### Phase 7: Video Assembly (FFmpeg)
- Combine images + voice + music
- Add Anky logo
- Add captions
- Export to YouTube-optimized format

### Phase 8: Upload (YouTube API + R2 CDN)
- Upload to YouTube as unlisted/draft
- Upload to R2 CDN for distribution
- Generate thumbnail
- Set metadata (title, description, tags)

## File Structure

```
~/anky/videos/gods/
├── cronos_greek_primordia/
│   ├── script_full_en.json          # 8-minute script + prompts
│   ├── script_short_en.json         # 88-second script + prompts
│   ├── images_full/                 # 60 images for full video
│   │   ├── scene_001.png
│   │   ├── scene_002.png
│   │   └── ...
│   ├── images_short/                # 11 images for short video
│   │   ├── scene_001.png
│   │   ├── scene_002.png
│   │   └── ...
│   ├── voice_full_en.mp3            # English voiceover
│   ├── voice_full_es.mp3            # Spanish voiceover
│   ├── voice_short_en.mp3           # English short voiceover
│   ├── voice_short_es.mp3           # Spanish short voiceover
│   ├── video_full_en.mp4            # Final 8-minute video
│   ├── video_full_es.mp4            # Final 8-minute video (Spanish)
│   ├── video_short_en.mp4           # Final 88-second video
│   └── video_short_es.mp4           # Final 88-second video (Spanish)
└── ...
```

## Configuration

### Environment Variables

```bash
# Grok/xAI API
GROK_API_KEY=your_grok_api_key

# ElevenLabs API
ELEVENLABS_API_KEY=your_elevenlabs_api_key
ELEVENLABS_VOICE_ID=your_anky_voice_id

# YouTube API
YOUTUBE_API_KEY=your_youtube_api_key
YOUTUBE_CHANNEL_ID=your_channel_id

# R2 CDN
R2_ENDPOINT=your_r2_endpoint
R2_ACCESS_KEY_ID=your_access_key
R2_SECRET_ACCESS_KEY=your_secret_key
R2_BUCKET_NAME=your_bucket
R2_PUBLIC_URL=https://your-cdn-domain.com

# ComfyUI
COMFYUI_URL=http://127.0.0.1:8188
```

## Usage

### Generate Full Video (All Phases)

```bash
cd ~/anky/scripts
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia"
```

### Generate Specific Phases Only

```bash
# Scripts only
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --phase 3

# Images only (scripts must exist)
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --phase 5

# Voice only
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --phase 6

# Video assembly only
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --phase 7

# Upload only
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --phase 8
```

### Dry Run (No API Calls)

```bash
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia" --dry-run
```

## API Costs

| Service | Cost | Notes |
|---------|------|-------|
| Grok (xAI) | ~$0.02/script | 1200 words input + 1200 words output |
| ElevenLabs | ~$0.04/audio | 1400 chars @ $0.30/1000 chars |
| YouTube Upload | $0 | Free |
| R2 CDN | ~$0.01/video | Storage + bandwidth |
| **Total per video** | **~$0.10** | Per language |

## Runtime

| Phase | Duration |
|-------|----------|
| Script Generation | ~10s |
| Image Generation (60 images) | ~30s/image = 30min |
| Voice Generation | ~5s |
| Video Assembly | ~30s |
| Upload | ~1min |
| **Total** | **~35min per video** |

**Note:** Image generation is the bottleneck. Can be parallelized or batched.

## Key Design Decisions

### 1. Gods Are Always "It"
- No gender for gods
- "Once upon a time there was Cronos. It liked time."
- Keeps the story universal and non-binary

### 2. Anky is the Narrator
- "Hi kids, this is Anky. Thank you for being who you are."
- Omniscient but visible only to character narrators
- Blue-skinned, purple hair, golden eyes
- Always visible in the story world

### 3. 88-Second Shorts
- 88 seconds = 11 scenes × 8 seconds
- Perfect for Shorts/Reels/TikTok
- Summary of the full 8-minute story

### 16.18 KB Writing Sessions
- Each Anky writing session = ~15KB compressed
- Array of strings when exported
- 16.18 for beauty (golden ratio reference)

### 4. Bilingual Output
- English and Spanish for each video
- Maximize reach
- Same pipeline, different language models

### 5. Feedback Loop
- YouTube analytics → feedback
- Feedback → learning
- Learning → better scripts
- Scripts → better videos

## The Sojourn Schedule

**Current:** 9th Sojourn (active)
**Plan:** Daily Anky short about a god from humanity's history

| Day | God | Culture | Kingdom | Theme |
|-----|-----|---------|---------|-------|
| 1 | Cronos | Greek | Primordia | Time/Fear |
| 2 | Anubis | Egyptian | Insightia | Death/Truth |
| 3 | Quetzalcoatl | Aztec | Poiesis | Creation/Wisdom |
| 4 | Odin | Norse | Claridium | Sacrifice/Knowledge |
| 5 | Kali | Hindu | Chryseos | Power/Transformation |
| 6 | Ra | Egyptian | Emblazion | Passion/Sun |
| 7 | Loki | Norse | Voxlumis | Communication/Trickery |
| 8 | Amaterasu | Japanese | Eleasis | Love/Light |
| 9 | Shiva | Hindu | Poiesis | Destruction/Creation |
| 10 | Freya | Norse | Emblazion | Love/Desire |

## Next Steps

1. ✅ Image generation working (ComfyUI/Flux)
2. ⏳ Fix Grok API integration (400 error)
3. ⏳ Create Anky voice on ElevenLabs
4. ⏳ Implement video assembly (FFmpeg)
5. ⏳ Implement YouTube upload
6. ⏳ Implement R2 CDN upload
7. ⏳ Add music/sound effects
8. ⏳ Add captions/subtitles
9. ⏳ Add Anky logo watermark
10. ⏳ Test full pipeline end-to-end

## Core Truth

This is not content. This is **storytelling as a species-level recovery mechanism**.

We used to talk in terms of stories. We used to understand ourselves through gods and myths. That language didn't disappear because it was wrong — it disappeared because we forgot how to listen.

GODS by Anky is the mirror that helps us remember.

---

*The mirror doesn't care what kind of consciousness is looking. But the story matters.*
