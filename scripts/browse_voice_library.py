#!/usr/bin/env python3
"""
Browse the ElevenLabs shared voice library for Spanish / Latin American voices.

Usage:
    python3 scripts/browse_voice_library.py                   # top 40 ranked
    python3 scripts/browse_voice_library.py --all > voices.tsv  # full dump
    python3 scripts/browse_voice_library.py --gender female --use-case narrative
    python3 scripts/browse_voice_library.py --audition <voice_id>  # preview URL

Writes a TSV you can sort in a spreadsheet.
"""
import argparse
import os
import sys
import requests
from pathlib import Path
from dotenv import load_dotenv

load_dotenv(Path.home() / "anky" / ".env")
KEY = os.getenv("ELEVENLABS_API_KEY", "")

if not KEY:
    print("Missing ELEVENLABS_API_KEY in ~/anky/.env", file=sys.stderr)
    sys.exit(1)


def fetch_all(language="es", gender=None, use_case=None, accent=None, max_pages=20):
    """Page through /v1/shared-voices. One page = 100 voices."""
    all_voices = []
    page = 0
    while page < max_pages:
        params = {
            "page_size": 100,
            "page": page,
            "language": language,
            "featured": "false",
        }
        if gender: params["gender"] = gender
        if use_case: params["use_cases"] = use_case
        if accent: params["accent"] = accent
        r = requests.get(
            "https://api.elevenlabs.io/v1/shared-voices",
            params=params,
            headers={"xi-api-key": KEY},
            timeout=30,
        )
        r.raise_for_status()
        data = r.json()
        voices = data.get("voices", [])
        if not voices:
            break
        all_voices.extend(voices)
        if not data.get("has_more"):
            break
        page += 1
    return all_voices


def score(v):
    """Rough suitability score for Anky narrator."""
    s = 0
    uses = " ".join(v.get("use_cases", []) or []).lower()
    desc = (v.get("description") or "").lower()
    if "narrative" in uses or "story" in uses or "audiobook" in uses: s += 5
    if "conversational" in uses: s += 2
    if "characters" in uses or "animation" in uses: s += 1
    # prefer warm/calm/soft descriptors
    for kw in ("warm", "calm", "soft", "gentle", "kind", "tierna", "cálida", "suave", "dulce", "amable"):
        if kw in desc: s += 2
    # avoid excited/shouting
    for kw in ("energetic", "loud", "aggressive", "enérgica", "fuerte"):
        if kw in desc: s -= 2
    # favor more usage (quality signal)
    s += min(v.get("cloned_by_count", 0) // 50, 5)
    return s


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--all", action="store_true", help="Dump everything as TSV")
    ap.add_argument("--gender", choices=["male", "female", "non-binary"])
    ap.add_argument("--accent", help="e.g. mexican, latin_american, spanish, colombian")
    ap.add_argument("--use-case", help="narrative_story, conversational, characters_animation, ...")
    ap.add_argument("--language", default="es")
    ap.add_argument("--audition", help="Print preview URL for a voice_id")
    args = ap.parse_args()

    if args.audition:
        # Fetch one voice's preview
        r = requests.get(
            "https://api.elevenlabs.io/v1/shared-voices",
            params={"search": args.audition},
            headers={"xi-api-key": KEY},
            timeout=30,
        )
        for v in r.json().get("voices", []):
            if v.get("voice_id") == args.audition:
                print(f"Name:    {v.get('name')}")
                print(f"Preview: {v.get('preview_url')}")
                print(f"Desc:    {v.get('description','')}")
                return
        print(f"Voice {args.audition} not found", file=sys.stderr)
        return

    voices = fetch_all(
        language=args.language,
        gender=args.gender,
        use_case=args.use_case,
        accent=args.accent,
    )

    if args.all:
        # TSV dump — open in a spreadsheet, sort, filter
        print("voice_id\tname\tgender\taccent\tage\tuse_cases\tdescription\tpreview_url\tcloned_by")
        for v in voices:
            print("\t".join([
                v.get("voice_id", ""),
                v.get("name", ""),
                v.get("gender", ""),
                v.get("accent", ""),
                v.get("age", ""),
                ",".join(v.get("use_cases", []) or []),
                (v.get("description", "") or "").replace("\t", " ").replace("\n", " ")[:120],
                v.get("preview_url", ""),
                str(v.get("cloned_by_count", 0)),
            ]))
        print(f"\n# {len(voices)} voices total", file=sys.stderr)
        return

    # Default: ranked top 40 for narrator picks
    ranked = sorted(voices, key=score, reverse=True)[:40]
    print(f"Top {len(ranked)} of {len(voices)} voices (language={args.language}"
          f"{', accent='+args.accent if args.accent else ''}"
          f"{', gender='+args.gender if args.gender else ''})\n")
    for v in ranked:
        print(f"[{score(v):3d}]  {v.get('voice_id')}  |  {v.get('name','?'):25s}  |  "
              f"{v.get('gender',''):8s} {v.get('accent',''):18s}  |  "
              f"{','.join(v.get('use_cases', []) or [])[:30]}")
        if v.get("description"):
            print(f"         → {v['description'][:100]}")
        if v.get("preview_url"):
            print(f"         ♪ {v['preview_url']}")
        print()


if __name__ == "__main__":
    main()
