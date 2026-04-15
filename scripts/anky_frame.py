#!/usr/bin/env python3
"""
anky_frame.py — Wrap an anky image with a kené frame that encodes the writing session.

The anky image is the art. The frame IS the writing.

Usage:
    python scripts/anky_frame.py                          # latest anky from DB
    python scripts/anky_frame.py --session-id <id>        # specific session
    python scripts/anky_frame.py --image img.png --stream data.anky  # manual

Output:
    data/framed/{anky_id}_framed.png
"""

import json
import math
import os
import sys
import hashlib
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

# ─── CONFIG ───────────────────────────────────────────────────────────────────

FRAME_WIDTH = 48          # pixels of frame on each side
CORNER_SIZE = 48          # corner decoration size
BG_COLOR = (8, 6, 18)    # deep void

# Kingdom color palettes — primary + accent
KINGDOM_PALETTES = {
    "Primordia":  {"primary": (204, 51, 51),   "accent": (255, 120, 80),  "dim": (80, 20, 20)},
    "Emblazion":  {"primary": (227, 111, 30),  "accent": (255, 180, 60),  "dim": (80, 40, 10)},
    "Chryseos":   {"primary": (230, 195, 30),  "accent": (255, 230, 100), "dim": (80, 70, 10)},
    "Eleasis":    {"primary": (46, 184, 76),   "accent": (120, 230, 140), "dim": (15, 60, 25)},
    "Voxlumis":   {"primary": (41, 150, 204),  "accent": (100, 200, 255), "dim": (15, 50, 70)},
    "Insightia":  {"primary": (106, 61, 181),  "accent": (160, 120, 230), "dim": (35, 20, 60)},
    "Claridium":  {"primary": (168, 85, 200),  "accent": (210, 150, 255), "dim": (55, 28, 65)},
    "Poiesis":    {"primary": (220, 210, 240), "accent": (240, 235, 255), "dim": (70, 65, 80)},
}

DEFAULT_PALETTE = {"primary": (180, 140, 220), "accent": (220, 200, 240), "dim": (50, 40, 60)}


def delta_to_color(delta_ms: float, palette: dict) -> tuple:
    """Map keystroke delta to a color between accent (fast) and primary (medium) and dim (slow)."""
    d = max(0, min(delta_ms, 5000))

    if d < 300:
        # Fast: accent → primary
        t = d / 300
        return blend(palette["accent"], palette["primary"], t)
    else:
        # Slow: primary → dim
        t = min((d - 300) / 2700, 1.0)
        return blend(palette["primary"], palette["dim"], t)


def blend(c1, c2, t):
    """Linear interpolation between two RGB colors."""
    return tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))


def compute_session_hash(stream: str) -> str:
    return hashlib.sha256(stream.encode("utf-8")).hexdigest()


def parse_stream(stream: str) -> list:
    """Parse .anky stream into [(delta_ms, token), ...]"""
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
    return keystrokes


def generate_framed_anky(
    anky_image: Image.Image,
    stream: str,
    kingdom: str = "Poiesis",
    title: str = "",
) -> Image.Image:
    """
    Wrap anky image with a kené-encoded frame.

    The frame has 4 sides. Keystrokes flow clockwise starting top-left:
        TOP: first keystrokes (the beginning)
        RIGHT: continues down
        BOTTOM: continues right-to-left (reversed visually)
        LEFT: continues up to close the loop

    Each keystroke = a small segment of the frame border.
    Color = typing speed. Brightness = character type.
    """
    palette = KINGDOM_PALETTES.get(kingdom, DEFAULT_PALETTE)
    keystrokes = parse_stream(stream)
    n = len(keystrokes)

    if not keystrokes:
        return anky_image

    # Output dimensions: anky image + frame on all sides
    iw, ih = anky_image.size
    fw = FRAME_WIDTH
    ow, oh = iw + fw * 2, ih + fw * 2

    img = Image.new("RGB", (ow, oh), BG_COLOR)
    draw = ImageDraw.Draw(img)

    # Paste anky image in center
    img.paste(anky_image, (fw, fw))

    # ── FRAME ENCODING ──
    # Calculate perimeter in pixels
    inner_top = fw
    inner_bottom = oh - fw
    inner_left = fw
    inner_right = ow - fw

    # Perimeter path (clockwise): top → right → bottom → left
    # Each side's pixel length
    top_len = iw
    right_len = ih
    bottom_len = iw
    left_len = ih
    total_perimeter = top_len + right_len + bottom_len + left_len

    # Distribute keystrokes evenly along perimeter
    pixels_per_keystroke = total_perimeter / n

    # For each keystroke, draw its segment on the frame
    for i, (delta, token) in enumerate(keystrokes):
        # Position along perimeter
        pos = i * pixels_per_keystroke

        # Determine which side and position on that side
        if pos < top_len:
            # TOP: left to right
            x = inner_left + pos
            y_start = 0
            y_end = fw
            orientation = "horizontal"
            seg_x = x
            seg_y = y_start
            seg_w = max(1, pixels_per_keystroke)
            seg_h = fw
        elif pos < top_len + right_len:
            # RIGHT: top to bottom
            local = pos - top_len
            x_start = ow - fw
            y = inner_top + local
            orientation = "vertical"
            seg_x = x_start
            seg_y = y
            seg_w = fw
            seg_h = max(1, pixels_per_keystroke)
        elif pos < top_len + right_len + bottom_len:
            # BOTTOM: right to left
            local = pos - top_len - right_len
            x = inner_right - local
            orientation = "horizontal"
            seg_x = x - max(1, pixels_per_keystroke)
            seg_y = oh - fw
            seg_w = max(1, pixels_per_keystroke)
            seg_h = fw
        else:
            # LEFT: bottom to top
            local = pos - top_len - right_len - bottom_len
            y = inner_bottom - local
            orientation = "vertical"
            seg_x = 0
            seg_y = y - max(1, pixels_per_keystroke)
            seg_w = fw
            seg_h = max(1, pixels_per_keystroke)

        # Color from delta
        color = delta_to_color(delta, palette)

        # Draw the segment
        draw.rectangle(
            [(int(seg_x), int(seg_y)), (int(seg_x + seg_w), int(seg_y + seg_h))],
            fill=color
        )

    # ── INNER DETAIL LINES ──
    # Add thin lines within the frame that show rhythm patterns
    # Group keystrokes into "phrases" (bursts between pauses)
    phrase_starts = [0]
    for i, (delta, _) in enumerate(keystrokes):
        if delta > 600 and i > 0:
            phrase_starts.append(i)

    # Draw phrase boundary marks as thin lines across the frame
    for start_idx in phrase_starts:
        pos = start_idx * pixels_per_keystroke

        if pos < top_len:
            x = inner_left + pos
            # Vertical tick mark inside the top frame
            draw.line([(int(x), fw - 8), (int(x), fw)],
                      fill=palette["accent"], width=1)
        elif pos < top_len + right_len:
            local = pos - top_len
            y = inner_top + local
            draw.line([(ow - fw, int(y)), (ow - fw + 8, int(y))],
                      fill=palette["accent"], width=1)
        elif pos < top_len + right_len + bottom_len:
            local = pos - top_len - right_len
            x = inner_right - local
            draw.line([(int(x), oh - fw), (int(x), oh - fw + 8)],
                      fill=palette["accent"], width=1)
        else:
            local = pos - top_len - right_len - bottom_len
            y = inner_bottom - local
            draw.line([(fw - 8, int(y)), (fw, int(y))],
                      fill=palette["accent"], width=1)

    # ── CORNER MARKS ──
    # Small kingdom-colored squares at corners
    cs = 6
    corners = [
        (0, 0),                    # top-left: start
        (ow - cs, 0),             # top-right
        (ow - cs, oh - cs),       # bottom-right
        (0, oh - cs),             # bottom-left: end
    ]
    for cx, cy in corners:
        draw.rectangle([(cx, cy), (cx + cs, cy + cs)], fill=palette["primary"])

    # ── INNER BORDER LINE ──
    # Thin line between frame and image
    draw.rectangle(
        [(fw - 1, fw - 1), (ow - fw, oh - fw)],
        outline=palette["dim"], width=1
    )

    # ── SESSION HASH in bottom frame ──
    session_hash = compute_session_hash(stream)
    hash_bytes = bytes.fromhex(session_hash)

    # Encode hash as small colored dots along the bottom-left of frame
    for i in range(32):
        r, g, b = hash_bytes[i], hash_bytes[(i + 10) % 32], hash_bytes[(i + 20) % 32]
        hx = 8 + i * 4
        hy = oh - 8
        draw.rectangle([(hx, hy), (hx + 2, hy + 2)], fill=(r, g, b))

    return img


# ─── MAIN ─────────────────────────────────────────────────────────────────────

def main():
    out_dir = Path("/home/kithkui/anky/data/framed")
    out_dir.mkdir(parents=True, exist_ok=True)

    import subprocess

    # Get DB URL
    env_file = Path("/home/kithkui/anky/.env")
    db_url = None
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            if line.startswith("DATABASE_URL="):
                db_url = line.split("=", 1)[1].strip().strip('"')

    if not db_url:
        print("No DATABASE_URL found")
        sys.exit(1)

    # Determine which session
    session_id = None
    if "--session-id" in sys.argv:
        session_id = sys.argv[sys.argv.index("--session-id") + 1]

    # Query
    if session_id:
        where = f"ws.id = '{session_id}'"
    else:
        where = "ws.is_anky = 1 AND ws.keystroke_deltas IS NOT NULL AND length(ws.keystroke_deltas) > 100"

    query = f"""
        SELECT ws.id, ws.content, ws.keystroke_deltas,
               a.id as anky_id, a.title, a.kingdom_name, a.image_path
        FROM writing_sessions ws
        JOIN ankys a ON a.writing_session_id = ws.id
        WHERE {where}
          AND a.image_path IS NOT NULL
        ORDER BY ws.created_at DESC LIMIT 1
    """

    result = subprocess.run(
        ["psql", db_url, "-t", "-A", "-F", "|", "-c", query],
        capture_output=True, text=True
    )

    if result.returncode != 0 or not result.stdout.strip():
        print(f"No data found. {result.stderr}")
        sys.exit(1)

    parts = result.stdout.strip().split("|")
    ws_id = parts[0]
    text = parts[1]
    deltas = json.loads(parts[2]) if parts[2] else []
    anky_id = parts[3]
    title = parts[4] or ""
    kingdom = parts[5] or "Poiesis"
    image_filename = parts[6]

    print(f"Session:  {ws_id}")
    print(f"Anky:     {anky_id}")
    print(f"Title:    {title}")
    print(f"Kingdom:  {kingdom}")
    print(f"Chars:    {len(text)}")
    print(f"Deltas:   {len(deltas)}")

    # Build the .anky stream
    from anky_encode import text_and_deltas_to_anky_stream
    stream = text_and_deltas_to_anky_stream(text, deltas)

    print(f"Stream:   {len(stream.encode('utf-8'))} bytes")
    print(f"Hash:     {compute_session_hash(stream)[:32]}...")

    # Load anky image
    img_path = Path(f"/home/kithkui/anky/data/images/{image_filename}")
    if not img_path.exists():
        # Try just the anky_id
        img_path = Path(f"/home/kithkui/anky/data/images/{anky_id}.png")
    if not img_path.exists():
        print(f"Image not found: {img_path}")
        sys.exit(1)

    anky_img = Image.open(img_path)
    print(f"Image:    {anky_img.size[0]}x{anky_img.size[1]}")

    # Generate framed version
    framed = generate_framed_anky(anky_img, stream, kingdom, title)

    # Save
    out_path = out_dir / f"{anky_id}_framed.png"
    framed.save(str(out_path), "PNG")
    print(f"\nSaved:    {out_path} ({os.path.getsize(out_path) / 1024:.0f} KB)")
    print(f"Size:     {framed.size[0]}x{framed.size[1]}")

    # Also save a wider frame version for better visibility
    global FRAME_WIDTH
    old_fw = FRAME_WIDTH
    FRAME_WIDTH = 96
    framed_wide = generate_framed_anky(anky_img, stream, kingdom, title)
    FRAME_WIDTH = old_fw

    out_wide = out_dir / f"{anky_id}_framed_wide.png"
    framed_wide.save(str(out_wide), "PNG")
    print(f"Wide:     {out_wide} ({os.path.getsize(out_wide) / 1024:.0f} KB)")
    print(f"Size:     {framed_wide.size[0]}x{framed_wide.size[1]}")


if __name__ == "__main__":
    main()
