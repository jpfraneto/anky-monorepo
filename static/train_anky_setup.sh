#!/bin/bash
set -e

# ── Anky LoRA Training Setup v3 ─────────────────────────────────────────────
# Configurable dataset URL + train settings for fresh RunPod pods
#
# One-liner:
#   HF_TOKEN=hf_xxx ANKY_TOKEN=xxx ANKY_DATASET_URL=https://... bash <(curl -fsSL https://anky.app/static/train_anky_setup.sh)
#
# Live dashboard: https://anky.app/training/live
# ─────────────────────────────────────────────────────────────────────────────

export HF_HOME=/workspace/hf_cache
export TMPDIR=/workspace/tmp
export PIP_CACHE_DIR=/workspace/pip_cache
mkdir -p $TMPDIR $PIP_CACHE_DIR

ANKY_DATASET_URL="${ANKY_DATASET_URL:-https://anky.app/static/anky-training-data.tar.gz}"
ANKY_DATASET_MIN_IMAGES="${ANKY_DATASET_MIN_IMAGES:-300}"
TRAIN_NAME="${TRAIN_NAME:-anky_flux_lora_v2}"
LORA_RANK="${LORA_RANK:-32}"
LORA_ALPHA="${LORA_ALPHA:-16}"
TRAIN_STEPS="${TRAIN_STEPS:-4500}"
SAVE_EVERY="${SAVE_EVERY:-500}"
SAMPLE_EVERY="${SAMPLE_EVERY:-500}"
LEARNING_RATE="${LEARNING_RATE:-1e-4}"
TORCH_INDEX_URL="${TORCH_INDEX_URL:-https://download.pytorch.org/whl/cu128}"

# Prompt for required/optional tokens if not provided.
if [ -z "${HF_TOKEN:-}" ]; then
  if [ -t 0 ]; then
    echo ""
    read -r -s -p "Enter HF_TOKEN (required): " HF_TOKEN
    echo ""
  else
    echo "HF_TOKEN is required. Set it before running this script."
    exit 1
  fi
fi

if [ -z "${ANKY_TOKEN:-}" ] && [ -t 0 ]; then
  echo ""
  read -r -p "Enter ANKY_TOKEN for live dashboard updates (optional, press Enter to skip): " ANKY_TOKEN
fi

echo "=== Training settings ==="
echo "Dataset URL:        $ANKY_DATASET_URL"
echo "Min image count:    $ANKY_DATASET_MIN_IMAGES"
echo "Run name:           $TRAIN_NAME"
echo "LoRA rank/alpha:    $LORA_RANK / $LORA_ALPHA"
echo "Train steps:        $TRAIN_STEPS"
echo "Save/sample every:  $SAVE_EVERY / $SAMPLE_EVERY"
echo "Learning rate:      $LEARNING_RATE"
if [ -n "${ANKY_TOKEN:-}" ]; then
  echo "Live updates:       enabled"
else
  echo "Live updates:       disabled (ANKY_TOKEN empty)"
fi

echo "=== Step 1: Verify GPU ==="
nvidia-smi
python3 -c "
import torch
assert torch.cuda.is_available(), 'CUDA NOT AVAILABLE'
p = torch.cuda.get_device_properties(0)
print(f'GPU: {p.name}')
print(f'VRAM: {p.total_memory/1e9:.1f} GB')
"

echo "=== Step 2: Install tmux ==="
apt-get install -y tmux curl 2>/dev/null || true
apt-get install -y python3-venv python3-pip 2>/dev/null || true

echo "=== Step 3: Download dataset ==="
if [ ! -d /workspace/dataset ] || [ $(ls /workspace/dataset/*.png 2>/dev/null | wc -l) -lt "$ANKY_DATASET_MIN_IMAGES" ]; then
  cd /workspace
  rm -rf /workspace/dataset
  echo "Downloading training data with parallel connections..."
  apt-get install -y aria2 2>/dev/null || true
  if command -v aria2c &>/dev/null; then
    aria2c -x 16 -s 16 -k 10M --file-allocation=none \
      -o dataset.tar.gz "$ANKY_DATASET_URL"
  else
    curl -L --progress-bar -o dataset.tar.gz "$ANKY_DATASET_URL"
  fi
  tar xzf dataset.tar.gz --no-same-owner
  for d in training-images anky-round-two final-training-dataset-for-round-two anky-round-two-dataset; do
    if [ -d "$d" ]; then
      mv "$d" dataset
      break
    fi
  done
  if [ ! -d dataset ]; then
    DETECTED_DIR="$(find /workspace -mindepth 1 -maxdepth 1 -type d ! -name '.*' \
      -exec sh -c 'c=$(ls "$1"/*.png 2>/dev/null | wc -l); [ "$c" -gt 0 ] && echo "$1"' _ {} \; | head -n 1)"
    if [ -n "$DETECTED_DIR" ]; then
      mv "$DETECTED_DIR" /workspace/dataset
    fi
  fi
  rm -f dataset.tar.gz
  if [ ! -d /workspace/dataset ]; then
    echo "Dataset extraction failed: could not locate extracted dataset folder."
    exit 1
  fi
  echo "Dataset: $(ls dataset/*.png | wc -l) images, $(ls dataset/*.txt | wc -l) captions"
else
  echo "Dataset already present: $(ls /workspace/dataset/*.png | wc -l) images"
fi

echo "=== Step 4: Download helper scripts ==="
curl -fsSL https://anky.app/static/watcher.py        -o /workspace/watcher.py
curl -fsSL https://anky.app/static/inference_server.py -o /workspace/inference_server.py

echo "=== Step 5: Install ai-toolkit ==="
if [ ! -d /workspace/ai-toolkit ]; then
  cd /workspace
  git clone https://github.com/ostris/ai-toolkit.git
  cd ai-toolkit
  git submodule update --init --recursive
fi

echo "=== Step 6: Setup Python venv ==="
if [ ! -d /workspace/venv ]; then
  python3 -m venv /workspace/venv
fi
if [ ! -f /workspace/venv/bin/activate ]; then
  rm -rf /workspace/venv
  python3 -m venv /workspace/venv
fi
source /workspace/venv/bin/activate

pip install torch torchvision torchaudio --index-url "$TORCH_INDEX_URL" -q

cd /workspace/ai-toolkit
pip install -r requirements.txt -q

pip install torch torchvision torchaudio \
  --index-url "$TORCH_INDEX_URL" --force-reinstall --no-deps -q

pip install requests flask diffusers accelerate -q

echo "=== Step 7: Login to HuggingFace ==="
huggingface-cli login --token "$HF_TOKEN"

echo "=== Step 8: Create training config ==="
cat > /workspace/train_anky.yaml << ENDCONFIG
job: extension
config:
  name: ${TRAIN_NAME}
  process:
    - type: sd_trainer
      training_folder: /workspace/output
      device: cuda:0
      trigger_word: "anky"
      network:
        type: lora
        linear: ${LORA_RANK}
        linear_alpha: ${LORA_ALPHA}
      save:
        dtype: float16
        save_every: ${SAVE_EVERY}
        max_step_saves_to_keep: 3
      datasets:
        - folder_path: /workspace/dataset
          caption_ext: .txt
          caption_dropout_rate: 0.05
          resolution:
            - 512
            - 768
            - 1024
          batch_size: 1
          cache_latents_to_disk: true
      train:
        batch_size: 1
        steps: ${TRAIN_STEPS}
        gradient_accumulation_steps: 1
        train_unet: true
        train_text_encoder: false
        gradient_checkpointing: true
        noise_scheduler: flowmatch
        optimizer: adamw8bit
        lr: ${LEARNING_RATE}
        ema_decay: 0.999
        dtype: bf16
      model:
        name_or_path: "black-forest-labs/FLUX.1-dev"
        is_flux: true
        quantize: false
      sample:
        sampler: flowmatch
        sample_every: ${SAMPLE_EVERY}
        width: 1024
        height: 1024
        prompts:
          - "anky sitting at a worn kitchen table at dawn, blue skin, amber glowing eyes, purple swirling hair with golden accents, golden jewelry, holding a mug with both hands, warm light through a window, painterly digital art"
          - "anky standing in the rain on a city sidewalk at night, blue skin, large pointed ears, golden amber eyes, purple hair, golden jewelry, looking up at the sky with a soft expression, cinematic illustration with cool blue tones"
          - "anky dancing alone in a small apartment living room, blue skin, purple swirling hair with golden spirals, amber glowing eyes, arms raised, laughing, late afternoon golden light, warm impressionist painting style"
          - "anky sitting cross-legged on a pier at dawn, blue skin, pointed ears, amber eyes, purple hair with gold, staring at the ocean, soft watercolor style with muted blues and warm golds"
          - "anky, close portrait, blue skin, large expressive golden amber glowing eyes, big pointed ears, purple swirling hair with golden spiral accents, golden jewelry on neck and ears, ancient yet childlike expression, painterly detailed"
        neg: ""
        seed: 42
        walk_seed: true
        guidance_scale: 3.5
        sample_steps: 28
ENDCONFIG

echo "=== Step 9: Launch in tmux ==="
echo ""
echo "Starting 3 tmux windows:"
echo "  [0] training  — ai-toolkit run.py"
echo "  [1] watcher   — posts live updates to anky.app/training/live"
echo "  [2] inference — starts after training completes"
echo ""
echo "To attach later:  tmux attach -t anky"
echo "Live dashboard:   https://anky.app/training/live"
echo ""

# Kill old session if it exists
tmux kill-session -t anky 2>/dev/null || true

# Window 0: training (log to file + stdout)
tmux new-session -d -s anky -n training
tmux send-keys -t anky:training \
  "source /workspace/venv/bin/activate && \
   cd /workspace/ai-toolkit && \
   python run.py /workspace/train_anky.yaml 2>&1 | tee /workspace/training.log; \
   echo 'TRAINING DONE'" Enter

# Window 1: watcher (starts immediately, polls log file)
tmux new-window -t anky -n watcher
tmux send-keys -t anky:watcher \
  "sleep 30 && \
   source /workspace/venv/bin/activate && \
   ANKY_TOKEN='${ANKY_TOKEN}' python3 /workspace/watcher.py" Enter

# Window 2: inference server (waits for training to finish, then starts)
tmux new-window -t anky -n inference
tmux send-keys -t anky:inference \
  "echo 'Waiting for training to complete...' && \
   while ! grep -q 'TRAINING DONE' /workspace/training.log 2>/dev/null && \
         ! grep -q 'training complete' /workspace/training.log 2>/dev/null; do \
     sleep 30; done; \
   echo 'Training done — starting inference server'; \
   source /workspace/venv/bin/activate && \
   ANKY_TOKEN='${ANKY_TOKEN}' python3 /workspace/inference_server.py" Enter

# Switch back to training window
tmux select-window -t anky:training

echo "=== All launched in tmux session 'anky' ==="
echo ""
echo "Commands:"
echo "  tmux attach -t anky              # attach and watch"
echo "  tmux attach -t anky:training     # watch training log"
echo "  tmux attach -t anky:watcher      # watch heartbeat pushes"
echo "  tmux attach -t anky:inference    # watch inference server"
echo ""
echo "Live dashboard: https://anky.app/training/live"
echo ""

# Attach to the session
tmux attach -t anky
