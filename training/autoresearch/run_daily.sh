#!/usr/bin/env bash
#
# Daily Anky autoresearch training run.
# Exports fresh writings, retrains tokenizer, runs one 5-minute training experiment.
#
# Designed to run via systemd timer at 4:00 AM Chile time.
# Uses GPU 1 (temporarily stops Ollama to free VRAM).
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ANKY_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_DIR="$SCRIPT_DIR/logs"
DATE=$(date +%Y-%m-%d)
LOG_FILE="$LOG_DIR/$DATE.log"
TRAIN_LOG="$SCRIPT_DIR/run.log"
ANKY_URL="http://localhost:8889"

mkdir -p "$LOG_DIR"

exec > >(tee -a "$LOG_FILE") 2>&1

echo "=== Anky Autoresearch Daily Run: $DATE ==="
echo "Started at $(date)"
echo ""

cd "$SCRIPT_DIR"

# --- Signal Anky: training started ---
curl -s -X POST "$ANKY_URL/api/v1/llm/training-status" \
  -H "Content-Type: application/json" \
  -d '{"status":"training"}' || true

# --- Step 1: Export fresh writings from SQLite ---
echo "[1/6] Exporting writings from database..."
uv run export_writings.py --db "$ANKY_DIR/data/anky.db"
echo ""

# --- Step 2: Retrain tokenizer on new data ---
echo "[2/6] Training tokenizer on updated corpus..."
uv run prepare.py --force
echo ""

# --- Step 3: Free GPU 1 by stopping Ollama ---
echo "[3/6] Stopping Ollama to free GPU 1..."
OLLAMA_WAS_RUNNING=false
if systemctl --user is-active ollama.service >/dev/null 2>&1; then
    systemctl --user stop ollama.service
    OLLAMA_WAS_RUNNING=true
    echo "  Ollama stopped."
    sleep 3
elif sudo systemctl is-active ollama.service >/dev/null 2>&1; then
    sudo systemctl stop ollama.service
    OLLAMA_WAS_RUNNING=true
    echo "  Ollama stopped (system service)."
    sleep 3
else
    echo "  Ollama not running as a service, checking process..."
    if pkill -f "ollama serve" 2>/dev/null; then
        OLLAMA_WAS_RUNNING=true
        echo "  Ollama process killed."
        sleep 3
    else
        echo "  Ollama not found, proceeding."
    fi
fi
echo ""

# --- Step 4: Run training on GPU 1 ---
echo "[4/6] Running 5-minute training experiment on GPU 1..."
export CUDA_VISIBLE_DEVICES=1
uv run train.py > "$TRAIN_LOG" 2>&1 || true
tail -20 "$TRAIN_LOG"
echo ""

# --- Step 5: Save results to database ---
echo "[5/6] Saving results to database..."
uv run save_results.py "$TRAIN_LOG" || echo "  WARNING: Failed to save results"
echo ""

# --- Step 6: Restart Ollama ---
echo "[6/6] Restarting Ollama..."
if [ "$OLLAMA_WAS_RUNNING" = true ]; then
    if systemctl --user cat ollama.service >/dev/null 2>&1; then
        systemctl --user start ollama.service
    elif sudo systemctl cat ollama.service >/dev/null 2>&1; then
        sudo systemctl start ollama.service
    else
        nohup ollama serve > /dev/null 2>&1 &
    fi
    echo "  Ollama restarted."
else
    echo "  Ollama was not running, skipping restart."
fi

# --- Signal Anky: training complete ---
curl -s -X POST "$ANKY_URL/api/v1/llm/training-status" \
  -H "Content-Type: application/json" \
  -d '{"status":"idle"}' || true

echo ""
echo "=== Daily run complete at $(date) ==="
echo "Log saved to $LOG_FILE"
