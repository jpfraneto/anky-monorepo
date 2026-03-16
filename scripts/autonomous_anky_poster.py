#!/usr/bin/env python3
"""
Queue-driven Instagram poster for Anky.

Source of truth:
- docs/marketing/instagram_queue.json

Persistent state:
- docs/marketing/instagram_queue_state.json

This script does not generate new Ankys. It publishes the next unposted queue item.
"""

from __future__ import annotations

import argparse
import json
import os
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse

import requests
from PIL import Image, ImageDraw, ImageFilter, ImageFont


REPO_ROOT = Path(__file__).resolve().parents[1]
HERMES_ENV = Path.home() / ".hermes" / ".env"
ANKY_ENV = REPO_ROOT / ".env"
ENV_EXAMPLE = REPO_ROOT / ".env.example"
QUEUE_PATH = REPO_ROOT / "docs" / "marketing" / "instagram_queue.json"
STATE_PATH = REPO_ROOT / "docs" / "marketing" / "instagram_queue_state.json"
IMAGES_DIR = REPO_ROOT / "data" / "images"
STATIC_DIR = REPO_ROOT / "static" / "autonomous"
BRAND_FONT_PATH = REPO_ROOT / "static" / "fonts" / "Righteous-Regular.ttf"

CANVAS_WIDTH = 1080
CANVAS_HEIGHT = 1350
CARD_RADIUS = 34

THEME_TINTS = {
    "intro": (13, 24, 49, 148),
    "insight": (26, 15, 43, 158),
    "practice": (10, 37, 42, 154),
    "cta": (40, 23, 8, 162),
}


def load_env() -> None:
    for env_path in (ANKY_ENV, HERMES_ENV):
        if not env_path.exists():
            continue
        for line in env_path.read_text().splitlines():
            if "=" not in line or line.strip().startswith("#"):
                continue
            key, val = line.strip().split("=", 1)
            os.environ.setdefault(key, val)


def load_queue() -> list[dict]:
    if not QUEUE_PATH.exists():
        raise FileNotFoundError(f"Queue file not found: {QUEUE_PATH}")
    return json.loads(QUEUE_PATH.read_text())


def load_state() -> dict:
    if not STATE_PATH.exists():
        return {"posted": {}}
    try:
        data = json.loads(STATE_PATH.read_text())
    except json.JSONDecodeError:
        return {"posted": {}}
    if "posted" not in data or not isinstance(data["posted"], dict):
        data["posted"] = {}
    return data


def save_state(state: dict) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(state, indent=2, ensure_ascii=True) + "\n")


def resolve_local_image_path(item: dict) -> Path | None:
    image_path = item.get("image_path")
    if image_path:
        candidate = Path(str(image_path).lstrip("/"))
        if candidate.is_absolute() and candidate.exists():
            return candidate
        repo_candidate = REPO_ROOT / candidate
        if repo_candidate.exists():
            return repo_candidate
        images_candidate = IMAGES_DIR / candidate.name
        if images_candidate.exists():
            return images_candidate

    image_url = item.get("image_url")
    if image_url:
        parsed = urlparse(str(image_url))
        candidate = Path(parsed.path.lstrip("/"))
        repo_candidate = REPO_ROOT / candidate
        if repo_candidate.exists():
            return repo_candidate
        images_candidate = IMAGES_DIR / candidate.name
        if images_candidate.exists():
            return images_candidate

    return None


def sanitize_caption(text: str) -> str:
    caption = str(text or "").strip()
    if len(caption) <= 2200:
        return caption
    return caption[:2197].rstrip() + "..."


def choose_queue_item(queue: list[dict], state: dict, anky_id: str | None, force: bool) -> dict:
    posted = state.get("posted", {})

    if anky_id:
        for item in queue:
            if item.get("anky_id") == anky_id:
                if not force and anky_id in posted:
                    raise RuntimeError(f"{anky_id} is already marked as posted")
                return item
        raise RuntimeError(f"{anky_id} not found in queue")

    for item in queue:
        item_id = item.get("anky_id")
        if force or item_id not in posted:
            return item

    raise RuntimeError("No unposted Instagram queue items remain")


def ensure_instagram_env() -> tuple[str, str]:
    ig_token = os.getenv("INSTAGRAM_ACCESS_TOKEN", "").strip()
    ig_user_id = os.getenv("INSTAGRAM_USER_ID", "").strip()

    missing = []
    if not ig_token:
        missing.append("INSTAGRAM_ACCESS_TOKEN")
    if not ig_user_id:
        missing.append("INSTAGRAM_USER_ID")

    if missing:
        raise RuntimeError(
            "Missing Instagram environment: "
            + ", ".join(missing)
            + f". Add them to {ANKY_ENV} (preferred) or {HERMES_ENV}. "
            + f"Template: {ENV_EXAMPLE}"
        )

    return ig_user_id, ig_token


def load_font(size: int, *, brand: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = []
    if brand:
        candidates.append(BRAND_FONT_PATH)
    candidates.extend(
        [
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
            Path("/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"),
            Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            Path("/usr/share/fonts/TTF/DejaVuSans.ttf"),
        ]
    )
    for path in candidates:
        if path.exists():
            return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


FONT_EYEBROW = load_font(34)
FONT_HEADLINE = load_font(68, brand=True)
FONT_BODY = load_font(34)
FONT_FOOTER = load_font(28)
FONT_INDEX = load_font(26)


def fit_cover(image: Image.Image, width: int, height: int) -> Image.Image:
    src_w, src_h = image.size
    scale = max(width / src_w, height / src_h)
    new_w = max(int(src_w * scale), 1)
    new_h = max(int(src_h * scale), 1)
    resized = image.resize((new_w, new_h), Image.Resampling.LANCZOS)
    left = max((new_w - width) // 2, 0)
    top = max((new_h - height) // 2, 0)
    return resized.crop((left, top, left + width, top + height))


def rounded_mask(size: tuple[int, int], radius: int) -> Image.Image:
    mask = Image.new("L", size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size[0], size[1]), radius=radius, fill=255)
    return mask


def draw_wrapped_text(
    draw: ImageDraw.ImageDraw,
    *,
    text: str,
    font: ImageFont.FreeTypeFont | ImageFont.ImageFont,
    fill: tuple[int, int, int],
    x: int,
    y: int,
    max_width: int,
    line_spacing: int,
) -> int:
    if not text:
        return y

    words = text.split()
    lines: list[str] = []
    current = ""

    for word in words:
        test = f"{current} {word}".strip()
        if draw.textlength(test, font=font) <= max_width:
            current = test
            continue
        if current:
            lines.append(current)
        current = word

    if current:
        lines.append(current)

    for line in lines:
        draw.text((x, y), line, font=font, fill=fill)
        bbox = draw.textbbox((x, y), line, font=font)
        y = bbox[3] + line_spacing

    return y


def render_carousel_slide(
    *,
    local_image: Path,
    slide: dict,
    title: str,
    slide_index: int,
    slide_total: int,
    output_path: Path,
) -> None:
    theme = slide.get("theme", "intro")
    tint = THEME_TINTS.get(theme, THEME_TINTS["intro"])

    source = Image.open(local_image).convert("RGB")
    background = fit_cover(source, CANVAS_WIDTH, CANVAS_HEIGHT).filter(ImageFilter.GaussianBlur(16))
    canvas = background.convert("RGBA")

    tint_layer = Image.new("RGBA", canvas.size, tint)
    canvas = Image.alpha_composite(canvas, tint_layer)

    card = Image.new("RGBA", (CANVAS_WIDTH - 96, CANVAS_HEIGHT - 120), (8, 12, 22, 132))
    card_mask = rounded_mask(card.size, CARD_RADIUS)
    canvas.paste(card, (48, 60), card_mask)

    image_box = (120, 118, CANVAS_WIDTH - 120, 530)
    hero = fit_cover(source, image_box[2] - image_box[0], image_box[3] - image_box[1]).convert("RGBA")
    hero_mask = rounded_mask(hero.size, 28)
    hero.putalpha(hero_mask)
    canvas.alpha_composite(hero, dest=(image_box[0], image_box[1]))

    hero_overlay = Image.new("RGBA", hero.size, (0, 0, 0, 0))
    ImageDraw.Draw(hero_overlay).rounded_rectangle(
        (0, 0, hero.size[0], hero.size[1]),
        radius=28,
        fill=(4, 8, 16, 10),
    )
    canvas.alpha_composite(hero_overlay, dest=(image_box[0], image_box[1]))

    draw = ImageDraw.Draw(canvas)
    draw.rounded_rectangle(
        (image_box[0] - 2, image_box[1] - 2, image_box[2] + 2, image_box[3] + 2),
        radius=30,
        outline=(232, 238, 250, 56),
        width=2,
    )
    draw.rounded_rectangle((120, 575, CANVAS_WIDTH - 120, CANVAS_HEIGHT - 120), radius=28, fill=(8, 11, 18, 205))

    accent = (255, 200, 104)
    headline_fill = (243, 247, 255)
    body_fill = (204, 216, 235)
    footer_fill = (155, 173, 197)

    counter = f"{slide_index}/{slide_total}"
    draw.text((CANVAS_WIDTH - 140, 96), counter, font=FONT_INDEX, fill=(229, 235, 249))
    draw.text((120, 96), "ankydotapp", font=FONT_INDEX, fill=(184, 201, 255))

    y = 620
    eyebrow = str(slide.get("eyebrow", "")).upper()
    if eyebrow:
        draw.text((148, y), eyebrow, font=FONT_EYEBROW, fill=accent)
        y += 64

    headline = str(slide.get("headline", "")).strip() or title
    y = draw_wrapped_text(
        draw,
        text=headline,
        font=FONT_HEADLINE,
        fill=headline_fill,
        x=148,
        y=y,
        max_width=CANVAS_WIDTH - 296,
        line_spacing=14,
    )

    y += 10
    body = str(slide.get("body", "")).strip()
    if body:
        y = draw_wrapped_text(
            draw,
            text=body,
            font=FONT_BODY,
            fill=body_fill,
            x=148,
            y=y,
            max_width=CANVAS_WIDTH - 296,
            line_spacing=12,
        )

    footer = str(slide.get("footer", "")).strip()
    if footer:
        draw.text((148, CANVAS_HEIGHT - 174), footer, font=FONT_FOOTER, fill=footer_fill)

    draw.text((CANVAS_WIDTH - 148, CANVAS_HEIGHT - 174), "anky.app", font=FONT_FOOTER, fill=footer_fill, anchor="ra")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(output_path, format="JPEG", quality=92)


def build_fallback_carousel_slides(item: dict) -> list[dict]:
    return [
        {
            "theme": "intro",
            "eyebrow": "anky named this one",
            "headline": item.get("title", "untitled anky"),
            "body": item.get("hook", ""),
            "footer": "what i see ->",
        },
        {
            "theme": "insight",
            "eyebrow": "what i see",
            "headline": item.get("insight_heading", "") or item.get("title", "untitled anky"),
            "body": item.get("insight_body", ""),
            "footer": "do this today ->",
        },
        {
            "theme": "practice",
            "eyebrow": "do this today",
            "headline": "Stay with the part that resists.",
            "body": item.get("do_this_today", ""),
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


def stage_single_asset(local_image: Path, item: dict) -> list[dict]:
    STATIC_DIR.mkdir(parents=True, exist_ok=True)
    stem = item.get("anky_id") or local_image.stem
    suffix = local_image.suffix or ".png"
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    public_name = f"{timestamp}_{stem}{suffix}"
    web_path = STATIC_DIR / public_name
    web_path.write_bytes(local_image.read_bytes())
    return [
        {
            "local_path": str(web_path),
            "public_name": public_name,
            "public_url": f"https://anky.app/static/autonomous/{public_name}",
        }
    ]


def stage_carousel_assets(local_image: Path, item: dict) -> list[dict]:
    STATIC_DIR.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    item_id = item.get("anky_id") or local_image.stem
    slides = item.get("slides") or build_fallback_carousel_slides(item)
    staged = []

    for slide_index, slide in enumerate(slides, start=1):
        public_name = f"{timestamp}_{item_id}_slide{slide_index:02d}.jpg"
        output_path = STATIC_DIR / public_name
        render_carousel_slide(
            local_image=local_image,
            slide=slide,
            title=str(item.get("title", "untitled anky")),
            slide_index=slide_index,
            slide_total=len(slides),
            output_path=output_path,
        )
        staged.append(
            {
                "local_path": str(output_path),
                "public_name": public_name,
                "public_url": f"https://anky.app/static/autonomous/{public_name}",
            }
        )

    return staged


def stage_public_assets(local_image: Path, item: dict) -> list[dict]:
    if item.get("post_type") == "quote-carousel":
        return stage_carousel_assets(local_image, item)
    return stage_single_asset(local_image, item)


def is_transient_graph_error(payload: dict, status_code: int) -> bool:
    error = payload.get("error") if isinstance(payload, dict) else None
    if status_code >= 500:
        return True
    if not isinstance(error, dict):
        return False
    if error.get("is_transient") is True:
        return True
    return error.get("code") in {1, 2, 4, 17, 32, 613}


def graph_post(endpoint: str, *, data: dict) -> dict:
    last_payload: dict | None = None

    for attempt in range(1, 5):
        try:
            response = requests.post(endpoint, data=data, timeout=30)
            try:
                payload = response.json()
            except ValueError:
                payload = {"raw": response.text}
        except requests.RequestException as exc:
            payload = {"error": {"message": str(exc), "is_transient": True}}
            response = None

        status_code = response.status_code if response is not None else 599
        last_payload = payload

        if status_code < 400 and not payload.get("error"):
            return payload

        if attempt < 4 and is_transient_graph_error(payload, status_code):
            sleep_seconds = 2 ** (attempt - 1)
            print(f"[RETRY] Graph API transient failure, sleeping {sleep_seconds}s before retry {attempt + 1}/4")
            time.sleep(sleep_seconds)
            continue

        raise RuntimeError(f"Graph API call failed: {payload}")

    raise RuntimeError(f"Graph API call failed: {last_payload}")


def verify_public_asset(public_url: str) -> None:
    try:
        response = requests.head(public_url, allow_redirects=True, timeout=20)
        if response.status_code >= 400 or not response.headers.get("content-type", "").startswith("image/"):
            response = requests.get(public_url, stream=True, allow_redirects=True, timeout=20)
    except requests.RequestException as exc:
        raise RuntimeError(f"Public asset check failed for {public_url}: {exc}") from exc

    content_type = response.headers.get("content-type", "")
    if response.status_code >= 400:
        raise RuntimeError(f"Public asset returned HTTP {response.status_code}: {public_url}")
    if not content_type.startswith("image/"):
        raise RuntimeError(f"Public asset is not an image ({content_type}): {public_url}")


def create_image_container(public_url: str, ig_user_id: str, ig_token: str, *, caption: str | None = None, is_carousel_item: bool = False) -> str:
    payload = {
        "image_url": public_url,
        "access_token": ig_token,
    }
    if caption:
        payload["caption"] = caption
    if is_carousel_item:
        payload["is_carousel_item"] = "true"

    container = graph_post(f"https://graph.facebook.com/v25.0/{ig_user_id}/media", data=payload)
    creation_id = container.get("id")
    if not creation_id:
        raise RuntimeError(f"Instagram container creation failed: {container}")
    return creation_id


def create_carousel_container(child_ids: list[str], caption: str, ig_user_id: str, ig_token: str) -> str:
    container = graph_post(
        f"https://graph.facebook.com/v25.0/{ig_user_id}/media",
        data={
            "media_type": "CAROUSEL",
            "children": ",".join(child_ids),
            "caption": caption,
            "access_token": ig_token,
        },
    )
    creation_id = container.get("id")
    if not creation_id:
        raise RuntimeError(f"Instagram carousel container creation failed: {container}")
    return creation_id


def publish_container(creation_id: str, ig_user_id: str, ig_token: str) -> str:
    publish_resp = graph_post(
        f"https://graph.facebook.com/v25.0/{ig_user_id}/media_publish",
        data={
            "creation_id": creation_id,
            "access_token": ig_token,
        },
    )
    post_id = publish_resp.get("id")
    if not post_id:
        raise RuntimeError(f"Instagram publish failed: {publish_resp}")
    return post_id


def post_to_instagram(item: dict, staged_assets: list[dict], ig_user_id: str, ig_token: str) -> tuple[str, str, list[str]]:
    caption = sanitize_caption(item.get("caption", ""))

    if item.get("post_type") == "quote-carousel":
        child_ids = [
            create_image_container(asset["public_url"], ig_user_id, ig_token, is_carousel_item=True)
            for asset in staged_assets
        ]
        creation_id = create_carousel_container(child_ids, caption, ig_user_id, ig_token)
        post_id = publish_container(creation_id, ig_user_id, ig_token)
        return creation_id, post_id, child_ids

    creation_id = create_image_container(staged_assets[0]["public_url"], ig_user_id, ig_token, caption=caption)
    post_id = publish_container(creation_id, ig_user_id, ig_token)
    return creation_id, post_id, []


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true", help="Print the next queued post without publishing")
    parser.add_argument("--check-env", action="store_true", help="Validate required Instagram environment and exit")
    parser.add_argument("--anky-id", help="Publish a specific queued anky_id")
    parser.add_argument("--force", action="store_true", help="Allow posting a queue item already marked as posted")
    return parser


def main() -> int:
    load_env()

    args = build_parser().parse_args()

    if args.check_env:
        try:
            ig_user_id, _ = ensure_instagram_env()
        except Exception as exc:
            print(f"[ERROR] {exc}")
            return 1
        print(f"[OK] Instagram environment is configured for user {ig_user_id}")
        print(f"[SOURCE] preferred={ANKY_ENV} fallback={HERMES_ENV}")
        return 0

    try:
        queue = load_queue()
        state = load_state()
        item = choose_queue_item(queue, state, args.anky_id, args.force)
    except Exception as exc:
        print(f"[ERROR] {exc}")
        return 1

    local_image = resolve_local_image_path(item)
    if not local_image:
        print(f"[ERROR] Could not resolve local image path for queue item {item.get('anky_id')}")
        return 1

    staged_assets = stage_public_assets(local_image, item)

    print(f"[QUEUE] rank={item.get('rank')} anky_id={item.get('anky_id')} type={item.get('post_type')} title={item.get('title')}")
    print(f"[IMAGE] source={local_image}")
    for asset in staged_assets:
        print(f"[ASSET] {asset['public_url']}")
    print(f"[CAPTION]\n{sanitize_caption(item.get('caption', ''))}\n")

    if args.dry_run:
        print("[DRY RUN] Skipping Instagram publish")
        return 0

    try:
        ig_user_id, ig_token = ensure_instagram_env()
    except Exception as exc:
        print(f"[ERROR] {exc}")
        return 1

    try:
        for asset in staged_assets:
            verify_public_asset(asset["public_url"])
    except Exception as exc:
        print(f"[ERROR] {exc}")
        return 1

    try:
        creation_id, post_id, child_ids = post_to_instagram(item, staged_assets, ig_user_id, ig_token)
    except Exception as exc:
        print(f"[ERROR] {exc}")
        return 1

    state.setdefault("posted", {})
    state["posted"][item["anky_id"]] = {
        "rank": item.get("rank"),
        "title": item.get("title"),
        "post_type": item.get("post_type"),
        "posted_at": datetime.now(timezone.utc).isoformat(),
        "instagram_post_id": post_id,
        "instagram_creation_id": creation_id,
        "instagram_child_creation_ids": child_ids,
        "asset_names": [asset["public_name"] for asset in staged_assets],
        "public_urls": [asset["public_url"] for asset in staged_assets],
    }
    save_state(state)

    print(f"[POSTED] instagram_post_id={post_id}")
    print(f"[STATE] {STATE_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
