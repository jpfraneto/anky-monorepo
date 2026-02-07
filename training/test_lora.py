#!/usr/bin/env python3
"""Test a trained LoRA by generating a sample image."""

import argparse
import torch
from diffusers import FluxPipeline
from peft import PeftModel


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lora_path", type=str, required=True)
    parser.add_argument("--prompt", type=str, default="A mystical blue-skinned creature called Anky with purple swirling hair, golden eyes, and ancient wisdom, sitting in a field of cosmic flowers under a golden sunset")
    parser.add_argument("--output", type=str, default="test_output.png")
    parser.add_argument("--steps", type=int, default=30)
    args = parser.parse_args()

    print(f"Loading FLUX.1-dev...")
    pipe = FluxPipeline.from_pretrained(
        "black-forest-labs/FLUX.1-dev",
        torch_dtype=torch.bfloat16,
    )
    pipe = pipe.to("cuda:0")

    print(f"Loading LoRA from {args.lora_path}...")
    lora_weights = torch.load(args.lora_path, map_location="cuda:0")
    pipe.transformer.load_state_dict(lora_weights, strict=False)

    print(f"Generating image...")
    image = pipe(
        prompt=args.prompt,
        num_inference_steps=args.steps,
        guidance_scale=7.5,
        width=768,
        height=768,
    ).images[0]

    image.save(args.output)
    print(f"Saved to {args.output}")


if __name__ == "__main__":
    main()
