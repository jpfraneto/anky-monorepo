#!/usr/bin/env python3
"""Build grids from the best existing anky images for the pitch deck."""

from PIL import Image
from pathlib import Path
import random

IMG_DIR = Path("data/images")
OUT_DIR = Path("data/pitch-deck")
OUT_DIR.mkdir(parents=True, exist_ok=True)

# Curated list of visually striking ankys (from the review above)
CURATED = [
    "25af77f1-0312-4baa-9c24-d3e508f2cd29",  # lightning temple
    "600dc9a7-b2e7-4d81-b25f-b0eb3c8ffb87",  # golden magic temple
    "22bd93ba-cf1d-4ca6-aad2-0ca5ff1f4fc6",  # dancing temple
    "41fab5d8-4453-4958-b7ab-4e61b62fe888",  # crying cosmic
    "7c7ad5fc-41d3-4864-93f9-e459a97d041a",  # mother child story
    "f48d194e-1777-40b0-8a74-cb6770783bc3",  # chains mirrors crying
    "0e0d671e-9919-49c5-87e2-2c920e05faeb",  # typing keyboard
    "d8444e7a-3ac9-4752-9b44-c1ee0150943a",  # edge of void
    "bee3a191-29a1-4f3f-8b6c-3a44de57aa4e",  # mirrors questions
    "f7f99255-356c-492f-a50a-c740e7aaf540",  # portrait close-up
    "18078d30-a35b-4351-97f1-eab8ca9224f0",  # cosmic fire meditation
    "aa7b56c4-e28a-449f-b2be-dbe4ee9862dd",  # large detailed
    "d6eca22f-b2e2-4c8d-9de7-ace5795e8576",  # large detailed
    "a5423c81-eba6-4e19-be0d-65778e25dc99",  # large detailed
    "084a75b2-4522-482b-87ee-cca646983c82",  # large detailed
    "a5716bb9-9f58-4b09-8e2a-3c836087c6c4",  # large detailed
]


def make_grid(ids, out_name, cols, cell_size=512):
    """Build a grid from image IDs."""
    imgs = []
    for uid in ids:
        p = IMG_DIR / f"{uid}.png"
        if p.exists():
            imgs.append(Image.open(p).resize((cell_size, cell_size), Image.LANCZOS))

    if not imgs:
        print(f"  No images found for {out_name}")
        return

    rows = (len(imgs) + cols - 1) // cols
    # Pad with black if needed
    while len(imgs) < rows * cols:
        imgs.append(Image.new('RGB', (cell_size, cell_size), (0, 0, 0)))

    grid_w = cell_size * cols
    grid_h = cell_size * rows
    grid = Image.new('RGB', (grid_w, grid_h), (0, 0, 0))

    for i, img in enumerate(imgs[:rows * cols]):
        r, c = divmod(i, cols)
        grid.paste(img, (c * cell_size, r * cell_size))

    out_path = OUT_DIR / out_name
    grid.save(out_path, quality=95)
    print(f"  Saved: {out_path} ({grid_w}x{grid_h})")


def make_random_grid(n, out_name, cols, cell_size=512):
    """Build a grid from randomly selected ankys."""
    all_pngs = list(IMG_DIR.glob("*.png"))
    # Filter out thumbs and small files
    all_pngs = [p for p in all_pngs if "_thumb" not in p.name and p.stat().st_size > 100_000]

    selected = random.sample(all_pngs, min(n, len(all_pngs)))
    imgs = [Image.open(p).resize((cell_size, cell_size), Image.LANCZOS) for p in selected]

    rows = (len(imgs) + cols - 1) // cols
    grid_w = cell_size * cols
    grid_h = cell_size * rows
    grid = Image.new('RGB', (grid_w, grid_h), (0, 0, 0))

    for i, img in enumerate(imgs):
        r, c = divmod(i, cols)
        grid.paste(img, (c * cell_size, r * cell_size))

    out_path = OUT_DIR / out_name
    grid.save(out_path, quality=95)
    print(f"  Saved: {out_path} ({grid_w}x{grid_h})")


print("Building pitch deck grids from existing ankys...\n")

# 3x3 curated grid (9 best)
print("1. Curated 3x3 (9 best ankys)")
make_grid(CURATED[:9], "existing_best_3x3.png", cols=3, cell_size=512)

# 4x4 curated grid (16 best)
print("2. Curated 4x4 (16 best ankys)")
make_grid(CURATED[:16], "existing_best_4x4.png", cols=4, cell_size=512)

# 6x4 random sample (24 ankys — shows volume)
print("3. Random 6x4 grid (24 random ankys)")
make_random_grid(24, "existing_random_6x4.png", cols=6, cell_size=384)

# 8x5 massive grid (40 ankys — wall of faces)
print("4. Massive 8x5 grid (40 ankys — the meme wall)")
make_random_grid(40, "existing_massive_8x5.png", cols=8, cell_size=256)

# 3x3 random for variety
print("5. Random 3x3 (different batch)")
make_random_grid(9, "existing_random_3x3.png", cols=3, cell_size=512)

print("\nDone!")
