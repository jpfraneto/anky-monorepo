# Anky Autoresearch

Training a language model from scratch on raw stream-of-consciousness writing.

## Context

The training data is NOT internet text. It is raw, unedited, stream-of-consciousness writing from Anky users — 8+ minute sessions where the writer cannot stop, edit, or backspace. This is the unfiltered train of thought. The model should learn to compress and predict this unique form of human expression.

The corpus is small (~100-200K tokens currently) but grows daily as new writing sessions are submitted. Each day, the tokenizer is retrained and a new training run begins. The model improves as the corpus grows.

## Setup

To set up a new experiment, work with the user to:

1. **Agree on a run tag**: propose a tag based on today's date (e.g. `mar8`). The branch `autoresearch/<tag>` must not already exist — this is a fresh run.
2. **Create the branch**: `git checkout -b autoresearch/<tag>` from current master.
3. **Read the in-scope files**: The repo is small. Read these files for full context:
   - `README.md` or this file — repository context.
   - `prepare.py` — fixed constants, data prep, tokenizer, dataloader, evaluation. Do not modify.
   - `train.py` — the file you modify. Model architecture, optimizer, training loop.
4. **Verify data exists**: Check that `data/` contains parquet shards and `tokenizer/` contains the trained tokenizer. If not, tell the human to run `python export_writings.py && python prepare.py`.
5. **Initialize results.tsv**: Create `results.tsv` with just the header row. The baseline will be recorded after the first run.
6. **Confirm and go**: Confirm setup looks good.

Once you get confirmation, kick off the experimentation.

## Hardware

- Single NVIDIA RTX 4090 (24GB VRAM)
- VRAM is limited — be conservative with model size
- The agent research loop uses `CUDA_VISIBLE_DEVICES=1` to target the second GPU

## Key differences from standard autoresearch

1. **Small corpus**: ~100-200K tokens. The model will see the data many times per 5-minute run. This means:
   - Overfitting is the primary risk, not underfitting
   - Regularization, dropout, and smaller models may outperform larger ones
   - The optimal architecture for this data size may differ significantly from internet-scale
2. **Domain-specific text**: Stream-of-consciousness has different statistics than edited text. Longer sentences, more repetition, emotional shifts, typos, no punctuation discipline. The tokenizer is trained on this data specifically.
3. **Daily retraining**: The corpus grows every day. Architectures that scale well with data growth are valuable.
4. **Small vocab**: 4096 tokens (vs 8192 in upstream). The domain is narrow enough that this suffices.

## Experimentation

Each experiment runs on a single GPU. The training script runs for a **fixed time budget of 5 minutes** (wall clock training time, excluding startup/compilation). You launch it simply as: `uv run train.py`.

**What you CAN do:**
- Modify `train.py` — this is the only file you edit. Everything is fair game: model architecture, optimizer, hyperparameters, training loop, batch size, model size, etc.

**What you CANNOT do:**
- Modify `prepare.py`. It is read-only. It contains the fixed evaluation, data loading, tokenizer, and training constants (time budget, sequence length, etc).
- Install new packages or add dependencies. You can only use what's already in `pyproject.toml`.
- Modify the evaluation harness. The `evaluate_bpb` function in `prepare.py` is the ground truth metric.

**The goal is simple: get the lowest val_bpb.** Since the time budget is fixed, you don't need to worry about training time — it's always 5 minutes. Everything is fair game: change the architecture, the optimizer, the hyperparameters, the batch size, the model size. The only constraint is that the code runs without crashing and finishes within the time budget.

**VRAM** is a hard constraint on RTX 4090 (24GB). OOM means the experiment is a crash.

**Simplicity criterion**: All else being equal, simpler is better. A small improvement that adds ugly complexity is not worth it. Conversely, removing something and getting equal or better results is a great outcome — that's a simplification win.

**The first run**: Your very first run should always be to establish the baseline, so you will run the training script as is.

## Output format

Once the script finishes it prints a summary like this:

```
---
val_bpb:          0.997900
training_seconds: 300.1
total_seconds:    325.9
peak_vram_mb:     18060.2
mfu_percent:      39.80
total_tokens_M:   499.6
num_steps:        953
num_params_M:     50.3
depth:            6
```

## Logging results

When an experiment is done, log it to `results.tsv` (tab-separated, NOT comma-separated).

The TSV has a header row and 5 columns:

```
commit	val_bpb	memory_gb	status	description
```

1. git commit hash (short, 7 chars)
2. val_bpb achieved (e.g. 1.234567) — use 0.000000 for crashes
3. peak memory in GB, round to .1f (e.g. 12.3 — divide peak_vram_mb by 1024) — use 0.0 for crashes
4. status: `keep`, `discard`, or `crash`
5. short text description of what this experiment tried

## The experiment loop

LOOP FOREVER:

1. Look at the git state: the current branch/commit we're on
2. Tune `train.py` with an experimental idea by directly hacking the code.
3. git commit
4. Run the experiment: `uv run train.py > run.log 2>&1`
5. Read out the results: `grep "^val_bpb:\|^peak_vram_mb:" run.log`
6. If the grep output is empty, the run crashed. Run `tail -n 50 run.log` to read the stack trace.
7. Record the results in the tsv
8. If val_bpb improved (lower), keep the git commit
9. If val_bpb is equal or worse, git reset back

**NEVER STOP**: Once the experiment loop has begun, do NOT pause to ask the human. The loop runs until manually interrupted.

## Research directions to consider

Given the unique properties of this dataset:
- **Dropout/regularization**: Critical with small corpus. Try different rates.
- **Model scaling**: The optimal depth/width for ~150K tokens is much smaller than for internet-scale.
- **Batch size tuning**: With few total tokens, gradient noise matters more.
- **Learning rate**: May need adjustment for the data scale.
- **Architecture simplifications**: With small data, simpler architectures may win.
- **Sequence length**: 2048 may be too long for typical 8-minute writings (~1000 words ≈ ~1300 tokens). Shorter sequences could help.
