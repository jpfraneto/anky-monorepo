#!/usr/bin/env python3
"""
anky_encode.py — Encode a writing session into the newline-delimited format
and generate a Shipibo-inspired kené pattern that IS the writing.

Usage:
    python scripts/anky_encode.py                  # fetch from DB
    python scripts/anky_encode.py --from-file raw.txt  # from exported file

Output:
    data/encoded/{session_id}.anky    — newline-delimited keystroke stream
    data/encoded/{session_id}.png     — kené pattern encoding
    data/encoded/{session_id}_debug.png — annotated version with legend
"""

import json
import math
import os
import sys
import hashlib
from pathlib import Path

# PIL for image generation
from PIL import Image, ImageDraw, ImageFont

# ─── CONFIG ───────────────────────────────────────────────────────────────────

WIDTH = 1920
HEIGHT = 1080
BG_COLOR = (8, 6, 18)           # deep void
BORDER_MARGIN = 60

# Kingdom color palette (chakra-aligned)
KINGDOM_COLORS = {
    "Primordia":  (204, 51, 51),     # root red
    "Emblazion":  (227, 111, 30),    # sacral orange
    "Chryseos":   (230, 195, 30),    # solar gold
    "Eleasis":    (46, 184, 76),     # heart green
    "Voxlumis":   (41, 150, 204),    # throat blue
    "Insightia":  (106, 61, 181),    # third eye indigo
    "Claridium":  (168, 85, 200),    # crown violet
    "Poiesis":    (220, 210, 240),   # transcendent white-violet
}

DEFAULT_COLOR = (180, 140, 220)  # fallback lavender


# ─── ENCODING ─────────────────────────────────────────────────────────────────

def text_and_deltas_to_anky_stream(text: str, deltas: list[float]) -> str:
    """
    Weave raw text + keystroke deltas into the canonical .anky format (spec v2.0.0):

        Every line:  {delta} {char}
        First line:  delta is 0
        No padding. No epoch. No sentinel. File ends when it ends.

    Banned keys (Enter, Backspace, Delete) are stripped.
    Characters are literal — spaces are just spaces.
    """
    lines = []
    chars = [ch for ch in text if ch not in ('\n', '\t', '\b', '\x7f')]

    for i, ch in enumerate(chars):
        if i == 0:
            lines.append(f"0 {ch}")
        else:
            delta = int(deltas[i - 1]) if i - 1 < len(deltas) else 0
            lines.append(f"{max(0, delta)} {ch}")

    return "\n".join(lines)


def anky_stream_to_text(stream: str) -> str:
    """Decode a .anky stream back to plaintext."""
    text = []
    for line in stream.strip().split("\n"):
        parts = line.split(" ", 1)
        if len(parts) != 2:
            continue
        text.append(parts[1])
    return "".join(text)


def compute_session_hash(stream: str) -> str:
    """SHA256 of the canonical .anky stream — this goes on-chain."""
    return hashlib.sha256(stream.encode("utf-8")).hexdigest()


# ─── VISUAL ENCODING (KENÉ PATTERN) ──────────────────────────────────────────

def delta_to_hue(delta_ms: float) -> float:
    """Map keystroke delta to hue. Fast = warm (red/orange), slow = cool (blue/violet)."""
    # Clamp to 0-5000ms range
    d = max(0, min(delta_ms, 5000))
    # 0ms → 0° (red), 200ms → 60° (yellow), 500ms → 120° (green),
    # 1000ms → 200° (blue), 5000ms → 270° (violet)
    hue = (d / 5000) * 270
    return hue


def hue_to_rgb(hue: float, saturation: float = 0.85, lightness: float = 0.55) -> tuple:
    """HSL to RGB."""
    h = hue / 360
    s = saturation
    l = lightness

    if s == 0:
        r = g = b = l
    else:
        def hue2rgb(p, q, t):
            if t < 0: t += 1
            if t > 1: t -= 1
            if t < 1/6: return p + (q - p) * 6 * t
            if t < 1/2: return q
            if t < 2/3: return p + (q - p) * (2/3 - t) * 6
            return p

        q = l * (1 + s) if l < 0.5 else l + s - l * s
        p = 2 * l - q
        r = hue2rgb(p, q, h + 1/3)
        g = hue2rgb(p, q, h)
        b = hue2rgb(p, q, h - 1/3)

    return (int(r * 255), int(g * 255), int(b * 255))


def generate_kene_pattern(stream: str, kingdom: str = "Poiesis", title: str = "") -> Image.Image:
    """
    Generate a Shipibo-inspired kené pattern that encodes the writing session.

    The pattern is a spiral that unfolds from the center.
    Each keystroke is a segment whose:
        - color = delta time (warm=fast, cool=slow)
        - length = proportional to delta (pauses create longer segments)
        - the spiral path follows the writing's rhythm

    The geometry IS the writing. Point a decoder at it, read back every keystroke.
    """
    img = Image.new("RGB", (WIDTH, HEIGHT), BG_COLOR)
    draw = ImageDraw.Draw(img)

    kingdom_color = KINGDOM_COLORS.get(kingdom, DEFAULT_COLOR)

    # Parse stream
    keystrokes = []
    for line in stream.strip().split("\n"):
        parts = line.split(" ", 1)
        if len(parts) != 2:
            continue
        try:
            delta = float(parts[0])
        except ValueError:
            continue
        keystrokes.append((delta, parts[1]))

    if not keystrokes:
        return img

    n = len(keystrokes)

    # ── LAYER 1: The Spiral Path (main encoding) ──
    # Archimedean spiral from center
    cx, cy = WIDTH // 2, HEIGHT // 2
    max_radius = min(WIDTH, HEIGHT) // 2 - BORDER_MARGIN

    # Calculate total "time" for spiral parameterization
    total_time = sum(max(d, 20) for d, _ in keystrokes)  # min 20ms per step

    # Spiral parameters
    turns = 8 + n / 500  # more keystrokes = more turns

    points = []
    cumulative = 0
    for delta, char in keystrokes:
        t = cumulative / total_time  # 0 to 1
        angle = t * turns * 2 * math.pi
        radius = t * max_radius

        x = cx + radius * math.cos(angle)
        y = cy + radius * math.sin(angle)
        points.append((x, y, delta, char))

        cumulative += max(delta, 20)

    # Draw the spiral path
    for i in range(1, len(points)):
        x0, y0, _, _ = points[i - 1]
        x1, y1, delta, char = points[i]

        hue = delta_to_hue(delta)
        color = hue_to_rgb(hue)

        # Line thickness varies with delta (pauses = thicker)
        thickness = max(1, min(4, int(delta / 200) + 1))

        draw.line([(x0, y0), (x1, y1)], fill=color, width=thickness)

    # ── LAYER 2: Pause markers (the "nodes" in the kené) ──
    # Mark significant pauses (>800ms) with small geometric shapes
    for x, y, delta, char in points:
        if delta > 800:
            # Size proportional to pause length
            size = min(12, int(delta / 400) + 2)
            # Diamond shape for pauses
            draw.polygon([
                (x, y - size),
                (x + size, y),
                (x, y + size),
                (x - size, y),
            ], fill=hue_to_rgb(delta_to_hue(delta), lightness=0.7),
               outline=kingdom_color)

    # ── LAYER 3: Word boundaries (the "rivers" in the kené) ──
    # Draw subtle connecting arcs between spaces
    space_points = [(x, y) for x, y, d, ch in points if ch == " "]
    for i in range(1, len(space_points), 3):
        x0, y0 = space_points[i - 1]
        x1, y1 = space_points[i]
        # Subtle arc
        draw.line([(x0, y0), (x1, y1)],
                  fill=(*kingdom_color, 40) if len(kingdom_color) == 3 else kingdom_color,
                  width=1)

    # ── LAYER 4: Border frame (kingdom identity) ──
    # Double border in kingdom color
    for offset in [0, 4]:
        m = BORDER_MARGIN - 20 + offset
        draw.rectangle(
            [(m, m), (WIDTH - m, HEIGHT - m)],
            outline=kingdom_color, width=1
        )

    # ── LAYER 5: Metadata strip at bottom ──
    # Session hash encoded as pixel colors in bottom strip
    session_hash = compute_session_hash(stream)
    hash_bytes = bytes.fromhex(session_hash)
    strip_y = HEIGHT - 30
    for i in range(0, 32, 3):
        r = hash_bytes[i] if i < 32 else 0
        g = hash_bytes[i + 1] if i + 1 < 32 else 0
        b = hash_bytes[i + 2] if i + 2 < 32 else 0
        x_start = BORDER_MARGIN + i * 8
        draw.rectangle([(x_start, strip_y), (x_start + 6, strip_y + 6)],
                       fill=(r, g, b))

    # ── LAYER 6: Stats text ──
    total_duration = sum(d for d, _ in keystrokes) / 1000
    word_count = sum(1 for _, ch in keystrokes if ch == " ") + 1

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 14)
        font_small = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 11)
        font_title = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 18)
    except (OSError, IOError):
        font = ImageFont.load_default()
        font_small = font
        font_title = font

    # Title
    if title:
        draw.text((BORDER_MARGIN + 10, BORDER_MARGIN - 15),
                  title.upper(), fill=kingdom_color, font=font_title)

    # Stats
    stats = f"{n} keystrokes | {word_count} words | {total_duration:.0f}s | {kingdom}"
    draw.text((BORDER_MARGIN + 10, HEIGHT - BORDER_MARGIN + 8),
              stats, fill=(120, 100, 140), font=font_small)

    # Hash
    draw.text((WIDTH - BORDER_MARGIN - 200, HEIGHT - BORDER_MARGIN + 8),
              f"sha256:{session_hash[:16]}...", fill=(80, 70, 100), font=font_small)

    return img


def generate_debug_image(stream: str, kingdom: str = "Poiesis") -> Image.Image:
    """Generate an annotated version showing the encoding legend."""
    img = generate_kene_pattern(stream, kingdom)
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 12)
    except (OSError, IOError):
        font = ImageFont.load_default()

    # Color legend
    legend_x = WIDTH - 250
    legend_y = 80
    draw.text((legend_x, legend_y), "DELTA → COLOR", fill=(180, 160, 200), font=font)

    deltas = [0, 50, 100, 200, 500, 1000, 2000, 5000]
    labels = ["0ms (burst)", "50ms", "100ms", "200ms (flow)", "500ms (think)",
              "1s (pause)", "2s (long pause)", "5s+ (deep pause)"]

    for i, (d, label) in enumerate(zip(deltas, labels)):
        y = legend_y + 20 + i * 18
        color = hue_to_rgb(delta_to_hue(d))
        draw.rectangle([(legend_x, y), (legend_x + 12, y + 12)], fill=color)
        draw.text((legend_x + 18, y), label, fill=(140, 120, 160), font=font)

    return img


# ─── MAIN ─────────────────────────────────────────────────────────────────────

def main():
    out_dir = Path("/home/kithkui/anky/data/encoded")
    out_dir.mkdir(parents=True, exist_ok=True)

    # Try to fetch from database
    text = None
    deltas = None
    session_id = None
    kingdom = "Poiesis"
    title = ""

    if "--from-file" in sys.argv:
        filepath = sys.argv[sys.argv.index("--from-file") + 1]
        with open(filepath) as f:
            stream = f.read()
        session_id = Path(filepath).stem
        print(f"Loaded {len(stream.splitlines())} keystrokes from {filepath}")
    else:
        # Fetch from Postgres
        try:
            import subprocess
            env_file = Path("/home/kithkui/anky/.env")
            db_url = None
            if env_file.exists():
                for line in env_file.read_text().splitlines():
                    if line.startswith("DATABASE_URL="):
                        db_url = line.split("=", 1)[1].strip().strip('"')

            if not db_url:
                print("No DATABASE_URL found in .env")
                sys.exit(1)

            query = """
                SELECT ws.id, ws.content, ws.keystroke_deltas,
                       ws.duration_seconds, ws.word_count,
                       a.title, a.kingdom_name
                FROM writing_sessions ws
                JOIN ankys a ON a.writing_session_id = ws.id
                WHERE ws.is_anky = 1
                  AND ws.keystroke_deltas IS NOT NULL
                  AND length(ws.keystroke_deltas) > 100
                ORDER BY ws.created_at DESC LIMIT 1
            """

            result = subprocess.run(
                ["psql", db_url, "-t", "-A", "-F", "|", "-c", query],
                capture_output=True, text=True
            )

            if result.returncode != 0:
                print(f"DB error: {result.stderr}")
                sys.exit(1)

            row = result.stdout.strip()
            if not row:
                print("No ankys with keystroke data found")
                sys.exit(1)

            parts = row.split("|")
            session_id = parts[0]
            text = parts[1]
            deltas = json.loads(parts[2]) if parts[2] else []
            title = parts[5] if len(parts) > 5 else ""
            kingdom = parts[6] if len(parts) > 6 and parts[6] else "Poiesis"

            print(f"Session: {session_id}")
            print(f"Title: {title}")
            print(f"Kingdom: {kingdom}")
            print(f"Text length: {len(text)} chars")
            print(f"Deltas: {len(deltas)} intervals")

        except Exception as e:
            print(f"Error fetching from DB: {e}")
            sys.exit(1)

        # Convert to stream
        stream = text_and_deltas_to_anky_stream(text, deltas)

    # Verify round-trip
    recovered = anky_stream_to_text(stream)
    if text and recovered == text:
        print("Round-trip verification: PERFECT")
    elif text:
        # Find first difference
        for i, (a, b) in enumerate(zip(recovered, text)):
            if a != b:
                print(f"Round-trip mismatch at char {i}: got '{a}' expected '{b}'")
                break
        print(f"Recovered {len(recovered)} chars vs original {len(text)} chars")

    # Save .anky stream
    stream_path = out_dir / f"{session_id}.anky"
    stream_path.write_text(stream)
    print(f"\nSaved stream: {stream_path}")
    print(f"Stream size: {len(stream.encode('utf-8'))} bytes ({len(stream.encode('utf-8')) / 1024:.1f} KB)")

    # Session hash
    session_hash = compute_session_hash(stream)
    print(f"Session hash: {session_hash}")

    # Generate kené pattern
    img = generate_kene_pattern(stream, kingdom, title)
    img_path = out_dir / f"{session_id}.png"
    img.save(str(img_path), "PNG")
    print(f"Saved kené: {img_path} ({os.path.getsize(img_path) / 1024:.0f} KB)")

    # Generate debug version
    debug_img = generate_debug_image(stream, kingdom)
    debug_path = out_dir / f"{session_id}_debug.png"
    debug_img.save(str(debug_path), "PNG")
    print(f"Saved debug: {debug_path}")

    # Print first 20 lines of stream
    print(f"\nFirst 20 keystrokes:")
    for line in stream.split("\n")[:20]:
        print(f"  {line}")

    print(f"\n... ({len(stream.splitlines())} total keystrokes)")


if __name__ == "__main__":
    main()
