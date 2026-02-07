#!/usr/bin/env python3
"""
Train a LoRA for FLUX.1-dev to learn the 'anky' concept.

Adapted from Z-Image-Turbo training script for FLUX.1-dev.
- FLUX.1-dev (FluxPipeline from diffusers)
- LoRA Rank 64, 4000 steps, 768x768, bf16
- Dual GPU: transformer on GPU 0, VAE + text encoders on GPU 1
- JSON-line progress output for Rust server parsing
"""

import os
import sys
import json
import argparse
import torch
from pathlib import Path
from diffusers import FluxPipeline, AutoencoderKL
from diffusers.optimization import get_scheduler
from peft import LoraConfig, get_peft_model
from torch.utils.data import Dataset, DataLoader
from torchvision import transforms
from PIL import Image


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset_dir", type=str, default="dataset")
    parser.add_argument("--output_dir", type=str, default="output")
    parser.add_argument("--max_train_steps", type=int, default=4000)
    parser.add_argument("--lora_rank", type=int, default=64)
    parser.add_argument("--learning_rate", type=float, default=1e-4)
    parser.add_argument("--resolution", type=int, default=768)
    parser.add_argument("--batch_size", type=int, default=1)
    parser.add_argument("--gradient_accumulation", type=int, default=4)
    parser.add_argument("--save_steps", type=int, default=500)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--gpu_id", type=int, default=0)
    parser.add_argument("--secondary_gpu_id", type=int, default=1)
    return parser.parse_args()


def log_json(**kwargs):
    """Output JSON line for Rust server to parse."""
    print(json.dumps(kwargs), flush=True)


class AnkyDataset(Dataset):
    def __init__(self, dataset_dir, resolution=768):
        self.dataset_dir = Path(dataset_dir)
        self.resolution = resolution
        self.image_files = []

        for ext in ["*.png", "*.jpg", "*.jpeg", "*.webp"]:
            for img_path in self.dataset_dir.glob(ext):
                caption_path = img_path.with_suffix(".txt")
                if caption_path.exists():
                    self.image_files.append((img_path, caption_path))

        log_json(event="dataset", pairs=len(self.image_files))

        self.transforms = transforms.Compose([
            transforms.Resize(resolution, interpolation=transforms.InterpolationMode.BILINEAR),
            transforms.CenterCrop(resolution),
            transforms.ToTensor(),
            transforms.Normalize([0.5], [0.5]),
        ])

    def __len__(self):
        return len(self.image_files)

    def __getitem__(self, idx):
        img_path, caption_path = self.image_files[idx]
        image = Image.open(img_path).convert("RGB")
        image = self.transforms(image)
        with open(caption_path, "r") as f:
            caption = f.read().strip()
        return {"pixel_values": image, "caption": caption}


def main():
    args = parse_args()
    torch.manual_seed(args.seed)

    log_json(event="start", config=vars(args))

    if torch.cuda.is_available():
        torch.cuda.empty_cache()
        for i in range(torch.cuda.device_count()):
            log_json(
                event="gpu",
                gpu=i,
                name=torch.cuda.get_device_name(i),
                memory_gb=round(torch.cuda.get_device_properties(i).total_memory / 1024**3, 1),
            )

    device = torch.device(f"cuda:{args.gpu_id}" if torch.cuda.is_available() else "cpu")

    # Load FLUX.1-dev
    log_json(event="loading", model="black-forest-labs/FLUX.1-dev")
    pipe = FluxPipeline.from_pretrained(
        "black-forest-labs/FLUX.1-dev",
        torch_dtype=torch.bfloat16,
        low_cpu_mem_usage=True,
    )
    pipe = pipe.to(device)

    # Setup LoRA
    log_json(event="lora_setup", rank=args.lora_rank)
    lora_config = LoraConfig(
        r=args.lora_rank,
        lora_alpha=args.lora_rank,
        target_modules=["to_q", "to_k", "to_v", "to_out.0"],
        lora_dropout=0.0,
        bias="none",
    )

    pipe.transformer = get_peft_model(pipe.transformer, lora_config)
    pipe.transformer.enable_gradient_checkpointing()

    if hasattr(pipe, "vae") and hasattr(pipe.vae, "enable_slicing"):
        pipe.vae.enable_slicing()

    # Freeze non-LoRA
    pipe.vae.requires_grad_(False)
    if hasattr(pipe, "text_encoder"):
        pipe.text_encoder.requires_grad_(False)
    if hasattr(pipe, "text_encoder_2"):
        pipe.text_encoder_2.requires_grad_(False)

    # Dual GPU
    if torch.cuda.device_count() > 1:
        secondary = f"cuda:{args.secondary_gpu_id}"
        pipe.vae = pipe.vae.to(secondary)
        if hasattr(pipe, "text_encoder"):
            pipe.text_encoder = pipe.text_encoder.to(secondary)
        if hasattr(pipe, "text_encoder_2"):
            pipe.text_encoder_2 = pipe.text_encoder_2.to(secondary)
        log_json(event="dual_gpu", primary=args.gpu_id, secondary=args.secondary_gpu_id)

    # Dataset
    dataset = AnkyDataset(args.dataset_dir, args.resolution)
    dataloader = DataLoader(dataset, batch_size=args.batch_size, shuffle=True, num_workers=4)

    # Optimizer
    optimizer = torch.optim.AdamW(
        pipe.transformer.parameters(),
        lr=args.learning_rate,
        betas=(0.9, 0.999),
        weight_decay=0.01,
    )

    lr_scheduler = get_scheduler(
        "constant",
        optimizer=optimizer,
        num_warmup_steps=500,
        num_training_steps=args.max_train_steps,
    )

    log_json(event="training_start", steps=args.max_train_steps)

    global_step = 0
    pipe.transformer.train()

    while global_step < args.max_train_steps:
        for batch in dataloader:
            pixel_values = batch["pixel_values"].to(device, dtype=torch.bfloat16)
            captions = batch["caption"]

            with torch.no_grad():
                vae_device = next(pipe.vae.parameters()).device
                latents = pipe.vae.encode(pixel_values.to(vae_device)).latent_dist.sample()
                latents = latents * pipe.vae.config.scaling_factor
                latents = latents.to(device)

            noise = torch.randn_like(latents)
            bsz = latents.shape[0]

            timesteps = torch.rand(bsz, device=device)
            sigmas = timesteps.view(-1, 1, 1, 1)
            noisy_latents = (1 - sigmas) * noise + sigmas * latents
            timesteps_scaled = (timesteps * 1000).long()

            with torch.no_grad():
                text_device = next(pipe.text_encoder.parameters()).device if hasattr(pipe, "text_encoder") else device
                prompt_embeds = pipe.encode_prompt(prompt=captions, device=text_device)
                if isinstance(prompt_embeds, (tuple, list)):
                    prompt_embeds = prompt_embeds[0]
                if hasattr(prompt_embeds, "to"):
                    prompt_embeds = prompt_embeds.to(device=device, dtype=torch.bfloat16)

            model_output = pipe.transformer(
                noisy_latents.to(torch.bfloat16),
                timesteps_scaled,
                prompt_embeds,
                return_dict=False,
            )

            if isinstance(model_output, (tuple, list)):
                model_pred = model_output[0]
                while isinstance(model_pred, (tuple, list)):
                    model_pred = model_pred[0]
            else:
                model_pred = model_output

            target = latents - noise
            loss = torch.nn.functional.mse_loss(model_pred.float(), target.float(), reduction="mean")
            loss.backward()

            if (global_step + 1) % args.gradient_accumulation == 0:
                optimizer.step()
                lr_scheduler.step()
                optimizer.zero_grad()

            global_step += 1

            # JSON progress for Rust
            log_json(
                step=global_step,
                total=args.max_train_steps,
                loss=round(loss.item(), 6),
                lr=lr_scheduler.get_last_lr()[0],
            )

            if global_step % args.save_steps == 0:
                ckpt_dir = Path(args.output_dir) / f"checkpoint-{global_step}"
                ckpt_dir.mkdir(parents=True, exist_ok=True)
                lora_dict = {k: v for k, v in pipe.transformer.state_dict().items()}
                torch.save(lora_dict, ckpt_dir / "pytorch_lora_weights.safetensors")
                log_json(event="checkpoint", step=global_step, path=str(ckpt_dir))

            if global_step >= args.max_train_steps:
                break

    # Save final
    final_dir = Path(args.output_dir) / "final"
    final_dir.mkdir(parents=True, exist_ok=True)
    lora_dict = {k: v for k, v in pipe.transformer.state_dict().items()}
    torch.save(lora_dict, final_dir / "pytorch_lora_weights.safetensors")
    with open(final_dir / "config.json", "w") as f:
        json.dump(vars(args), f, indent=2)

    log_json(event="complete", path=str(final_dir))


if __name__ == "__main__":
    main()
