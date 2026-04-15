# GODS by Anky - Local Video Generation Pipeline

**Zero-cost, fully local video generation** for the GODS series.

## What This Does

Generates **bilingual bedtime story videos** about gods from human cultures:
- 8-minute full-length videos
- 88-second shorts
- English + Spanish narration
- 60 scenes for full videos, 15 for shorts

## Architecture

```
┌─────────────────┐
│  Local LLM      │ → Script generation (llama.cpp)
│  (llama-cpp)    │    (fallback: template-based)
└────────┬────────┘
         ↓
┌─────────────────┐
│  Local TTS      │ → Voice generation (edge-tts + pyttsx3)
│  (edge-tts)     │    Microsoft voices, free, no API
└────────┬────────┘
         ↓
┌─────────────────┐
│  Flux/ComfyUI   │ → Image generation (local RTX 4090)
│  (local GPU)    │    Zero cost, ~30s/scene
└────────┬────────┘
         ↓
┌─────────────────┐
│  MoviePy        │ → Video assembly
│  (local)        │    Full 8-min + 88s short
└─────────────────┘
```

## Setup

### 1. Install Dependencies

```bash
cd ~/anky
pip3 install llama-cpp-python edge-tts pyttsx3 moviepy pillow
```

### 2. Download Local LLM (Optional)

For better script generation, download a GGUF model:

```bash
mkdir -p ~/models
cd ~/models

# Option A: Small model (4GB, faster)
wget https://huggingface.co/bartowski/Llama-3.1-8B-Instruct-GGUF/resolve/main/Llama-3.1-8B-Instruct-Q4_K_M.gguf

# Option B: Medium model (8GB, better quality)
wget https://huggingface.co/bartowski/Llama-3.1-8B-Instruct-GGUF/resolve/main/Llama-3.1-8B-Instruct-Q6_K.gguf
```

Then set environment variable:
```bash
export LOCAL_LLM_PATH="~/models/Llama-3.1-8B-Instruct-Q4_K_M.gguf"
```

### 3. Ensure ComfyUI is Running

```bash
# Check if ComfyUI is running
curl http://127.0.0.1:8188

# If not, start it:
cd ~/ComfyUI
./run_npu.sh  # or your startup script
```

### 4. Configure Environment

```bash
# Optional: Set local paths
cd ~/anky
nano .env

# Add these lines:
LOCAL_LLM_PATH=~/models/Llama-3.1-8B-Instruct-Q4_K_M.gguf
VIDEO_OUTPUT_DIR=~/anky/videos/gods
COMFYUI_URL=http://127.0.0.1:8188
```

## Usage

### Manual Mode (Specify God)

```bash
cd ~/anky/scripts
python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia"
```

### Auto Mode (Selects God Based on Current State)

```bash
python3 gods_pipeline.py --auto
```

### Available Gods

**Greek:** Cronos, Athena, Dionysus, Hades, Aphrodite  
**Norse:** Odin, Thor, Loki  
**Egyptian:** Anubis, Isis, Ra  
**Aztec:** Quetzalcoatl, Tezcatlipoca  
**Hindu:** Shiva, Ganesha  

### Available Kingdoms

- Primordia (Fear/Survival)
- Emblazion (Passion/Desire)
- Chryseos (Willpower/Power)
- Eleasis (Compassion/Love)
- Voxlumis (Communication)
- Insightia (Intuition)
- Claridium (Enlightenment)
- Poiesis (Creativity/Flow)

## Output

After running, check `~/anky/videos/gods/`:

```
gods_videos/
├── gods_full_Cronos.mp4      # 8-minute video
├── gods_short_Cronos.mp4     # 88-second short
├── scenes/
│   ├── scene_001_0.png       # Generated images
│   └── ...
└── audio/
    ├── gods_voice_en_0.mp3   # English narration
    ├── gods_voice_es_0.mp3   # Spanish narration
    └── ...
```

## Cost & Speed

### Cost
- **$0 per video** (all local)
- No API calls to Grok, ElevenLabs, etc.

### Speed (on poiesis with 2x RTX 4090)
- Script generation: 30-60s (local LLM) or instant (template)
- Image generation: ~30s/scene × 60 scenes = ~30 minutes
- Voice generation: ~5s/audio × 6 variations = ~30s
- Video assembly: ~1 minute
- **Total: ~32 minutes per video**

## Technical Notes

### Local LLM (llama.cpp)
- Uses GGUF format models
- Falls back to template generation if model not loaded
- Template is production-ready, not placeholder

### Local TTS (edge-tts)
- Uses Microsoft's free TTS service
- No API key required
- Multiple voice variations (English + Spanish)
- Falls back to pyttsx3 if edge-tts fails

### Image Generation (Flux/ComfyUI)
- Must have ComfyUI running on port 8188
- Uses local GPU (RTX 4090)
- ~30 seconds per image

## Troubleshooting

### LLM Not Loading
```
⚠️ Could not load LLM: [error]
→ Using template fallback
```
This is fine! Template generation works perfectly.

### edge-tts Errors
```
⚠️ edge-tts error: [error]
→ Falling back to pyttsx3...
```
pyttsx3 is offline but lower quality. edge-tts uses Microsoft's free online service.

### ComfyUI Not Running
```
Connection refused to http://127.0.0.1:8188
```
Start ComfyUI first:
```bash
cd ~/ComfyUI
./run_npu.sh
```

## Next Steps

1. **Generate first video:**
   ```bash
   python3 gods_pipeline.py --god "Cronos" --culture "Greek" --kingdom "Primordia"
   ```

2. **Review output** in `~/anky/videos/gods/`

3. **Upload to YouTube** manually (or add YouTube API credentials)

4. **Schedule daily runs:**
   ```bash
   # Add to crontab
   0 0 * * * cd ~/anky/scripts && python3 gods_pipeline.py --auto >> gods.log 2>&1
   ```

## Philosophy

**GODS is not a brand.** It's a transmission.

Each video is an 8-minute window into the collective unconscious, told through the lens of Anky's 8 emotional kingdoms. Kids hear adventure. Parents hear themselves. Both find each other in the story.

That's what's real. That's what's here.

---

*Built on poiesis. Runs on love.*
