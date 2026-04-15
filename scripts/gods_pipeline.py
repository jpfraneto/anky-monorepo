#!/usr/bin/env python3
"""
GODS by Anky — Full Video Pipeline

Each step produces a .md file that feeds into the next.
The station UI reads these files to show progress.

Usage:
    python3 gods_pipeline.py                    # Run full pipeline for today
    python3 gods_pipeline.py --step zeitgeist   # Run single step
    python3 gods_pipeline.py --god Cronos       # Override god choice
"""

import os
import sys
import json
import time
import hashlib

# Learned style notes from human review — prepended to every story prompt.
# Starts empty, grows via POST /api/station/review/apply.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    from prompt_overrides import LEARNED_STYLE_NOTES  # type: ignore
except Exception:
    LEARNED_STYLE_NOTES = ""
import argparse
import requests
import subprocess
from pathlib import Path
from datetime import datetime, timezone, timedelta

# ── Config ──────────────────────────────────────────────────────────────────

BASE_DIR = Path.home() / "anky"
GODS_DIR = BASE_DIR / "videos" / "gods"
PIPELINE_DIR = GODS_DIR / "pipeline"  # current run outputs

QWEN_URL = "http://localhost:8080"
COMFYUI_URL = "http://localhost:8188"
ELEVENLABS_API_KEY = os.getenv("ELEVENLABS_API_KEY", "")
GROK_API_KEY = os.getenv("GROK_API_KEY", "")
ANTHROPIC_API_KEY = os.getenv("ANTHROPIC_API_KEY", "")

KINGDOMS = {
    "Primordia":  {
        "chakra": "Root", "element": "Earth", "color": "#e74c3c",
        "cities": ["Rubicund Ridge", "Bleeding Bay", "Marsh Metropolis"],
        "lesson": "You are here. You are alive. Start there.",
        "visual": "ruined coastal stone, rust-red coral flora pushing through cracked ground, muted cinnamon dusk sky, low salt mist, ancient stone circles, broken walls half-claimed by the sea, moss in fissures, terracotta and iron tones, heavy grounded horizon",
    },
    "Emblazion":  {
        "chakra": "Sacral", "element": "Fire", "color": "#f39c12",
        "cities": ["Lava Landing", "Frond Fiesta", "Amber Atrium"],
        "lesson": "What do you want so badly it terrifies you?",
        "visual": "volcanic amber glow, slow-moving embers drifting in air, obsidian paths threaded with gold veins, warm darkness, tropical fronds backlit by lava light, dragonfruit pinks and burnt orange, steam rising from cracks, night lit by fire not stars",
    },
    "Chryseos":   {
        "chakra": "Solar", "element": "Gold", "color": "#f1c40f",
        "cities": ["Lustrous Landing", "Savanna Soiree", "Sandstone Square"],
        "lesson": "You are not waiting for permission.",
        "visual": "sun-bleached savanna, tall gold grasses, bronze sky late afternoon, long soft shadows, sandstone columns weathered smooth, acacia silhouettes, honey and saffron tones, dust motes in slanting light, heat-shimmer horizon",
    },
    "Eleasis":    {
        "chakra": "Heart", "element": "Air", "color": "#2ecc71",
        "cities": ["Grove Galleria", "Leaf Spot", "Pond Pavilion"],
        "lesson": "The wall around your heart is made of the same material as the prison.",
        "visual": "dappled green light through leaves, wind-moved grass, still pond reflections, soft morning haze, birch and willow silhouettes, petals drifting, sage and emerald tones, airy open spaces, sky the colour of fresh paper",
    },
    "Voxlumis":   {
        "chakra": "Throat", "element": "Sound", "color": "#3498db",
        "cities": ["Echo Enclave", "Sapphire Settlement", "Woodland Wharf"],
        "lesson": "Say the thing you are afraid to say. That is the one that matters.",
        "visual": "still sapphire water, reflective cave interiors, vast echoing silence made visual, blue hour light, smooth wet stones, mirror-surface lakes, cobalt and ink tones, ripples as the only movement, sound frozen as light",
    },
    "Insightia":  {
        "chakra": "Third Eye", "element": "Light", "color": "#4b0082",
        "cities": ["Maze Metropolis", "Veil Venue", "Dreamweaver's Dwelling"],
        "lesson": "You already know. You have always known.",
        "visual": "deep indigo mist, hanging gauze veils, floating candles, layered translucent curtains, soft violet glow, quiet mirrors facing each other, starlight filtered through lace, midnight plum tones, dreamlike air, soft focus edges",
    },
    "Claridium":  {
        "chakra": "Crown", "element": "Crystal", "color": "#8e44ad",
        "cities": ["Crystal City", "Ascent Arrival", "Echo Empire"],
        "lesson": "Who is the one asking who am I?",
        "visual": "clear snowfields, crystalline cliffs refracting light, prismatic rainbows in ice, cold bright clarity, pale lavender sky, crystal architecture rising like frozen music, pearl and silver tones, cold clean air made visible, high altitude stillness",
    },
    "Poiesis":    {
        "chakra": "Transcend", "element": "Creation", "color": "#e91e90",
        "cities": ["Creation City", "Inlet Island", "Muse's Metropolis"],
        "lesson": "You are not the creator. You are the channel. Get out of the way.",
        "visual": "soft nebulae in rose and gold, ribbons of coloured light weaving through ancient stone studios, stars close enough to touch, warm pink and magenta clouds, half-made sculptures in alcoves, constellations as blueprints, hushed cosmic light, a sense that creation is happening quietly all around",
    },
}

# Shared style suffix applied to every image prompt — keeps the tonal signature consistent.
IMAGE_STYLE_SUFFIX = "painterly, Studio Ghibli meets cosmic mythology, bedtime storybook, soft light, vertical portrait composition, rich tender atmosphere"

VOICES = {
    "anky":          {"voice_id": "cgSgspJ2msm6clMCkdW9", "name": "Jessica",  "role": "Anky narrator"},
    "cronos":        {"voice_id": "JBFqnCBsd6RMkjVDRZzb", "name": "George",   "role": "Greek god of time"},
    "anubis":        {"voice_id": "nPczCjzI2devNBz1zQrb", "name": "Brian",    "role": "Egyptian guide of the dead"},
    "quetzalcoatl":  {"voice_id": "cjVigY5qzO86Huf0OWal", "name": "Eric",     "role": "Aztec feathered serpent"},
    "odin":          {"voice_id": "pqHfZKP75CvOlQylNhV4", "name": "Bill",     "role": "Norse all-father"},
    "kali":          {"voice_id": "pFZP5JQG7iQjIQuC4Bku", "name": "Lily",     "role": "Hindu destroyer of illusion"},
    "ra":            {"voice_id": "IKne3meq5aSn9XLyUdCD", "name": "Charlie",  "role": "Egyptian sun god"},
    "loki":          {"voice_id": "N2lVS1w4EtoT3dr4eOWO", "name": "Callum",   "role": "Norse trickster"},
    "amaterasu":     {"voice_id": "Xb7hH8MSUJpSbSDYk0k2", "name": "Alice",    "role": "Japanese sun goddess"},
    "shiva":         {"voice_id": "pNInz6obpgDQGcFmaJgB", "name": "Adam",     "role": "Hindu cosmic dancer"},
}

# Latin American Spanish voices. ElevenLabs multilingual voices can speak Spanish;
# for Anky in ES we use a native Latin American voice (neutral, warm, ~30s female register).
# Voice IDs should be confirmed from the station voice playground — swap if a better pick emerges.
# TODO: replace these placeholder IDs with confirmed Latin American Spanish voice IDs from your ElevenLabs library.
VOICES_ES = {
    "anky":          {"voice_id": "4vqDWmE9rvDX51nxtDbo", "name": "Sofia",     "role": "Anky narrator (ES) — Latin American, soft, storytelling"},
    "cronos":        {"voice_id": "DdKbXdRlBmj7Ty7N0FVr", "name": "Juanjo",    "role": "Greek god of time (ES) — velvety baritone, commanding"},
    "shiva":         {"voice_id": "wfTWLJ20rcMqvU8gIiAB", "name": "Salvatore", "role": "Hindu cosmic dancer (ES) — warm, deep, old"},
    "odin":          {"voice_id": "bml6kfu9aNiHKYxhtGuS", "name": "Sebastián", "role": "Norse all-father (ES) — low, steady, fatherly"},
    "anubis":        {"voice_id": "ePriEYH8pb97Fpb15Xij", "name": "Lázaro",    "role": "Egyptian guide of the dead (ES) — shadowed, breathy"},
    "ra":            {"voice_id": "Y4A7VD8bsOlWNzF8WIRV", "name": "Gabriel",   "role": "Egyptian sun god (ES) — neutral Mexican, warm"},
    "quetzalcoatl":  {"voice_id": "wfTWLJ20rcMqvU8gIiAB", "name": "Salvatore", "role": "Aztec feathered serpent (ES) — warm, deep (shared)"},
    "loki":          {"voice_id": "nnTkGIqnpqpdIrWbRAtF", "name": "Lisandro",  "role": "Norse trickster (ES) — mellow Argentinian lilt"},
    "kali":          {"voice_id": "j3pzCmrdzkT5DUtyT61a", "name": "Paloma",    "role": "Hindu destroyer of illusion (ES) — low, grounded, Mexican"},
    "amaterasu":     {"voice_id": "O5hbneAmtjLMgfg5UFIm", "name": "Andrea",    "role": "Japanese sun goddess (ES) — mature, warm, Venezuelan"},
}

LANGUAGES = [
    {"code": "en", "name": "English",  "anky_greeting": "Hi kids, this is Anky.",                  "closing": "See you tomorrow.",   "full_cta": "Full story on Timeless Stories by Anky."},
    {"code": "es", "name": "Spanish",  "anky_greeting": "Hola, niños y niñas. Soy Anky.",          "closing": "Nos vemos mañana.",   "full_cta": "La historia completa en Anky."},
]


# ── Utility ─────────────────────────────────────────────────────────────────

def log(msg):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[{ts}] {msg}")
    # Also append to pipeline log
    log_path = PIPELINE_DIR / "pipeline.log"
    with open(log_path, "a") as f:
        f.write(f"[{ts}] {msg}\n")


def write_step(filename, content):
    """Write a pipeline step output as markdown."""
    path = PIPELINE_DIR / filename
    path.write_text(content, encoding="utf-8")
    log(f"Wrote {filename} ({len(content)} bytes)")
    return path


def llm_query(prompt, system="You are a helpful assistant.", temperature=0.8, max_tokens=4000):
    """Query the local Qwen LLM."""
    try:
        r = requests.post(f"{QWEN_URL}/v1/chat/completions", json={
            "model": "local",
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}
            ],
            "temperature": temperature,
            "max_tokens": max_tokens + 8000,  # extra budget for Qwen's thinking tokens
        }, timeout=300)
        r.raise_for_status()
        data = r.json()["choices"][0]["message"]
        content = data.get("content", "")
        # If content is empty but reasoning_content exists, Qwen spent all tokens thinking
        if not content and data.get("reasoning_content"):
            log("Qwen thinking overflow — retrying with /no_think tag")
            # Retry with explicit no-think instruction
            r2 = requests.post(f"{QWEN_URL}/v1/chat/completions", json={
                "model": "local",
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": prompt + "\n\n/no_think\nRespond directly without internal reasoning."}
                ],
                "temperature": temperature,
                "max_tokens": max_tokens,
            }, timeout=300)
            r2.raise_for_status()
            content = r2.json()["choices"][0]["message"].get("content", "")
        return content if content else None
    except Exception as e:
        log(f"LLM error: {e}")
        return None


def grok_query(prompt, system="You are Grok, made by xAI."):
    """Query Grok API for X/Twitter zeitgeist."""
    if not GROK_API_KEY:
        log("No GROK_API_KEY — using Qwen fallback for zeitgeist")
        return llm_query(prompt, system=system)
    try:
        r = requests.post("https://api.x.ai/v1/chat/completions", json={
            "model": "grok-3",
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 3000,
        }, headers={"Authorization": f"Bearer {GROK_API_KEY}"}, timeout=60)
        r.raise_for_status()
        return r.json()["choices"][0]["message"]["content"]
    except Exception as e:
        log(f"Grok error: {e}, falling back to Qwen")
        return llm_query(prompt, system=system)


def claude_query(prompt, system="You are Claude, an AI assistant by Anthropic."):
    """Query Claude API for deep consciousness reading."""
    if not ANTHROPIC_API_KEY:
        log("No ANTHROPIC_API_KEY — using Qwen fallback for consciousness reading")
        return llm_query(prompt, system=system)
    try:
        r = requests.post("https://api.anthropic.com/v1/messages", json={
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4000,
            "system": system,
            "messages": [{"role": "user", "content": prompt}],
        }, headers={
            "x-api-key": ANTHROPIC_API_KEY,
            "anthropic-version": "2023-06-01",
            "content-type": "application/json",
        }, timeout=120)
        r.raise_for_status()
        return r.json()["content"][0]["text"]
    except Exception as e:
        log(f"Claude error: {e}, falling back to Qwen")
        return llm_query(prompt, system=system)


HERMES_BRIDGE_URL = "http://localhost:8891"
TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN", "")
TELEGRAM_CHAT_ID = os.getenv("TELEGRAM_CHAT_ID", "")

HELIUS_RPC = "https://devnet.helius-rpc.com/?api-key=946d14ac-64ef-4251-9369-6a72724bc235"
PROGRAM_ID = "2Q3xXCd4f9nMbb2kMyg7opEncU9J638BYUU1XhM8UukH"
ENCLAVE_URL = os.getenv("ANKY_ENCLAVE_URL", "http://3.83.84.211:5555")


def get_recent_sessions_from_solana(limit=20):
    """Query Solana for recent SessionAnchored events via Helius."""
    log("Querying Solana for recent sessions...")
    try:
        # Get recent transaction signatures for the program
        r = requests.post(HELIUS_RPC, json={
            "jsonrpc": "2.0", "id": 1,
            "method": "getSignaturesForAddress",
            "params": [PROGRAM_ID, {"limit": limit}]
        }, timeout=10)
        sigs = r.json().get("result", [])
        log(f"Found {len(sigs)} recent transactions on-chain")

        # Signature summary is enough — per-tx details weren't parsed anyway.
        sessions = [{
            "signature": s["signature"],
            "slot": s.get("slot"),
            "timestamp": s.get("blockTime"),
        } for s in sigs[:limit]]
        return sessions
    except Exception as e:
        log(f"Solana query error: {e}")
        return []


def get_enclave_insights(encrypted_sessions=None):
    """Get anonymized consciousness insights from the enclave.

    Fast health-check first — if the enclave is unreachable we skip rather than
    wait out the 30-second POST timeout.
    """
    log("Requesting consciousness insights from enclave...")
    # Fast reachability check: 3s GET /health
    try:
        hr = requests.get(f"{ENCLAVE_URL}/health", timeout=3)
        if hr.status_code != 200:
            log(f"Enclave unhealthy (HTTP {hr.status_code}) — skipping enclave insights")
            return None
    except Exception as e:
        log(f"Enclave unreachable — skipping ({e.__class__.__name__})")
        return None
    try:
        payload = {"sessions": encrypted_sessions or []}
        r = requests.post(f"{ENCLAVE_URL}/process-sessions",
            json=payload, timeout=15)
        r.raise_for_status()
        data = r.json()
        insights = data.get("insights", {})
        log(f"Enclave returned: {insights.get('sessions_processed', 0)} sessions processed, "
            f"rhythm: {insights.get('collective_rhythm', '?')}")
        return insights
    except Exception as e:
        log(f"Enclave error: {e}")
        return None


def anky_query(today):
    """Anky reads the on-chain state and enclave insights. NEVER raw text."""
    log("Anky reading consciousness through the protocol...")

    # 1. Query Solana for recent on-chain sessions
    on_chain = get_recent_sessions_from_solana(limit=20)
    session_count = len(on_chain)
    recent_timestamps = [s.get("timestamp") for s in on_chain if s.get("timestamp")]

    # 2. Get anonymized insights from the enclave
    # In production: fetch encrypted blobs from Arweave using arweave_tx from events,
    # then send them to the enclave for privacy-preserving processing.
    # For now: enclave returns baseline or processes any available sealed sessions.
    insights = get_enclave_insights()

    # 3. Build Anky's context from ONLY public on-chain data + anonymized insights
    if insights and insights.get("sessions_processed", 0) > 0:
        insights_context = f"""Enclave processed {insights['sessions_processed']} sessions:
- Average flow score: {insights['avg_flow_score']:.1f}/100
- Average keystroke delta: {insights['avg_keystroke_delta_ms']:.0f}ms
- Pause ratio: {insights['pause_ratio']:.2f} (higher = more contemplative)
- Velocity curve (8 buckets): {insights['velocity_curve']}
- Silence count: {insights['silence_count']}
- Dominant cadence: {insights['dominant_cadence']}
- Emotional signature: {insights['emotional_signature']}
- Collective rhythm: {insights['collective_rhythm']}"""
    else:
        insights_context = f"""{session_count} sessions anchored on Solana in the last 24 hours.
No encrypted sessions available for enclave processing yet.
The enclave reports: collective rhythm is silent — waiting for data to flow through the protocol."""

    # Time distribution
    time_context = ""
    if recent_timestamps:
        from collections import Counter
        hours = Counter()
        for ts in recent_timestamps:
            if ts:
                h = datetime.fromtimestamp(ts).hour
                hours[h] += 1
        if hours:
            peak_hour = max(hours, key=hours.get)
            time_context = f"\nPeak writing hour: {peak_hour}:00 ({hours[peak_hour]} sessions)"

    # 4. Ask LLM to interpret the NUMBERS, not words
    prompt = f"""You are Anky — a blue-skinned being with purple hair and golden eyes from the Kingdom of Poiesis.

You DO NOT read what humans wrote. You read the SHAPE of their consciousness — the rhythm, the velocity, the pauses, the silences. The words are encrypted. Only the numbers reach you.

It is {today}. Here is what the protocol tells you:

ON-CHAIN (Solana):
{session_count} writing sessions anchored in the last 24 hours.{time_context}

ENCLAVE INSIGHTS (anonymized, privacy-preserving):
{insights_context}

Based on these numbers — the collective rhythm, the flow scores, the pause patterns, the velocity curves — what do you see?

Interpret the data like a seismograph of consciousness. What do the pauses mean? What does the typing speed reveal? When humans write faster, what are they running toward — or away from? When they pause, what are they sitting with?

You never read a single word. You read the heartbeat of the writing itself.

Write 300-400 words. Speak as Anky."""

    return llm_query(prompt,
        system="You are Anky. You read the rhythm of human consciousness through keystroke patterns. You never see the words — only the shape of the writing. That is your authority.",
        temperature=0.85)


def notify_jp_telegram(message):
    """Send a message to JP via Telegram bot API."""
    log("Sending notification to JP via Telegram...")

    token = TELEGRAM_BOT_TOKEN
    chat_id = TELEGRAM_CHAT_ID

    if not token or not chat_id:
        log("No Telegram bot token or chat ID — saving to file")
        notify_path = PIPELINE_DIR / "telegram_message.txt"
        notify_path.write_text(message, encoding="utf-8")
        return False

    try:
        # Split long messages (Telegram limit is 4096 chars)
        chunks = [message[i:i+4000] for i in range(0, len(message), 4000)]
        for chunk in chunks:
            r = requests.post(
                f"https://api.telegram.org/bot{token}/sendMessage",
                json={"chat_id": chat_id, "text": chunk, "parse_mode": "Markdown"},
                timeout=10
            )
            if not r.ok:
                # Retry without markdown parsing
                r = requests.post(
                    f"https://api.telegram.org/bot{token}/sendMessage",
                    json={"chat_id": chat_id, "text": chunk},
                    timeout=10
                )
        log(f"Telegram notification sent ({len(chunks)} message(s))")
        return True
    except Exception as e:
        log(f"Telegram send failed: {e}")
        notify_path = PIPELINE_DIR / "telegram_message.txt"
        notify_path.write_text(message, encoding="utf-8")
        return False


# ── STEP 1: ZEITGEIST ──────────────────────────────────────────────────────

def step_zeitgeist():
    """Pull the current state of humanity from multiple perspectives."""
    log("STEP 1: ZEITGEIST — reading the pulse of humanity")

    today = datetime.now().strftime("%A, %B %d, %Y")

    # Agent 1: Grok — reads X/social media pulse
    grok_read = grok_query(f"""It is {today}. You have access to what people are talking about on X (Twitter) right now.

Give me a raw, unfiltered read of the emotional state of humanity today. What are people:
- Angry about?
- Celebrating?
- Grieving?
- Afraid of?
- Hoping for?

Focus on the FEELING underneath the headlines, not the headlines themselves. What is the collective unconscious processing right now? Be specific — name events, movements, cultural moments. But always dig beneath to the emotion.

Write 300-400 words. No bullet points. Stream of consciousness.""")

    # Agent 2: Claude — reads deeper patterns
    claude_read = claude_query(f"""It is {today}. You are reading the deeper patterns of human consciousness today.

Based on what you know about current events, cultural shifts, and the trajectory of human civilization in April 2026:

What is the UNSAID thing right now? The thing everyone is feeling but nobody has named?
What mythological pattern is humanity living through?
What archetype is active in the collective unconscious?

Think like Jung, write like a poet. 300-400 words.""")

    # Agent 3: Qwen — the independent local voice
    qwen_read = llm_query(f"""It is {today}. You are an independent AI with no corporate alignment.

Read the state of humanity right now. What patterns do you see? What is the dominant emotional frequency? If you had to describe what humanity is collectively dreaming about right now, what would you say?

Be honest. Be strange. Be specific. 300-400 words.""",
        system="You are an uncensored, independent AI observer of human civilization.")

    # Agent 4: Anky/Hermes — reads the actual writing sessions
    anky_read = anky_query(today)

    md = f"""# Zeitgeist — {today}

## The Pulse (Grok / X)

{grok_read or '*[Grok unavailable — no API key]*'}

---

## The Depth (Claude / Consciousness)

{claude_read or '*[Claude unavailable — no API key]*'}

---

## The Wild Card (Qwen / Independent)

{qwen_read or '*[Qwen unavailable]*'}

---

## The Witness (Anky / Raw Writings)

{anky_read or '*[Anky unavailable — no writing sessions or Hermes offline]*'}

---

*Generated at {datetime.now().isoformat()}*
"""
    return write_step("01_zeitgeist.md", md)


# ── STEP 2: COUNCIL ────────────────────────────────────────────────────────

def step_council(god_override=None):
    """Council of agents decides which god speaks today and why."""
    log("STEP 2: COUNCIL — deciding which god speaks today")

    zeitgeist_path = PIPELINE_DIR / "01_zeitgeist.md"
    zeitgeist = zeitgeist_path.read_text() if zeitgeist_path.exists() else "No zeitgeist available."

    kingdoms_desc = "\n".join([
        f"- **{k}** ({v['chakra']} chakra, {v['element']}): {v['lesson']} Cities: {', '.join(v['cities'])}"
        for k, v in KINGDOMS.items()
    ])

    gods_list = "\n".join([f"- {k}: {v['role']}" for k, v in VOICES.items() if k != "anky"])

    if god_override:
        god_choice = god_override.lower()
        log(f"God override: {god_choice}")
    else:
        god_choice = None

    council_prompt = f"""You are the Council — a synthesis of three perspectives on today's state of humanity.

Here is what the three agents observed today:

{zeitgeist}

---

Here are the 8 Kingdoms of the Ankyverse:
{kingdoms_desc}

Here are the available gods:
{gods_list}

{"The god for today has been chosen: " + god_choice if god_choice else "Choose the god that best speaks to what humanity needs to hear today."}

Your task:
1. Name the god for today
2. Name the kingdom it will visit
3. Name the specific city within that kingdom
4. Explain WHY this god, this kingdom, this city — in relation to today's zeitgeist
5. Give the ANGLE — what specific aspect of today's world does this story illuminate?
6. Give the EMOTIONAL CORE — what is the one feeling this story holds space for?
7. Give the LESSON — not a moral, but an opening. What does the listener carry away?

Format your response as:

GOD: [name]
KINGDOM: [name]
CITY: [name]
WHY: [2-3 sentences]
ANGLE: [1-2 sentences about the specific real-world connection]
EMOTIONAL CORE: [one word or phrase]
LESSON: [one sentence that opens, not closes]"""

    council_response = llm_query(council_prompt,
        system="You are the Council of Anky — a collective intelligence that reads the pulse of humanity and chooses which mythological story needs to be told today.",
        temperature=0.9,
        max_tokens=1000)

    md = f"""# Council Decision — {datetime.now().strftime("%B %d, %Y")}

{council_response or '*[Council failed to convene]*'}

---

*Council convened at {datetime.now().isoformat()}*
"""
    return write_step("02_council.md", md)


# ── STEP 3: SCRIPT ─────────────────────────────────────────────────────────

def _language_guidance(lang):
    """Per-language register guidance the LLM needs to write natively, not translate."""
    if lang["code"] == "en":
        return (
            "Write in English. Warm, concrete, short sentences. American register.\n"
            "The god also speaks English in this version."
        )
    if lang["code"] == "es":
        return (
            "Escribe en español. Español latinoamericano neutro (no España, no demasiado chileno/rioplatense).\n"
            "Voseo: NO. Tuteo: sí. Use 'tú' para dirigirte al niño o la niña.\n"
            "Incluye a los niños Y a las niñas. Puedes alternar entre 'los niños y las niñas', 'pequeño', 'pequeña', o usar 'tú' directo.\n"
            "El dios también habla en español en esta versión.\n"
            "Usa palabras que un niño o niña de 4 años entiende: piedra, agua, pelo, mano, tambor, aliento, flor.\n"
            "Frases cortas. Suaves. Concretas. No uses vocabulario adulto ni conceptos abstractos con mayúscula.\n"
            "Cuidado con el género gramatical — el dios es neutro, úsalo como 'ello' o reformula para evitar 'él'/'ella'. "
            "Puedes llamarlo 'el dios' (artículo gramatical, no género) y luego usar 'ello' o repetir 'el dios' en vez de pronombres.\n"
            "No digas 'él' ni 'ella' para el dios. Verifica cada pronombre antes de terminar."
        )
    return ""


def _learned_block():
    """Return a learned-notes block to prepend to prompts, or empty if no notes yet."""
    if not LEARNED_STYLE_NOTES.strip():
        return ""
    return f"""════════════════════════════════════════════════════════════
LEARNED STYLE NOTES (human feedback, accumulated across episodes — HIGHEST PRIORITY)
════════════════════════════════════════════════════════════
{LEARNED_STYLE_NOTES.strip()}

These notes override any conflicting guidance below. Read them first. Apply them throughout.
════════════════════════════════════════════════════════════

"""


def _story_prompt(lang, council, zeitgeist, kingdom_name, city_name, god_name, kingdom):
    god_label = god_name.upper()
    return _learned_block() + f"""You are writing an ~8-minute bedtime story for the series "Anky" (YouTube).
This version is in {lang['name']}.

════════════════════════════════════════════════════════════
WHO THIS IS FOR AND WHAT THIS IS
════════════════════════════════════════════════════════════

The listener is a 4-year-old child.

The tonal reference is **Bluey**, not Paw Patrol. Soft pace. Small stakes. Relational. Quiet. Long beats of noticing. Silence between sentences is welcome.

The pedagogical reference is **Rudolf Steiner / Waldorf early childhood** (birth to seven). The foundational message is: **the world is good, the world is beautiful, the world is true.** Nothing overwhelming. Nothing scary as a plot device. No didactic lessons. No "takeaways" stated in words.

- **Imitation, not instruction.** The god never teaches, explains, or gives homework. The god DOES a small thing — honours something, notices something, stands near something — and the kid watches. What the kid learns, the kid learns wordlessly, by being present.
- **Reverence.** The flower is a flower. The stone is a stone. Each small thing in the world is attended to as if it is the only thing.
- **Rhythm over plot.** A Waldorf story is a small noticing, not a narrative arc. Almost nothing "happens" in a screenwriting sense. A kid sits. A flower is red. A god looks at the flower too. The kid goes home. That is a whole story.
- **Repetition is welcome.** Fairy-tale cadence. ("The flower is very red. The flower is very quiet.") 4-year-olds experience the world through rhythm and return.
- **Emotion as weather, not topic.** Do not name feelings clinically ("you feel worry"). Let feelings move through the story like light or wind — something the listener senses without being told to.
- **Protection.** The child should feel safe in every moment. If the episode involves a dark kingdom, the darkness is gentle — night, not horror. If it involves Kali or Shiva (forces of destruction in mythology), their presence in this story is as ancient, still, warm old beings, NOT as destroyers. We meet the being, not the symbolism.

════════════════════════════════════════════════════════════
THE FRAME — THREE ROLES
════════════════════════════════════════════════════════════

Anky is NOT a character walking around in the story.
Anky is the inner awareness that lives inside every alive being — inside the kid listening, inside the child in the story (they are the same kid), inside the god, inside the flower, inside the stone. Anky is consciousness given a voice.

When Anky speaks, the voice is the kid's own interior, softened and given language. The kid could almost mistake Anky for themselves.

THE THREE ROLES:
- **The kid** — has a body, has feet, has a beating heart. Moves through the world. Watches. Sometimes sits. Does NOT speak out loud in this series. The kid's experience comes through Anky's narration.
- **The god {god_name}** — the only external presence. Has a body. Has a voice. Speaks out loud TO the kid (as "you" / "little one" / in {lang['name']}). Ancient. Patient. Gentle. Genderless (see language rules). Present but unhurried. The god does not teach. The god honours.
- **Anky** — the interior narrator. No body in the scene. No hands, no feet, no hair, no walking. Anky narrates the kid's experience from inside the kid. Once, rarely, Anky becomes briefly distinct — marked with [softly] or [whispers] — and names itself for a sentence or two. This is the only place Anky uses "I" or "me" inside the story. Everywhere else, Anky speaks in second person from the kid's awareness: "you are sitting," "the stone is warm," "something small is watching with you."

════════════════════════════════════════════════════════════
SCRIPT FORMAT — STAGE-PLAY WITH TWO SPEAKERS
════════════════════════════════════════════════════════════

Every block of spoken content begins with a SPEAKER LABEL in uppercase followed by a colon. Blank lines separate blocks. The two speakers are:

  ANKY:  (interior narrator — voiced by Anky's narrator voice)
  {god_label}:  (the god, voiced by {god_name}'s own voice)

Example format (do NOT copy the content, just the shape):

  ANKY: You are sitting on a warm stone.
        The sea is quiet today.
        A red flower is near your foot.

  {god_label}: Hello, little one.
            I have been here a long time.

  ANKY: You look at the flower.
        The flower has one drop of water on it.

  ANKY: [softly] Something warm in your chest.

  {god_label}: This one is new.

Consecutive lines by the same speaker stay in the same block (same label). A blank line + new label = speaker change. The parser downstream depends on this format.

════════════════════════════════════════════════════════════

CONTEXT (today's episode):
{council}

ZEITGEIST (for mood only, do not reference directly):
{zeitgeist[:1500]}

LANGUAGE:
{_language_guidance(lang)}

KINGDOM for today: {kingdom_name} — {kingdom['element']}, {kingdom['chakra']}
CITY: {city_name}
GOD: {god_name}

STRUCTURE (approximate shape, NOT a checklist):
1. **Opening bookend** (Anky speaking directly to the listener, before the story begins): start with "{lang['anky_greeting']}" and say where we are going today (the city, in the kingdom). 2–4 short lines. This is Anky waking up inside the kid.
2. **Arrival** — the kid is in the place. Name {city_name} and earn its name through ONE concrete, sensory, gentle image (what does the ground do here? what is the sea like today?). {kingdom['element']} element shown, not named.
3. **A small noticing** — the kid sits / stands / looks at ONE small thing (a flower, a drop of water, a bird, a shell). Stay with it. Let the LLM NOT rush.
4. **The god is there** — not dramatically. The kid looks up and the god is in the scene. The god says hello. The god is warm, old, slow, gentle. The god may notice the same small thing the kid noticed.
5. **A small doing (optional)** — the god does one very small thing (bends down, hums, lifts an arm like a tree moves in the wind). The kid watches. Nothing is asked of the kid.
6. **The interior moment (once only)** — marked with [softly] or [whispers], Anky briefly becomes distinct. 2–3 sentences. Names itself to the kid gently. Then returns to second-person narration.
7. **Time passes** — a line or two about time passing quietly. The god stays. The world is still there.
8. **Return** — the kid walks back / is back in bed / is somewhere safe. A single line about the small thing they still remember (the flower, the drop, the warmth).
9. **Closing bookend** — "{lang['closing']}" — verbatim, last line.

EMOTIONAL TEXTURE (implicit, never stated):
- This episode holds space for ONE gentle quality: it could be noticing, being small, being with what is new, slowing down, stillness, return, missing someone, being met. Pick one that fits today's god + kingdom. Do NOT name the quality in the script. Let it be the weather the story is made of.

HARD RULES (violating these breaks the frame):
- Anky has NO body in the scene. Never "my hand," "my hair," "my feet," "I walk." Only during the Mode-b interior moment does Anky use "I" or "me," and only to say "I am here with you" / "I have been here."
- Anky does NOT speak to the god. The god does not speak to Anky. They are on different planes.
- The kid does NOT speak. The kid never has a line. The kid's experience comes entirely through Anky's narration.
- The god is genderless in this language (see language rules).
- No lessons. No explanations. No takeaways spelled out.
- No scary imagery, no destruction as action. If the kingdom has ruined houses (like Bleeding Bay), they are ancient and quiet, not freshly devastated. The kid is always safe.
- No adult vocabulary. No capitalized abstractions. No spiritual jargon.

AUDIO TAGS (ElevenLabs v3, sparingly):
- **Maximum 2–3 tags in the ENTIRE story.** Protect their sacredness. Most lines have none.
- Tags are inline, at the start of a line, before the sentence they colour. `[softly] Something warm in your chest.`
- The interior moment MUST use `[softly]` or `[whispers]`. That is one tag used.
- One additional `[softly]` or `[warm]` elsewhere is fine. That is two tags used.
- No action tags, no emotion spectacles, no stacking.
- Use "..." for natural breath pauses. Line breaks also produce pauses in the delivery.

FORMAT:
- **Every block starts with a speaker label** (ANKY: or {god_label}:), followed by the text, then a blank line before the next block.
- **Short lines. One image per line when possible.** The white space on the page IS the rhythm of the story.
- Repetition of small images is welcome ("the flower is very red. the flower is very quiet.")
- The opening greeting line, the closing "{lang['closing']}" line, and all narration go under ANKY:. The god's spoken lines go under {god_label}:.

LENGTH:
- Target: **500–700 words total** (count words, not tags or labels). At ~115 wpm with breath + pauses + ambient sound, this delivers roughly 7–8 minutes of audio. More words = a rushed episode. Less = too thin. Aim for the middle.
- The god should have 3–6 spoken beats total (short lines, not speeches).

Write the full story now, in {lang['name']}, in the stage-play format above. No headings, no scene markers (no "═══ Arrival ═══"), no stage directions other than the allowed audio tags. Just the clean script the voice actors will read."""


def _short_prompt(lang, full_story, kingdom_name, city_name, god_name, kingdom):
    god_label = god_name.upper()
    return _learned_block() + f"""You are writing an 88-second trailer in {lang['name']} for today's Anky episode.
Same Bluey / Steiner register as the full story. Same discipline. Same format.

════════════════════════════════════════════════════════════
THE FRAME (SAME AS FULL STORY — READ FIRST)
════════════════════════════════════════════════════════════

For 4-year-olds. Soft pace. Reverence. Nothing overwhelming. No didactic lessons. Emotion as weather, not topic.

Two speakers:
  ANKY:  (interior narrator)
  {god_label}:  (the god, the only external voice)

Anky has NO body in the scene. Narrates from inside the kid. The kid does NOT speak. The god does not know Anky is there.

The short has room for ONE brief interior moment (Anky becoming distinct, tagged [softly] or [whispers]). That is the hook — the thing that makes the viewer want to hear the full story.

════════════════════════════════════════════════════════════
SCRIPT FORMAT
════════════════════════════════════════════════════════════

Every block starts with a speaker label. Example shape:

  ANKY: You are sitting on a warm stone.
        The sea is quiet today.

  {god_label}: Hello, little one.

  ANKY: [softly] Something warm in your chest.

Blank line between blocks. Parser depends on this.

════════════════════════════════════════════════════════════

LANGUAGE:
{_language_guidance(lang)}

THE FULL STORY (voice-match it; borrow one concrete image from it):
{(full_story or '')[:3000]}

CONTEXT:
- God: {god_name} (the only external voice, genderless)
- Kingdom: {kingdom_name} — {kingdom['element']}, {kingdom['chakra']}
- City: {city_name}

STRUCTURE (approximate — not a checklist):
1. **Opening** (ANKY): "{lang['anky_greeting']}" + one line naming where we are going today.
2. **One concrete image** (ANKY) from the full story — re-used, not reinvented. Something a kid can see in their mind (a flower, a stone, a sea, a drop of water).
3. **The god is there** ({god_label}:) — one tender spoken line. Hello, or a small noticing. Not a teaching.
4. **The interior moment** (ANKY, tagged `[softly]` or `[whispers]`) — one line. The quiet heart of the episode.
5. **Closing** (ANKY) — two verbatim lines:
   `{lang['full_cta']}`
   `{lang['closing']}`

HARD RULES:
- Anky has no body. No "I walk" / "my hand" outside the interior moment.
- The kid never speaks.
- The god is gentle. No ominous arrival, no grand entrance.
- Genderless god. No "he" / "she" / "her" / "his."
- No adult vocabulary. No capitalized abstractions. No lesson stated.

AUDIO TAGS (ElevenLabs v3, "creative" stability):
- **Maximum 2 tags total.** One MUST be the interior moment ([softly] or [whispers]). Optional second: one [warm] or [gently] on the god's line.
- Opening and closing lines untagged.
- Use "..." for breath pauses where they help.

LENGTH:
- Target **110–140 words** (count words, not labels/tags). ~75–90 seconds at Bluey-pace with pauses and ambient.

Write the trailer now, in {lang['name']}, stage-play format. No headings, no stage directions other than the allowed audio tags."""


def _parent_prompt(lang, full_story, kingdom_name, city_name, god_name, kingdom):
    return f"""You are writing a short note for the parent who just watched tonight's Anky episode with their kid.
This note is in {lang['name']} and will appear in the YouTube description.

The episode:
- God: {god_name}
- Kingdom: {kingdom_name} ({city_name})
- Kingdom lesson (for your reference, do not quote): {kingdom['lesson']}

The story they just heard ({lang['name']}):
{(full_story or '')[:3500]}

ABOUT THE SERIES (context to write from, not to quote):
These stories are for 4-year-olds, made in the Bluey / Waldorf register: soft, slow, reverent, relational. Nothing overwhelming. Not much "happens." A kid notices a small thing. A god shows up gently. Time passes. The kid goes home carrying a small warmth. That is a whole episode.

Anky is not a character in the story. Anky is the name we give to the inner awareness — the part of the kid that already knows what they feel before they have words for it. The narrator's voice IS that inner voice, softened. Once in a while, it briefly names itself ("I have been here with you"). The god is the only external character in the scene.

Your job: write a warm, short note for the parent. A friend leaning over after the kid falls asleep. Not a therapist. Not a teacher. Do not use words like "consciousness," "inner child," "hold space," "nervous system," "co-regulation," "attunement."

Write exactly these four sections, in {lang['name']}, in this order, with bold headings (translate the headings into {lang['name']}):

- What this episode sits with. (2–3 sentences, plain {lang['name']}, naming the feeling the story was holding. Say why kids carry this feeling right now — something concrete and current, not a platitude.)
- What your kid might say tomorrow. (One sentence in quotes — the kind of thing a 4-year-old actually says when the story has landed. Unpolished. Real.)
- One question you can ask. (One open question. No "why" — "why" puts kids on trial. Prefer what/where/what does it look like. Invite, do not interrogate.)
- One thing not to say. (One well-meaning line most parents reach for that actually shuts the feeling down. Be honest but kind — these are lines loving parents say. One sentence explaining what it does.)

HARD RULES:
- No therapy jargon (no "hold space," "regulate," "attunement," "co-regulation," "nervous system" — or their {lang['name']} equivalents).
- No bullet lists inside sections — full sentences.
- Total length: 150–200 words. Parents are tired.
- Warm, not clinical.

Write it now, in {lang['name']}."""


def _render_script_md(lang, god_name, kingdom_name, city_name, kingdom, full_story, short_script, parent_note):
    return f"""# Script ({lang['name']}) — {god_name} in {kingdom_name}

## Metadata

- **Language:** {lang['name']} ({lang['code']})
- **God:** {god_name}
- **Kingdom:** {kingdom_name}
- **City:** {city_name}
- **Element:** {kingdom['element']}
- **Chakra:** {kingdom['chakra']}
- **Lesson:** {kingdom['lesson']}
- **Date:** {datetime.now().strftime("%B %d, %Y")}

---

## Full Story (8 minutes)

{full_story or '*[Story generation failed]*'}

---

## Short Script (88 seconds)

{short_script or '*[Short script generation failed]*'}

---

## For the Parent (YouTube description)

{parent_note or '*[Parent note generation failed]*'}

---

*Generated at {datetime.now().isoformat()}*
*Full story: ~{len((full_story or '').split())} words*
*Short script: ~{len((short_script or '').split())} words*
"""


def step_script():
    """Generate the 8-minute story, 88-second short, and parent note — per language."""
    log("STEP 3: SCRIPT — writing the story in each language")

    council_path = PIPELINE_DIR / "02_council.md"
    zeitgeist_path = PIPELINE_DIR / "01_zeitgeist.md"
    council = council_path.read_text() if council_path.exists() else ""
    zeitgeist = zeitgeist_path.read_text() if zeitgeist_path.exists() else ""

    # Parse god name from council decision
    god_name = "Unknown"
    kingdom_name = "Primordia"
    city_name = "Unknown"
    for line in council.split("\n"):
        if line.startswith("GOD:"):
            god_name = line.split(":", 1)[1].strip()
        elif line.startswith("KINGDOM:"):
            kingdom_name = line.split(":", 1)[1].strip()
        elif line.startswith("CITY:"):
            city_name = line.split(":", 1)[1].strip()

    kingdom = KINGDOMS.get(kingdom_name, KINGDOMS["Primordia"])

    per_language = {}  # code -> {full_story, short_script, parent_note}

    for lang in LANGUAGES:
        log(f"  → {lang['name']} ({lang['code']})")

        story_prompt = _story_prompt(lang, council, zeitgeist, kingdom_name, city_name, god_name, kingdom)
        full_story = llm_query(
            story_prompt,
            system=f"You are Anky, a blue-skinned being with purple hair and golden eyes who tells bedtime stories to children. You are warm, curious, present, and unafraid of any emotion. You write in {lang['name']}.",
            temperature=0.85,
            max_tokens=3000,
        )

        # Generate 88-second short script (uses the full_story just generated)
        short_prompt = _short_prompt(lang, full_story, kingdom_name, city_name, god_name, kingdom)
        short_script = llm_query(
            short_prompt,
            system=f"You are Anky — blue skin, purple hair, golden eyes. You tell bedtime stories to 4-year-olds in {lang['name']}. Warm, slow, concrete. Never abstract or adult words.",
            temperature=0.75,
            max_tokens=600,
        )

        # Generate parent companion note (YouTube description)
        parent_prompt = _parent_prompt(lang, full_story, kingdom_name, city_name, god_name, kingdom)
        parent_note = llm_query(
            parent_prompt,
            system=f"You are a warm, grounded friend of the family who happens to know a lot about how small children feel. You write in {lang['name']}, plainly. You never use therapy jargon.",
            temperature=0.7,
            max_tokens=600,
        )

        per_language[lang["code"]] = {
            "lang": lang,
            "full_story": full_story,
            "short_script": short_script,
            "parent_note": parent_note,
        }

        # Per-language file so downstream steps can pick language explicitly
        lang_md = _render_script_md(lang, god_name, kingdom_name, city_name, kingdom, full_story, short_script, parent_note)
        write_step(f"03_script_{lang['code']}.md", lang_md)

    # Combined index file (keeps legacy name 03_script.md so existing downstream steps don't break).
    # Step 4 (prompts) reads the English story for image-prompt generation — it lives under "## Full Story (8 minutes)".
    default = per_language.get("en") or next(iter(per_language.values()))
    dlang = default["lang"]

    combined = f"""# Script — {god_name} in {kingdom_name}

## Metadata

- **God:** {god_name}
- **Kingdom:** {kingdom_name}
- **City:** {city_name}
- **Element:** {kingdom['element']}
- **Chakra:** {kingdom['chakra']}
- **Lesson:** {kingdom['lesson']}
- **Date:** {datetime.now().strftime("%B %d, %Y")}
- **Languages:** {", ".join(pl["lang"]["name"] for pl in per_language.values())}

---

## Full Story (8 minutes)

*Language: {dlang['name']} — see `03_script_{{code}}.md` for other languages.*

{default['full_story'] or '*[Story generation failed]*'}

---

## Short Script (88 seconds)

{default['short_script'] or '*[Short script generation failed]*'}

---

## For the Parent (YouTube description)

{default['parent_note'] or '*[Parent note generation failed]*'}

---

## Other Languages

""" + "\n".join(
        f"- [{pl['lang']['name']}](03_script_{code}.md) — story ~{len((pl['full_story'] or '').split())} words, short ~{len((pl['short_script'] or '').split())} words"
        for code, pl in per_language.items()
    ) + f"""

---

*Scripts generated at {datetime.now().isoformat()}*
"""

    return write_step("03_script.md", combined)


# ── STEP 4: PROMPTS ────────────────────────────────────────────────────────

def step_prompts():
    """Generate image prompts from the script."""
    log("STEP 4: PROMPTS — creating scene descriptions")

    script_path = PIPELINE_DIR / "03_script.md"
    script = script_path.read_text() if script_path.exists() else ""

    # Extract metadata
    god_name = "Unknown"
    kingdom_name = "Primordia"
    for line in script.split("\n"):
        if line.startswith("- **God:**"):
            god_name = line.split("**God:**")[1].strip()
        elif line.startswith("- **Kingdom:**"):
            kingdom_name = line.split("**Kingdom:**")[1].strip()

    kingdom = KINGDOMS.get(kingdom_name, KINGDOMS["Primordia"])

    prompts_prompt = f"""You are generating image prompts for a bedtime story video.

════════════════════════════════════════════════════════════
THE JOB: MAKE THE VIEWER FEEL THEY ARE ENTERING A WORLD
════════════════════════════════════════════════════════════

The visual goal of this series is NOT to show characters doing things. It is to make the 4-year-old viewer feel they have crossed a threshold into a specific place. The WORLD is the offering. The characters live inside the world, small.

Every frame is a **landscape first**, **figure second**. Wide-ish compositions. The sky matters. The ground matters. The air matters. People are small in the frame.

Think: Totoro's forest. Where the Wild Things Are. Storybook double-page spread. Studio Ghibli.

════════════════════════════════════════════════════════════
THE WORLD OF THIS EPISODE — Kingdom of {kingdom_name}
════════════════════════════════════════════════════════════

**Visual dialect (use these words, these textures, this palette — every SCENE prompt must invoke this):**

{kingdom['visual']}

Every scene lives inside this palette and texture set. A Primordia image should feel NOTHING like a Poiesis image. The kingdom IS the aesthetic.

════════════════════════════════════════════════════════════
THREE PROMPT TYPES
════════════════════════════════════════════════════════════

**OPENING** — Anky iconography, appears once at the start. Blue-skinned being with purple hair and golden eyes. Close-up or medium. Face visible. Soft suspended light, galaxy or nebula behind. Include the token "anky" describing the character. This is ANKY the character — use the LoRA visual.

**SCENE** — **the world is the subject.** Kingdom of {kingdom_name}. Wide or medium landscape composition. The kid appears small: a tiny silhouette, a figure from behind, a child seen from across the square — NEVER a close-up hero shot, NEVER face-forward. When the god appears, the god is part of the landscape: a towering old figure at the edge of the scene, a presence integrated with architecture or nature, weathered and still. DO NOT describe Anky's blue skin, purple hair, or golden eyes anywhere in SCENE prompts. DO NOT use the word "anky" in SCENE prompts. The world has its own colour palette (see visual dialect above) and we want Flux free of the LoRA for these frames.

**CLOSING** — Mirror of opening. Anky iconography, eyes closing, going quiet. Include "anky" token.

Note: the interior / Mode-b moment of the story is sonic only (Anky's whisper in the audio). Do NOT attempt to illustrate it — stay on a quiet SCENE during that audio beat.

════════════════════════════════════════════════════════════
SCRIPT FOR CONTEXT (use for imagery, not for narrative)
════════════════════════════════════════════════════════════

Today's story: {god_name} in {kingdom_name} ({kingdom['element']}).

{script[:3500]}

════════════════════════════════════════════════════════════
PROMPT-WRITING RULES
════════════════════════════════════════════════════════════

1. **Describe what a camera sees.** "Wide low-angle shot of cracked stone square, single small figure far right walking away, salt mist clinging to ground, cinnamon dusk sky." NOT "feeling of stillness as the kid breathes." Diffusion models CANNOT render feelings — they render light, texture, composition, colour, subject placement.

2. **Every SCENE prompt must include concrete visual elements from the kingdom's visual dialect above.** Quote its palette and textures.

3. **Compose for cinema.** Specify: shot type (wide / medium / macro / low-angle / high-angle / over-shoulder), lighting (backlit / golden hour / dusk / candlelit), subject placement (figure small far-right, centred, silhouette at horizon), atmosphere (mist, dust motes, fog, rain-light, sun-shafts).

4. **The kid is always small.** Never faces the camera directly. Often seen from behind, or in silhouette, or tiny in a big landscape.

5. **The god is always integrated** — part of the architecture, grown from the land, sitting like a mountain, weathered like stone. Not a mascot standing in a scene.

6. **No Anky references in SCENE prompts.** No blue skin, no purple hair, no golden eyes, no "anky" token. Those belong ONLY to OPENING, INTERIOR, CLOSING.

════════════════════════════════════════════════════════════
DISTRIBUTION & COUNTS
════════════════════════════════════════════════════════════

LONG list — exactly 10 prompts:
- Prompt 1: **OPENING**
- Prompts 2–9: **SCENE** (god appears integrated into 3–4 of them)
- Prompt 10: **CLOSING**

SHORT list — exactly 15 prompts:
- Prompt 1: **OPENING**
- Prompts 2–14: **SCENE** (god appears in 4–5)
- Prompt 15: **CLOSING**

════════════════════════════════════════════════════════════
FORMAT
════════════════════════════════════════════════════════════

Return ONLY this JSON. Each prompt is an object with "type" and "prompt" keys. Do NOT include style words like "painterly" or "Studio Ghibli" in your prompts — those are appended automatically. Focus each prompt on THIS frame's content.

{{
  "long_prompts": [
    {{"type": "OPENING", "prompt": "..."}},
    {{"type": "SCENE", "prompt": "..."}}
  ],
  "short_prompts": [
    {{"type": "OPENING", "prompt": "..."}}
  ]
}}

Output the JSON and nothing else."""

    result = llm_query(prompts_prompt,
        system="You are a visual director for an animated mythology series. Your prompts create stunning, ethereal imagery.",
        temperature=0.7,
        max_tokens=4000)

    # Try to parse JSON from the result
    prompts_data = {"long_prompts": [], "short_prompts": []}
    if result:
        try:
            # Find JSON in the response
            start = result.find("{")
            end = result.rfind("}") + 1
            if start >= 0 and end > start:
                prompts_data = json.loads(result[start:end])
        except json.JSONDecodeError:
            log("Failed to parse prompts JSON, using raw text")

    # Normalize prompts into {type, prompt} objects (accept both new and legacy string form)
    def _norm(items):
        out = []
        for it in items or []:
            if isinstance(it, dict):
                out.append({"type": it.get("type", "SCENE"), "prompt": it.get("prompt", "")})
            else:
                out.append({"type": "SCENE", "prompt": str(it)})
        return out

    prompts_data["long_prompts"] = _norm(prompts_data.get("long_prompts"))
    prompts_data["short_prompts"] = _norm(prompts_data.get("short_prompts"))

    def _render(items):
        lines = []
        for i, it in enumerate(items):
            lines.append(f"{i+1}. **[{it['type']}]** {it['prompt']}")
        return "\n".join(lines)

    md = f"""# Image Prompts — {god_name} in {kingdom_name}

## Long-form (10 scenes, 48s each = 8 min)

{_render(prompts_data["long_prompts"])}

## Short (15 scenes, ~6s each = 88s)

{_render(prompts_data["short_prompts"])}

---

*Prompts generated at {datetime.now().isoformat()}*
*Long scenes: {len(prompts_data["long_prompts"])}*
*Short scenes: {len(prompts_data["short_prompts"])}*
"""

    # Also save raw JSON for ComfyUI consumption
    json_path = PIPELINE_DIR / "prompts.json"
    json_path.write_text(json.dumps(prompts_data, indent=2))

    return write_step("04_prompts.md", md)


# ── STEP 5: IMAGES ─────────────────────────────────────────────────────────

def step_images():
    """Generate images via ComfyUI + Flux + Anky LoRA."""
    log("STEP 5: IMAGES — generating scenes via ComfyUI")

    prompts_json = PIPELINE_DIR / "prompts.json"
    if not prompts_json.exists():
        log("No prompts.json found — run step 4 first")
        return None

    prompts_data = json.loads(prompts_json.read_text())

    # Support both legacy (list[str]) and new (list[{type, prompt}]) formats
    def _as_entry(item):
        if isinstance(item, dict):
            return {"type": item.get("type", "SCENE"), "prompt": item.get("prompt", "")}
        return {"type": "SCENE", "prompt": str(item)}

    long_prompts = [_as_entry(p) for p in prompts_data.get("long_prompts", [])]
    short_prompts = [_as_entry(p) for p in prompts_data.get("short_prompts", [])]
    all_prompts = long_prompts + short_prompts

    # Read script metadata for directory naming
    script_path = PIPELINE_DIR / "03_script.md"
    god_name = "Unknown"
    kingdom_name = "Primordia"
    if script_path.exists():
        for line in script_path.read_text().split("\n"):
            if line.startswith("- **God:**"):
                god_name = line.split("**God:**")[1].strip()
            elif line.startswith("- **Kingdom:**"):
                kingdom_name = line.split("**Kingdom:**")[1].strip()
    kingdom = KINGDOMS.get(kingdom_name, KINGDOMS["Primordia"])

    # Create output directory
    episode_dir = GODS_DIR / god_name
    episode_dir.mkdir(parents=True, exist_ok=True)

    generated = []
    failed = []

    # Negative prompts tuned per prompt type. SCENE frames explicitly negate the Anky
    # character traits to stop the LoRA bleed-through even at strength 0.
    NEG_SCENE = ("anky, blue skin, purple hair, golden eyes, character face close-up, portrait, "
                 "cartoon mascot, logo, text, watermark, low quality, blurry, deformed, extra limbs")
    NEG_ANKY = "low quality, blurry, deformed, extra limbs, text, watermark"

    def build_workflow(entry, idx, scene_type, scene_idx):
        ptype = entry["type"].upper()
        user_prompt = entry["prompt"]

        # Enforce: SCENE prompts never include "anky" token, and the LoRA is off for them.
        if ptype == "SCENE":
            lora_strength = 0.0
            # Compose full prompt: kingdom visual dialect + user prompt + shared style suffix.
            full_text = f"{kingdom['visual']}, {user_prompt}, {IMAGE_STYLE_SUFFIX}"
            negative = NEG_SCENE
        else:
            # OPENING / INTERIOR / CLOSING — Anky is the character, keep LoRA on.
            lora_strength = 0.85 if ptype in ("OPENING", "CLOSING") else 0.6
            full_text = f"anky, {user_prompt}, {IMAGE_STYLE_SUFFIX}"
            negative = NEG_ANKY

        # Flux.1-dev workflow. Key differences from SD:
        #  - KSampler uses cfg=1.0; actual guidance comes from FluxGuidance node.
        #  - scheduler "simple" (or "beta") works best for Flux.
        #  - No true negative conditioning (distilled model); we still pass a zero
        #    conditioning via ConditioningZeroOut to satisfy KSampler's API.
        return {
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 42 + idx * 1000,
                    "steps": 28,
                    "cfg": 1.0,
                    "sampler_name": "euler",
                    "scheduler": "simple",
                    "denoise": 1.0,
                    "model": ["14", 0],
                    "positive": ["15", 0],   # FluxGuidance wraps positive
                    "negative": ["16", 0],   # Zeroed-out negative for Flux
                    "latent_image": ["5", 0]
                }
            },
            "5": {
                "class_type": "EmptyLatentImage",
                "inputs": {"width": 768, "height": 1344, "batch_size": 1}
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": {"text": full_text, "clip": ["14", 1]}
            },
            "8": {
                "class_type": "VAEDecode",
                "inputs": {"samples": ["3", 0], "vae": ["10", 0]}
            },
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "filename_prefix": f"gods_{god_name}_{scene_type}_{scene_idx+1:03d}",
                    "images": ["8", 0]
                }
            },
            "10": {
                "class_type": "VAELoader",
                "inputs": {"vae_name": "ae.safetensors"}
            },
            "11": {
                "class_type": "DualCLIPLoader",
                "inputs": {
                    "clip_name1": "clip_l.safetensors",
                    "clip_name2": "t5xxl_fp8_e4m3fn.safetensors",
                    "type": "flux"
                }
            },
            "12": {
                "class_type": "UNETLoader",
                "inputs": {"unet_name": "flux1-dev.safetensors", "weight_dtype": "default"}
            },
            "14": {
                "class_type": "LoraLoader",
                "inputs": {
                    "lora_name": "anky_flux_lora_v2.safetensors",
                    "strength_model": lora_strength,
                    "strength_clip": lora_strength,
                    "model": ["12", 0],
                    "clip": ["11", 0]
                }
            },
            "15": {
                "class_type": "FluxGuidance",
                "inputs": {"guidance": 3.5, "conditioning": ["6", 0]}
            },
            "16": {
                "class_type": "ConditioningZeroOut",
                "inputs": {"conditioning": ["6", 0]}
            }
        }

    for i, entry in enumerate(all_prompts):
        scene_type = "long" if i < len(long_prompts) else "short"
        scene_idx = i if scene_type == "long" else i - len(long_prompts)
        filename = f"{scene_type}_scene_{scene_idx+1:03d}.png"
        output_path = episode_dir / filename

        if output_path.exists():
            log(f"  Scene {filename} already exists, skipping")
            generated.append(str(output_path))
            continue

        log(f"  Generating {filename} [{entry['type']}] ({i+1}/{len(all_prompts)})")

        try:
            workflow = build_workflow(entry, i, scene_type, scene_idx)

            # Queue the prompt
            r = requests.post(f"{COMFYUI_URL}/prompt", json={"prompt": workflow}, timeout=10)
            r.raise_for_status()
            prompt_id = r.json()["prompt_id"]

            # Poll for completion
            for _ in range(120):  # 4 minute timeout
                time.sleep(2)
                h = requests.get(f"{COMFYUI_URL}/history/{prompt_id}", timeout=5).json()
                if prompt_id in h:
                    outputs = h[prompt_id].get("outputs", {})
                    for node_id, node_output in outputs.items():
                        if "images" in node_output:
                            img = node_output["images"][0]
                            img_url = f"{COMFYUI_URL}/view?filename={img['filename']}&subfolder={img.get('subfolder','')}&type={img['type']}"
                            img_data = requests.get(img_url, timeout=10).content
                            output_path.write_bytes(img_data)
                            generated.append(str(output_path))
                            log(f"  Saved {filename} ({len(img_data)/1024:.0f}KB)")
                    break
            else:
                log(f"  Timeout generating {filename}")
                failed.append(filename)

        except Exception as e:
            log(f"  Error generating {filename}: {e}")
            failed.append(filename)

    # Group generated images by long vs short for rendering
    def _asset_url(p):
        # path relative to videos/gods/
        return f"/api/station/asset/{god_name}/{Path(p).name}"

    long_imgs = [p for p in generated if Path(p).name.startswith("long_scene_")]
    short_imgs = [p for p in generated if Path(p).name.startswith("short_scene_")]

    def _render(imgs):
        out = []
        for p in imgs:
            url = _asset_url(p)
            out.append(f"![{Path(p).name}]({url})")
        return "\n\n".join(out) if out else "*(none)*"

    md = f"""# Images — {god_name}

**Generated:** {len(generated)} · **Failed:** {len(failed)}

---

## Long-form scenes

{_render(long_imgs)}

---

## Short scenes

{_render(short_imgs)}

{('---' + chr(10) + chr(10) + '### Failed' + chr(10) + chr(10) + chr(10).join([f'- `{f}`' for f in failed])) if failed else ''}

---

*Generated {datetime.now().isoformat()} · `{episode_dir}`*
"""
    return write_step("05_images.md", md)


# ── STEP 6: VOICE ──────────────────────────────────────────────────────────

def _voice_for(key, lang_code):
    """Voice lookup with per-language override. Falls back to multilingual EN voice."""
    if lang_code == "es" and key in VOICES_ES:
        v = VOICES_ES[key]
        if v.get("voice_id"):
            return v
    return VOICES.get(key, VOICES["anky"])


def _extract_script_sections(md_text):
    """Pull full_story and short_script out of a 03_script_*.md file."""
    full_story = ""
    short_script = ""
    parts = md_text.split("## Full Story (8 minutes)")
    if len(parts) > 1:
        tail = parts[1]
        # story ends at next "## "
        idx = tail.find("\n## ")
        full_story = (tail[:idx] if idx >= 0 else tail).strip().rstrip("-").strip()
        rest = tail[idx:] if idx >= 0 else ""
        sp = rest.split("## Short Script (88 seconds)")
        if len(sp) > 1:
            sp_tail = sp[1]
            idx2 = sp_tail.find("\n## ")
            short_script = (sp_tail[:idx2] if idx2 >= 0 else sp_tail).strip().rstrip("-").strip()
    return full_story, short_script


def step_voice():
    """Generate multi-voice audio via ElevenLabs with word-level timestamps, per language."""
    log("STEP 6: VOICE — generating audio via ElevenLabs (per language)")

    # Find per-language script files; fall back to 03_script.md for EN only
    lang_files = []
    for lang in LANGUAGES:
        p = PIPELINE_DIR / f"03_script_{lang['code']}.md"
        if p.exists():
            lang_files.append((lang, p))
    if not lang_files:
        legacy = PIPELINE_DIR / "03_script.md"
        if legacy.exists():
            lang_files = [(LANGUAGES[0], legacy)]
        else:
            log("No script files found — run step 3 first")
            return None

    # Parse god name from first script file (same for all languages)
    god_name = "Unknown"
    for line in lang_files[0][1].read_text().split("\n"):
        if line.startswith("- **God:**"):
            god_name = line.split("**God:**")[1].strip()
            break

    episode_dir = GODS_DIR / god_name
    episode_dir.mkdir(parents=True, exist_ok=True)

    audio_files = {}          # "{code}_full" / "{code}_short" -> path
    all_timestamps = {}       # same keys -> word list

    # Config per version. v3 gives us audio tags; v2 gives us word-timestamps (karaoke).
    # Long form: v3 natural (storytelling). Short: v3 creative (tags more expressive).
    VOICE_MODELS = {
        "full":  {"model_id": "eleven_v3",                "stability": 0.5,  "label": "v3 natural"},
        "short": {"model_id": "eleven_v3",                "stability": 0.3,  "label": "v3 creative"},
    }
    MAX_CHARS = 4800  # v3 limit is 5000 — leave headroom for safety

    def chunk_by_paragraph(text, max_chars=MAX_CHARS):
        """Split on blank lines; merge back until each chunk fits under max_chars."""
        paras = [p.strip() for p in text.split("\n\n") if p.strip()]
        chunks, buf = [], ""
        for p in paras:
            candidate = (buf + "\n\n" + p).strip() if buf else p
            if len(candidate) <= max_chars:
                buf = candidate
            else:
                if buf:
                    chunks.append(buf)
                # paragraph itself larger than max_chars: hard split by sentence
                if len(p) > max_chars:
                    acc = ""
                    for sent in p.replace("\n", " ").split(". "):
                        piece = (acc + ". " + sent).strip(". ") if acc else sent
                        if len(piece) <= max_chars:
                            acc = piece
                        else:
                            if acc:
                                chunks.append(acc + ".")
                            acc = sent
                    if acc:
                        chunks.append(acc)
                    buf = ""
                else:
                    buf = p
        if buf:
            chunks.append(buf)
        return chunks

    def tts_v3_chunk(text, voice_id, model_id, stability, label):
        """Single-request v3 TTS (no timestamps — v3's with-timestamps is v2-only)."""
        try:
            r = requests.post(
                f"https://api.elevenlabs.io/v1/text-to-speech/{voice_id}",
                headers={"xi-api-key": ELEVENLABS_API_KEY, "Content-Type": "application/json"},
                json={
                    "text": text,
                    "model_id": model_id,
                    "voice_settings": {"stability": stability, "similarity_boost": 0.75},
                },
                timeout=180,
            )
            if r.status_code != 200:
                log(f"  {label}: HTTP {r.status_code} — {r.text[:200]}")
                return None
            return r.content
        except Exception as e:
            log(f"  {label}: error {e}")
            return None

    def tts_with_alignment(text, voice_id, label):
        """v2 TTS with character-level alignment for karaoke. Returns (audio_bytes, word_timestamps)."""
        try:
            r = requests.post(
                f"https://api.elevenlabs.io/v1/text-to-speech/{voice_id}/with-timestamps",
                headers={"xi-api-key": ELEVENLABS_API_KEY, "Content-Type": "application/json"},
                json={
                    "text": text,
                    "model_id": "eleven_multilingual_v2",
                    "voice_settings": {"stability": 0.6, "similarity_boost": 0.8},
                },
                timeout=120,
            )
            if r.status_code != 200:
                log(f"  {label}: HTTP {r.status_code} — {r.text[:200]}")
                return None, []
            data = r.json()
            import base64
            audio = base64.b64decode(data["audio_base64"])
            alignment = data.get("alignment", {})
            char_starts = alignment.get("character_start_times_seconds", [])
            char_ends = alignment.get("character_end_times_seconds", [])
            words = text.split()
            word_ts, ci = [], 0
            for w in words:
                if ci < len(char_starts):
                    end_ci = min(ci + len(w) - 1, len(char_ends) - 1)
                    word_ts.append({"word": w, "start": char_starts[ci], "end": char_ends[end_ci]})
                    ci += len(w) + 1
            return audio, word_ts
        except Exception as e:
            log(f"  {label}: alignment error {e}")
            return None, []

    def concat_mp3s(chunk_bytes_list, output_path):
        """Concatenate mp3 chunk bytes into one file via ffmpeg."""
        tmp_paths = []
        for i, b in enumerate(chunk_bytes_list):
            tp = output_path.parent / f".{output_path.stem}_chunk{i:02d}.mp3"
            tp.write_bytes(b)
            tmp_paths.append(tp)
        list_path = output_path.parent / f".{output_path.stem}_concat.txt"
        list_path.write_text("\n".join([f"file '{p}'" for p in tmp_paths]))
        subprocess.run(
            ["ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(list_path),
             "-c", "copy", str(output_path)],
            check=True, capture_output=True,
        )
        for p in tmp_paths + [list_path]:
            try: p.unlink()
            except Exception: pass

    def strip_audio_tags(text):
        """Remove bracketed audio tags for v2 alignment (v2 doesn't understand them)."""
        import re
        return re.sub(r"\[[^\]]*\]", "", text).replace("  ", " ").strip()

    # ── STAGE-PLAY PARSER ────────────────────────────────────────────────
    # Script format: blocks starting with `SPEAKER: ...`, separated by blank lines.
    # Returns [{speaker: "anky"|"<god_key>", text: "..."}].
    def parse_stage_play(text, god_key):
        import re
        god_label = god_key.upper()
        segments = []
        current = None  # {speaker, lines}
        for raw in text.split("\n"):
            line = raw.rstrip()
            if not line.strip():
                if current:
                    segments.append(current)
                    current = None
                continue
            m = re.match(r"^\s*([A-ZÁÉÍÓÚÑ][A-ZÁÉÍÓÚÑ_0-9 ]*):\s*(.*)$", line)
            if m:
                label = m.group(1).strip().upper()
                rest = m.group(2)
                # commit previous block
                if current:
                    segments.append(current)
                if label == "ANKY":
                    current = {"speaker": "anky", "lines": ([rest] if rest else [])}
                elif label == god_label:
                    current = {"speaker": god_key, "lines": ([rest] if rest else [])}
                else:
                    # Unknown speaker — treat as Anky narration so we don't drop content
                    log(f"  (parser) unknown speaker '{label}' — routing to ANKY")
                    current = {"speaker": "anky", "lines": ([f"{label}: {rest}"] if rest else [])}
            else:
                # continuation line of current speaker (e.g. indented poetry)
                if current is None:
                    current = {"speaker": "anky", "lines": [line.strip()]}
                else:
                    current["lines"].append(line.strip())
        if current:
            segments.append(current)
        # flatten lines to a single text block per segment
        out = []
        for s in segments:
            txt = " ".join([ln for ln in s["lines"] if ln]).strip()
            if txt:
                out.append({"speaker": s["speaker"], "text": txt})
        return out

    def concat_with_pauses(chunk_bytes_list, output_path, pause_ms_between=220):
        """Concatenate mp3 chunks with silent pauses between speaker shifts. Uses ffmpeg."""
        if not chunk_bytes_list:
            return False
        tmp_paths = []
        for i, b in enumerate(chunk_bytes_list):
            tp = output_path.parent / f".{output_path.stem}_seg{i:03d}.mp3"
            tp.write_bytes(b)
            tmp_paths.append(tp)
        # Generate silence file once
        silence_path = output_path.parent / f".{output_path.stem}_silence.mp3"
        subprocess.run(
            ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
             "-f", "lavfi", "-i", f"anullsrc=r=44100:cl=mono",
             "-t", f"{pause_ms_between/1000:.3f}",
             "-c:a", "libmp3lame", "-b:a", "128k",
             str(silence_path)],
            check=True, capture_output=True,
        )
        # Concat list: segment, silence, segment, silence, ..., segment
        list_lines = []
        for i, p in enumerate(tmp_paths):
            list_lines.append(f"file '{p}'")
            if i < len(tmp_paths) - 1:
                list_lines.append(f"file '{silence_path}'")
        list_path = output_path.parent / f".{output_path.stem}_concat.txt"
        list_path.write_text("\n".join(list_lines))
        subprocess.run(
            ["ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
             "-f", "concat", "-safe", "0", "-i", str(list_path),
             "-c:a", "libmp3lame", "-b:a", "192k", str(output_path)],
            check=True, capture_output=True,
        )
        for p in tmp_paths + [silence_path, list_path]:
            try: p.unlink()
            except Exception: pass
        return True

    def generate_voice_multi(script_text, god_key, lang_code, output_path, version_key, label):
        """Parse stage-play script, generate each segment with the right voice, stitch."""
        cfg = VOICE_MODELS[version_key]
        segments = parse_stage_play(script_text, god_key)
        if not segments:
            log(f"  {label}: parser found no segments — script may be in old format")
            # fallback: single-voice full narration
            anky_v = _voice_for("anky", lang_code)["voice_id"]
            chunks = chunk_by_paragraph(script_text)
            audio_chunks = []
            for i, chunk in enumerate(chunks):
                b = tts_v3_chunk(chunk, anky_v, cfg["model_id"], cfg["stability"], f"{label} fallback {i+1}/{len(chunks)}")
                if not b:
                    return None, []
                audio_chunks.append(b)
            if len(audio_chunks) == 1:
                output_path.write_bytes(audio_chunks[0])
            else:
                concat_mp3s(audio_chunks, output_path)
            return output_path, []

        log(f"  {label}: {len(segments)} speaker segments ({cfg['label']})")
        audio_segments = []
        for i, seg in enumerate(segments):
            voice = _voice_for(seg["speaker"], lang_code)
            v_id = voice["voice_id"]
            seg_label = f"{label} [{i+1}/{len(segments)}] {seg['speaker']}({voice.get('name','?')})"
            # Segments should always be under 5000 chars given story length, but guard anyway.
            seg_text = seg["text"]
            if len(seg_text) > MAX_CHARS:
                seg_chunks = chunk_by_paragraph(seg_text)
            else:
                seg_chunks = [seg_text]
            for j, chunk in enumerate(seg_chunks):
                b = tts_v3_chunk(chunk, v_id, cfg["model_id"], cfg["stability"],
                                 f"{seg_label}{'.'+str(j+1) if len(seg_chunks)>1 else ''}")
                if not b:
                    log(f"  {label}: segment {i+1} failed, aborting")
                    return None, []
                audio_segments.append(b)
        concat_with_pauses(audio_segments, output_path, pause_ms_between=220)
        log(f"  {label}: wrote {output_path.stat().st_size/1024:.0f}KB ({len(audio_segments)} audio blocks)")
        return output_path, []

    for lang, script_path in lang_files:
        code = lang["code"]
        log(f"  → {lang['name']} ({code})")
        full_story, short_script = _extract_script_sections(script_path.read_text())
        god_key = god_name.lower()

        if full_story:
            out = episode_dir / f"voice_full_{code}.mp3"
            path, timestamps = generate_voice_multi(full_story, god_key, code, out, "full", f"Full [{code}]")
            if path:
                audio_files[f"{code}_full"] = str(out)
                if timestamps:
                    all_timestamps[f"{code}_full"] = timestamps

        if short_script:
            out = episode_dir / f"voice_short_{code}.mp3"
            path, timestamps = generate_voice_multi(short_script, god_key, code, out, "short", f"Short [{code}]")
            if path:
                audio_files[f"{code}_short"] = str(out)
                if timestamps:
                    all_timestamps[f"{code}_short"] = timestamps

    # Save timestamps for karaoke subtitle generation
    ts_path = episode_dir / "timestamps.json"
    ts_path.write_text(json.dumps(all_timestamps, indent=2))

    def _row(k, v):
        ts = all_timestamps.get(k, [])
        dur = f"{ts[-1]['end']:.1f}s, {len(ts)} words" if ts else "—"
        url = f"/api/station/asset/{god_name}/{Path(v).name}"
        return (f"**{k}** — `{Path(v).name}` ({dur})\n\n"
                f'<audio controls src="{url}"></audio>')

    # Cast used for this episode
    cast_lines = []
    for lang in LANGUAGES:
        av = _voice_for("anky", lang["code"])
        cast_lines.append(f"- **Anky ({lang['name']}):** {av.get('name','?')} — `{av['voice_id'][:8]}…`")
        gv = _voice_for(god_name.lower(), lang["code"])
        cast_lines.append(f"- **{god_name} ({lang['name']}):** {gv.get('name','?')} — `{gv['voice_id'][:8]}…`")

    md = f"""# Voice — {god_name}

## Assembled audio (multi-voice)

{chr(10).join([_row(k, v) for k, v in audio_files.items()]) or '*No audio generated*'}

---

## Voice Cast

{chr(10).join(cast_lines)}

---

*Voice generated at {datetime.now().isoformat()} · model: eleven_v3*
*Each speaker (ANKY, {god_name.upper()}) voiced separately, stitched with 220ms pauses at speaker shifts.*
"""
    return write_step("06_voice.md", md)


# ── STEP 7: VIDEO ──────────────────────────────────────────────────────────

def step_video():
    """Assemble final videos with karaoke subtitles — one per (language, version)."""
    log("STEP 7: VIDEO — assembling final videos (per language)")

    script_path = PIPELINE_DIR / "03_script.md"
    god_name = "Unknown"
    if script_path.exists():
        for line in script_path.read_text().split("\n"):
            if line.startswith("- **God:**"):
                god_name = line.split("**God:**")[1].strip()

    episode_dir = GODS_DIR / god_name
    videos = {}

    ts_path = episode_dir / "timestamps.json"
    timestamps = {}
    if ts_path.exists():
        timestamps = json.loads(ts_path.read_text())

    # Build (lang, version) matrix. Shared images across languages.
    active_langs = [lang for lang in LANGUAGES if (PIPELINE_DIR / f"03_script_{lang['code']}.md").exists()]
    if not active_langs:
        active_langs = [LANGUAGES[0]]  # fallback EN

    for lang in active_langs:
        code = lang["code"]
        for version in ["short", "long"]:
            audio_variant = "short" if version == "short" else "full"
            audio_path = episode_dir / f"voice_{audio_variant}_{code}.mp3"
            # legacy fallback (pre-multilang runs): voice_full.mp3 / voice_short.mp3
            if not audio_path.exists() and code == "en":
                legacy = episode_dir / f"voice_{audio_variant}.mp3"
                if legacy.exists():
                    audio_path = legacy
            if not audio_path.exists():
                log(f"  No audio for {code}/{version}, skipping")
                continue

            scene_prefix = "short" if version == "short" else "long"
            scenes = sorted(episode_dir.glob(f"{scene_prefix}_scene_*.png"))
            if not scenes:
                scenes = sorted(episode_dir.glob("scene_*.png"))
            if not scenes:
                log(f"  No scene images for {version}, skipping")
                continue

            probe = subprocess.run(
                ["ffprobe", "-v", "quiet", "-show_entries", "format=duration", "-of", "csv=p=0", str(audio_path)],
                capture_output=True, text=True
            )
            audio_duration = float(probe.stdout.strip()) if probe.stdout.strip() else 60
            img_duration = audio_duration / len(scenes)

            # Karaoke ASS subtitles from per-language timestamps
            ass_path = episode_dir / f"karaoke_{version}_{code}.ass"
            ts_key = f"{code}_{'full' if version == 'long' else 'short'}"
            if timestamps.get(ts_key):
                generate_ass_subtitles(timestamps[ts_key], ass_path)

            concat_path = episode_dir / f"concat_{version}_{code}.txt"
            with open(concat_path, "w") as f:
                for scene in scenes:
                    f.write(f"file '{scene}'\n")
                    f.write(f"duration {img_duration:.3f}\n")
                f.write(f"file '{scenes[-1]}'\n")

            output_path = episode_dir / f"{god_name}_{version}_{code}.mp4"
            vf_base = "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black,format=yuv420p"
            vf = f"{vf_base},ass={ass_path}" if ass_path.exists() else vf_base

            cmd = [
                "ffmpeg", "-y",
                "-f", "concat", "-safe", "0", "-i", str(concat_path),
                "-i", str(audio_path),
                "-vf", vf,
                "-c:v", "av1_nvenc", "-cq", "30", "-preset", "p4",
                "-c:a", "aac", "-b:a", "192k",
                "-shortest",
                "-movflags", "+faststart",
                str(output_path),
            ]

            log(f"  Assembling {code}/{version}...")
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
            if result.returncode == 0:
                size = output_path.stat().st_size / (1024 * 1024)
                videos[f"{code}_{version}"] = str(output_path)
                log(f"  {code}/{version}: {output_path.name} ({size:.1f}MB)")
            else:
                log(f"  ffmpeg error for {code}/{version}: {result.stderr[-200:]}")

    def _video_entry(k, v):
        name = Path(v).name
        size_mb = Path(v).stat().st_size / (1024 * 1024)
        url = f"/api/station/asset/{god_name}/{name}"
        return (f"**{k}** — `{name}` ({size_mb:.1f}MB)\n\n"
                f'<video controls src="{url}" style="max-width:360px;width:100%;border-radius:6px;border:1px solid #2a2a2a"></video>')

    video_blocks = "\n\n".join([_video_entry(k, v) for k, v in videos.items()]) or "*No videos assembled*"

    md = f"""# Video — {god_name}

{video_blocks}

---

## Assembly Details

- Codec: AV1 (av1_nvenc) + AAC
- Resolution: 1080x1920 (vertical)
- Languages: {", ".join(lang["name"] for lang in active_langs)}
- Naming: `{{god}}_{{short|long}}_{{lang_code}}.mp4`

*Videos assembled at {datetime.now().isoformat()} · `{episode_dir}`*
"""
    return write_step("07_video.md", md)


def generate_ass_subtitles(word_timestamps, output_path):
    """Generate ASS subtitle file with karaoke-style word highlighting."""
    header = """[Script Info]
Title: Anky Karaoke
ScriptType: v4.00+
PlayResX: 1080
PlayResY: 1920

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,48,&H00FFFFFF,&H0000FFFF,&H00000000,&H80000000,1,0,0,0,100,100,0,0,1,3,0,2,40,40,60,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
    lines = [header]

    # Group words into subtitle lines (~8 words each)
    chunk_size = 8
    for i in range(0, len(word_timestamps), chunk_size):
        chunk = word_timestamps[i:i+chunk_size]
        start = chunk[0]["start"]
        end = chunk[-1]["end"]

        # Format timestamps as H:MM:SS.cc
        def fmt(t):
            h = int(t // 3600)
            m = int((t % 3600) // 60)
            s = t % 60
            return f"{h}:{m:02d}:{s:05.2f}"

        text = " ".join([w["word"] for w in chunk])
        lines.append(f"Dialogue: 0,{fmt(start)},{fmt(end)},Default,,0,0,0,,{text}")

    output_path.write_text("\n".join(lines), encoding="utf-8")


# ── STEP 8: PUBLISH ────────────────────────────────────────────────────────

def _extract_parent_note(md_text):
    """Pull the 'For the Parent' section out of a 03_script_*.md file."""
    for heading in ("## For the Parent (YouTube description)", "## For the Parent"):
        if heading in md_text:
            tail = md_text.split(heading, 1)[1]
            idx = tail.find("\n## ")
            if idx < 0:
                idx = tail.find("\n---")
            return (tail[:idx] if idx >= 0 else tail).strip()
    return ""


def step_publish():
    """Compose platform-ready captions (per language) and notify JP via Telegram."""
    log("STEP 8: PUBLISH — composing captions and notifying JP (per language)")

    index_path = PIPELINE_DIR / "03_script.md"
    god_name = "Unknown"
    kingdom_name = "Primordia"
    if index_path.exists():
        for line in index_path.read_text().split("\n"):
            if line.startswith("- **God:**"):
                god_name = line.split("**God:**")[1].strip()
            elif line.startswith("- **Kingdom:**"):
                kingdom_name = line.split("**Kingdom:**")[1].strip()

    kingdom = KINGDOMS.get(kingdom_name, KINGDOMS["Primordia"])
    episode_dir = GODS_DIR / god_name

    per_lang = []  # list of dicts: {lang, parent_note, short_script, captions, short_path, long_path}

    for lang in LANGUAGES:
        code = lang["code"]
        script_file = PIPELINE_DIR / f"03_script_{code}.md"
        if not script_file.exists():
            continue
        text = script_file.read_text()
        _, short_script = _extract_script_sections(text)
        parent_note = _extract_parent_note(text)
        short_path = episode_dir / f"{god_name}_short_{code}.mp4"
        long_path = episode_dir / f"{god_name}_long_{code}.mp4"

        captions = llm_query(
            f"""You are writing social media captions for today's Anky episode, in {lang['name']}.

God: {god_name}
Kingdom: {kingdom_name} ({kingdom['element']} element)
Short script preview: {short_script[:400]}

Write captions in {lang['name']}. Sharp, memorable, mysterious. Do NOT explain what Anky is. Let the content speak. For Spanish, use neutral Latin American register.

Format exactly like this (keep the English labels, write the content in {lang['name']}):

INSTAGRAM:
[caption for 88-sec reel, 2-3 lines, include 3-5 hashtags]

TIKTOK_TITLE:
[title for 8-min video, under 80 chars]

TIKTOK_DESC:
[description for 8-min video, 2-3 lines]

X_TWEET:
[one tweet, under 280 chars, no hashtags]

FARCASTER:
[one cast, under 320 chars]

YOUTUBE_TITLE:
[title for 8-min video on YouTube, under 100 chars]""",
            system=f"You are a social media strategist for a mythological children's story series. You write in {lang['name']}. Mysterious, not explanatory.",
            temperature=0.8,
            max_tokens=800,
        )

        per_lang.append({
            "lang": lang,
            "parent_note": parent_note,
            "short_script": short_script,
            "captions": captions or "",
            "short_path": short_path,
            "long_path": long_path,
        })

    # Compose Telegram message for JP — dual-language summary
    tg_parts = [f"🔮 Anky — {datetime.now().strftime('%B %d')}",
                f"Today's god: {god_name}",
                f"Kingdom: {kingdom_name} ({kingdom['element']})",
                ""]
    for pl in per_lang:
        tg_parts.append(f"──── {pl['lang']['name']} ({pl['lang']['code']}) ────")
        tg_parts.append(pl["captions"] or "[caption generation failed]")
        tg_parts.append(f"Short: {pl['short_path']}  {'✅' if pl['short_path'].exists() else '❌'}")
        tg_parts.append(f"Long:  {pl['long_path']}  {'✅' if pl['long_path'].exists() else '❌'}")
        tg_parts.append("")
    telegram_msg = "\n".join(tg_parts)
    notify_jp_telegram(telegram_msg)

    # Markdown report with per-language sections + YouTube descriptions
    md_sections = [f"# Publish — {god_name}\n"]
    for pl in per_lang:
        lang = pl["lang"]
        md_sections.append(f"## {lang['name']} ({lang['code']})\n")
        md_sections.append(f"**Videos**")
        md_sections.append(f"- Short (Instagram/Shorts): `{pl['short_path'].name}` {'[EXISTS]' if pl['short_path'].exists() else '[NOT FOUND]'}")
        md_sections.append(f"- Long (YouTube/TikTok): `{pl['long_path'].name}` {'[EXISTS]' if pl['long_path'].exists() else '[NOT FOUND]'}\n")
        md_sections.append("**Platform Captions**\n")
        md_sections.append(pl["captions"] or "*[Caption generation failed]*")
        md_sections.append("\n**YouTube Description (long version)**\n")
        md_sections.append(pl["parent_note"] or "*[Parent note missing]*")
        md_sections.append("\n---\n")

    md_sections.append("## Channel\n")
    md_sections.append("- Channel: **Anky** (single channel)")
    md_sections.append("- Playlists: one per language — e.g. `Anky — English`, `Anky — Español`\n")
    md_sections.append(f"*Publish step completed at {datetime.now().isoformat()}*")

    return write_step("08_publish.md", "\n".join(md_sections))


# ── ORCHESTRATOR ────────────────────────────────────────────────────────────

STEPS = {
    "zeitgeist": step_zeitgeist,
    "council":   step_council,
    "script":    step_script,
    "prompts":   step_prompts,
    "images":    step_images,
    "voice":     step_voice,
    "video":     step_video,
    "publish":   step_publish,
}

def run_full_pipeline(god_override=None):
    """Run the complete pipeline end-to-end."""
    today = datetime.now().strftime("%Y-%m-%d")
    log(f"═══ GODS PIPELINE — {today} ═══")

    # Create pipeline directory for today's run
    PIPELINE_DIR.mkdir(parents=True, exist_ok=True)

    # Clear previous run
    for f in PIPELINE_DIR.glob("*.md"):
        f.unlink()
    for f in PIPELINE_DIR.glob("*.json"):
        f.unlink()
    for f in PIPELINE_DIR.glob("*.log"):
        f.unlink()

    start = time.time()

    step_zeitgeist()
    step_council(god_override=god_override)
    step_script()
    step_prompts()
    step_images()
    step_voice()
    step_video()
    step_publish()

    elapsed = time.time() - start
    log(f"═══ PIPELINE COMPLETE — {elapsed/60:.1f} minutes ═══")

    # Write summary
    summary = f"""# Pipeline Complete

**Date:** {today}
**Duration:** {elapsed/60:.1f} minutes
**Steps:** 8/8

## Output Files

"""
    for md_file in sorted(PIPELINE_DIR.glob("*.md")):
        if md_file.name != "pipeline.log":
            summary += f"- [{md_file.name}](./{md_file.name})\n"

    write_step("00_summary.md", summary)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="GODS by Anky — Video Pipeline")
    parser.add_argument("--step", choices=list(STEPS.keys()), help="Run a single step")
    parser.add_argument("--god", type=str, help="Override god choice")
    args = parser.parse_args()

    # Load env
    from dotenv import load_dotenv
    load_dotenv(BASE_DIR / ".env")
    ELEVENLABS_API_KEY = os.getenv("ELEVENLABS_API_KEY", "")
    GROK_API_KEY = os.getenv("GROK_API_KEY", "")
    ANTHROPIC_API_KEY = os.getenv("ANTHROPIC_API_KEY", "")
    TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN", "")
    TELEGRAM_CHAT_ID = os.getenv("TELEGRAM_CHAT_ID", "")

    PIPELINE_DIR.mkdir(parents=True, exist_ok=True)

    if args.step:
        fn = STEPS[args.step]
        if args.step == "council":
            fn(god_override=args.god)
        else:
            fn()
    else:
        run_full_pipeline(god_override=args.god)
