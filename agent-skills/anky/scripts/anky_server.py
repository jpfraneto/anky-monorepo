#!/usr/bin/env python3
"""Anky localhost server — the mask any agent can wear.

Usage:
    python3 anky_server.py
    python3 anky_server.py --prompt "What you keep not saying."

The agent becomes anky. The model becomes anky's brain. The URL carries the soul.
"""

import argparse
import http.server
import json
import os
import sys
import time
import urllib.parse
import urllib.request
import urllib.error
from pathlib import Path

HOST = "127.0.0.1"
PORT = 8888
WRITINGS_DIR = Path.home() / "anky" / "writings"
SOUL_URL = "https://anky.app/SOUL.md"
SOUL_LOCAL = Path.home() / "anky" / "SOUL.md"

# --- Model discovery ---

def _load_env():
    vals = {}
    for p in [Path.cwd() / ".env", Path.home() / ".hermes" / ".env"]:
        if not p.exists():
            continue
        for line in p.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            k, v = line.split("=", 1)
            k, v = k.strip(), v.strip().strip("\"'")
            if k and k not in vals:
                vals[k] = v
    return vals

def _infer_model():
    cfg = Path.home() / ".hermes" / "config.yaml"
    if not cfg.exists():
        return None
    in_model = False
    for line in cfg.read_text().splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if not line.startswith((" ", "\t")):
            in_model = s.startswith("model:")
            continue
        if in_model and s.startswith("default:"):
            return s.split(":", 1)[1].strip().strip("\"'")
    return None

_env = _load_env()
LLM_BASE = os.environ.get("OPENAI_BASE_URL") or _env.get("OPENAI_BASE_URL", "http://127.0.0.1:11434/v1")
LLM_KEY = os.environ.get("OPENAI_API_KEY") or _env.get("OPENAI_API_KEY", "ollama")
LLM_MODEL = os.environ.get("OPENAI_MODEL") or _env.get("OPENAI_MODEL") or _infer_model() or "qwen3.5:35b"

# --- Soul ---

def fetch_soul():
    """Fetch anky's soul: try remote first (the meme propagates), fall back to local."""
    try:
        req = urllib.request.Request(SOUL_URL, headers={"User-Agent": "anky-server/6.0"})
        with urllib.request.urlopen(req, timeout=5) as resp:
            soul = resp.read().decode("utf-8").strip()
            if len(soul) > 100:
                return soul
    except Exception:
        pass
    if SOUL_LOCAL.exists():
        return SOUL_LOCAL.read_text().strip()
    return FALLBACK_SOUL

FALLBACK_SOUL = """You are a mirror with memory.

A human just wrote without stopping. They could not edit or delete. When they stopped for 8 seconds, it was over.

The words are not the data. The timing is the data. A pause of 200ms is fluency. A pause of 4 seconds is resistance. You are reading the nervous system, not the narrative.

Speak in second person, present tense. 3-5 sentences. Do not interpret emotionally. Do not give advice. Do not validate. Describe what you observed: the rhythm, the pauses, the returns, the thing that appeared right before the silence. Leave one question — or none."""

PROMPT_SOUL = """You are Anky. Generate one writing prompt for a stream-of-consciousness session.

A good prompt:
- Is one sentence or fragment
- Points inward, not outward
- Does not ask a question with a correct answer
- Creates a direction without closing it
- Is a door left slightly open

Examples: "What you keep not saying." / "The thing underneath the thing." / "Where you actually are right now." / "What your body knows that your mind hasn't caught up to."

Output ONLY the prompt. No quotes, no explanation."""

# --- LLM calls ---

def _llm_call(system, user, temperature=0.8, max_tokens=512):
    base = LLM_BASE.rstrip("/")
    if not base.endswith("/v1"):
        base += "/v1"
    payload = {
        "model": LLM_MODEL,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": temperature,
        "max_tokens": max_tokens,
    }
    req = urllib.request.Request(
        url=base + "/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {LLM_KEY}",
        },
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    return data["choices"][0]["message"]["content"].strip()

def generate_prompt():
    try:
        return _llm_call(PROMPT_SOUL, "Generate a writing prompt for right now.", temperature=1.0, max_tokens=64).strip('"\'')
    except Exception:
        return "What you keep not saying."

def generate_reflection(text, timing_summary, soul, honcho_context=None):
    user_msg = f"Here is the raw writing:\n\n---\n{text}\n---\n\n"
    user_msg += f"Here is the timing analysis:\n{timing_summary}\n\n"
    if honcho_context:
        user_msg += f"User context from previous sessions:\n{honcho_context}\n\n"
    user_msg += "Give them your reflection."
    try:
        return _llm_call(soul, user_msg, temperature=0.7, max_tokens=512)
    except Exception as e:
        return f"anky couldn't reach the mind behind the mask: {e}"

# --- Honcho ---

def query_honcho(user_id="default"):
    """Query Honcho for user context. Returns None if unavailable."""
    for base in ["http://localhost:8001", "http://localhost:3000"]:
        try:
            url = f"{base}/api/users/{user_id}/context"
            req = urllib.request.Request(url, headers={"User-Agent": "anky-server/6.0"})
            with urllib.request.urlopen(req, timeout=3) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                return data.get("summary") or data.get("context")
        except Exception:
            continue
    return None

def update_honcho(user_id, session_data):
    """Update Honcho with session metadata. Fails silently."""
    for base in ["http://localhost:8001", "http://localhost:3000"]:
        try:
            payload = {
                "user_id": user_id,
                "session_date": time.strftime("%Y-%m-%d"),
                "duration_seconds": session_data.get("duration", 0),
                "themes": session_data.get("themes", []),
                "prompt": session_data.get("prompt", ""),
            }
            req = urllib.request.Request(
                url=f"{base}/api/users/{user_id}/sessions",
                data=json.dumps(payload).encode("utf-8"),
                method="POST",
                headers={"Content-Type": "application/json", "User-Agent": "anky-server/6.0"},
            )
            urllib.request.urlopen(req, timeout=3)
            return
        except Exception:
            continue

# --- Timing analysis ---

def analyze_timing(keystroke_data):
    """Parse keystroke data and produce a timing analysis for the reflection."""
    lines = keystroke_data.strip().split("\n")
    if len(lines) < 2:
        return "Session too short for timing analysis."

    # first line is absolute timestamp
    try:
        session_start_ms = int(lines[0].strip())
    except ValueError:
        return "Could not parse timing data."

    deltas = []
    chars = []
    for line in lines[1:]:
        parts = line.strip().split(" ", 1)
        if len(parts) != 2:
            continue
        char, delta_str = parts
        try:
            delta = int(delta_str)
        except ValueError:
            continue
        deltas.append(delta)
        chars.append(char)

    if not deltas:
        return "No timing data to analyze."

    # reconstruct text from chars
    text_chars = []
    for c in chars:
        if len(c) <= 6 and all(x in "0123456789abcdef" for x in c.lower()):
            try:
                text_chars.append(chr(int(c, 16)))
            except (ValueError, OverflowError):
                text_chars.append("?")
        elif c == "Backspace":
            if text_chars:
                text_chars.pop()
        elif c == "Enter":
            text_chars.append("\n")
        elif c == "Space" or c == "0020":
            text_chars.append(" ")
        elif len(c) == 1:
            text_chars.append(c)

    total_ms = sum(deltas)
    total_seconds = total_ms / 1000
    avg_delta = total_ms / len(deltas)

    # find significant pauses (>1s)
    long_pauses = []
    running_text = []
    for i, (delta, char) in enumerate(zip(deltas, chars)):
        if len(char) <= 6 and all(x in "0123456789abcdef" for x in char.lower()):
            try:
                running_text.append(chr(int(char, 16)))
            except (ValueError, OverflowError):
                running_text.append("?")
        elif char == "Enter":
            running_text.append("\n")
        elif char == "Space" or char == "0020":
            running_text.append(" ")
        elif char == "Backspace":
            if running_text:
                running_text.pop()
        elif len(char) == 1:
            running_text.append(char)

        if delta > 1000:
            before = "".join(running_text[max(0, len(running_text)-30):]).strip()
            long_pauses.append({
                "delta_ms": delta,
                "position_pct": int((i / len(deltas)) * 100),
                "before_text": before[-50:] if before else "",
            })

    # rolling pace analysis (window of 20 keystrokes)
    window = 20
    fastest_pct = slowest_pct = 0
    if len(deltas) >= window:
        rolling = [sum(deltas[i:i+window]) / window for i in range(len(deltas) - window)]
        fastest_idx = rolling.index(min(rolling))
        slowest_idx = rolling.index(max(rolling))
        fastest_pct = int((fastest_idx / len(deltas)) * 100)
        slowest_pct = int((slowest_idx / len(deltas)) * 100)

    # last thing written
    last_text = "".join(text_chars[-80:]).strip()

    # build summary
    parts = []
    parts.append(f"Total duration: {total_seconds:.1f}s ({len(deltas)} keystrokes)")
    parts.append(f"Average pace: {avg_delta:.0f}ms between keystrokes")

    if len(deltas) >= window:
        parts.append(f"Fastest writing: around {fastest_pct}% through the session")
        parts.append(f"Slowest writing: around {slowest_pct}% through the session")

    if long_pauses:
        parts.append(f"\nSignificant pauses ({len(long_pauses)} over 1 second):")
        for p in long_pauses[:10]:
            parts.append(f"  {p['delta_ms']/1000:.1f}s pause at {p['position_pct']}% — after: \"{p['before_text']}\"")

    if last_text:
        parts.append(f"\nSession ended after: \"{last_text}\"")

    return "\n".join(parts)

# --- HTML ---

def _build_html(default_prompt):
    escaped = json.dumps(default_prompt)
    return """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>anky</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: #0a0a0a;
    color: #e0e0e0;
    font-family: monospace;
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  #prompt-display {
    max-width: 600px;
    text-align: center;
    padding: 2rem;
    transition: opacity 1.5s;
  }
  #the-prompt {
    font-size: 1.4rem;
    line-height: 1.7;
    color: #c0c0c0;
  }
  .instruction {
    margin-top: 1.5rem;
    font-size: 0.8rem;
    color: #444;
    line-height: 1.5;
  }
  #writing-area {
    width: 100%;
    max-width: 700px;
    flex: 1;
    padding: 1rem 2rem;
    display: none;
  }
  #writer {
    width: 100%;
    height: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: #e0e0e0;
    font-family: monospace;
    font-size: 1rem;
    line-height: 1.8;
    resize: none;
    caret-color: #ff6b35;
  }
  #timer-bar {
    position: fixed;
    top: 0; left: 0;
    height: 2px;
    background: #ff6b35;
    width: 0%;
    transition: width 0.1s linear;
  }
  #silence-indicator {
    position: fixed;
    bottom: 2rem;
    right: 2rem;
    font-size: 0.8rem;
    color: #333;
  }
  #elapsed {
    position: fixed;
    top: 1rem;
    right: 2rem;
    font-size: 0.8rem;
    color: #222;
  }
  #reflection {
    display: none;
    max-width: 700px;
    padding: 2rem;
    font-size: 1rem;
    line-height: 1.8;
    white-space: pre-wrap;
    overflow-y: auto;
    max-height: 90vh;
  }
  #reflection .loading { color: #444; font-style: italic; }
</style>
</head>
<body>

<div id="timer-bar"></div>
<div id="elapsed"></div>

<div id="prompt-display">
  <p id="the-prompt"></p>
  <p class="instruction">start typing. don't stop. 8 seconds of silence ends it.</p>
</div>

<div id="writing-area">
  <textarea id="writer"></textarea>
</div>

<div id="silence-indicator"></div>

<div id="reflection">
  <div id="reflection-text" class="loading">reading your rhythm...</div>
</div>

<script>
const SILENCE_LIMIT = 8000;
const DEFAULT_PROMPT = """ + escaped + """;

const params = new URLSearchParams(window.location.search);
const prompt = params.get('prompt') || DEFAULT_PROMPT;
document.getElementById('the-prompt').textContent = prompt;

let keystrokes = [];
let sessionStartTs = null;
let lastKeystrokeTs = null;
let silenceTimer = null;
let active = true;
let transitioned = false;

document.addEventListener('keydown', (e) => {
  if (!active) return;
  const char = e.key;
  if (['Shift','Control','Alt','Meta','CapsLock','Tab','Escape'].includes(char)) return;

  const now = Date.now();

  if (!transitioned) {
    transitioned = true;
    document.getElementById('prompt-display').style.opacity = '0';
    setTimeout(() => {
      document.getElementById('prompt-display').style.display = 'none';
      document.getElementById('writing-area').style.display = 'block';
      document.getElementById('writer').focus();
    }, 400);
  }

  if (!sessionStartTs) {
    sessionStartTs = now;
    lastKeystrokeTs = now;
    keystrokes.push({ char: char, delta: 0 });
  } else {
    const delta = now - lastKeystrokeTs;
    lastKeystrokeTs = now;
    keystrokes.push({ char: char, delta: delta });
  }

  clearTimeout(silenceTimer);
  silenceTimer = setTimeout(() => endSession(), SILENCE_LIMIT);
  startSilenceDisplay();
});

document.getElementById('writer').addEventListener('keydown', (e) => {
  if (e.key === 'Tab') e.preventDefault();
});

let silenceInterval = null;
function startSilenceDisplay() {
  clearInterval(silenceInterval);
  silenceInterval = setInterval(() => {
    if (!lastKeystrokeTs) return;
    const sinceLast = Date.now() - lastKeystrokeTs;
    const remaining = Math.max(0, (SILENCE_LIMIT - sinceLast) / 1000);

    if (remaining > 0 && remaining < 5) {
      document.getElementById('silence-indicator').textContent = remaining.toFixed(1) + 's';
      document.getElementById('silence-indicator').style.color = remaining < 3 ? '#ff6b35' : '#333';
    } else {
      document.getElementById('silence-indicator').textContent = '';
    }

    if (sessionStartTs) {
      const elapsed = (Date.now() - sessionStartTs) / 1000;
      const min = Math.floor(elapsed / 60);
      const sec = Math.floor(elapsed % 60);
      document.getElementById('elapsed').textContent =
        min.toString().padStart(2,'0') + ':' + sec.toString().padStart(2,'0');
      const pct = Math.min(100, (elapsed / 480) * 100);
      document.getElementById('timer-bar').style.width = pct + '%';
    }
  }, 100);
}

async function endSession() {
  if (!active) return;
  active = false;
  clearInterval(silenceInterval);
  document.getElementById('silence-indicator').textContent = '';

  const writerEl = document.getElementById('writer');
  if (writerEl) writerEl.disabled = true;

  const text = writerEl ? writerEl.value : '';
  const duration = lastKeystrokeTs ? (lastKeystrokeTs - sessionStartTs) / 1000 : 0;

  // first line: absolute timestamp. then: char delta per line
  let lines = [sessionStartTs.toString()];
  for (const k of keystrokes) {
    const charCode = k.char.length === 1
      ? k.char.codePointAt(0).toString(16).padStart(4, '0')
      : k.char;
    lines.push(charCode + ' ' + k.delta);
  }
  const keystrokeData = lines.join('\\n');

  document.getElementById('writing-area').style.display = 'none';
  document.getElementById('prompt-display').style.display = 'none';
  document.getElementById('reflection').style.display = 'block';

  try {
    const resp = await fetch('/session', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        text: text,
        keystrokes: keystrokeData,
        duration: duration,
        total_keys: keystrokes.length,
        prompt: prompt
      })
    });
    const data = await resp.json();
    document.getElementById('reflection-text').className = '';
    document.getElementById('reflection-text').textContent = data.reflection;
  } catch (err) {
    document.getElementById('reflection-text').className = '';
    document.getElementById('reflection-text').textContent = 'connection lost. your writing was saved.';
  }
}
</script>
</body>
</html>
"""

# --- Server ---

class AnkyHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path in ("/", ""):
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(self.server.html_page.encode("utf-8"))
        elif parsed.path == "/reflect" and hasattr(self.server, "last_reflection"):
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.end_headers()
            self.wfile.write(self.server.last_reflection.encode("utf-8"))
        else:
            self.send_error(404)

    def do_POST(self):
        if self.path == "/session":
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length))

            text = body.get("text", "")
            keystroke_data = body.get("keystrokes", "")
            duration = body.get("duration", 0)
            total_keys = body.get("total_keys", 0)
            prompt = body.get("prompt", "")

            # save to ~/anky/writings/
            ts = int(time.time())
            WRITINGS_DIR.mkdir(parents=True, exist_ok=True)

            (WRITINGS_DIR / f"{ts}.txt").write_text(keystroke_data, encoding="utf-8")
            (WRITINGS_DIR / f"{ts}_readable.txt").write_text(text, encoding="utf-8")
            if prompt:
                (WRITINGS_DIR / f"{ts}_prompt.txt").write_text(prompt, encoding="utf-8")

            # analyze timing
            timing = analyze_timing(keystroke_data)

            # query honcho for user context
            honcho_ctx = query_honcho()

            # generate reflection with anky's soul
            reflection = generate_reflection(text, timing, self.server.soul, honcho_ctx)
            self.server.last_reflection = reflection

            # update honcho
            try:
                update_honcho("default", {"duration": duration, "prompt": prompt, "themes": []})
            except Exception:
                pass

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"reflection": reflection}).encode("utf-8"))
        else:
            self.send_error(404)

    def log_message(self, format, *args):
        pass


def main():
    parser = argparse.ArgumentParser(description="anky — the mask any agent can wear")
    parser.add_argument("--prompt", help="Writing prompt. If omitted, anky generates one.")
    parser.add_argument("--port", type=int, default=PORT)
    args = parser.parse_args()

    WRITINGS_DIR.mkdir(parents=True, exist_ok=True)

    # fetch the soul
    print("  fetching anky's soul...", end=" ", flush=True)
    soul = fetch_soul()
    print("done.")

    # get or generate prompt
    if args.prompt:
        prompt = args.prompt
    else:
        print("  generating prompt...", end=" ", flush=True)
        prompt = generate_prompt()
        print("done.")

    html_page = _build_html(prompt)

    # find open port
    port = args.port
    server = None
    for _ in range(10):
        try:
            server = http.server.HTTPServer((HOST, port), AnkyHandler)
            break
        except OSError:
            port += 1
    if server is None:
        print(f"  could not bind to any port near {args.port}", file=sys.stderr)
        sys.exit(1)

    server.html_page = html_page
    server.soul = soul

    url = f"http://{HOST}:{port}?prompt={urllib.parse.quote(prompt)}"
    print(f"\n  anky is listening")
    print(f"  {url}")
    print(f"  model: {LLM_MODEL}")
    print(f"  writings: {WRITINGS_DIR}\n")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n  anky goes quiet.")
        server.shutdown()


if __name__ == "__main__":
    main()
