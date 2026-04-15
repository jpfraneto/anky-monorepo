#!/usr/bin/env python3
"""
Simple ComfyUI image generator for GODS pipeline
Uses the same workflow as your existing anky image generation
"""

import requests
import time
from pathlib import Path

COMFYUI_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = Path("~/anky/videos/gods").expanduser()

def generate_anky_image(prompt: str, seed: int = 42) -> Path:
    """
    Generate single image via ComfyUI using Flux
    Returns Path to generated image
    """
    # FLUX workflow - exact copy from Rust comfyui.rs
    workflow = {
        "client_id": "gods_pipeline",
        "prompt": {
            # 1: Load UNet
            "1": {
                "class_type": "UNETLoader",
                "inputs": {
                    "unet_name": "flux1-dev.safetensors",
                    "weight_dtype": "fp8_e4m3fn"
                }
            },
            # 2: Load VAE
            "2": {
                "class_type": "VAELoader",
                "inputs": {"vae_name": "ae.safetensors"}
            },
            # 3: Load CLIP (dual: clip_l + t5)
            "3": {
                "class_type": "DualCLIPLoader",
                "inputs": {
                    "clip_name1": "clip_l.safetensors",
                    "clip_name2": "t5xxl_fp8_e4m3fn.safetensors",
                    "type": "flux"
                }
            },
            # 4: Apply LoRA to UNet + CLIP
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
            # 5: Encode positive prompt
            "5": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["4", 1],
                    "text": prompt
                }
            },
            # 6: Empty latent
            "6": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "width": 1024,
                    "height": 1024,
                    "batch_size": 1
                }
            },
            # 7: Sample
            "7": {
                "class_type": "KSampler",
                "inputs": {
                    "model": ["4", 0],
                    "positive": ["5", 0],
                    "negative": ["5", 0],
                    "latent_image": ["6", 0],
                    "seed": seed,
                    "steps": 20,
                    "cfg": 3.5,
                    "sampler_name": "euler",
                    "scheduler": "simple",
                    "denoise": 1.0
                }
            },
            # 8: Decode
            "8": {
                "class_type": "VAEDecode",
                "inputs": {
                    "samples": ["7", 0],
                    "vae": ["2", 0]
                }
            },
            # 9: Save
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "images": ["8", 0],
                    "filename_prefix": "GODS"
                }
            }
        }
    }
    
    try:
        # Send prompt
        response = requests.post(f"{COMFYUI_URL}/prompt", json=workflow)
        result = response.json()
        
        if "prompt_id" not in result:
            print(f"    ComfyUI error: {result}")
            return None
        
        prompt_id = result["prompt_id"]
        
        # Poll for completion (up to 240s)
        for i in range(120):
            time.sleep(2)
            
            history_response = requests.get(f"{COMFYUI_URL}/history/{prompt_id}")
            if history_response.status_code == 200:
                history = history_response.json()
                if prompt_id in history:
                    history_data = history[prompt_id]
                    # Extract image filename
                    outputs = history_data.get("outputs", {})
                    for node_id, node_data in outputs.items():
                        if "images" in node_data:
                            image_data = node_data["images"][0]
                            filename = image_data["filename"]
                            
                            # Fetch image
                            view_response = requests.get(
                                f"{COMFYUI_URL}/view",
                                params={"filename": filename, "type": "output"}
                            )
                            
                            if view_response.status_code == 200:
                                OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
                                image_path = OUTPUT_DIR / f"gods_{seed}.png"
                                
                                with open(image_path, "wb") as f:
                                    f.write(view_response.content)
                                
                                print(f"    Generated {image_path}")
                                return image_path
        
        print(f"    Timeout waiting for image")
        return None
        
    except Exception as e:
        print(f"    Error: {e}")
        return None

if __name__ == "__main__":
    # Test
    test_prompt = "anky, a blue-skinned being with purple hair and golden eyes, mystical forest, cinematic lighting"
    result = generate_anky_image(test_prompt, seed=42)
    print(f"Result: {result}")
