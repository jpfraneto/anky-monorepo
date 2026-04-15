#!/usr/bin/env python3
"""
Anky Bell Generator - Creates the gong bell for the 8-minute writing ritual
"""

print("Loading transformers...")
from transformers import AutoProcessor, MusicgenForConditionalGeneration
import scipy.io.wavfile
import torch
import os

print("Checking for existing models...")
# Check if we have the model cached
cache_dir = os.path.expanduser("~/.cache/huggingface")
print(f"Cache dir: {cache_dir}")

print("\nLoading MusicGen-small (this downloads the model first time)...")
print("Please wait - model is ~500MB...")

try:
    processor = AutoProcessor.from_pretrained("facebook/musicgen-small")
    print("Processor loaded")
    
    model = MusicgenForConditionalGeneration.from_pretrained("facebook/musicgen-small")
    print("Model loaded")
    
    # Move to GPU if available
    if torch.cuda.is_available():
        model = model.to("cuda")
        print("Using GPU")
    else:
        print("Using CPU")
    
    # The bell
    descriptions = ["deep meditative gong bell, spiritual, resonant, sustained tone, fading, ceremonial"]
    
    print("\nGenerating the bell...")
    print("The bell marks. The bell is powerful.")
    
    inputs = processor(text=descriptions, padding=True, return_tensors="pt")
    
    if torch.cuda.is_available():
        inputs = inputs.to("cuda")
    
    with torch.no_grad():
        audio_values = model.generate(
            **inputs,
            do_sample=True,
            guidance_scale=3,
            max_new_tokens=384  # ~9 seconds
        )
    
    sampling_rate = model.config.audio_encoder.sampling_rate
    scipy.io.wavfile.write("anky_bell.wav", rate=sampling_rate, data=audio_values[0, 0].cpu().numpy())
    
    print(f"\nBell created: anky_bell.wav")
    print(f"Duration: ~{audio_values.shape[-1] / sampling_rate:.1f} seconds")
    print("\nThe bell is ready.")
    
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
