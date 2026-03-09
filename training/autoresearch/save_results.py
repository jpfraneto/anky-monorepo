"""
Parse training run output and save results to Anky's SQLite database.
Called by run_daily.sh after each training run.

Usage:
    python save_results.py run.log
    python save_results.py  # reads from stdin
"""

import re
import sys
import sqlite3
import os
from datetime import datetime

DB_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "data", "anky.db")
META_PATH = os.path.join(os.path.dirname(__file__), "data", "meta.txt")


def parse_log(text):
    """Extract metrics from training output."""
    metrics = {}
    patterns = {
        "val_bpb": r"^val_bpb:\s+([\d.]+)",
        "training_seconds": r"^training_seconds:\s+([\d.]+)",
        "peak_vram_mb": r"^peak_vram_mb:\s+([\d.]+)",
        "mfu_percent": r"^mfu_percent:\s+([\d.]+)",
        "total_tokens_m": r"^total_tokens_M:\s+([\d.]+)",
        "num_steps": r"^num_steps:\s+(\d+)",
        "num_params_m": r"^num_params_M:\s+([\d.]+)",
        "depth": r"^depth:\s+(\d+)",
    }
    for key, pat in patterns.items():
        m = re.search(pat, text, re.MULTILINE)
        if m:
            metrics[key] = float(m.group(1)) if "." in m.group(1) else int(m.group(1))

    # Extract epoch from the last training step line
    epoch_match = re.findall(r"epoch:\s+(\d+)", text)
    if epoch_match:
        metrics["epochs"] = int(epoch_match[-1])

    return metrics


def parse_meta():
    """Read corpus metadata from export."""
    meta = {}
    if os.path.exists(META_PATH):
        with open(META_PATH) as f:
            for line in f:
                key, val = line.strip().split("=", 1)
                meta[key] = int(val) if val.isdigit() else val
    return meta


def save(metrics, meta, db_path):
    """Insert training run into database."""
    conn = sqlite3.connect(db_path)

    # Ensure table exists
    conn.execute("""
        CREATE TABLE IF NOT EXISTS llm_training_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_date TEXT NOT NULL,
            val_bpb REAL NOT NULL,
            training_seconds REAL NOT NULL,
            peak_vram_mb REAL NOT NULL,
            mfu_percent REAL NOT NULL,
            total_tokens_m REAL NOT NULL,
            num_steps INTEGER NOT NULL,
            num_params_m REAL NOT NULL,
            depth INTEGER NOT NULL,
            corpus_sessions INTEGER NOT NULL,
            corpus_words INTEGER NOT NULL,
            corpus_tokens INTEGER NOT NULL,
            epochs INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'complete',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    """)

    run_date = datetime.now().strftime("%Y-%m-%d")

    conn.execute(
        """INSERT OR REPLACE INTO llm_training_runs
           (run_date, val_bpb, training_seconds, peak_vram_mb, mfu_percent,
            total_tokens_m, num_steps, num_params_m, depth,
            corpus_sessions, corpus_words, corpus_tokens, epochs, status)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'complete')""",
        (
            run_date,
            metrics.get("val_bpb", 0),
            metrics.get("training_seconds", 0),
            metrics.get("peak_vram_mb", 0),
            metrics.get("mfu_percent", 0),
            metrics.get("total_tokens_m", 0),
            metrics.get("num_steps", 0),
            metrics.get("num_params_m", 0),
            metrics.get("depth", 0),
            meta.get("sessions", 0),
            meta.get("total_words", 0),
            int(meta.get("total_chars", 0)) // 4,  # estimate tokens
            metrics.get("epochs", 0),
        ),
    )
    conn.commit()
    conn.close()
    print(f"Saved training run for {run_date}: val_bpb={metrics.get('val_bpb', '?')}")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        with open(sys.argv[1]) as f:
            text = f.read()
    else:
        text = sys.stdin.read()

    metrics = parse_log(text)
    if "val_bpb" not in metrics:
        print("ERROR: Could not find val_bpb in output. Training may have crashed.")
        sys.exit(1)

    meta = parse_meta()
    save(metrics, meta, DB_PATH)
