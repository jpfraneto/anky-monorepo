#!/usr/bin/env python3
"""
Anky training watcher — runs on RunPod alongside ai-toolkit.
Tails the training log, detects new sample images, and POSTs
live status to anky.app every 30s.

Usage (auto-started by train_anky_setup.sh):
    ANKY_TOKEN=xxx python3 /workspace/watcher.py
"""

import base64
import json
import os
import re
import time
from datetime import datetime, timezone
from pathlib import Path

import requests

ANKY_URL     = "https://anky.app/api/training/heartbeat"
ANKY_TOKEN   = os.environ.get("ANKY_TOKEN", "")
OUTPUT_DIR   = Path("/workspace/output/anky_flux_lora_v2")
LOG_FILE     = Path("/workspace/training.log")
TOTAL_STEPS  = 4500
INFERENCE_URL = os.environ.get("RUNPOD_PUBLIC_IP", "")

seen_samples = set()


def tail_log(n=80):
    if not LOG_FILE.exists():
        return ""
    lines = LOG_FILE.read_text(errors="replace").splitlines()
    return "\n".join(lines[-n:])


def parse_step_loss(log_tail):
    """Parse current step and loss from ai-toolkit output."""
    # ai-toolkit format: "  step: 100/4500 | loss: 0.1234"
    # or "step: 100, loss: 0.1234"
    patterns = [
        r"step[:\s]+(\d+)\s*/\s*\d+.*?loss[:\s]+([\d.eE+-]+)",
        r"(\d+)/4500.*?loss.*?([\d.eE+-]+)",
        r"step\s+(\d+).*?loss\s+([\d.eE+-]+)",
    ]
    for pat in patterns:
        matches = re.findall(pat, log_tail, re.IGNORECASE)
        if matches:
            step, loss = matches[-1]
            try:
                return int(step), float(loss)
            except ValueError:
                pass
    return None, None


def get_new_samples():
    """Return list of new sample images as {name, data} dicts."""
    samples_dir = OUTPUT_DIR / "samples"
    if not samples_dir.exists():
        return []

    new = []
    for img in sorted(samples_dir.glob("*.png"), key=lambda p: p.stat().st_mtime):
        if img.name not in seen_samples:
            seen_samples.add(img.name)
            try:
                b64 = base64.b64encode(img.read_bytes()).decode()
                new.append({"name": img.name, "data": b64})
                print(f"[watcher] new sample: {img.name}")
            except Exception as e:
                print(f"[watcher] failed to read sample {img.name}: {e}")
    return new


def detect_status():
    """Detect if training is done or failed from log."""
    if not LOG_FILE.exists():
        return "training"
    tail = LOG_FILE.read_text(errors="replace")[-2000:]
    if "training complete" in tail.lower() or "finished training" in tail.lower():
        return "done"
    if "error" in tail.lower() and "traceback" in tail.lower():
        return "failed"
    return "training"


def push(step, loss, samples, status):
    payload = {
        "step": step,
        "total_steps": TOTAL_STEPS,
        "loss": loss,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "log_tail": tail_log(),
        "samples": samples if samples else None,
        "status": status,
        "inference_url": INFERENCE_URL or None,
    }
    try:
        r = requests.post(
            ANKY_URL,
            json=payload,
            headers={"x-training-token": ANKY_TOKEN},
            timeout=15,
        )
        print(f"[watcher] pushed — step={step} loss={loss} status={status} → HTTP {r.status_code}")
    except Exception as e:
        print(f"[watcher] push failed: {e}")


print(f"[watcher] starting — reporting to {ANKY_URL}")
print(f"[watcher] output dir: {OUTPUT_DIR}")

while True:
    log_tail = tail_log()
    step, loss = parse_step_loss(log_tail)
    samples = get_new_samples()
    status = detect_status()

    push(step, loss, samples, status)

    if status == "done":
        print("[watcher] training complete, exiting")
        break
    if status == "failed":
        print("[watcher] training failed, exiting")
        break

    time.sleep(30)
