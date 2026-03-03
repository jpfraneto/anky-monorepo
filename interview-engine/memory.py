"""Persistent memory for Anky interviews — delegates to main Rust app API with local SQLite fallback."""

import os
import sqlite3
import logging
import uuid
from datetime import datetime, timezone

import requests

log = logging.getLogger(__name__)

RUST_API = os.environ.get("ANKY_API_URL", "http://127.0.0.1:8889")
API_TIMEOUT = 5  # seconds

DB_PATH = os.path.join(os.path.dirname(__file__), "memory.db")


# --- Local SQLite fallback ---

def _connect() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    return conn


def init_db():
    """Create local fallback tables if they don't exist."""
    conn = _connect()
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS interviews (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT 'anonymous',
            guest_name TEXT NOT NULL DEFAULT 'guest',
            started_at TEXT NOT NULL,
            ended_at TEXT,
            summary TEXT
        );

        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            interview_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (interview_id) REFERENCES interviews(id)
        );

        CREATE INDEX IF NOT EXISTS idx_interviews_user_id ON interviews(user_id);
        CREATE INDEX IF NOT EXISTS idx_messages_interview_id ON messages(interview_id);
    """)
    conn.commit()
    conn.close()
    log.info("Local fallback database initialized at %s", DB_PATH)


def _api_ok() -> bool:
    """Quick check if the Rust API is reachable."""
    try:
        resp = requests.get(f"{RUST_API}/health", timeout=2)
        return resp.status_code == 200
    except Exception:
        return False


# --- Public API ---

def start_interview(user_id: str = "anonymous", guest_name: str = "guest") -> str:
    """Start a new interview, return the interview ID (text UUID)."""
    interview_id = str(uuid.uuid4())
    is_anonymous = (not user_id) or user_id == "anonymous"

    # Try Rust API first
    try:
        resp = requests.post(
            f"{RUST_API}/api/interview/start",
            json={
                "id": interview_id,
                "user_id": None if is_anonymous else user_id,
                "guest_name": guest_name,
                "is_anonymous": is_anonymous,
            },
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        log.info("Started interview %s via Rust API (user=%s)", interview_id, user_id)
        return interview_id
    except Exception as e:
        log.warning("Rust API unavailable for start_interview, using local DB: %s", e)

    # Local fallback
    conn = _connect()
    now = datetime.now(timezone.utc).isoformat()
    conn.execute(
        "INSERT INTO interviews (id, user_id, guest_name, started_at) VALUES (?, ?, ?, ?)",
        (interview_id, user_id, guest_name, now),
    )
    conn.commit()
    conn.close()
    log.info("Started interview %s locally (user=%s name=%s)", interview_id, user_id, guest_name)
    return interview_id


def save_message(interview_id: str, role: str, content: str):
    """Save a single message turn (role='anky' or 'guest')."""
    # Try Rust API first
    try:
        resp = requests.post(
            f"{RUST_API}/api/interview/message",
            json={
                "interview_id": interview_id,
                "role": role,
                "content": content,
            },
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        return
    except Exception as e:
        log.warning("Rust API unavailable for save_message, using local DB: %s", e)

    # Local fallback
    conn = _connect()
    now = datetime.now(timezone.utc).isoformat()
    conn.execute(
        "INSERT INTO messages (interview_id, role, content, created_at) VALUES (?, ?, ?, ?)",
        (interview_id, role, content, now),
    )
    conn.commit()
    conn.close()


def end_interview(interview_id: str):
    """Mark interview as ended."""
    try:
        resp = requests.post(
            f"{RUST_API}/api/interview/end",
            json={"interview_id": interview_id},
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        return
    except Exception as e:
        log.warning("Rust API unavailable for end_interview, using local DB: %s", e)

    conn = _connect()
    now = datetime.now(timezone.utc).isoformat()
    conn.execute("UPDATE interviews SET ended_at = ? WHERE id = ?", (now, interview_id))
    conn.commit()
    conn.close()


def save_interview_summary(interview_id: str, summary: str):
    """Store the LLM-generated summary for a completed interview."""
    try:
        resp = requests.post(
            f"{RUST_API}/api/interview/end",
            json={"interview_id": interview_id, "summary": summary},
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        log.info("Saved summary for interview %s via Rust API", interview_id)
        return
    except Exception as e:
        log.warning("Rust API unavailable for save_interview_summary, using local DB: %s", e)

    conn = _connect()
    conn.execute("UPDATE interviews SET summary = ? WHERE id = ?", (summary, interview_id))
    conn.commit()
    conn.close()
    log.info("Saved summary for interview %s locally", interview_id)


def get_past_conversations(user_id: str, limit: int = 5) -> list[dict]:
    """Get the last N interview summaries for a user."""
    if not user_id or user_id == "anonymous":
        return []

    # Try Rust API first
    try:
        resp = requests.get(
            f"{RUST_API}/api/interview/history/{user_id}",
            params={"limit": limit},
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        data = resp.json()
        if isinstance(data, list):
            log.info("Got %d past conversations from Rust API", len(data))
            return data
    except Exception as e:
        log.warning("Rust API unavailable for get_past_conversations, using local DB: %s", e)

    # Local fallback
    conn = _connect()
    rows = conn.execute(
        """SELECT id, guest_name, started_at, summary
           FROM interviews
           WHERE user_id = ? AND summary IS NOT NULL
           ORDER BY started_at DESC
           LIMIT ?""",
        (user_id, limit),
    ).fetchall()

    conversations = []
    for row in rows:
        msgs = conn.execute(
            """SELECT role, content FROM messages
               WHERE interview_id = ?
               ORDER BY created_at
               LIMIT 6""",
            (row["id"],),
        ).fetchall()

        conversations.append({
            "id": row["id"],
            "guest_name": row["guest_name"],
            "started_at": row["started_at"],
            "summary": row["summary"],
            "recent_messages": [{"role": m["role"], "content": m["content"]} for m in msgs],
        })

    conn.close()
    return conversations


def get_user_context(user_id: str) -> dict | None:
    """Fetch rich user context from the main Rust app (profile, writings, past interviews)."""
    if not user_id or user_id == "anonymous":
        return None

    try:
        resp = requests.get(
            f"{RUST_API}/api/interview/user-context/{user_id}",
            timeout=API_TIMEOUT,
        )
        resp.raise_for_status()
        data = resp.json()
        if data:
            log.info("Got user context for %s from Rust API", user_id)
            return data
    except Exception as e:
        log.warning("Failed to get user context from Rust API: %s", e)

    return None


def get_interview_transcript(interview_id: str) -> list[dict]:
    """Get full transcript for an interview (local fallback only)."""
    conn = _connect()
    rows = conn.execute(
        "SELECT role, content FROM messages WHERE interview_id = ? ORDER BY created_at",
        (interview_id,),
    ).fetchall()
    conn.close()
    return [{"role": r["role"], "content": r["content"]} for r in rows]


# Initialize local fallback on import
init_db()
