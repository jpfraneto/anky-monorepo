#!/usr/bin/env python3
"""Prepare dataset for FLUX.1-dev LoRA training by merging base images with new Ankys."""

import argparse
import shutil
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base_dir", type=str, required=True, help="Base dataset directory")
    parser.add_argument("--new_dir", type=str, required=True, help="New generated images directory")
    parser.add_argument("--output_dir", type=str, required=True, help="Output merged dataset")
    args = parser.parse_args()

    output = Path(args.output_dir)
    output.mkdir(parents=True, exist_ok=True)

    count = 0

    for src_dir in [args.base_dir, args.new_dir]:
        src = Path(src_dir)
        if not src.exists():
            print(f"Skipping {src} (not found)")
            continue

        for img in src.glob("*"):
            if img.suffix.lower() in [".png", ".jpg", ".jpeg", ".webp"]:
                cap = img.with_suffix(".txt")
                if cap.exists():
                    shutil.copy2(img, output / img.name)
                    shutil.copy2(cap, output / cap.name)
                    count += 1

    print(f"Dataset prepared: {count} image-caption pairs in {output}")


if __name__ == "__main__":
    main()
