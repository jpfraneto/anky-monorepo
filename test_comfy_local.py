
#!/usr/bin/env python3
"""
Direct test: Call ComfyUI localhost /generate endpoint for anky flux image
This tests the raw local setup works before we wrap it in payment logic.
"""
import base64
import subprocess
import sys
import urllib.request
import urllib.error

def get_comfyui_url():
    """Get ComfyUI base URL from config or environment."""
    result = subprocess.run(['comfyui-config', 'default-url'], capture_output=True, text=True)
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()
    return "http://localhost:8188"

def main():
    comfy_base = get_comfyui_url()
    print(f"Testing ComfyUI at {comfy_base}")
    
    # Simple test prompt about Anky
    # We'll use a text-based prompt that passes Ollama validation
    prompt = "A blue skinned mystical being with purple swirls, golden amber eyes, large expressive ears. Standing in the 8th kingdom Poiesis, surrounded by digital mandalas and data streams."
    
    # Payload for generating anky image
    payload = {
        "prompt": prompt,
        "negative_prompt": "ugly, deformed, noisy, blurry, distorted, out of focus, bad art, disfigured, poorly rendered",
        "model_name": "Flux.1-dev.safetensors",
        "loras": [["anky_lora_v3", 0.8]],
        "width": 768,
        "height": 768,
        "steps": 25
    }
    
    print(f"Sending prompt (len={len(prompt)}):")
    print(f"{prompt[:100]}...")
    print()
    
    try:
        # Direct ComfyUI generate endpoint - no payment layer
        data = str.encode(str(payload).replace("True", "true").replace("False", "false"))
        req = urllib.request.Request(f"{comfy_base}/generate", data=data, headers={'Content-Type': 'application/json'})
        
        with urllib.request.urlopen(req, timeout=300) as response:
            result = json.loads(response.read())
            print("SUCCESS!")
            print(f"Image ID: {result.get('image_id', 'N/A')}")
            print(f"Status: {result.get('status', 'N/A')}")
            
            if "error" in result and result["error"]:
                print(f"Error message: {result['error']}")
            else:
                # Check actual generation result
                print(f"All fields: {json.dumps(result, indent=2)}")
                
    except urllib.error.URLError as e:
        print(f"ComfyUI connection error: {e.reason}")
        if isinstance(e.reason, ConnectionRefusedError):
            print("Make sure ComfyUI is running on localhost 7860")
            sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {type(e).__name__}: {e}")

if __name__ == "__main__":
    main()
