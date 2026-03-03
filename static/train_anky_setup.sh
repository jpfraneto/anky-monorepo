#!/bin/bash
set -e

# Everything lives in /workspace (the large volume disk)
export HF_HOME=/workspace/hf_cache
export TMPDIR=/workspace/tmp
export PIP_CACHE_DIR=/workspace/pip_cache
mkdir -p $TMPDIR $PIP_CACHE_DIR

echo "=== Step 1: Verify GPU ==="
nvidia-smi
python3 -c "import torch; assert torch.cuda.is_available(), 'CUDA NOT AVAILABLE'; print('GPU:', torch.cuda.get_device_name(0))"

echo "=== Step 2: Download dataset ==="
if [ ! -d /workspace/dataset ] || [ $(ls /workspace/dataset/*.png 2>/dev/null | wc -l) -lt 100 ]; then
  cd /workspace
  curl -o dataset.tar.gz https://anky.app/static/anky-training-data.tar.gz
  tar xzf dataset.tar.gz --no-same-owner
  mv training-images dataset
  echo "Dataset: $(ls dataset/*.png | wc -l) images"
else
  echo "Dataset already exists: $(ls /workspace/dataset/*.png | wc -l) images"
fi

echo "=== Step 3: Install ai-toolkit ==="
if [ ! -d /workspace/ai-toolkit ]; then
  cd /workspace
  git clone https://github.com/ostris/ai-toolkit.git
  cd ai-toolkit
  git submodule update --init --recursive
fi

echo "=== Step 4: Setup Python venv in /workspace ==="
if [ ! -d /workspace/venv ]; then
  python3 -m venv /workspace/venv
fi
source /workspace/venv/bin/activate

# Install torch 2.5.1 first (2.4.x lacks enable_gqa support for FLUX)
pip install torch==2.5.1+cu124 torchvision==0.20.1+cu124 torchaudio==2.5.1+cu124 \
  --index-url https://download.pytorch.org/whl/cu124

# Install ai-toolkit requirements (torch already pinned above, won't downgrade)
cd /workspace/ai-toolkit
pip install -r requirements.txt

# Re-pin torch in case requirements.txt pulled a different version
pip install torch==2.5.1+cu124 torchvision==0.20.1+cu124 torchaudio==2.5.1+cu124 \
  --index-url https://download.pytorch.org/whl/cu124 --force-reinstall --no-deps

echo "=== Step 5: Login to HuggingFace ==="
huggingface-cli login --token "$HF_TOKEN"

echo "=== Step 6: Create training config ==="
cat > /workspace/train_anky.yaml << 'ENDCONFIG'
job: extension
config:
  name: anky_flux_lora
  process:
    - type: sd_trainer
      training_folder: /workspace/output
      device: cuda:0
      trigger_word: "anky"
      network:
        type: lora
        linear: 16
        linear_alpha: 16
      save:
        dtype: float16
        save_every: 500
        max_step_saves_to_keep: 2
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
        steps: 3000
        gradient_accumulation_steps: 1
        train_unet: true
        train_text_encoder: false
        gradient_checkpointing: true
        noise_scheduler: flowmatch
        optimizer: adamw8bit
        lr: 1e-4
        ema_decay: 0.999
        dtype: bf16
      model:
        name_or_path: "black-forest-labs/FLUX.1-dev"
        is_flux: true
        quantize: true
      sample:
        sampler: flowmatch
        sample_every: 500
        width: 1024
        height: 1024
        prompts:
          - "anky sitting on a mountain top at sunset, golden hour lighting, mystical atmosphere"
          - "anky dancing in a cosmic void surrounded by stars and nebulae"
          - "anky meditating under a giant ancient tree, soft light filtering through leaves"
        neg: ""
        seed: 42
        walk_seed: true
        guidance_scale: 3.5
        sample_steps: 28
ENDCONFIG

echo "=== Step 7: Starting training ==="
source /workspace/venv/bin/activate
cd /workspace/ai-toolkit && python run.py /workspace/train_anky.yaml
