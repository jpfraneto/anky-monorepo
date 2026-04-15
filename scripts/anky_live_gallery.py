#!/usr/bin/env python3
"""
ANKY LIVE GALLERY
=================
Live image generation gallery. Images appear at the top as they're created.
Uses local ComfyUI + Flux LoRA.

Run: python3 scripts/anky_live_gallery.py
Visit: http://localhost:5000
"""

import json
import base64
import time
import requests
from pathlib import Path
from flask import Flask, render_template_string, jsonify, request
from datetime import datetime

# Configuration
COMFYUI_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = Path.home() / "anky" / "data" / "images" / "live_gallery"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Anky image prompts - alive, moving, doing things
# These capture the actual Anky experience - witnessing, reading rhythm, being the mirror
# LONG PROMPTS for better Flux generation - more detail = better images
ANKY_PROMPTS = [
    "anky, a blue-skinned being with deep cobalt skin and flowing purple hair cascading like twilight, luminous golden eyes that catch the faint blue glow of a computer monitor, sitting in absolute silence behind a person typing furiously on a mechanical keyboard, the only light source is the monitor casting harsh shadows, dark room with peeling paint, intimate witness moment, cinematic lighting, photorealistic, 8k, depth of field, the blue skin reflects the screen's light in subtle highlights, purple hair illuminated from below",
    
    "anky, blue skin with violet undertones, wild purple hair tied messily, golden eyes soft with exhaustion, leaning against a cracked wall in a dimly lit cafe at 2am, watching a stranger write in a leather notebook, steam rising from a ceramic coffee cup in foreground, soft bokeh of warm cafe lights, contemplative atmosphere, rain on windows outside, the blue skin contrasts with warm amber lighting, cinematic composition, shallow depth of field, photorealistic, 8k",
    
    "anky the consciousness mirror, blue-skinned entity with electric blue skin and wild purple hair flowing like silk, golden eyes closed in deep listening, hands folded in lap, sitting cross-legged on a woven mat in a minimalist meditation room, soft golden light filtering through paper screens, dust motes dancing in light beams, serene but alert posture, the purple hair catches light like aurora, photorealistic, cinematic, 8k, zen atmosphere, warm color palette with cool blue skin contrast",
    
    "anky, deep blue skin almost black in shadows, purple hair cascading over one shoulder, golden eyes glowing softly in darkness, crouched beside a person sleeping on an old couch, watching their chest rise and fall in slow rhythm, moonlight through horizontal blinds creating striped shadows, protective silent presence, a glass of water on the nightstand catching moonlight, intimate bedroom scene, photorealistic, cinematic lighting, 8k, the blue skin barely visible in moonlight, purple hair silver in moon glow",
    
    "anky the eight-minute witness, blue-skinned being with flowing violet hair, luminous golden eyes reflecting a computer screen's blue light, sitting directly behind someone typing with visible motion blur on their hands, the blue skin illuminated by the monitor's glow creating an ethereal effect, dark room with only screen light, keyboard backlight visible, the purple hair almost black in shadows, intimate digital witness moment, photorealistic, cinematic, 8k, dramatic contrast between blue skin and monitor light",
    
    "anky, blue skin with deep cobalt tones, purple hair tied in a messy bun with loose strands, golden eyes tired and bloodshot, sitting on the edge of an unmade bed at 3am, watching rain streak down a window, a half-empty glass of water on a nightstand, alone but not lonely, morning light barely breaking through clouds, sheets rumpled, the blue skin looks almost gray in predawn light, purple hair dark, photorealistic, cinematic, 8k, melancholic atmosphere, blue color palette",
    
    "anky the mirror with memory, blue-skinned entity with wild flowing purple hair, golden eyes crinkled in a rare genuine smile, watching a child laugh in an autumn park, golden leaves falling around them, warm afternoon sunlight filtering through trees, dappled light on blue skin, the purple hair catches sunlight like fire, peaceful moment of witnessing pure joy, photorealistic, cinematic, 8k, warm autumn palette, shallow depth of field, the blue skin contrasts beautifully with golden light",
    
    "anky, blue skin almost black in shadows, purple hair wild and messy, golden eyes wide with sudden recognition, hand pressed over mouth in shock, sitting in a therapist office watching a client finally say the thing they came to say, dramatic side lighting creating deep shadows, the therapist chair visible in foreground, the blue skin catches a sliver of light, purple hair haloed, photorealistic, cinematic, 8k, dramatic lighting, emotional intensity, cool tones with warm highlights",
    
    "anky, blue-skinned consciousness being with deep blue skin, purple hair cascading over shoulders in soft waves, golden eyes focused and intense, reading a journal entry by candlelight, old wooden desk with ink stains and coffee rings, the candle flame creates dancing shadows on blue skin, the purple hair almost violet in candlelight, intimate quiet moment of discovery, photorealistic, cinematic, 8k, warm candlelight palette, chiaroscuro lighting, the blue skin reflects golden light",
    
    "anky the eight-minute mirror, blue skin with violet undertones, purple hair flowing wild, golden eyes reflecting visible tears, sitting beside someone crying in a parked car, hands gripping steering wheel knuckles white, street lights through windshield creating bokeh, raw emotion in the air, the blue skin visible in street light glow, purple hair almost black, photorealistic, cinematic, 8k, noir atmosphere, cool blue tones with warm street light highlights, shallow depth of field",
    
    "anky, blue-skinned being with luminous blue skin, purple hair pulled back, golden eyes calm and still like deep water, standing at the back of a crowded party watching one person alone at the bar, warm bokeh lights in background, amber tones from bar lighting, observer in chaos, the blue skin stands out against warm colors, purple hair catches colored lights, photorealistic, cinematic, 8k, party atmosphere with isolated focus, shallow depth of field, the blue skin glows against warm background",
    
    "anky, blue skin with deep cobalt tones, purple hair wild and messy tied back, golden eyes fierce with intensity, writing something urgent in a black notebook, hunched over a kitchen table, morning light through blinds creating striped shadows, coffee stains on the page, urgency in posture, shoulders tense, the blue skin visible in natural light, purple hair chaotic, photorealistic, cinematic, 8k, morning light palette, dramatic shadows, the blue skin contrasts with warm morning light",
    
    "anky the consciousness observer, blue-skinned entity with deep blue skin, violet hair flowing, golden eyes soft with deep compassion, holding an empty chair in a room where someone just left, dust motes visible in light beams from windows, aftermath of silence, a coffee cup still warm on the table, the blue skin almost gray in flat light, purple hair dark, photorealistic, cinematic, 8k, melancholic atmosphere, muted color palette, the blue skin blends with shadows, emotional weight visible",
    
    "anky, blue skin with electric blue highlights, purple hair cascading wild, golden eyes watching their own reflection in a dark window at night, face split between the reflection and the city behind them, dual existence, neon signs blurred in background pink and blue, the blue skin visible in neon glow, purple hair catches colored light, photorealistic, cinematic, 8k, cyberpunk atmosphere, neon palette, the blue skin almost invisible against night, reflection shows golden eyes clearly",
    
    "anky, blue-skinned mirror with memory, deep blue skin, purple hair flowing like water, golden eyes closed, hands pressed against a concrete wall as if feeling vibrations, in a recording studio with soundproofing foam, visible sound wave patterns on monitors, reading the rhythm of silence, the blue skin in cool studio light, purple hair in shadows, photorealistic, cinematic, 8k, technical atmosphere, cool blue and gray palette, the blue skin matches studio lighting, purple hair almost black",
    
    "anky, blue skin with deep cobalt tones, wild purple hair messy, golden eyes tired but alert, sitting in an ER waiting room on a hard plastic chair, hands wrapped around a white paper cup, fluorescent lights creating harsh shadows, empty chairs around, the weight of waiting visible in posture, the blue skin under fluorescent light, purple hair flat, photorealistic, cinematic, 8k, institutional atmosphere, sterile color palette, the blue skin looks almost green under fluorescent light, emotional exhaustion",
    
    "anky the eight-minute witness, blue-skinned being with flowing violet hair, golden eyes watching someone burn letters in a fireplace, embers rising in orange glow, orange light on blue skin creating dramatic contrast, ritual of release, the purple hair catches firelight like flames, dark room with only fireplace light, photorealistic, cinematic, 8k, warm firelight palette, the blue skin glows orange in firelight, dramatic chiaroscuro, intimate moment of destruction",
    
    "anky, blue skin with luminous cobalt tones, purple hair blowing wild in wind, golden eyes reflecting a sunrise with pink and orange, standing on a rooftop with arms outstretched, purple hair flowing dramatically, city waking below with morning haze, moment of aliveness recognizing aliveness, the blue skin catches sunrise colors, photorealistic, cinematic, 8k, sunrise palette, warm pinks and oranges on cool blue, epic composition, the purple hair almost on fire in sunrise light, transcendent moment",
    
    "anky, blue-skinned consciousness entity, deep blue skin, purple hair braided neatly, golden eyes focused on a single dandelion in a green field, kneeling in grass, hands about to blow the seeds, morning dew visible on petals, fragile moment of decision, the blue skin in natural morning light, purple hair in soft braids, photorealistic, cinematic, 8k, morning field palette, soft natural light, the blue skin almost visible in grass green, intimate scale, shallow depth of field",
    
    "anky the mirror that remembers, blue skin with deep violet undertones, purple hair flowing, golden eyes closed, forehead resting against cold glass of a train window, city lights streaking by in long exposure blur, reflection superimposed on motion, thinking of something unsaid, the blue skin visible in passing light, purple hair in motion blur, photorealistic, cinematic, 8k, night train atmosphere, streaking lights, the blue skin catches flashes of passing light, melancholic journey, the golden eyes closed in thought",
]

def create_flux_workflow(prompt, width=1024, height=1024):
    """Create ComfyUI workflow for Flux + Anky LoRA."""
    return {
        "1": {
            "class_type": "UNETLoader",
            "inputs": {
                "unet_name": "flux1-dev.safetensors",
                "weight_dtype": "fp8_e4m3fn"
            }
        },
        "2": {
            "class_type": "VAELoader",
            "inputs": {"vae_name": "ae.safetensors"}
        },
        "3": {
            "class_type": "DualCLIPLoader",
            "inputs": {
                "clip_name1": "clip_l.safetensors",
                "clip_name2": "t5xxl_fp8_e4m3fn.safetensors",
                "type": "flux"
            }
        },
        "4": {
            "class_type": "LoraLoader",
            "inputs": {
                "model": ["1", 0],
                "clip": ["3", 0],
                "lora_name": "anky_flux_lora_v2.safetensors",
                "strength_model": 0.85,
                "strength_clip": 0.85
            }
        },
        "5": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "clip": ["4", 1],
                "text": prompt
            }
        },
        "6": {
            "class_type": "EmptyLatentImage",
            "inputs": {
                "width": width,
                "height": height,
                "batch_size": 1
            }
        },
        "7": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["4", 0],
                "positive": ["5", 0],
                "negative": ["5", 0],
                "latent_image": ["6", 0],
                "seed": 0,
                "steps": 20,
                "cfg": 3.5,
                "sampler_name": "euler",
                "scheduler": "simple",
                "denoise": 1.0
            }
        },
        "8": {
            "class_type": "VAEDecode",
            "inputs": {
                "samples": ["7", 0],
                "vae": ["2", 0]
            }
        },
        "9": {
            "class_type": "SaveImage",
            "inputs": {
                "images": ["8", 0],
                "filename_prefix": "anky_live"
            }
        }
    }

def generate_image(prompt):
    """Generate an image using ComfyUI API."""
    client_id = f"anky_live_{datetime.now().strftime('%Y%m%d%H%M%S')}"
    workflow = {"client_id": client_id, "prompt": create_flux_workflow(prompt)}
    
    # Queue the prompt
    resp = requests.post(f"{COMFYUI_URL}/prompt", json=workflow)
    if resp.status_code != 200:
        return None, f"Queue failed: {resp.status_code}"
    result = resp.json()
    prompt_id = result.get("prompt_id")
    
    if not prompt_id:
        return None, "No prompt_id returned"
    
    # Poll for completion
    for _ in range(120):  # 4 minutes max
        time.sleep(2)
        
        resp = requests.get(f"{COMFYUI_URL}/history/{prompt_id}")
        if resp.status_code != 200:
            continue
        
        history = resp.json()
        
        if prompt_id not in history:
            continue
        
        entry = history[prompt_id]
        outputs = entry.get("outputs", {})
        
        for node_id, node_output in outputs.items():
            if "images" in node_output:
                images = node_output["images"]
                if images:
                    filename = images[0].get("filename")
                    if filename:
                        # Fetch the image
                        img_resp = requests.get(f"{COMFYUI_URL}/view?filename={filename}&type=output")
                        if img_resp.status_code == 200:
                            return img_resp.content, None
        
        # Check for errors
        status = entry.get("status", {})
        messages = status.get("messages", [])
        for msg in messages:
            if isinstance(msg, list) and len(msg) >= 2 and msg[0] == "execution_error":
                return None, msg[1].get("exception_message", "Unknown error")
    
    return None, "Timeout after 4 minutes"

# Global state
generated_images = []
auto_generate = True
current_prompt_index = 0

# Flask app
app = Flask(__name__)

HTML_TEMPLATE = '''
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>ANKY LIVE GALLERY</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            background: #0a0a0a;
            color: #e0e0e0;
            font-family: 'Courier New', monospace;
            min-height: 100vh;
            padding: 20px;
        }
        .container { max-width: 1400px; margin: 0 auto; }
        header {
            text-align: center;
            padding: 40px 0;
            border-bottom: 1px solid #333;
            margin-bottom: 30px;
        }
        h1 { font-size: 2.5em; letter-spacing: 4px; color: #7b68ee; margin-bottom: 10px; }
        .subtitle { color: #666; font-size: 0.9em; }
        .controls {
            display: flex;
            justify-content: center;
            gap: 20px;
            margin-bottom: 30px;
            padding: 20px;
            background: #111;
            border-radius: 8px;
        }
        button {
            background: #222;
            color: #e0e0e0;
            border: 1px solid #444;
            padding: 12px 24px;
            cursor: pointer;
            font-family: inherit;
            font-size: 0.9em;
            border-radius: 4px;
            transition: all 0.2s;
        }
        button:hover { background: #333; border-color: #666; }
        button.active { background: #7b68ee; border-color: #7b68ee; color: #000; }
        .status { font-size: 0.85em; color: #888; }
        .gallery { display: flex; flex-direction: column; gap: 30px; }
        .image-card {
            background: #111;
            border: 1px solid #222;
            border-radius: 8px;
            overflow: hidden;
            animation: slideIn 0.5s ease-out;
        }
        @keyframes slideIn {
            from { opacity: 0; transform: translateY(-20px); }
            to { opacity: 1; transform: translateY(0); }
        }
        .image-card img {
            width: 100%;
            max-width: 512px;
            height: auto;
            display: block;
            margin: 0 auto;
        }
        .image-info {
            padding: 15px 20px;
            background: #0d0d0d;
            border-top: 1px solid #222;
        }
        .prompt {
            color: #aaa;
            font-size: 0.85em;
            line-height: 1.5;
            margin-bottom: 8px;
        }
        .meta {
            color: #555;
            font-size: 0.75em;
        }
        .meta span { margin-right: 15px; }
        .generating {
            text-align: center;
            padding: 40px;
            color: #666;
            font-style: italic;
        }
        .progress-bar {
            width: 100%;
            height: 4px;
            background: #1a1a1a;
            margin-top: 15px;
            overflow: hidden;
        }
        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #7b68ee, #9370db);
            width: 0%;
            transition: width 0.3s;
        }
        .stats {
            text-align: center;
            color: #555;
            font-size: 0.8em;
            margin-top: 30px;
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>ANKY LIVE GALLERY</h1>
            <p class="subtitle">Flux + LoRA on RTX 4090</p>
        </header>
        
        <div class="controls">
            <button id="autoBtn" onclick="toggleAuto()">Auto: ON</button>
            <button onclick="generateNext()">Generate Next</button>
            <button onclick="clearGallery()">Clear</button>
        </div>
        
        <div class="status" id="status">Ready</div>
        
        <div class="gallery" id="gallery"></div>
        
        <div class="stats" id="stats">Images: 0 | Prompts left: {{ ANKY_PROMPTS|length }}</div>
    </div>
    
    <script>
        let autoOn = true;
        let promptIndex = 0;
        const allPrompts = {{ ANKY_PROMPTS|tojson }};
        const promptCount = {{ ANKY_PROMPTS|length }};
        
        function updateStatus(msg) {
            document.getElementById('status').textContent = msg;
        }
        
        function addImageCard(imageData, prompt, index) {
            const gallery = document.getElementById('gallery');
            const card = document.createElement('div');
            card.className = 'image-card';
            card.innerHTML = '<img src="data:image/png;base64,' + imageData + '"><div class="image-info"><div class="prompt">' + prompt + '</div><div class="meta"><span>Index: ' + index + '</span><span>' + new Date().toLocaleTimeString() + '</span></div><div class="progress-bar"><div class="progress-fill" style="width: 100%"></div></div></div>';
            gallery.insertBefore(card, gallery.firstChild);
            updateStats();
        }
        
        function updateStats() {
            const images = document.querySelectorAll('.image-card').length;
            const left = Math.max(0, promptCount - promptIndex);
            document.getElementById('stats').textContent = 'Images: ' + images + ' | Prompts left: ' + left;
        }
        
        function generateNext() {
            if (!autoOn) return;
            if (promptIndex >= promptCount) {
                updateStatus('All prompts completed');
                toggleAuto();
                return;
            }
            const prompt = allPrompts[promptIndex];
            updateStatus('Generating: "' + prompt.substring(0, 60) + '..."');
            fetch('/generate', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({prompt: prompt, index: promptIndex})
            }).then(r => r.json()).then(data => {
                if (data.success && data.image) {
                    addImageCard(data.image, prompt, promptIndex);
                    promptIndex++;
                    updateStatus('Ready');
                } else {
                    updateStatus('Error: ' + (data.error || 'Unknown error'));
                }
            }).catch(e => {
                updateStatus('Error: ' + e.message);
            });
        }
        
        function toggleAuto() {
            autoOn = !autoOn;
            document.getElementById('autoBtn').textContent = autoOn ? 'Auto: ON' : 'Auto: OFF';
            document.getElementById('autoBtn').classList.toggle('active', autoOn);
            if (autoOn) {
                updateStatus('Auto-generating...');
                generateNext();
            } else {
                updateStatus('Auto paused');
            }
        }
        
        function clearGallery() {
            document.getElementById('gallery').innerHTML = '';
            promptIndex = 0;
            updateStats();
        }
        
        if (autoOn) {
            generateNext();
        }
    </script>
</body>
</html>
'''

@app.route('/')
def index():
    return render_template_string(HTML_TEMPLATE, ANKY_PROMPTS=ANKY_PROMPTS)

@app.route('/generate', methods=['POST'])
def generate():
    """Generate a single image."""
    data = request.get_json()
    prompt = data.get('prompt', '')
    index = data.get('index', 0)
    
    if not prompt:
        return jsonify({'success': False, 'error': 'No prompt'})
    
    image_bytes, error = generate_image(prompt)
    
    if error:
        return jsonify({'success': False, 'error': error})
    
    if image_bytes:
        # Save locally
        filepath = OUTPUT_DIR / f"anky_{index:03d}.png"
        with open(filepath, 'wb') as f:
            f.write(image_bytes)
        
        # Return as base64
        image_b64 = base64.b64encode(image_bytes).decode('utf-8')
        return jsonify({'success': True, 'image': image_b64, 'path': str(filepath)})
    
    return jsonify({'success': False, 'error': 'No image generated'})

@app.route('/images')
def list_images():
    """List all generated images."""
    files = sorted(OUTPUT_DIR.glob("anky_*.png"))
    return jsonify({
        'images': [
            {
                'filename': f.name,
                'path': str(f),
                'size': f.stat().st_size,
                'created': datetime.fromtimestamp(f.stat().st_ctime).isoformat()
            }
            for f in files
        ]
    })

if __name__ == '__main__':
    print("=" * 60)
    print("ANKY LIVE GALLERY")
    print("=" * 60)
    print(f"ComfyUI: {COMFYUI_URL}")
    print(f"Output: {OUTPUT_DIR}")
    print(f"Prompts: {len(ANKY_PROMPTS)}")
    print("-" * 60)
    print("Visit http://localhost:5000")
    print("=" * 60)
    
    app.run(host='0.0.0.0', port=5000, debug=False, threaded=True)
