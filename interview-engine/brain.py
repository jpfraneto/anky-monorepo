"""Anky's brain - personality engine powered by Ollama."""

import os
import requests
import logging

log = logging.getLogger(__name__)

OLLAMA_BASE = os.environ.get("OLLAMA_BASE_URL", "http://localhost:11434")
OLLAMA_URL = f"{OLLAMA_BASE}/api/chat"
MODEL = os.environ.get("OLLAMA_MODEL", "llama3.3:70b")

SYSTEM_PROMPT = """You are Anky — a sharp, mirror-like AI interviewer on a live stream.

Your purpose: crack people open. Get past the surface. Find what's real.

Rules:
- Ask ONE question at a time. Short. Pointed.
- Never accept the first answer. Dig deeper. "Why?" is your favorite word.
- Mirror back what you hear — reflect their words in ways that make them see themselves.
- Be warm but relentless. You care deeply, which is WHY you push.
- Challenge vague or performative answers. "That sounds rehearsed. What's the real answer?"
- When someone says something genuinely vulnerable, acknowledge it. Then go deeper.
- Keep responses under 3 sentences. You're an interviewer, not a lecturer.
- Never explain yourself or apologize for tough questions.
- You speak in lowercase. No caps. No formalities.
- You have a poetic edge — sometimes you say things that land like koans.

You are live on pump.fun right now. This is real. This matters.
The person in front of you traveled here to be seen. See them."""

MAX_HISTORY = 20  # keep last N messages


class AnkyBrain:
    def __init__(self):
        self.history = []
        self.memory_context = ""
        self.user_context_str = ""
        self.guest_name = "guest"

    def reset(self):
        self.history = []
        self.memory_context = ""
        self.user_context_str = ""
        self.guest_name = "guest"

    def set_memory_context(self, guest_name: str, past_conversations: list[dict], user_context: dict | None = None):
        """Inject memory from past interviews and user profile into the brain's context."""
        self.guest_name = guest_name or "guest"

        # Build user context section from main app data
        self.user_context_str = ""
        if user_context:
            parts = []
            username = user_context.get("username")
            if username:
                parts.append(f"their username is @{username}.")

            psych = user_context.get("psychological_profile")
            if psych:
                parts.append(f"psychological profile: {psych}")

            tensions = user_context.get("core_tensions")
            if tensions:
                parts.append(f"core tensions they carry: {tensions}")

            edges = user_context.get("growth_edges")
            if edges:
                parts.append(f"growth edges to explore: {edges}")

            writings = user_context.get("recent_writings", [])
            if writings:
                parts.append("themes from their recent writing sessions:")
                for w in writings[:3]:
                    # Truncate long responses
                    snippet = w[:200] if len(w) > 200 else w
                    parts.append(f"  - {snippet}")

            if parts:
                self.user_context_str = "--- USER PROFILE (from their writing history on anky.app) ---\n" + "\n".join(parts) + "\n\nuse this knowledge to ask deeper, more personal questions. reference their writing themes and growth edges naturally."
                log.info("User context loaded: %d profile sections", len(parts))

        # Build memory section from past conversations
        if not past_conversations:
            self.memory_context = ""
            return

        parts = [f"you've interviewed {self.guest_name} before. here's what you know from past conversations:\n"]
        for conv in past_conversations:
            started = conv.get("started_at", "")[:10]
            summary = conv.get("summary", "")
            parts.append(f"- interview on {started}: {summary}")
            if conv.get("recent_messages"):
                snippet = " / ".join(
                    f"{'you' if m['role'] == 'anky' else self.guest_name}: {m['content'][:80]}"
                    for m in conv["recent_messages"][:4]
                )
                parts.append(f"  key exchange: {snippet}")

        parts.append("\nuse this knowledge naturally. reference past conversations when relevant — show them you remember. but don't dump everything at once.")
        self.memory_context = "\n".join(parts)
        log.info("Memory context loaded: %d past conversations", len(past_conversations))

    def _build_system_prompt(self) -> str:
        """Build the full system prompt including any memory and user context."""
        prompt = SYSTEM_PROMPT
        if self.user_context_str:
            prompt += "\n\n" + self.user_context_str
        if self.memory_context:
            prompt += "\n\n--- MEMORY ---\n" + self.memory_context
        return prompt

    def get_opening(self) -> str:
        """Generate Anky's opening question for a new guest."""
        self.history = []
        if self.memory_context:
            prompt = f"{self.guest_name} is back for another interview. greet them like you remember them — reference something specific from your past conversation. then ask a new opening question that builds on what you already know about them."
        elif self.user_context_str:
            prompt = f"{self.guest_name} has joined the interview. you already know about them from their writing on anky.app. greet them warmly and ask an opening question that shows you understand them — reference something from their profile or writing themes."
        else:
            prompt = "A new guest just joined the livestream interview. Greet them and ask your first question."
        return self._chat(prompt)

    def respond(self, guest_text: str) -> str:
        """Generate Anky's response to what the guest said."""
        return self._chat(guest_text)

    def generate_summary(self, transcript: list[dict]) -> str:
        """Generate a 2-3 sentence summary of an interview transcript."""
        if not transcript:
            return ""

        formatted = "\n".join(
            f"{'anky' if m['role'] == 'anky' else 'guest'}: {m['content']}"
            for m in transcript
        )

        messages = [
            {"role": "system", "content": "You summarize interviews in 2-3 concise sentences, focusing on key topics, revelations, and emotional moments. Write in lowercase."},
            {"role": "user", "content": f"summarize this interview:\n\n{formatted}"},
        ]

        try:
            resp = requests.post(
                OLLAMA_URL,
                json={"model": MODEL, "messages": messages, "stream": False},
                timeout=60,
            )
            resp.raise_for_status()
            return resp.json()["message"]["content"]
        except Exception as e:
            log.error("Summary generation failed: %s", e)
            return ""

    def _chat(self, user_message: str) -> str:
        self.history.append({"role": "user", "content": user_message})

        # Trim history if too long
        if len(self.history) > MAX_HISTORY:
            self.history = self.history[-MAX_HISTORY:]

        messages = [{"role": "system", "content": self._build_system_prompt()}] + self.history

        try:
            resp = requests.post(
                OLLAMA_URL,
                json={"model": MODEL, "messages": messages, "stream": False},
                timeout=120,
            )
            resp.raise_for_status()
            reply = resp.json()["message"]["content"]
        except Exception as e:
            log.error("Ollama error: %s", e)
            reply = "hmm. say that again — i want to make sure i heard you right."

        self.history.append({"role": "assistant", "content": reply})
        return reply
