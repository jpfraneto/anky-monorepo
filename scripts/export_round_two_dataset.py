#!/usr/bin/env python3
"""Assemble the current Anky training dataset into a portable export.

This exporter builds a flat folder of image + .txt pairs from the sources that
are actually recoverable in this repo today:

1. `data/training-images/` (round-one approved pairs)
2. approved images in `data/generations/*/review.json`
3. caption-backed images in `data/images/`, using prompts stored in `data/anky.db`

It deduplicates by filename, preferring the first source in the list above.
If requested, it also writes a `.tar.gz` archive and can upload that archive to
Hugging Face as a dataset repo artifact.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sqlite3
import tarfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable


ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "data"
DB_PATH = DATA_DIR / "anky.db"
TRAINING_IMAGES_DIR = DATA_DIR / "training-images"
GENERATIONS_DIR = DATA_DIR / "generations"
GALLERY_DIR = DATA_DIR / "images"


@dataclass
class Pair:
    image_path: Path
    caption: str
    source: str


def normalize_caption(text: str) -> str:
    clean = (text or "").strip()
    if not clean:
        return ""
    return clean if clean.endswith("\n") else clean + "\n"


def load_training_pairs() -> Dict[str, Pair]:
    pairs: Dict[str, Pair] = {}
    if not TRAINING_IMAGES_DIR.exists():
        return pairs

    for image_path in sorted(TRAINING_IMAGES_DIR.glob("*.png")):
        caption_path = image_path.with_suffix(".txt")
        if not caption_path.exists():
            continue
        caption = normalize_caption(caption_path.read_text())
        if not caption:
            continue
        pairs[image_path.name] = Pair(image_path=image_path, caption=caption, source="training-images")

    return pairs


def iter_approved_generation_pairs() -> Iterable[Pair]:
    if not GENERATIONS_DIR.exists():
        return

    for batch_dir in sorted(p for p in GENERATIONS_DIR.iterdir() if p.is_dir()):
        review_path = batch_dir / "review.json"
        if not review_path.exists():
            continue

        review = json.loads(review_path.read_text())
        for image_id, meta in sorted(review.items()):
            if meta.get("decision") != "approved":
                continue
            image_path = batch_dir / "images" / f"{image_id}.png"
            caption_path = image_path.with_suffix(".txt")
            if not image_path.exists() or not caption_path.exists():
                continue
            caption = normalize_caption(caption_path.read_text())
            if not caption:
                continue
            yield Pair(
                image_path=image_path,
                caption=caption,
                source=f"generation:{batch_dir.name}",
            )


def load_gallery_captions() -> Dict[str, str]:
    captions: Dict[str, str] = {}
    if GALLERY_DIR.exists():
        for caption_path in GALLERY_DIR.glob("*.txt"):
            text = normalize_caption(caption_path.read_text())
            if text:
                captions[caption_path.with_suffix(".png").name] = text

    if not DB_PATH.exists():
        return captions

    conn = sqlite3.connect(DB_PATH)
    try:
        cur = conn.cursor()

        for image_path, prompt in cur.execute(
            """
            SELECT image_path, image_prompt
            FROM ankys
            WHERE image_path IS NOT NULL
              AND TRIM(image_path) <> ''
              AND image_prompt IS NOT NULL
              AND TRIM(image_prompt) <> ''
            """
        ):
            if image_path and image_path not in captions:
                captions[image_path] = normalize_caption(prompt)

        for image_path, prompt in cur.execute(
            """
            SELECT image_path, prompt_text
            FROM prompts
            WHERE image_path IS NOT NULL
              AND TRIM(image_path) <> ''
              AND prompt_text IS NOT NULL
              AND TRIM(prompt_text) <> ''
            """
        ):
            if image_path and image_path not in captions:
                captions[image_path] = normalize_caption(prompt)
    finally:
        conn.close()

    return {name: caption for name, caption in captions.items() if caption}


def iter_gallery_pairs() -> Iterable[Pair]:
    captions = load_gallery_captions()
    if not GALLERY_DIR.exists():
        return

    for image_path in sorted(GALLERY_DIR.glob("*.png")):
        caption = captions.get(image_path.name, "")
        if not caption:
            continue
        yield Pair(image_path=image_path, caption=caption, source="gallery-db")


def assemble_pairs() -> tuple[Dict[str, Pair], dict]:
    pairs: Dict[str, Pair] = {}
    stats = {
        "source_added": Counter(),
        "source_skipped_duplicate": Counter(),
        "gallery_png_total": 0,
        "gallery_captioned": 0,
    }

    gallery_caption_names = set(load_gallery_captions().keys())
    stats["gallery_png_total"] = sum(1 for _ in GALLERY_DIR.glob("*.png")) if GALLERY_DIR.exists() else 0
    stats["gallery_captioned"] = len(
        [name for name in gallery_caption_names if (GALLERY_DIR / name).exists() and name.endswith(".png")]
    )

    for name, pair in load_training_pairs().items():
        pairs[name] = pair
        stats["source_added"][pair.source] += 1

    for pair in iter_approved_generation_pairs():
        if pair.image_path.name in pairs:
            stats["source_skipped_duplicate"][pair.source] += 1
            continue
        pairs[pair.image_path.name] = pair
        stats["source_added"][pair.source] += 1

    for pair in iter_gallery_pairs():
        if pair.image_path.name in pairs:
            stats["source_skipped_duplicate"][pair.source] += 1
            continue
        pairs[pair.image_path.name] = pair
        stats["source_added"][pair.source] += 1

    return pairs, stats


def write_export(output_dir: Path, pairs: Dict[str, Pair], manifest: dict) -> None:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    for filename, pair in sorted(pairs.items()):
        target_image = output_dir / filename
        target_caption = output_dir / f"{Path(filename).stem}.txt"
        shutil.copy2(pair.image_path, target_image)
        target_caption.write_text(pair.caption)

    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def make_archive(output_dir: Path, archive_path: Path) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if archive_path.exists():
        archive_path.unlink()

    with tarfile.open(archive_path, "w:gz") as tar:
        tar.add(output_dir, arcname=output_dir.name)


def maybe_upload(archive_path: Path, repo_id: str, path_in_repo: str, token: str | None) -> None:
    if not token:
        raise SystemExit("HF_TOKEN is required to upload")

    from huggingface_hub import HfApi

    api = HfApi(token=token)
    api.upload_file(
        path_or_fileobj=str(archive_path),
        path_in_repo=path_in_repo,
        repo_id=repo_id,
        repo_type="dataset",
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        default=str(DATA_DIR / "exports" / "anky-round-two"),
        help="Directory to write the flat export into",
    )
    parser.add_argument(
        "--archive",
        default=str(DATA_DIR / "exports" / "anky-round-two.tar.gz"),
        help="Path for the tar.gz archive",
    )
    parser.add_argument(
        "--upload-to",
        help="Optional Hugging Face dataset repo id, e.g. user/anky-round-two",
    )
    parser.add_argument(
        "--path-in-repo",
        default="anky-round-two.tar.gz",
        help="Dataset repo path for the uploaded archive",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the computed counts without writing files",
    )
    args = parser.parse_args()

    pairs, stats = assemble_pairs()
    manifest = {
        "total_pairs": len(pairs),
        "sources_added": dict(stats["source_added"]),
        "sources_skipped_duplicate": dict(stats["source_skipped_duplicate"]),
        "gallery_png_total": stats["gallery_png_total"],
        "gallery_captioned": stats["gallery_captioned"],
        "gallery_missing_captions": max(stats["gallery_png_total"] - stats["gallery_captioned"], 0),
        "notes": [
            "This export only includes images with recoverable captions/prompts in the current repo.",
            "The gallery view may show more images, but uncaptioned images are excluded from LoRA export.",
        ],
    }

    print(json.dumps(manifest, indent=2))

    if args.dry_run:
        return

    output_dir = Path(args.output_dir)
    archive_path = Path(args.archive)

    write_export(output_dir, pairs, manifest)
    make_archive(output_dir, archive_path)
    print(f"Wrote export dir: {output_dir}")
    print(f"Wrote archive:    {archive_path}")

    if args.upload_to:
        maybe_upload(
            archive_path=archive_path,
            repo_id=args.upload_to,
            path_in_repo=args.path_in_repo,
            token=os.environ.get("HF_TOKEN"),
        )
        print(f"Uploaded {archive_path.name} to {args.upload_to}:{args.path_in_repo}")


if __name__ == "__main__":
    main()
