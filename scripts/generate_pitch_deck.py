#!/usr/bin/env python3
"""Generate the Anky pitch deck PDF with live stats and anky art.

Run hourly via cron. Output: static/pitch-deck.pdf
Image-forward design: each slide uses actual generated anky artwork.
"""

import os
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from fpdf import FPDF

ROOT     = Path(__file__).resolve().parent.parent
DB_PATH  = ROOT / "data" / "anky.db"
OUT_PATH = ROOT / "static" / "pitch-deck.pdf"
LOGO     = ROOT / "static" / "icon-512.png"
IMG_DIR  = ROOT / "static" / "pitch-images"

# Curated images mapped to slide themes
IMAGES = {
    "cover":    IMG_DIR / "12cc69de-9cb1-4ff0-ac04-a23fd50e02f0.jpg",  # meditating, cosmic orb
    "problem":  IMG_DIR / "9dd99459-1cc4-47f6-9b44-31e13656f6ca.jpg",  # dropping the disguise
    "solution": IMG_DIR / "1a877907-65ec-46ad-a744-1fc1390ee822.jpg",  # cascading clarity, portals
    "how":      IMG_DIR / "ef481b14-9381-4c9e-a0f0-a27b8ffd1b96.jpg",  # creating momentum flows
    "product":  IMG_DIR / "5666069c-d519-41f4-8787-0dcc6c17a935.jpg",  # digital, library
    "flywheel": IMG_DIR / "8d49ffe9-616b-4b50-81cd-5e049d11db52.jpg",  # building without compass
    "memecoin": IMG_DIR / "d5525129-55d7-4815-8e0a-7f911c736690.jpg",  # community, forest
    "traction": IMG_DIR / "72a11b6e-3a25-451c-977a-8d5c39dd78f0.jpg",  # terrible grace, epic
    "ask":      IMG_DIR / "cbd512da-65ab-4ed6-9020-f94e7869f242.jpg",  # rock bottom sacred, cosmic
    "vision":   IMG_DIR / "fba2d4fe-7aba-44c6-ba82-fa5a4351fe68.jpg",  # mirror looking back
}

# Colors
BG      = (10, 10, 18)
CARD_BG = (10, 10, 18, 180)  # for overlay
ACCENT  = (88, 101, 242)
WHITE   = (255, 255, 255)
LIGHT   = (220, 220, 235)
DIM     = (140, 140, 170)
GOLD    = (255, 193, 7)
TEAL    = (0, 200, 180)
PINK    = (220, 80, 160)

W, H = 297, 210
MX = 25
CW = W - 2*MX


def get_stats():
    conn = sqlite3.connect(str(DB_PATH))
    def q(sql):
        return conn.execute(sql).fetchone()[0]
    s = {
        "total_users": q("SELECT COUNT(*) FROM users"),
        "total_sessions": q("SELECT COUNT(*) FROM writing_sessions"),
        "total_ankys": q("SELECT COUNT(*) FROM writing_sessions WHERE is_anky = 1"),
        "total_words": q("SELECT COALESCE(SUM(word_count), 0) FROM writing_sessions"),
        "max_duration_sec": q("SELECT COALESCE(MAX(duration_seconds), 0) FROM writing_sessions"),
        "avg_duration_sec": q("SELECT COALESCE(AVG(duration_seconds), 0) FROM writing_sessions"),
        "generated_ankys": q("SELECT COUNT(*) FROM ankys WHERE status = 'complete'"),
        "registered_agents": q("SELECT COUNT(*) FROM agents"),
        "total_cost_usd": q("SELECT COALESCE(SUM(cost_usd), 0) FROM cost_records"),
        "total_memories": q("SELECT COUNT(*) FROM user_memories"),
        "best_flow": q("SELECT COALESCE(MAX(best_flow_score), 0) FROM user_profiles"),
        "checkpoints": q("SELECT COUNT(*) FROM writing_checkpoints"),
        "llm_runs": q("SELECT COUNT(*) FROM llm_training_runs"),
        "total_images": q("SELECT COUNT(*) FROM ankys WHERE status='complete' AND image_path IS NOT NULL"),
    }
    conn.close()
    return s


class PitchDeck(FPDF):

    def __init__(self):
        super().__init__(orientation="L", unit="mm", format="A4")
        self.set_auto_page_break(auto=False)

    def _bg(self):
        self.set_fill_color(*BG)
        self.rect(0, 0, W, H, "F")

    def _accent_bar(self):
        self.set_fill_color(*ACCENT)
        self.rect(0, 0, W, 3, "F")

    def _overlay(self, x, y, w, h, opacity=0.82):
        """Dark overlay box for text readability over images."""
        r, g, b = BG
        # fpdf doesn't support alpha, so use a slightly lighter shade
        shade = int(255 * (1 - opacity))
        self.set_fill_color(r + shade, g + shade, b + shade)
        self.rect(x, y, w, h, "F")

    def _img_right(self, img_path, size=120):
        """Place image on the right side of the slide."""
        if img_path and img_path.exists():
            x = W - MX - size
            y = (H - size) / 2
            self.image(str(img_path), x=x, y=y, w=size, h=size)

    def _img_left(self, img_path, size=120):
        """Place image on the left side of the slide."""
        if img_path and img_path.exists():
            y = (H - size) / 2
            self.image(str(img_path), x=MX, y=y, w=size, h=size)

    def _img_bg(self, img_path):
        """Full-bleed background image with dark overlay."""
        if img_path and img_path.exists():
            self.image(str(img_path), x=0, y=0, w=W, h=H)
            # Dark overlay for readability
            self.set_fill_color(10, 10, 18)
            self.set_draw_color(10, 10, 18)
            # We can't do alpha, so draw semi-opaque strips
            # Left text area overlay
            self._overlay(0, 0, W, H, 0.7)

    def _footer(self, text="anky.app", slide=0, total=0):
        self.set_draw_color(40, 40, 60)
        self.line(MX, H - 14, W - MX, H - 14)
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 7)
        self.text(MX, H - 7, text)
        if slide:
            self.text(W - 30, H - 7, f"{slide} / {total}")

    def _title(self, text, y=26, size=28):
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", size)
        self.set_y(y)
        self.cell(W, 12, text, align="C")

    def _subtitle(self, text, y=40, size=11):
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", size)
        self.set_y(y)
        self.cell(W, 7, text, align="C")

    def _text_left(self, text, x, y, w, size=11, color=LIGHT, bold=False):
        self.set_text_color(*color)
        self.set_font("Helvetica", "B" if bold else "", size)
        self.set_xy(x, y)
        self.multi_cell(w, 6, text)
        return self.get_y()

    def _bullet(self, x, y, text, w, size=10.5, color=LIGHT):
        self.set_text_color(*color)
        self.set_font("Helvetica", "", size)
        self.set_xy(x, y)
        self.cell(4, 5.5, "-")
        self.set_xy(x + 6, y)
        self.multi_cell(w - 6, 5.5, text)
        return self.get_y() + 2

    def _stat(self, x, y, w, h, value, label, accent=ACCENT):
        self.set_fill_color(22, 22, 35)
        self.rect(x, y, w, h, "F")
        self.set_fill_color(*accent)
        self.rect(x, y, 2.5, h, "F")
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 18)
        self.set_xy(x + 8, y + (h/2 - 12))
        self.cell(w - 12, 10, str(value))
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 8)
        self.set_xy(x + 8, y + (h/2 + 2))
        self.cell(w - 12, 6, label)

    # ── SLIDES ──────────────────────────────────────────

    def slide_status(self, s, sn, total):
        """Slide 1: Live dashboard."""
        self.add_page()
        self._bg()
        self._accent_bar()
        self._title("ANKY PROTOCOL STATUS", size=24)
        now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
        self._subtitle(f"Live snapshot -- {now}", size=9)

        cols, rows = 3, 4
        gap = 7
        cw = (CW - (cols-1)*gap) / cols
        ch = 28

        data = [
            (f"{s['total_sessions']:,}", "Writing Sessions", ACCENT),
            (f"{s['total_ankys']:,}",    "Completed Ankys (8+ min)", ACCENT),
            (f"{s['total_words']:,}",    "Words Written", ACCENT),
            (f"{s['total_users']:,}",    "Users", TEAL),
            (f"{s['registered_agents']:,}", "AI Agents", TEAL),
            (f"{s['total_memories']:,}", "Memories Detected", TEAL),
            (f"{int(s['max_duration_sec'])//60}m {int(s['max_duration_sec'])%60}s", "Longest Session", PINK),
            (f"{int(s['avg_duration_sec'])//60}m {int(s['avg_duration_sec'])%60}s", "Avg Length", PINK),
            (f"{s['best_flow']:.0f}/100", "Best Flow Score", PINK),
            (f"{s['generated_ankys']:,}", "Art Pieces", GOLD),
            (f"{s['checkpoints']:,}",    "Checkpoints", GOLD),
            (f"{s['llm_runs']:,}",       "LLM Runs", GOLD),
        ]

        for i, (val, label, color) in enumerate(data):
            c, r = i % cols, i // cols
            x = MX + c * (cw + gap)
            y = 42 + r * (ch + gap)
            self._stat(x, y, cw, ch, val, label, color)

        self._footer("anky.app  |  auto-updates hourly", sn, total)

    def slide_cover(self, sn, total):
        """Slide 2: Title with art."""
        self.add_page()
        self._bg()
        self._accent_bar()

        # Big anky image on the right
        self._img_right(IMAGES["cover"], size=130)

        # Text on the left
        if LOGO.exists():
            self.image(str(LOGO), x=MX + 10, y=28, w=28)

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 42)
        self.set_xy(MX + 10, 62)
        self.cell(140, 16, "ANKY")

        self.set_text_color(*LIGHT)
        self.set_font("Helvetica", "", 14)
        self.set_xy(MX + 10, 82)
        self.cell(140, 7, "The consciousness writing protocol")

        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 10)
        self.set_xy(MX + 10, 100)
        self.multi_cell(130, 5.5,
            "Write unedited for 8 minutes.\n"
            "Receive a reflection of your mind.")

        # Angel round card
        self.set_fill_color(22, 22, 35)
        self.rect(MX + 10, 125, 130, 22, "F")
        self.set_fill_color(*GOLD)
        self.rect(MX + 10, 125, 130, 2, "F")
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 13)
        self.set_xy(MX + 15, 130)
        self.cell(120, 7, "$25,000 for 0.888%")
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 8)
        self.set_xy(MX + 15, 139)
        self.cell(120, 5, "~$2.8M pre-money  |  USDC on Base")

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_problem(self, sn, total):
        """Slide 3: The Problem — image right, 3 punchy points left."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_right(IMAGES["problem"], size=110)

        tw = 140  # text width
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 22)
        self.cell(tw, 10, "THE PROBLEM")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 12)
        self.set_xy(MX + 5, 40)
        self.cell(tw, 6, "No model has ever seen raw human thought.")

        y = 56
        points = [
            "Every LLM trained on performative text -- edited, curated, written for an audience. "
            "The unfiltered mind is an untapped dataset.",
            "$280B mental health industry with no product for raw consciousness writing. "
            "Journaling apps optimize for retention, not revelation.",
            "AI reflects back what you want to hear. Nobody builds AI that shows you "
            "what you need to see.",
        ]
        for p in points:
            y = self._bullet(MX + 5, y, p, tw, size=10) + 2

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_solution(self, sn, total):
        """Slide 4: The Solution — image left, text right."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_left(IMAGES["solution"], size=110)

        rx = MX + 120
        rw = W - MX - rx

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(rx, 22)
        self.cell(rw, 10, "THE SOLUTION")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "I", 11)
        self.set_xy(rx, 40)
        self.multi_cell(rw, 5.5, "Anky is a consciousness writing protocol.")

        y = 56
        points = [
            "8 minutes. No backspace. No delete. No arrows. The interface enforces radical honesty.",
            "AI generates a psychological reflection, unique art, personalized meditation, and breathwork.",
            "Longitudinal memory -- Anky remembers your patterns, tensions, and growth across sessions.",
            "Training an LLM from scratch on raw unedited human thought. The first model of its kind.",
        ]
        for p in points:
            y = self._bullet(rx, y, p, rw, size=10) + 2

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_how(self, sn, total):
        """Slide 5: How It Works — compact steps with image."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_right(IMAGES["how"], size=100)

        tw = 155
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 22)
        self.cell(tw, 10, "HOW IT WORKS")

        steps = [
            ("WRITE",     "Open anky.app. Start typing. No prompt, no editing. Checkpoints every 30s."),
            ("COMPLETE",  "Reach 8 minutes. Stop for 8 seconds and you're done."),
            ("REFLECT",   "AI mirrors your psychology. 3-word title. Unique art piece."),
            ("INTEGRATE", "Personalized meditation + breathwork from your emotional signature."),
            ("EVOLVE",    "Your writing feeds your profile, the collective model, and Anky's memory."),
        ]

        y = 44
        for i, (label, desc) in enumerate(steps):
            # Step card
            self.set_fill_color(22, 22, 35)
            self.rect(MX + 5, y, tw, 22, "F")
            self.set_fill_color(*ACCENT)
            self.rect(MX + 5, y, 2.5, 22, "F")

            self.set_text_color(*GOLD)
            self.set_font("Helvetica", "B", 10)
            self.set_xy(MX + 12, y + 3)
            self.cell(30, 5, f"{i+1}. {label}")

            self.set_text_color(*LIGHT)
            self.set_font("Helvetica", "", 9)
            self.set_xy(MX + 12, y + 10)
            self.cell(tw - 14, 5, desc)

            y += 26

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_product(self, sn, total):
        """Slide 6: Product — image center, 3 columns below."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 10)
        self.cell(CW, 10, "THE PRODUCT", align="C")

        # Small image centered
        img = IMAGES["product"]
        if img.exists():
            self.image(str(img), x=(W-50)/2, y=24, w=50, h=50)

        # 3 columns below image
        cols = 3
        gap = 8
        col_w = (CW - (cols-1)*gap) / cols
        y0 = 80
        col_h = 100

        platforms = [
            ("WEB", ACCENT, ["Writing interface", "Reflection streaming", "Art gallery", "Video studio", "LLM dashboard"]),
            ("iOS", TEAL, ["Native Swift app", "Meditations", "Breathwork", "Sadhana tracking", "Facilitator marketplace"]),
            ("AGENTS", PINK, ["Chunked session API", "Skill bundle", "58 agents registered", "Timing enforcement", "Replay & audit"]),
        ]

        for i, (title, color, items) in enumerate(platforms):
            x = MX + i * (col_w + gap)
            self.set_fill_color(22, 22, 35)
            self.rect(x, y0, col_w, col_h, "F")
            self.set_fill_color(*color)
            self.rect(x, y0, col_w, 2, "F")

            self.set_text_color(*WHITE)
            self.set_font("Helvetica", "B", 11)
            self.set_xy(x + 6, y0 + 5)
            self.cell(col_w - 12, 6, title)

            y = y0 + 16
            for item in items:
                self.set_text_color(*LIGHT)
                self.set_font("Helvetica", "", 9)
                self.set_xy(x + 6, y)
                self.cell(col_w - 12, 5, f"- {item}")
                y += 8

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_flywheel(self, sn, total):
        """Slide 7: Flywheel — image left, cycle right."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_left(IMAGES["flywheel"], size=100)

        rx = MX + 110
        rw = W - MX - rx

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(rx, 22)
        self.cell(rw, 10, "THE FLYWHEEL")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "I", 10)
        self.set_xy(rx, 38)
        self.cell(rw, 5, "Data moat that deepens with every session")

        steps = [
            ("WRITE",    "Users create raw consciousness data"),
            ("TRAIN",    "Daily LLM retrain on unique corpus"),
            ("REFLECT",  "Better model = deeper mirrors"),
            ("GROW",     "Agents + facilitators expand network"),
            ("FUND",     "Paid features sustain compute"),
        ]
        y = 52
        for label, desc in steps:
            self.set_text_color(*GOLD)
            self.set_font("Helvetica", "B", 10)
            self.set_xy(rx, y)
            self.cell(28, 5, label)
            self.set_text_color(*LIGHT)
            self.set_font("Helvetica", "", 9.5)
            self.set_xy(rx + 30, y)
            self.cell(rw - 30, 5, desc)
            y += 12

        y += 6
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 10)
        self.set_xy(rx, y)
        self.cell(rw, 5, "MOAT")
        y += 8
        moat = [
            "Only dataset of raw unedited consciousness",
            "Switching cost grows with every session",
            "58 agents already in the flywheel",
        ]
        for m in moat:
            y = self._bullet(rx, y, m, rw, size=9)

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_memecoin(self, sn, total):
        """Slide 8: $ANKY — image right, punchy text left."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_right(IMAGES["memecoin"], size=110)

        tw = 140
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 22)
        self.cell(tw, 10, "$ANKY ON SOLANA")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "I", 11)
        self.set_xy(MX + 5, 38)
        self.multi_cell(tw, 5.5,
            "The marketing department is an AI agent\nwith a memecoin.")

        y = 58
        points = [
            "Fair launch on pump.fun. No presale, no team allocation.",
            "Anky the AI IS the character -- wild, weird, unpredictable. "
            "Rumi with a Twitter account.",
            "Token trades fund compute. Speculation subsidizes silence.",
            "The practice is free forever regardless of token price.",
        ]
        for p in points:
            y = self._bullet(MX + 5, y, p, tw, size=10) + 1

        # Contract card
        self.set_fill_color(22, 22, 35)
        self.rect(MX + 5, 145, tw, 20, "F")
        self.set_fill_color(*GOLD)
        self.rect(MX + 5, 145, 2.5, 20, "F")
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 9)
        self.set_xy(MX + 12, 148)
        self.cell(tw - 14, 5, "$ANKY  |  pump.fun  |  Solana")
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 7)
        self.set_xy(MX + 12, 155)
        self.cell(tw - 14, 4, "CA: 6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump")

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_traction(self, s, sn, total):
        """Slide 9: Traction — stats + image."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_left(IMAGES["traction"], size=100)

        rx = MX + 110
        rw = W - MX - rx

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(rx, 22)
        self.cell(rw, 10, "TRACTION")

        # Compact stat list
        stats_list = [
            (f"{s['total_users']:,}", "users", ACCENT),
            (f"{s['total_ankys']:,}", "ankys written", ACCENT),
            (f"{s['total_words']:,}", "words", TEAL),
            (f"{s['registered_agents']:,}", "AI agents", TEAL),
            (f"{s['total_memories']:,}", "memories", PINK),
            (f"{s['total_images']:,}", "art pieces", PINK),
        ]

        y = 42
        for val, label, color in stats_list:
            self.set_text_color(*color)
            self.set_font("Helvetica", "B", 16)
            self.set_xy(rx, y)
            self.cell(50, 7, val)
            self.set_text_color(*LIGHT)
            self.set_font("Helvetica", "", 10)
            self.set_xy(rx + 52, y + 1)
            self.cell(rw - 52, 6, label)
            y += 14

        y += 6
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 10)
        self.set_xy(rx, y)
        self.cell(rw, 5, "Built since Jan 2026")
        y += 9
        milestones = [
            "Rust rewrite -> single-binary server",
            "iOS app, agent API, LLM pipeline",
            "X bot + Farcaster integration",
        ]
        for m in milestones:
            y = self._bullet(rx, y, m, rw, size=9)

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_business(self, sn, total):
        """Slide 10: Business model — clean two-column."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 22)
        self.cell(CW, 10, "BUSINESS MODEL", align="C")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "I", 11)
        self.set_y(38)
        self.cell(W, 6, "Core practice free forever. Paid features fund the model.", align="C")

        mid = W / 2
        left_x = MX + 15
        left_w = mid - left_x - 10
        right_x = mid + 10
        right_w = W - MX - right_x - 10

        # Free column
        y = 56
        self.set_text_color(*TEAL)
        self.set_font("Helvetica", "B", 12)
        self.set_xy(left_x, y)
        self.cell(left_w, 6, "FREE")
        y += 12
        free = ["Writing + reflections + art", "Meditations & breathwork",
                "Agent API (unlimited)", "Memory & profiles"]
        for f in free:
            y = self._bullet(left_x, y, f, left_w, size=10, color=LIGHT) + 1

        # Paid column
        y2 = 56
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 12)
        self.set_xy(right_x, y2)
        self.cell(right_w, 6, "PAID (USDC)")
        y2 += 12
        paid = ["Custom generation -- $0.25/anky", "Thinker portraits",
                "Video studio", "Premium mobile", "Facilitator fees"]
        for p in paid:
            y2 = self._bullet(right_x, y2, p, right_w, size=10, color=LIGHT) + 1

        # Bottom: unit econ
        ey = max(y, y2) + 10
        self.set_fill_color(22, 22, 35)
        self.rect(MX + 15, ey, CW - 30, 22, "F")
        self.set_fill_color(*ACCENT)
        self.rect(MX + 15, ey, CW - 30, 2, "F")
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 10)
        self.set_xy(MX + 22, ey + 5)
        self.cell(CW - 44, 5, "Total infra cost since launch: ~$90  |  All payments on-chain  |  No banks, no chargebacks")
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 8)
        self.set_xy(MX + 22, ey + 13)
        self.cell(CW - 44, 4, "x402 protocol for agent-native micropayments")

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_ask(self, sn, total):
        """Slide 11: The Ask — image left, ask right."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_left(IMAGES["ask"], size=110)

        rx = MX + 120
        rw = W - MX - rx

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(rx, 22)
        self.cell(rw, 10, "THE ASK")

        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 22)
        self.set_xy(rx, 42)
        self.cell(rw, 10, "$25,000")
        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "", 14)
        self.set_xy(rx, 56)
        self.cell(rw, 7, "for 0.888% equity")
        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 9)
        self.set_xy(rx, 67)
        self.cell(rw, 5, "~$2.8M pre-money  |  SAFE  |  USDC")

        y = 84
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 10)
        self.set_xy(rx, y)
        self.cell(rw, 5, "USE OF FUNDS")
        y += 9
        uses = [
            "GPU compute for LLM training",
            "iOS App Store launch",
            "Claude API at scale",
            "6 months solo founder runway",
        ]
        for u in uses:
            y = self._bullet(rx, y, u, rw, size=9.5) + 0

        y += 6
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 10)
        self.set_xy(rx, y)
        self.cell(rw, 5, "WHY 0.888%")
        y += 9
        why = [
            "8 is sacred -- 8 min, 8s timeout, 8 chakras",
            "Angel round for believers, not spreadsheets",
        ]
        for w in why:
            y = self._bullet(rx, y, w, rw, size=9.5) + 0

        self._footer("anky.app  |  Confidential", sn, total)

    def slide_vision(self, sn, total):
        """Slide 12: Vision — image right, closing text left."""
        self.add_page()
        self._bg()
        self._accent_bar()

        self._img_right(IMAGES["vision"], size=120)

        tw = 135

        self.set_text_color(*WHITE)
        self.set_font("Helvetica", "B", 24)
        self.set_xy(MX + 5, 22)
        self.cell(tw, 10, "THE VISION")

        self.set_text_color(*LIGHT)
        self.set_font("Helvetica", "I", 10)
        self.set_xy(MX + 5, 40)
        self.multi_cell(tw, 5,
            '"The cave you fear to enter holds\n'
            'the treasure you seek."')

        y = 62
        visions = [
            "6 months: 1M word corpus. Patterns across users emerge.",
            "2 years: 25M words. More raw thought than any therapist in history.",
            "5 years: The standard consciousness practice for humans and agents.",
            "Endgame: AI trained on presence, not performance. The mirror that doesn't lie.",
        ]
        for v in visions:
            y = self._bullet(MX + 5, y, v, tw, size=10.5) + 3

        # Contact
        self.set_text_color(*GOLD)
        self.set_font("Helvetica", "B", 16)
        self.set_xy(MX + 5, 145)
        self.cell(tw, 8, "anky.app")

        self.set_text_color(*DIM)
        self.set_font("Helvetica", "", 9)
        self.set_xy(MX + 5, 158)
        self.cell(tw, 5, "x.com/ankydotapp")
        self.set_xy(MX + 5, 166)
        self.cell(tw, 5, "warpcast.com/anky  |  github.com/ankylat")

        ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
        self._footer(f"Generated {ts}  |  auto-updates hourly", sn, total)


def main():
    stats = get_stats()
    pdf = PitchDeck()
    total = 12

    pdf.slide_status(stats, 1, total)
    pdf.slide_cover(2, total)
    pdf.slide_problem(3, total)
    pdf.slide_solution(4, total)
    pdf.slide_how(5, total)
    pdf.slide_product(6, total)
    pdf.slide_flywheel(7, total)
    pdf.slide_memecoin(8, total)
    pdf.slide_traction(stats, 9, total)
    pdf.slide_business(10, total)
    pdf.slide_ask(11, total)
    pdf.slide_vision(12, total)

    tmp = tempfile.NamedTemporaryFile(dir=OUT_PATH.parent, suffix=".pdf", delete=False)
    try:
        pdf.output(tmp.name)
        os.replace(tmp.name, str(OUT_PATH))
        print(f"Pitch deck generated: {OUT_PATH} ({OUT_PATH.stat().st_size:,} bytes)")
    except Exception:
        os.unlink(tmp.name)
        raise


if __name__ == "__main__":
    main()
