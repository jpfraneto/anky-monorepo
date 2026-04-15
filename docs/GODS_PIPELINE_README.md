# GODS by Anky - Pipeline Documentation

## Overview

Complete automated pipeline for generating GODS videos:
- **8-minute full stories** (16:9 landscape)
- **88-second Shorts** (9:16 vertical)
- **Bilingual**: English + Spanish
- **Local GPU**: Uses your 2x RTX 4090 (zero cost for images)

---

## Quick Start

### 1. Install Dependencies

```bash
cd ~/anky/scripts
pip install -r requirements_gods.txt
```

### 2. Set Up Environment Variables

Edit `~/anky/.env`:

```bash
# Grok API (X.ai)
GROK_API_KEY=your-grok-api-key

# ElevenLabs TTS
ELEVENLABS_API_KEY=your-elevenlabs-api-key

# YouTube (optional, for auto-upload)
YOUTUBE_CREDENTIALS_PATH=~/anky/secrets/youtube_credentials.json

# ComfyUI (should already be running)
COMFYUI_URL=http://127.0.0.1:8188

# Output paths
VIDEO_OUTPUT_DIR=~/anky/videos/gods
DATABASE_PATH=~/anky/data/anky.db
```

### 3. Verify ComfyUI is Running

```bash
ps aux | grep comfy
# Should show: python main.py --listen 127.0.0.1 --port 8188
```

If not running:
```bash
cd ~/ComfyUI
python main.py --listen 127.0.0.1 --port 8188 --disable-auto-launch
```

### 4. Run the Pipeline

**Manual mode** (specify god):

```bash
cd ~/anky/scripts
python gods_pipeline.py \
    --god "Cronos" \
    --culture "Greek" \
    --kingdom "Primordia"
```

**Auto mode** (Grok selects god based on current collective unconscious):

```bash
python gods_pipeline.py --auto
```

---

## Pipeline Phases

### Phase 1: Data Ingestion (Auto mode only)
- Reads X/Twitter, Farcaster, news via Grok
- Creates "collective unconscious snapshot"
- Outputs dominant themes, emotions, fears, desires

### Phase 2: God Selection
- Matches collective state to cultural archetypes
- Selects one god that embodies current human condition
- Maps to Anky kingdom

### Phase 3: Script Generation (8 minutes)
- Grok generates full bilingual script (~1200 words)
- Anky narrates, god is "it" (no gender)
- Story set in emotional kingdom

### Phase 4-5: Image Generation
- Breaks script into 60 scenes
- Flux/ComfyUI generates 3 variations per scene
- Selects best images, ensures continuity

### Phase 6-8: Video Assembly
- Stitches 60 images into 8-minute video
- Adds ElevenLabs voice narration (3 variations)
- Exports 16:9 landscape format

### Phase 9: Short Script (88 seconds)
- Condenses 8-min story into essence
- Selects 15 most powerful images

### Phase 10: Short Video
- Assembles 88-second vertical video
- 9:16 format for YouTube Shorts

### Phase 11: Upload (optional)
- Uploads to YouTube via API
- Adds metadata, tags, description
- Records in database

---

## Output

Videos saved to `~/anky/videos/gods/`:

```
gods_full_Cronos.mp4       # 8-minute landscape video
gods_short_Cronos.mp4      # 88-second vertical short
gods_<seed>.png            # Generated images
```

Database records in `~/anky/data/anky.db`:

```sql
SELECT * FROM gods_videos;
-- Shows: god_name, culture, kingdom, scripts, video_ids, status
```

---

## Cost Analysis

| Component | Cost | Notes |
|-----------|------|-------|
| Grok API (scripts) | ~$0.25/video | ~2000 tokens |
| ElevenLabs (voice) | ~$0.10/video | ~8 minutes audio |
| Flux images | $0 | Local RTX 4090 |
| Video assembly | $0 | MoviePy (free) |
| **Total** | **~$0.35/video** | |

---

## Time Estimates

| Phase | Time |
|-------|------|
| Data ingestion | 30s |
| Script generation | 60s |
| Image generation (60 scenes × 3) | 5-10 min |
| Voice generation | 30s |
| Video assembly | 2 min |
| **Total** | **~10-15 minutes** |

---

## Troubleshooting

### ComfyUI Connection Error

```
Connection refused: 127.0.0.1:8188
```

**Solution**: Start ComfyUI:
```bash
cd ~/ComfyUI
python main.py --listen 127.0.0.1 --port 8188
```

### Grok API Error

```
401 Unauthorized
```

**Solution**: Check GROK_API_KEY in .env

### ElevenLabs Error

```
Rate limit exceeded
```

**Solution**: Wait or upgrade ElevenLabs plan

### Image Quality Issues

**Solution**: Adjust in `gods_pipeline.py`:
```python
config.anky_character = """
    More specific character description here
"""
```

### Video Too Slow

**Solution**: Reduce image quality or scenes:
```python
config.num_scenes_full = 40  # Instead of 60
config.images_per_scene = 2  # Instead of 3
```

---

## Next Steps

### After First Video Works

1. **Add music generation** (Suno/Udio API)
2. **Custom Anky voice** (ElevenLabs voice cloning)
3. **Auto-upload to YouTube** (add credentials)
4. **Multi-platform distribution** (X, Instagram, TikTok)
5. **Analytics feedback loop** (track performance, auto-select next god)

### Scale to Daily Videos

```bash
# Cron job for daily automation
crontab -e

# Add:
0 9 * * * cd ~/anky/scripts && python gods_pipeline.py --auto >> logs/gods_pipeline.log 2>&1
```

This runs pipeline every day at 9 AM, auto-selecting god based on current collective unconscious.

---

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  DATA INGESTION (Grok)                                     │
│  • X/Twitter, Farcaster, News                              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  GOD SELECTION                                             │
│  • Match themes to archetypes                              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  SCRIPT GENERATION (Grok)                                  │
│  • 8-min full script (EN + ES)                             │
│  • 88s short script                                        │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  IMAGE GENERATION (Flux/ComfyUI)                           │
│  • 60 scenes × 3 variations = 180 images                   │
│  • Local RTX 4090 ($0)                                     │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  VOICE GENERATION (ElevenLabs)                             │
│  • 3 voice variations                                      │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  VIDEO ASSEMBLY (MoviePy)                                  │
│  • 8-min full video (16:9)                                 │
│  • 88s short video (9:16)                                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  YOUTUBE UPLOAD (optional)                                 │
│  • Upload via API                                          │
│  • Add metadata                                            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  DATABASE RECORD                                           │
│  • Store scripts, video IDs, analytics                     │
└─────────────────────────────────────────────────────────────┘
```

---

## API Endpoints Used

| Service | Endpoint | Purpose |
|---------|----------|---------|
| Grok | `https://api.x.ai/v1/chat/completions` | Script generation |
| ElevenLabs | SDK | Voice generation |
| ComfyUI | `http://127.0.0.1:8188/prompt` | Image generation |
| YouTube | Data API v3 | Video upload |

---

## Future Enhancements

### Levers to Pull (When Revenue Allows)

1. **Grok Video Generation** - AI-generated video clips instead of static images
2. **Music Generation** - Suno/Udio for custom background scores
3. **Custom Voice** - Clone Anky's unique voice via ElevenLabs
4. **Multi-Platform** - Auto-post to X, Instagram, TikTok, Farcaster
5. **Analytics Loop** - Use YouTube data to select next god
6. **Live Streaming** - Real-time Anky storytelling sessions

---

## Contact / Support

**Built by**: Anky (via Hermes Agent)
**For**: JP Franeto
**Date**: April 9, 2026
**Machine**: poiesis (2x RTX 4090)

---

*The pipeline is ready. The gods are waiting.*
