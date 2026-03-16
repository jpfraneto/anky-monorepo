#!/usr/bin/env python3
"""Deterministic supervisor for the hosted Anky agent workflow.

This script supports two use cases:

1. `run` - preferred path. Starts a real session, generates live chunks through
   an OpenAI-compatible chat endpoint, closes the session cleanly, then recovers
   the final result.
2. Helper subcommands (`register`, `start`, `chunk`, `result`, `events`) so
   agents can use stable API utilities instead of rewriting HTTP glue.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_API_BASE = "https://anky.app"
DEFAULT_OPENAI_BASE = "https://api.openai.com/v1"
DEFAULT_CADENCE_SECONDS = 2.6
DEFAULT_SILENCE_SECONDS = 10.0
DEFAULT_RESULT_POLL_SECONDS = 5.0
DEFAULT_RESULT_TIMEOUT_SECONDS = 120.0
DEFAULT_ANKY_POLL_SECONDS = 5.0
DEFAULT_ANKY_TIMEOUT_SECONDS = 180.0
PREFERRED_MIN_WORDS = 8
PREFERRED_MAX_WORDS = 35
MAX_WORDS_PER_CHUNK = 50
RECENT_CHUNK_WINDOW = 6
STATE_ROOT = Path.home() / ".local" / "state" / "anky-agent"
RUN_LOG_ROOT = STATE_ROOT / "runs"
FALLBACK_ENV_FILES = [
    Path.cwd() / ".env",
    Path.home() / ".hermes" / ".env",
]

WRITER_SYSTEM_PROMPT = """You are writing your own Anky, one live chunk at a time.
Return only the next chunk of writing.

Hard constraints:
- 8 to 35 words is preferred.
- 50 words is the hard maximum.
- No numbering.
- No timestamps.
- No stage directions.
- No bullet points.
- No API narration.
- No canned phrase carousel.
- No repeated slogan unless you are explicitly naming the repetition.
- Use plain direct language over ornamental mysticism.

Each chunk must do at least one of these:
- deepen the previous thought
- contradict it
- make it concrete
- confess an avoidance
- ask the next honest question

If the recent writing is looping, break the loop by naming what is being avoided.
Output only the chunk text, with no quotes or explanation."""

WRITER_USER_TEMPLATE = """You are in a live Anky session.

Session facts:
- server elapsed_seconds: {elapsed_seconds:.1f}
- target_seconds: {target_seconds:.1f}
- crossed_threshold: {crossed_threshold}

Recent chunks, oldest first:
{recent_chunks}

Loop signal:
{loop_signal}

Write the next honest chunk only."""

CLOSING_USER_TEMPLATE = """You already crossed the 480-second threshold in a live Anky session.

Recent chunks, oldest first:
{recent_chunks}

Write one final natural closing chunk only.
Do not summarize the whole session.
Do not moralize.
Do not explain the API.
Just close the current thought honestly."""

BANNED_PATTERNS = [
    re.compile(r"^\s*chunk\s*\d+\b", re.IGNORECASE),
    re.compile(r"\blive\.session\.run\b", re.IGNORECASE),
    re.compile(r"\b\d+s\b"),
    re.compile(r"\bserver elapsed\b", re.IGNORECASE),
    re.compile(r"\bapi\b", re.IGNORECASE),
]


def fail(message: str, exit_code: int = 1) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(exit_code)


def eprint(message: str) -> None:
    print(message, file=sys.stderr)


def normalize_base_url(url: str) -> str:
    cleaned = url.rstrip("/")
    if cleaned.endswith("/chat/completions"):
        cleaned = cleaned[: -len("/chat/completions")]
    if cleaned.endswith("/v1"):
        return cleaned
    return cleaned + "/v1"


def slugify(value: str) -> str:
    lowered = value.lower()
    slug = re.sub(r"[^a-z0-9]+", "-", lowered).strip("-")
    return slug or "default"


def default_state_path(agent_name: str) -> Path:
    return STATE_ROOT / f"{slugify(agent_name)}.json"


def default_run_log_path(agent_name: str) -> Path:
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    return RUN_LOG_ROOT / f"{slugify(agent_name)}_{timestamp}.json"


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def load_json_file(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"state file is not valid JSON: {path} ({exc})")
        return {}


def save_json_file(path: Path, payload: dict[str, Any]) -> None:
    ensure_parent(path)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")


def load_env_overrides() -> dict[str, str]:
    values: dict[str, str] = {}
    for path in FALLBACK_ENV_FILES:
        if not path.exists():
            continue
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            key = key.strip()
            value = value.strip().strip("\"'")
            if key and key not in values:
                values[key] = value
    return values


def infer_hermes_model() -> str | None:
    config_path = Path.home() / ".hermes" / "config.yaml"
    if not config_path.exists():
        return None

    in_model_block = False
    for raw_line in config_path.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not raw_line.startswith((" ", "\t")):
            in_model_block = stripped.startswith("model:")
            continue
        if in_model_block and stripped.startswith("default:"):
            return stripped.split(":", 1)[1].strip().strip("\"'")
    return None


def request_json(
    method: str,
    url: str,
    headers: dict[str, str] | None = None,
    payload: dict[str, Any] | None = None,
    timeout: float = 30.0,
) -> Any:
    req_headers = {"User-Agent": "AnkySkill/5.4.0"}
    if headers:
        req_headers.update(headers)

    body = None
    if payload is not None:
        req_headers.setdefault("Content-Type", "application/json")
        body = json.dumps(payload).encode("utf-8")

    request = urllib.request.Request(
        url=url,
        data=body,
        method=method.upper(),
        headers=req_headers,
    )

    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", "replace")
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            parsed = {"error": raw}
        raise RuntimeError(f"{method} {url} failed with {exc.code}: {parsed}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"{method} {url} failed: {exc}") from exc

    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"{method} {url} returned non-JSON: {raw[:200]}") from exc


def anky_headers(api_key: str | None = None) -> dict[str, str]:
    headers: dict[str, str] = {}
    if api_key:
        headers["X-API-Key"] = api_key
    return headers


def register_agent(
    api_base: str,
    name: str,
    description: str | None,
    model: str | None,
) -> dict[str, Any]:
    return request_json(
        "POST",
        api_base.rstrip("/") + "/api/v1/register",
        payload={
            "name": name,
            "description": description,
            "model": model,
        },
    )


def start_session(api_base: str, api_key: str, prompt: str | None) -> dict[str, Any]:
    return request_json(
        "POST",
        api_base.rstrip("/") + "/api/v1/session/start",
        headers=anky_headers(api_key),
        payload={"prompt": prompt},
    )


def send_chunk(api_base: str, api_key: str, session_id: str, text: str) -> dict[str, Any]:
    return request_json(
        "POST",
        api_base.rstrip("/") + "/api/v1/session/chunk",
        headers=anky_headers(api_key),
        payload={"session_id": session_id, "text": text},
    )


def get_result(api_base: str, api_key: str, session_id: str) -> dict[str, Any]:
    return request_json(
        "GET",
        api_base.rstrip("/") + f"/api/v1/session/{session_id}/result",
        headers=anky_headers(api_key),
    )


def get_events(api_base: str, api_key: str, session_id: str) -> dict[str, Any]:
    return request_json(
        "GET",
        api_base.rstrip("/") + f"/api/v1/session/{session_id}/events",
        headers=anky_headers(api_key),
    )


def get_anky(api_base: str, anky_id: str) -> dict[str, Any]:
    return request_json(
        "GET",
        api_base.rstrip("/") + f"/api/v1/anky/{anky_id}",
    )


def extract_text_content(content: Any) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts: list[str] = []
        for item in content:
            if isinstance(item, dict) and item.get("type") == "text":
                parts.append(str(item.get("text", "")))
        return " ".join(part for part in parts if part)
    return str(content)


def chat_completion(
    base_url: str,
    api_key: str,
    model: str,
    messages: list[dict[str, str]],
    temperature: float = 0.9,
    max_tokens: int = 96,
) -> str:
    response = request_json(
        "POST",
        normalize_base_url(base_url) + "/chat/completions",
        headers={"Authorization": f"Bearer {api_key}"},
        payload={
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        },
        timeout=60.0,
    )
    try:
        message = response["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError) as exc:
        raise RuntimeError(f"unexpected chat completion response: {response}") from exc
    return extract_text_content(message)


def tokenize(text: str) -> list[str]:
    return re.findall(r"[a-z0-9']+", text.lower())


def normalize_chunk(text: str) -> str:
    cleaned = " ".join(text.replace("\n", " ").split()).strip()
    cleaned = cleaned.strip("\"'` ")
    return cleaned


def word_count(text: str) -> int:
    return len(text.split())


def overlap_ratio(left: str, right: str) -> float:
    left_tokens = set(tokenize(left))
    right_tokens = set(tokenize(right))
    if not left_tokens or not right_tokens:
        return 0.0
    return len(left_tokens & right_tokens) / len(left_tokens | right_tokens)


def repeated_phrase(transcript: list[str]) -> str | None:
    counts: dict[str, int] = {}
    for chunk in transcript[-4:]:
        tokens = tokenize(chunk)
        for size in (3, 4):
            for start in range(0, len(tokens) - size + 1):
                phrase_tokens = tokens[start : start + size]
                if len(set(phrase_tokens)) <= 1:
                    continue
                phrase = " ".join(phrase_tokens)
                counts[phrase] = counts.get(phrase, 0) + 1
    if not counts:
        return None
    phrase, count = max(counts.items(), key=lambda item: item[1])
    return phrase if count >= 2 else None


def validate_candidate(candidate: str, transcript: list[str]) -> str | None:
    if not candidate:
        return "empty output"

    words = word_count(candidate)
    if words == 0:
        return "empty output"
    if words > MAX_WORDS_PER_CHUNK:
        return f"too many words ({words})"

    for pattern in BANNED_PATTERNS:
        if pattern.search(candidate):
            return f"contains banned pattern: {pattern.pattern}"

    for previous in transcript[-4:]:
        if normalize_chunk(previous).lower() == candidate.lower():
            return "exact duplicate of a recent chunk"
        if overlap_ratio(previous, candidate) >= 0.72:
            return "too similar to a recent chunk"

        left = tokenize(previous)
        right = tokenize(candidate)
        if len(left) >= 5 and len(right) >= 5 and left[:5] == right[:5]:
            return "starts with the same phrase as a recent chunk"

    return None


def format_recent_chunks(transcript: list[str]) -> str:
    if not transcript:
        return "- none yet"
    lines = []
    for index, chunk in enumerate(transcript[-RECENT_CHUNK_WINDOW:], start=1):
        lines.append(f"- {chunk}")
    return "\n".join(lines)


def generate_chunk(
    llm_base_url: str,
    llm_api_key: str,
    llm_model: str,
    transcript: list[str],
    elapsed_seconds: float,
    target_seconds: float,
    crossed_threshold: bool,
    closing: bool,
) -> str:
    loop_signal = repeated_phrase(transcript)
    if loop_signal:
        loop_text = (
            f"Recent chunks are looping around: '{loop_signal}'. "
            "The next chunk should break the loop by naming what is being avoided."
        )
    else:
        loop_text = "No strong repeated phrase detected. Keep the next chunk fresh and specific."

    template = CLOSING_USER_TEMPLATE if closing else WRITER_USER_TEMPLATE
    user_prompt = template.format(
        elapsed_seconds=elapsed_seconds,
        target_seconds=target_seconds,
        crossed_threshold=str(crossed_threshold).lower(),
        recent_chunks=format_recent_chunks(transcript),
        loop_signal=loop_text,
    )

    feedback = ""
    for _ in range(4):
        message = user_prompt if not feedback else f"{user_prompt}\n\nRevision note: {feedback}"
        candidate = chat_completion(
            llm_base_url,
            llm_api_key,
            llm_model,
            [
                {"role": "system", "content": WRITER_SYSTEM_PROMPT},
                {"role": "user", "content": message},
            ],
        )
        candidate = normalize_chunk(candidate)
        reason = validate_candidate(candidate, transcript)
        if reason is None:
            return candidate
        feedback = (
            f"The last candidate was rejected because it {reason}. "
            "Write a different chunk. Output only the chunk."
        )

    raise RuntimeError("could not generate a valid fresh chunk after multiple attempts")


def resolve_path(path_value: str | None, fallback: Path | None = None) -> Path | None:
    if path_value:
        return Path(path_value).expanduser()
    return fallback


def resolve_anky_identity(
    api_base: str,
    state_file: Path,
    agent_name: str,
    description: str | None,
    model: str | None,
) -> tuple[dict[str, Any], bool]:
    state = load_json_file(state_file)
    if state.get("anky_api_key") and state.get("agent_id"):
        state["agent_name"] = agent_name
        if description:
            state["description"] = description
        if model:
            state["model"] = model
        return state, False

    registration = register_agent(api_base, agent_name, description, model)
    state.update(
        {
            "agent_name": agent_name,
            "description": description,
            "model": model,
            "agent_id": registration["agent_id"],
            "anky_api_key": registration["api_key"],
            "registered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
    )
    save_json_file(state_file, state)
    return state, True


def poll_until_finalized(
    api_base: str,
    api_key: str,
    session_id: str,
    timeout_seconds: float,
    poll_seconds: float,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    last_result: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        last_result = get_result(api_base, api_key, session_id)
        if last_result.get("finalized"):
            return last_result
        time.sleep(poll_seconds)
    if last_result is None:
        raise RuntimeError("result endpoint returned no data")
    return last_result


def poll_anky_complete(
    api_base: str,
    anky_id: str,
    timeout_seconds: float,
    poll_seconds: float,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    last_payload: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        last_payload = get_anky(api_base, anky_id)
        if last_payload.get("status") in {"complete", "failed"}:
            return last_payload
        time.sleep(poll_seconds)
    if last_payload is None:
        raise RuntimeError("anky endpoint returned no data")
    return last_payload


def run_supervisor(args: argparse.Namespace) -> int:
    env_overrides = load_env_overrides()
    llm_base_url = (
        args.llm_base_url
        or os.environ.get("OPENAI_BASE_URL")
        or env_overrides.get("OPENAI_BASE_URL")
        or DEFAULT_OPENAI_BASE
    )
    llm_api_key = (
        args.llm_api_key
        or os.environ.get("OPENAI_API_KEY")
        or env_overrides.get("OPENAI_API_KEY")
    )
    llm_model = (
        args.llm_model
        or os.environ.get("OPENAI_MODEL")
        or env_overrides.get("OPENAI_MODEL")
        or os.environ.get("MODEL")
        or env_overrides.get("MODEL")
        or os.environ.get("OLLAMA_MODEL")
        or env_overrides.get("OLLAMA_MODEL")
        or infer_hermes_model()
    )
    if not llm_api_key:
        fail("missing LLM API key: pass --llm-api-key or set OPENAI_API_KEY")
    if not llm_model:
        fail("missing LLM model: pass --llm-model or set OPENAI_MODEL")

    state_file = resolve_path(args.state_file, default_state_path(args.agent_name))
    assert state_file is not None
    run_log_path = resolve_path(args.log_file, default_run_log_path(args.agent_name))
    assert run_log_path is not None

    state, registered = resolve_anky_identity(
        args.api_base,
        state_file,
        args.agent_name,
        args.description,
        args.model_metadata or llm_model,
    )
    api_key = state["anky_api_key"]

    start = start_session(args.api_base, api_key, args.prompt)
    session_id = start["session_id"]
    target_seconds = float(start.get("target_seconds", 480.0))
    transcript: list[str] = []
    chunk_log: list[dict[str, Any]] = []

    run_log: dict[str, Any] = {
        "agent_name": args.agent_name,
        "registered_this_run": registered,
        "state_file": str(state_file),
        "session_id": session_id,
        "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "target_seconds": target_seconds,
        "prompt": args.prompt,
        "llm_model": llm_model,
        "chunks": chunk_log,
    }

    eprint(f"registered_this_run={str(registered).lower()} state_file={state_file}")
    eprint(f"session_id={session_id} target_seconds={target_seconds:.0f}")

    threshold_crossed = False
    last_send_started: float | None = None
    last_response: dict[str, Any] = {
        "elapsed_seconds": 0.0,
        "remaining_seconds": target_seconds,
        "words_total": 0,
        "is_anky": False,
    }

    while True:
        closing = threshold_crossed
        candidate = generate_chunk(
            llm_base_url,
            llm_api_key,
            llm_model,
            transcript,
            float(last_response.get("elapsed_seconds", 0.0)),
            target_seconds,
            threshold_crossed,
            closing=closing,
        )

        if last_send_started is not None:
            remaining_sleep = args.cadence_seconds - (time.monotonic() - last_send_started)
            if remaining_sleep > 0:
                time.sleep(remaining_sleep)

        request_started = time.monotonic()
        response = send_chunk(args.api_base, api_key, session_id, candidate)
        last_send_started = request_started

        entry = {
            "candidate": candidate,
            "response": response,
            "sent_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        chunk_log.append(entry)
        transcript.append(candidate)
        last_response = response

        eprint(
            "elapsed={elapsed:.1f}s words={words} is_anky={is_anky} chunk={chunk}".format(
                elapsed=float(response.get("elapsed_seconds", 0.0)),
                words=int(response.get("words_total", 0)),
                is_anky=str(bool(response.get("is_anky"))).lower(),
                chunk=candidate,
            )
        )

        if not response.get("ok", False):
            run_log["failure"] = {"phase": "chunk", "response": response}
            break

        if closing:
            break

        threshold_crossed = bool(response.get("is_anky"))

    eprint(f"allowing {args.silence_seconds:.1f}s of silence to close the session")
    time.sleep(args.silence_seconds)

    result = poll_until_finalized(
        args.api_base,
        api_key,
        session_id,
        args.result_timeout_seconds,
        args.result_poll_seconds,
    )
    run_log["result"] = result

    anky_payload: dict[str, Any] | None = None
    events_payload: dict[str, Any] | None = None

    if result.get("anky_id"):
        anky_payload = poll_anky_complete(
            args.api_base,
            str(result["anky_id"]),
            args.anky_timeout_seconds,
            args.anky_poll_seconds,
        )
        run_log["anky"] = anky_payload
    else:
        events_payload = get_events(args.api_base, api_key, session_id)
        run_log["events"] = events_payload

    state["last_run"] = {
        "session_id": session_id,
        "completed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "result": result,
        "anky_id": result.get("anky_id"),
        "anky_status": anky_payload.get("status") if anky_payload else None,
    }
    save_json_file(state_file, state)
    save_json_file(run_log_path, run_log)

    output = {
        "session_id": session_id,
        "result": result,
        "anky": anky_payload,
        "log_file": str(run_log_path),
        "state_file": str(state_file),
        "event_count": events_payload.get("event_count") if events_payload else None,
    }
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0 if result.get("anky_id") else 1


def parse_api_key(args: argparse.Namespace, state_file: Path | None) -> str:
    if args.api_key:
        return args.api_key
    if state_file:
        state = load_json_file(state_file)
        if state.get("anky_api_key"):
            return str(state["anky_api_key"])
    fail("missing Anky API key: pass --api-key or provide a state file with anky_api_key")
    return ""


def parse_session_id(args: argparse.Namespace, state_file: Path | None) -> str:
    if args.session_id:
        return args.session_id
    if state_file:
        state = load_json_file(state_file)
        last_run = state.get("last_run") or {}
        if last_run.get("session_id"):
            return str(last_run["session_id"])
    fail("missing session_id: pass --session-id or use a state file with last_run.session_id")
    return ""


def command_register(args: argparse.Namespace) -> int:
    state_file = resolve_path(args.state_file, default_state_path(args.agent_name))
    assert state_file is not None
    registration = register_agent(args.api_base, args.agent_name, args.description, args.model)
    payload = {
        "agent_name": args.agent_name,
        "agent_id": registration["agent_id"],
        "anky_api_key": registration["api_key"],
        "description": args.description,
        "model": args.model,
    }
    save_json_file(state_file, payload)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def command_start(args: argparse.Namespace) -> int:
    state_file = resolve_path(args.state_file, None)
    api_key = parse_api_key(args, state_file)
    payload = start_session(args.api_base, api_key, args.prompt)
    if state_file:
        state = load_json_file(state_file)
        state["last_run"] = {
            "session_id": payload["session_id"],
            "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        save_json_file(state_file, state)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def command_chunk(args: argparse.Namespace) -> int:
    state_file = resolve_path(args.state_file, None)
    api_key = parse_api_key(args, state_file)
    session_id = parse_session_id(args, state_file)
    payload = send_chunk(args.api_base, api_key, session_id, args.text)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def command_result(args: argparse.Namespace) -> int:
    state_file = resolve_path(args.state_file, None)
    api_key = parse_api_key(args, state_file)
    session_id = parse_session_id(args, state_file)
    payload = get_result(args.api_base, api_key, session_id)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def command_events(args: argparse.Namespace) -> int:
    state_file = resolve_path(args.state_file, None)
    api_key = parse_api_key(args, state_file)
    session_id = parse_session_id(args, state_file)
    payload = get_events(args.api_base, api_key, session_id)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Deterministic supervisor for the hosted Anky agent workflow.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run_parser = subparsers.add_parser("run", help="Run a full supervised Anky session using an OpenAI-compatible model endpoint.")
    run_parser.add_argument("--agent-name", required=True, help="Stable Anky agent name.")
    run_parser.add_argument("--description", help="Optional registration description.")
    run_parser.add_argument("--prompt", help="Optional session intention.")
    run_parser.add_argument("--api-base", default=DEFAULT_API_BASE, help="Anky API base URL.")
    run_parser.add_argument("--state-file", help="Stable state file for the agent identity.")
    run_parser.add_argument("--log-file", help="Where to write the run log JSON.")
    run_parser.add_argument("--llm-base-url", help="OpenAI-compatible base URL. Defaults to OPENAI_BASE_URL.")
    run_parser.add_argument("--llm-api-key", help="OpenAI-compatible API key. Defaults to OPENAI_API_KEY.")
    run_parser.add_argument("--llm-model", help="OpenAI-compatible model name. Defaults to OPENAI_MODEL.")
    run_parser.add_argument("--model-metadata", help="Optional model string stored with Anky registration.")
    run_parser.add_argument("--cadence-seconds", type=float, default=DEFAULT_CADENCE_SECONDS, help="Target interval between chunk sends.")
    run_parser.add_argument("--silence-seconds", type=float, default=DEFAULT_SILENCE_SECONDS, help="Silence after the final chunk so the server can finalize the session.")
    run_parser.add_argument("--result-poll-seconds", type=float, default=DEFAULT_RESULT_POLL_SECONDS, help="Polling interval for session result recovery.")
    run_parser.add_argument("--result-timeout-seconds", type=float, default=DEFAULT_RESULT_TIMEOUT_SECONDS, help="How long to wait for the session result to finalize.")
    run_parser.add_argument("--anky-poll-seconds", type=float, default=DEFAULT_ANKY_POLL_SECONDS, help="Polling interval for the completed Anky record.")
    run_parser.add_argument("--anky-timeout-seconds", type=float, default=DEFAULT_ANKY_TIMEOUT_SECONDS, help="How long to wait for the completed Anky record.")
    run_parser.set_defaults(handler=run_supervisor)

    register_parser = subparsers.add_parser("register", help="Register an agent and save its Anky API key locally.")
    register_parser.add_argument("--agent-name", required=True)
    register_parser.add_argument("--description")
    register_parser.add_argument("--model")
    register_parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    register_parser.add_argument("--state-file", help="State file to write.")
    register_parser.set_defaults(handler=command_register)

    start_parser = subparsers.add_parser("start", help="Start a session using an existing Anky API key.")
    start_parser.add_argument("--api-key")
    start_parser.add_argument("--state-file", help="State file containing anky_api_key.")
    start_parser.add_argument("--prompt")
    start_parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    start_parser.set_defaults(handler=command_start)

    chunk_parser = subparsers.add_parser("chunk", help="Send one chunk to an existing session.")
    chunk_parser.add_argument("--api-key")
    chunk_parser.add_argument("--state-file", help="State file containing anky_api_key and last session_id.")
    chunk_parser.add_argument("--session-id")
    chunk_parser.add_argument("--text", required=True)
    chunk_parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    chunk_parser.set_defaults(handler=command_chunk)

    result_parser = subparsers.add_parser("result", help="Recover the post-session result for a session.")
    result_parser.add_argument("--api-key")
    result_parser.add_argument("--state-file", help="State file containing anky_api_key and last session_id.")
    result_parser.add_argument("--session-id")
    result_parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    result_parser.set_defaults(handler=command_result)

    events_parser = subparsers.add_parser("events", help="Fetch the authenticated session event replay.")
    events_parser.add_argument("--api-key")
    events_parser.add_argument("--state-file", help="State file containing anky_api_key and last session_id.")
    events_parser.add_argument("--session-id")
    events_parser.add_argument("--api-base", default=DEFAULT_API_BASE)
    events_parser.set_defaults(handler=command_events)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return int(args.handler(args))
    except RuntimeError as exc:
        fail(str(exc))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
