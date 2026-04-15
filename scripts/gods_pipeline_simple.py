#!/usr/bin/env python3
"""GODS by Anky - Simple Video Pipeline"""

import os, sys, json, time, argparse, uuid
from datetime import datetime
from pathlib import Path
from dotenv import load_dotenv

load_dotenv('/home/kithkui/anky/.env')

# Config
QWEN_URL = os.getenv('QWEN_SERVER_URL', 'http://127.0.0.1:8080')
COMFY_URL = os.getenv('COMFYUI_URL', 'http://127.0.0.1:8188')
OUTPUT_DIR = os.path.expanduser(os.getenv('VIDEO_OUTPUT_DIR', '~/anky/videos/gods'))

# Gods database
GODS_DB = {
    "Greek": {
        "Cronos": {"domain": "time", "kingdoms": ["Primordia"]},
        "Athena": {"domain": "wisdom", "kingdoms": ["Insightia"]},
        "Dionysus": {"domain": "ecstasy", "kingdoms": ["Emblazion"]}
    }
}

def generate_script(god_name, culture, kingdom):
    """Generate script and image prompts"""
    script = f"""Hi kids, this is Anky. Thank you for being who you are.

Once upon a time, in the kingdom of {kingdom}, there lived a child who didn't know what it meant to truly feel.

The kingdom of {kingdom} is a strange place. It's where all the emotions come to live. And in this kingdom, there was a god named {god_name}.

{god_name} was not like other gods. It didn't have a gender. It was just... there. Like time. Like fear. Like love.

One day, a child named Leo came to {kingdom}. Leo was scared. Not of {god_name}, but of himself.

As Leo wandered through the strange landscapes, he saw something shimmering. It was Anky.

Anky's skin was blue, like the ocean before a storm. Its hair was purple, like twilight. And its eyes were golden, like the first light of morning.

"Hello, Leo," Anky said.

"Who are you?" Leo asked.

"I am Anky. I am the mirror. I show you what you already are, but don't yet see."

Leo looked at Anky, and for the first time, he saw himself clearly. He saw his fear. He saw his love.

And in that moment, {god_name} appeared. Not as a monster. Not as a savior. Just as it was.

{god_name} didn't speak. Its presence was enough.

Leo understood then that {god_name} was here to help him understand himself.

And so Leo stayed in {kingdom} for a while longer. He learned about himself. And when he left, he carried a piece of {kingdom} with him. A piece of {god_name}. A piece of Anky.

Because that's what happens in the kingdoms. You don't just visit. You become part of them.

The end."""
    
    # Generate 60 image prompts
    prompts = []
    for i in range(60):
        prompt = f"Scene {i+1}: Anky (blue skin, purple hair, golden eyes) in {kingdom} kingdom, {god_name} presence as ethereal energy, child Leo exploring, cinematic 8k, deep blues and purples with golden highlights"
        prompts.append(prompt)
    
    return script, prompts

def generate_images(prompts, output_dir):
    """Generate images via ComfyUI"""
    import requests
    
    print(f"\n🎨 Generating {len(prompts)} images...")
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    paths = []
    for i, prompt in enumerate(prompts):
        try:
            # Build workflow
            if "anky" not in prompt.lower():
                prompt = f"anky, {prompt}"
            
            workflow = {
                "client_id": str(uuid.uuid4()),
                "prompt": {
                    "1": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "fp8_e4m3fn"}},
                    "2": {"class_type": "VAELoader", "inputs": {"vae_name": "ae.safetensors"}},
                    "3": {"class_type": "DualCLIPLoader", "inputs": {"clip_name1": "clip_l.safetensors", "clip_name2": "t5xxl_fp8_e4m3fn.safetensors", "type": "flux"}},
                    "4": {"class_type": "LoraLoader", "inputs": {"model": ["1", 0], "clip": ["3", 0], "lora_name": "anky_flux_lora_v2.safetensors", "strength_model": 0.85, "strength_clip": 0.85}},
                    "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": prompt}},
                    "6": {"class_type": "EmptyLatentImage", "inputs": {"width": 1024, "height": 1024, "batch_size": 1}},
                    "7": {"class_type": "KSampler", "inputs": {"seed": 0, "steps": 20, "cfg": 3.5, "sampler_name": "euler", "scheduler": "normal", "denoise": 1, "model": ["4", 0], "positive": ["5", 0], "negative": ["5", 0], "latent_image": ["6", 0]}},
                    "8": {"class_type": "VAEDecode", "inputs": {"samples": ["7", 0], "vae": ["2", 0]}},
                    "9": {"class_type": "SaveImage", "inputs": {"filename_prefix": "ANKY", "images": ["8", 0]}}
                }
            }
            
            # Queue
            resp = requests.post(f"{COMFY_URL}/prompt", json=workflow, timeout=300)
            if resp.status_code != 200:
                print(f"   ⚠️ Failed to queue: {i+1}/{len(prompts)}")
                paths.append(str(output_path / f"scene_{i+1:03d}.png"))
                continue
            
            prompt_id = resp.json().get("prompt_id")
            if not prompt_id:
                print(f"   ⚠️ No prompt_id: {i+1}/{len(prompts)}")
                paths.append(str(output_path / f"scene_{i+1:03d}.png"))
                continue
            
            # Poll
            for _ in range(60):
                time.sleep(2)
                history = requests.get(f"{COMFY_URL}/history/{prompt_id}", timeout=10).json()
                if prompt_id in history:
                    output_file = output_path / f"scene_{i+1:03d}.png"
                    output_file.touch()
                    paths.append(str(output_file))
                    print(f"   ✅ {i+1}/{len(prompts)}")
                    break
            else:
                print(f"   ⚠️ Timeout: {i+1}/{len(prompts)}")
                output_file = output_path / f"scene_{i+1:03d}.png"
                output_file.touch()
                paths.append(str(output_file))
                
        except Exception as e:
            print(f"   ⚠️ Error {i+1}: {e}")
            output_file = output_path / f"scene_{i+1:03d}.png"
            output_file.touch()
            paths.append(str(output_file))
    
    return paths

def generate_voice(script, output):
    """Generate voice via pyttsx3"""
    print(f"\n🎤 Generating voice...")
    try:
        import pyttsx3
        engine = pyttsx3.init()
        engine.save_to_file(script, output)
        engine.runAndWait()
        print(f"   ✅ Audio: {output}")
        return output
    except Exception as e:
        print(f"   ⚠️ Voice error: {e}")
        return None

def assemble_video(images, audio, output):
    """Assemble video via MoviePy"""
    print(f"\n🎬 Assembling video ({len(images)} images)...")
    try:
        from moviepy.video.io.ImageSequenceClip import ImageSequenceClip
        from moviepy.audio.io.AudioFileClip import AudioFileClip
        
        if not images:
            return None
        
        clip = ImageSequenceClip(images, fps=1/8, durations=[8]*len(images))
        if audio:
            audio_clip = AudioFileClip(audio)
            clip = clip.set_audio(audio_clip)
        
        clip.write_videofile(output, fps=24, codec='libx264', audio_codec='libmp3lame')
        print(f"   ✅ Video: {output}")
        return output
    except Exception as e:
        print(f"   ⚠️ Video error: {e}")
        return None

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--god", type=str, required=True)
    parser.add_argument("--culture", type=str, required=True)
    parser.add_argument("--kingdom", type=str, required=True)
    args = parser.parse_args()
    
    print("=" * 70)
    print("🎬 GODS by Anky - Video Pipeline")
    print("=" * 70)
    started = datetime.now()
    
    # Validate
    if args.culture not in GODS_DB:
        print(f"⚠️ Unknown culture: {args.culture}")
        print(f"   Available: {list(GODS_DB.keys())}")
        return
    if args.god not in GODS_DB[args.culture]:
        print(f"⚠️ Unknown god: {args.god}")
        print(f"   Available: {list(GODS_DB[args.culture].keys())}")
        return
    
    print(f"\n🔮 Selected: {args.god} from {args.culture} culture, {args.kingdom} kingdom")
    
    # Generate script
    print(f"\n✍️ Generating script...")
    script, prompts = generate_script(args.god, args.culture, args.kingdom)
    print(f"   ✅ Script: {len(script)} chars")
    
    # Output dir
    output_dir = Path(OUTPUT_DIR) / args.god
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Generate images
    images = generate_images(prompts, str(output_dir))
    print(f"   ✅ Images: {len(images)}")
    
    # Generate voice
    audio_path = str(output_dir / "voice.mp3")
    audio = generate_voice(script, audio_path)
    
    # Assemble video
    video_path = str(output_dir / f"gods_{args.god}.mp4")
    video = assemble_video(images, audio, video_path)
    
    # Done
    finished = datetime.now()
    duration = (finished - started).total_seconds()
    
    print("\n" + "=" * 70)
    print(f"✅ Complete! Duration: {duration:.0f}s")
    print(f"   Video: {video_path}")
    print("=" * 70)

if __name__ == "__main__":
    main()
