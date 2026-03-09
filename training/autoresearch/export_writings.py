"""
Export Anky writing sessions from SQLite to parquet format for autoresearch training.

Usage:
    python export_writings.py                    # export all writings
    python export_writings.py --ankys-only       # only 8+ minute sessions
    python export_writings.py --val-ratio 0.05   # custom validation split

Outputs parquet files to data/ with a "text" column matching autoresearch's expected format.
"""

import os
import sys
import sqlite3
import argparse
import random

import pyarrow as pa
import pyarrow.parquet as pq

DB_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "data", "anky.db")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "data")


def export(db_path, output_dir, ankys_only=False, val_ratio=0.05, seed=42):
    os.makedirs(output_dir, exist_ok=True)

    conn = sqlite3.connect(db_path)
    if ankys_only:
        query = "SELECT content FROM writing_sessions WHERE is_anky = 1 AND content IS NOT NULL AND LENGTH(content) > 100 ORDER BY created_at"
    else:
        query = "SELECT content FROM writing_sessions WHERE content IS NOT NULL AND LENGTH(content) > 100 ORDER BY created_at"

    rows = conn.execute(query).fetchall()
    conn.close()

    texts = [row[0] for row in rows]
    total_chars = sum(len(t) for t in texts)
    total_words = sum(len(t.split()) for t in texts)

    print(f"Exported {len(texts)} writing sessions")
    print(f"  Total characters: {total_chars:,}")
    print(f"  Total words: {total_words:,}")
    print(f"  Estimated tokens: ~{total_chars // 4:,}")

    # Split into train and val
    random.seed(seed)
    indices = list(range(len(texts)))
    random.shuffle(indices)
    n_val = max(1, int(len(texts) * val_ratio))
    val_indices = set(indices[:n_val])

    train_texts = [texts[i] for i in range(len(texts)) if i not in val_indices]
    val_texts = [texts[i] for i in range(len(texts)) if i in val_indices]

    print(f"  Train: {len(train_texts)} sessions ({sum(len(t) for t in train_texts):,} chars)")
    print(f"  Val:   {len(val_texts)} sessions ({sum(len(t) for t in val_texts):,} chars)")

    # Write train shard
    train_table = pa.table({"text": train_texts})
    train_path = os.path.join(output_dir, "shard_00000.parquet")
    pq.write_table(train_table, train_path)
    print(f"  Wrote {train_path}")

    # Write val shard (pinned as last shard, matching autoresearch convention)
    val_table = pa.table({"text": val_texts})
    val_path = os.path.join(output_dir, "shard_00001.parquet")
    pq.write_table(val_table, val_path)
    print(f"  Wrote {val_path}")

    # Write metadata
    meta_path = os.path.join(output_dir, "meta.txt")
    with open(meta_path, "w") as f:
        f.write(f"sessions={len(texts)}\n")
        f.write(f"train_sessions={len(train_texts)}\n")
        f.write(f"val_sessions={len(val_texts)}\n")
        f.write(f"total_chars={total_chars}\n")
        f.write(f"total_words={total_words}\n")
        f.write(f"ankys_only={ankys_only}\n")

    print("\nDone. Ready for prepare.py")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", default=DB_PATH, help="Path to anky.db")
    parser.add_argument("--output", default=OUTPUT_DIR, help="Output directory")
    parser.add_argument("--ankys-only", action="store_true", help="Only export 8+ minute sessions")
    parser.add_argument("--val-ratio", type=float, default=0.05, help="Fraction for validation")
    args = parser.parse_args()
    export(args.db, args.output, args.ankys_only, args.val_ratio)
