#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FFMPEG_BIN="${ROOT_DIR}/tools/ffmpeg-static/ffmpeg-7.0.2-amd64-static/ffmpeg"
OUT_DIR="${ROOT_DIR}/data/images/landing_gifs"
MAX_FILES="${1:-48}"

if [[ ! -x "$FFMPEG_BIN" ]]; then
  echo "ffmpeg binary not found: $FFMPEG_BIN"
  echo "Download a static build into tools/ffmpeg-static first."
  exit 1
fi

mkdir -p "$OUT_DIR"

mapfile -t paths < <(ls -1t "$ROOT_DIR"/videos/*__scene_*.mp4 | head -n "$MAX_FILES")

count=0
for p in "${paths[@]}"; do
  base="$(basename "$p" .mp4)"
  out="$OUT_DIR/${base}.gif"

  "$FFMPEG_BIN" -hide_banner -loglevel error \
    -ss 0 -t 3 -i "$p" \
    -vf "fps=6,scale=200:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=48[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5" \
    -loop 0 -y "$out"

  count=$((count + 1))
done

echo "Generated ${count} gif(s) in ${OUT_DIR}"
