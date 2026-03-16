#!/usr/bin/env python3
"""
Build a repo-backed Instagram content queue from recent written Ankys.

Outputs:
- docs/marketing/instagram_queue.json
- docs/marketing/instagram_queue.md
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DB_PATH = REPO_ROOT / "data" / "anky.db"
OUTPUT_DIR = REPO_ROOT / "docs" / "marketing"
JSON_PATH = OUTPUT_DIR / "instagram_queue.json"
MD_PATH = OUTPUT_DIR / "instagram_queue.md"

POST_TYPES = ["still-image", "quote-carousel", "reel-text"]
SKIP_HEADINGS = {"do this today", "what i see"}


def clean_text(text: str) -> str:
    text = text.replace("\r", "")
    text = re.sub(r"^##+\s*", "", text, flags=re.MULTILINE)
    text = text.replace("**", "")
    text = re.sub(r"\s+", " ", text).strip()
    return text


def truncate(text: str, limit: int) -> str:
    text = clean_text(text)
    if len(text) <= limit:
        return text
    clipped = text[: limit - 1].rsplit(" ", 1)[0].strip()
    return (clipped or text[: limit - 1]).rstrip(" .,;:") + "…"


def extract_section(reflection: str, heading: str) -> str:
    md_pattern = rf"##\s*{re.escape(heading)}\s*(.*?)(?=\n##\s|\Z)"
    md_match = re.search(md_pattern, reflection, flags=re.IGNORECASE | re.DOTALL)
    if md_match:
        return md_match.group(1).strip()

    bold_pattern = rf"\*\*\s*{re.escape(heading)}\s*\*\*\s*(.*?)(?=\n\s*\*\*|\n##\s|\Z)"
    bold_match = re.search(bold_pattern, reflection, flags=re.IGNORECASE | re.DOTALL)
    if bold_match:
        return bold_match.group(1).strip()

    return ""


def extract_first_paragraph(text: str) -> str:
    parts = [part.strip() for part in re.split(r"\n\s*\n", text) if part.strip()]
    return clean_text(parts[0]) if parts else ""


def extract_first_insight_heading(reflection: str) -> str:
    what_i_see = extract_section(reflection, "what i see") or reflection
    for match in re.finditer(r"\*\*(.+?)\*\*", what_i_see, flags=re.DOTALL):
        heading = clean_text(match.group(1))
        if heading.lower() not in SKIP_HEADINGS:
            return heading
    return ""


def extract_first_insight_body(reflection: str) -> str:
    what_i_see = extract_section(reflection, "what i see") or reflection
    for match in re.finditer(
        r"\*\*(.+?)\*\*\s*(.*?)(?=\n\s*\*\*|\n##\s|\Z)",
        what_i_see,
        flags=re.DOTALL,
    ):
        heading = clean_text(match.group(1))
        if heading.lower() not in SKIP_HEADINGS:
            return extract_first_paragraph(match.group(2))
    return ""


def build_hook(title: str, insight_heading: str, idx: int) -> str:
    if idx % 3 == 0 and insight_heading:
        return f"anky saw this pattern: {insight_heading.lower()}"
    if idx % 3 == 1:
        return "8 minutes where you cannot edit yourself."
    return f"today's anky: {title.lower()}"


def build_overlay(title: str, insight_heading: str, action: str, idx: int) -> str:
    if idx % 3 == 0 and insight_heading:
        return truncate(insight_heading, 60)
    if idx % 3 == 1 and action:
        return truncate(action, 90)
    return truncate(title, 60)


def build_caption(
    title: str,
    hook: str,
    action: str,
    insight_heading: str,
    insight_body: str,
) -> str:
    parts = [
        hook,
        f"anky named this one: {title}",
    ]
    if insight_heading:
        parts.append(f"what i see: {insight_heading.lower()}")
    if insight_body:
        parts.append(truncate(insight_body, 260))
    if action:
        parts.append("do this today:")
        parts.append(truncate(action, 240))
    parts.append("8 minutes. no backspace. stop for 8 seconds and it ends.")
    parts.append("write: anky.app")
    return "\n\n".join(parts)


def build_carousel_slides(
    title: str,
    hook: str,
    action: str,
    insight_heading: str,
    insight_body: str,
) -> list[dict]:
    practice_body = action or "Pause before the next performance and notice who it is for."
    insight_title = insight_heading or title
    insight_copy = insight_body or hook

    return [
        {
            "theme": "intro",
            "eyebrow": "anky named this one",
            "headline": truncate(title, 48),
            "body": truncate(hook, 120),
            "footer": "what i see ->",
        },
        {
            "theme": "insight",
            "eyebrow": "what i see",
            "headline": truncate(insight_title, 72),
            "body": truncate(insight_copy, 180),
            "footer": "do this today ->",
        },
        {
            "theme": "practice",
            "eyebrow": "do this today",
            "headline": "Stay with the part that resists.",
            "body": truncate(practice_body, 190),
            "footer": "the practice ->",
        },
        {
            "theme": "cta",
            "eyebrow": "the practice",
            "headline": "8 minutes. no backspace.",
            "body": "Stop for 8 seconds and it ends.\n\nWrite at anky.app",
            "footer": "anky.app",
        },
    ]


def load_recent_written_ankys(limit: int) -> list[dict]:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    rows = conn.execute(
        """
        SELECT id, created_at, title, reflection, image_path, image_webp
        FROM ankys
        WHERE origin = 'written' AND status = 'complete'
        ORDER BY datetime(created_at) DESC
        LIMIT ?
        """,
        (limit,),
    ).fetchall()
    conn.close()
    return [dict(row) for row in rows]


def build_queue(limit: int) -> list[dict]:
    records = load_recent_written_ankys(limit)
    queue = []
    for idx, record in enumerate(records, start=1):
        reflection = record.get("reflection") or ""
        action = extract_first_paragraph(extract_section(reflection, "do this today"))
        insight_heading = extract_first_insight_heading(reflection)
        insight_body = extract_first_insight_body(reflection)
        title = clean_text(record.get("title") or "untitled anky")
        hook = build_hook(title, insight_heading, idx - 1)
        overlay = build_overlay(title, insight_heading, action, idx - 1)
        post_type = POST_TYPES[(idx - 1) % len(POST_TYPES)]
        slides = (
            build_carousel_slides(title, hook, action, insight_heading, insight_body)
            if post_type == "quote-carousel"
            else []
        )
        queue.append(
            {
                "rank": idx,
                "post_type": post_type,
                "anky_id": record["id"],
                "created_at": record["created_at"],
                "title": title,
                "image_path": record.get("image_path"),
                "image_webp": record.get("image_webp"),
                "image_url": f"https://anky.app/data/images/{record['image_path']}"
                if record.get("image_path")
                else None,
                "hook": hook,
                "overlay_text": overlay,
                "insight_heading": insight_heading,
                "insight_body": truncate(insight_body, 280),
                "do_this_today": truncate(action, 240),
                "caption": build_caption(title, hook, action, insight_heading, insight_body),
                "slides": slides,
            }
        )
    return queue


def write_outputs(queue: list[dict]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    JSON_PATH.write_text(json.dumps(queue, indent=2, ensure_ascii=True) + "\n")

    md_lines = [
        "# Instagram Queue",
        "",
        "Generated from recent `origin='written'` Ankys with `status='complete'`.",
        "",
    ]

    for item in queue:
        md_lines.extend(
            [
                f"## {item['rank']}. {item['title']}",
                "",
                f"- Post type: `{item['post_type']}`",
                f"- Created at: `{item['created_at']}`",
                f"- Anky ID: `{item['anky_id']}`",
                f"- Image: `{item['image_path']}`",
                f"- Hook: {item['hook']}",
                f"- Overlay: {item['overlay_text']}",
                f"- Insight: {item['insight_heading'] or 'n/a'}",
                f"- Do this today: {item['do_this_today'] or 'n/a'}",
                f"- Slides: `{len(item.get('slides', []))}`",
                "",
                "```text",
                item["caption"],
                "```",
                "",
            ]
        )

        if item.get("slides"):
            for idx, slide in enumerate(item["slides"], start=1):
                md_lines.extend(
                    [
                        f"Slide {idx}: `{slide['theme']}`",
                        "",
                        "```text",
                        f"{slide['eyebrow']}\n\n{slide['headline']}\n\n{slide['body']}\n\n{slide['footer']}",
                        "```",
                        "",
                    ]
                )

    MD_PATH.write_text("\n".join(md_lines))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--limit", type=int, default=9, help="Number of recent written Ankys")
    args = parser.parse_args()

    queue = build_queue(args.limit)
    write_outputs(queue)
    print(f"Wrote {len(queue)} queue items to {JSON_PATH}")
    print(f"Wrote markdown summary to {MD_PATH}")


if __name__ == "__main__":
    main()
