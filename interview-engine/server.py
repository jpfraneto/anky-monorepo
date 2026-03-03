"""Anky Interview Engine — WebSocket server + interview orchestrator.

Accepts browser audio, runs STT → LLM → TTS → RTMP pipeline.
"""

#!/usr/bin/env python3
import os, sys

# Pin Voxtral STT to physical GPU 1 (Ollama uses GPU 0)
# When running via systemd, CUDA_VISIBLE_DEVICES is set in the service file.
# This fallback is for manual runs.
if "CUDA_VISIBLE_DEVICES" not in os.environ:
    os.environ["CUDA_VISIBLE_DEVICES"] = "1"

# Ensure we're running in the venv
if not sys.prefix.endswith(".venv"):
    print("ERROR: Not running in the venv! Run:")
    print("  source .venv/bin/activate && python server.py")
    sys.exit(1)

import asyncio
import base64
import json
import logging
import signal
import struct
import sys
import time
from enum import Enum

import numpy as np

from dotenv import load_dotenv
load_dotenv()

import websockets

from brain import AnkyBrain
from audio import transcribe, synthesize, wav_duration_seconds, pcm_to_wav, _get_voxtral, _get_piper
from compositor import Compositor
import memory

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
log = logging.getLogger("server")

HOST = "0.0.0.0"
PORT = 8890

# Time limits (seconds)
ANON_TIME_LIMIT = 300     # 5 minutes for anonymous users
AUTH_TIME_LIMIT = 1800    # 30 minutes for authenticated users
TIME_WARNING_BEFORE = 60  # warn 1 minute before cutoff


class State(Enum):
    WAITING = "waiting"        # no guest connected
    CONSENTED = "consented"    # guest connected, consent given, waiting for mic
    SPEAKING = "speaking"      # Anky is speaking (TTS playing)
    LISTENING = "listening"    # guest is speaking, collecting audio
    THINKING = "thinking"      # processing: STT → LLM → TTS


class InterviewSession:
    """Manages a single interview session with one guest."""

    def __init__(self):
        self.brain = AnkyBrain()
        self.compositor = Compositor()
        self.state = State.WAITING
        self.ws = None
        self.audio_chunks: list[bytes] = []
        self.silence_start: float | None = None
        self._has_speech: bool = False  # did we detect any speech in current turn?
        self._silence_task: asyncio.Task | None = None
        self._time_limit_task: asyncio.Task | None = None
        self._time_warning_sent: bool = False
        self._active = False
        self.guest_name = "guest"
        self.user_id = "anonymous"
        self.interview_id: str | None = None
        self.start_time: float | None = None
        self.max_duration: int = ANON_TIME_LIMIT

        # Silence detection config
        self.SILENCE_THRESHOLD = 200    # RMS below this = silence
        self.SILENCE_DURATION = 1.5     # seconds of silence before auto-send
        self.MIN_SPEECH_CHUNKS = 3      # minimum chunks with speech before auto-send

    async def handle_connection(self, ws):
        """Handle a new WebSocket connection from a guest."""
        if self._active:
            await ws.send(json.dumps({"type": "error", "message": "Interview already in progress"}))
            await ws.close()
            return

        self._active = True
        self.ws = ws
        self.state = State.WAITING
        self.brain.reset()
        self.audio_chunks = []
        self.guest_name = "guest"
        self.user_id = "anonymous"
        self.interview_id = None
        self.start_time = None
        self.max_duration = ANON_TIME_LIMIT
        self._time_warning_sent = False
        self.compositor.has_guest = True
        self.compositor.stream_status = "idle"

        log.info("Guest connected from %s", ws.remote_address)
        await self._send_state()

        try:
            async for message in ws:
                await self._handle_message(message)
        except websockets.ConnectionClosed:
            log.info("Guest disconnected")
        finally:
            # Cancel time limit task
            if self._time_limit_task and not self._time_limit_task.done():
                self._time_limit_task.cancel()
                self._time_limit_task = None

            await self._end_interview()
            self._active = False
            self.state = State.WAITING
            self.compositor.active_speaker = "none"
            self.compositor.guest_name = "guest"
            self.compositor.has_guest = False
            self.compositor.stream_status = "idle"
            self.ws = None

    async def _handle_message(self, message):
        """Route incoming WebSocket messages."""
        if isinstance(message, bytes):
            # Binary = raw PCM int16 16000Hz mono from ScriptProcessor
            if self.state == State.LISTENING:
                self.audio_chunks.append(message)
                # Pipe directly to RTMP stream in real-time
                self.compositor.queue_raw_pcm(message)

                # Silence detection: check RMS energy of this chunk
                try:
                    samples = np.frombuffer(message, dtype=np.int16).astype(np.float32)
                    rms = np.sqrt(np.mean(samples ** 2)) if len(samples) > 0 else 0
                except Exception:
                    rms = 0

                if rms > self.SILENCE_THRESHOLD:
                    # Speech detected
                    self._has_speech = True
                    self.silence_start = None
                    # Cancel any pending silence timer
                    if self._silence_task and not self._silence_task.done():
                        self._silence_task.cancel()
                        self._silence_task = None
                else:
                    # Silence — start timer if we've had speech
                    if self._has_speech and self.silence_start is None:
                        self.silence_start = time.monotonic()
                        self._silence_task = asyncio.create_task(self._silence_timeout())
            return

        # JSON text messages
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            return

        msg_type = data.get("type")

        if msg_type == "identity":
            # Authenticated user from webapp
            uid = data.get("user_id", "")
            uname = data.get("username", "")
            if uid:
                self.user_id = uid
                self.max_duration = AUTH_TIME_LIMIT
                log.info("Identified user: %s (%s) — time limit: %ds", uname, uid, self.max_duration)
            if uname:
                self.guest_name = uname
                self.compositor.guest_name = uname

        elif msg_type == "consent":
            log.info("Guest gave recording consent")
            self.state = State.CONSENTED
            await self._send_state()
            # Send time limit info to frontend
            await self._send({
                "type": "time_config",
                "max_duration": self.max_duration,
                "is_authenticated": self.user_id != "anonymous",
            })

        elif msg_type == "username":
            name = data.get("name", "").strip()[:30]
            if name:
                self.guest_name = name
                self.compositor.guest_name = name
                log.info("Guest username: %s", name)

        elif msg_type == "pfp":
            pfp_b64 = data.get("data", "")
            if pfp_b64:
                try:
                    pfp_bytes = base64.b64decode(pfp_b64)
                    self.compositor.set_guest_image(pfp_bytes)
                    log.info("Guest PFP received (%d bytes)", len(pfp_bytes))
                    await self._send({"type": "pfp_ack"})
                except Exception as e:
                    log.warning("Bad PFP data: %s", e)

        elif msg_type == "start":
            await self._start_interview()

        elif msg_type == "audio_end":
            if self.state == State.LISTENING:
                await self._process_guest_audio()

        elif msg_type == "silence":
            if self.state == State.LISTENING and self.audio_chunks:
                await self._process_guest_audio()

        elif msg_type == "leave":
            log.info("Guest chose to leave the interview")
            if self.ws:
                await self.ws.close()

    async def _start_interview(self):
        """Anky introduces and asks the opening question."""
        self.state = State.SPEAKING
        self.compositor.active_speaker = "anky"
        self.compositor.stream_status = "thinking"
        self.start_time = time.monotonic()
        await self._send_state()

        # Start persistent interview record
        self.interview_id = await asyncio.to_thread(
            memory.start_interview, self.user_id, self.guest_name
        )

        # Start time limit enforcement
        self._time_limit_task = asyncio.create_task(self._time_limit_enforcement())

        # Load memory for returning guests
        past = await asyncio.to_thread(memory.get_past_conversations, self.user_id)

        # Fetch rich user context from main Rust app
        user_context = None
        if self.user_id != "anonymous":
            user_context = await asyncio.to_thread(memory.get_user_context, self.user_id)

        self.brain.set_memory_context(self.guest_name, past, user_context)

        # Generate opening via LLM
        opening = await asyncio.to_thread(self.brain.get_opening)
        log.info("Anky opening: %s", opening)

        # Save to memory
        if self.interview_id:
            await asyncio.to_thread(memory.save_message, self.interview_id, "anky", opening)

        self.compositor.add_transcript("anky", opening)
        self.compositor.stream_status = "speaking"

        # Generate TTS
        tts_wav = await asyncio.to_thread(synthesize, opening)
        duration = wav_duration_seconds(tts_wav)
        log.info("TTS generated: %d bytes, %.1fs", len(tts_wav), duration)

        # Feed TTS audio to the RTMP stream
        if tts_wav:
            self.compositor.queue_tts_audio(tts_wav)

        # Send text + audio to browser
        await self._send({
            "type": "anky_says",
            "text": opening,
            "audio": base64.b64encode(tts_wav).decode() if tts_wav else None,
        })

        # Wait for TTS to finish playing
        if duration > 0:
            await asyncio.sleep(duration + 0.5)

        # Switch to listening
        self.state = State.LISTENING
        self.compositor.active_speaker = "guest"
        self.compositor.stream_status = "listening"
        self.audio_chunks = []
        self._has_speech = False
        self.silence_start = None
        await self._send_state()

    async def _time_limit_enforcement(self):
        """Enforce interview time limit — warn before cutoff, then end."""
        try:
            warning_time = self.max_duration - TIME_WARNING_BEFORE
            if warning_time > 0:
                await asyncio.sleep(warning_time)
                # Send warning
                remaining = TIME_WARNING_BEFORE
                log.info("Time warning: %d seconds remaining", remaining)
                await self._send({
                    "type": "time_warning",
                    "remaining": remaining,
                })
                self._time_warning_sent = True
                await asyncio.sleep(TIME_WARNING_BEFORE)
            else:
                await asyncio.sleep(self.max_duration)

            # Time's up
            log.info("Interview time limit reached (%ds)", self.max_duration)
            is_anon = self.user_id == "anonymous"
            msg = "your 5 minutes are up. sign in to anky.app for longer interviews." if is_anon else "that's 30 minutes. thanks for being here."
            await self._send({
                "type": "time_up",
                "message": msg,
            })

            # Give a moment for the message to arrive, then close
            await asyncio.sleep(2)
            if self.ws:
                await self.ws.close()

        except asyncio.CancelledError:
            pass  # Interview ended before time limit

    async def _silence_timeout(self):
        """Wait for silence duration, then auto-trigger processing."""
        try:
            await asyncio.sleep(self.SILENCE_DURATION)
            # Still in listening state and still silent?
            if self.state == State.LISTENING and self._has_speech and self.audio_chunks:
                log.info("Auto-sending after %.1fs of silence", self.SILENCE_DURATION)
                await self._process_guest_audio()
        except asyncio.CancelledError:
            pass  # Speech resumed, timer cancelled

    async def _recover_to_listening(self, reason: str):
        """Recover from a failed processing step — go back to listening."""
        log.warning("Recovering to listening state: %s", reason)
        await self._send({"type": "error", "message": reason})
        self.state = State.LISTENING
        self.compositor.active_speaker = "guest"
        self.compositor.stream_status = "listening"
        self.audio_chunks = []
        self._has_speech = False
        self.silence_start = None
        await self._send_state()

    async def _process_guest_audio(self):
        """Process collected audio: STT → LLM → TTS."""
        if not self.audio_chunks:
            return

        # Cancel any pending silence timer
        if self._silence_task and not self._silence_task.done():
            self._silence_task.cancel()
            self._silence_task = None

        self.state = State.THINKING
        self.compositor.active_speaker = "none"
        self.compositor.stream_status = "transcribing"
        await self._send_state()
        await self._send({"type": "processing", "stage": "transcribing"})

        # Combine raw PCM chunks and wrap in WAV header for Voxtral
        pcm_blob = b"".join(self.audio_chunks)
        self.audio_chunks = []
        self._has_speech = False
        self.silence_start = None
        log.info("Processing %d bytes of guest PCM audio", len(pcm_blob))

        wav_blob = pcm_to_wav(pcm_blob, sample_rate=16000)

        # STT with async timeout (safety net on top of audio.py's internal timeout)
        try:
            transcript = await asyncio.wait_for(
                asyncio.to_thread(transcribe, wav_blob, True),
                timeout=45,
            )
        except asyncio.TimeoutError:
            log.error("STT timed out at async level (45s) — recovering to listening")
            await self._recover_to_listening("transcription timed out — try again")
            return
        except Exception as e:
            log.error("STT unexpected error: %s", e)
            await self._recover_to_listening("transcription failed — try again")
            return

        if not transcript.strip():
            log.info("No speech detected, resuming listening")
            self.state = State.LISTENING
            self.compositor.active_speaker = "guest"
            self.compositor.stream_status = "listening"
            await self._send_state()
            return

        log.info("Guest said: %s", transcript)
        self.compositor.add_transcript("guest", transcript)
        await self._send({"type": "guest_transcript", "text": transcript})

        # Save guest message to memory
        if self.interview_id:
            await asyncio.to_thread(memory.save_message, self.interview_id, "guest", transcript)

        # LLM response
        await self._send({"type": "processing", "stage": "thinking"})
        self.compositor.stream_status = "thinking"
        self.state = State.SPEAKING
        self.compositor.active_speaker = "anky"
        await self._send_state()

        try:
            reply = await asyncio.wait_for(
                asyncio.to_thread(self.brain.respond, transcript),
                timeout=120,
            )
        except asyncio.TimeoutError:
            log.error("LLM response timed out (120s) — recovering to listening")
            await self._recover_to_listening("anky's brain timed out — try again")
            return
        except Exception as e:
            log.error("LLM unexpected error: %s", e)
            await self._recover_to_listening("something went wrong — try again")
            return

        log.info("Anky reply: %s", reply)
        self.compositor.add_transcript("anky", reply)

        # Save Anky's reply to memory
        if self.interview_id:
            await asyncio.to_thread(memory.save_message, self.interview_id, "anky", reply)

        # TTS
        self.compositor.stream_status = "speaking"
        try:
            tts_wav = await asyncio.wait_for(
                asyncio.to_thread(synthesize, reply),
                timeout=30,
            )
        except asyncio.TimeoutError:
            log.error("TTS timed out (30s) — sending text only")
            tts_wav = b""
        except Exception as e:
            log.error("TTS error: %s", e)
            tts_wav = b""

        duration = wav_duration_seconds(tts_wav)
        log.info("TTS generated: %d bytes, %.1fs", len(tts_wav), duration)

        # Feed TTS audio to the RTMP stream
        if tts_wav:
            self.compositor.queue_tts_audio(tts_wav)

        await self._send({
            "type": "anky_says",
            "text": reply,
            "audio": base64.b64encode(tts_wav).decode() if tts_wav else None,
        })

        # Wait for TTS playback
        if duration > 0:
            await asyncio.sleep(duration + 0.5)

        # Back to listening
        self.state = State.LISTENING
        self.compositor.active_speaker = "guest"
        self.compositor.stream_status = "listening"
        self.audio_chunks = []
        self._has_speech = False
        self.silence_start = None
        await self._send_state()

    async def _end_interview(self):
        """Generate summary and finalize the interview record."""
        if not self.interview_id:
            return

        interview_id = self.interview_id
        duration_seconds = None
        if self.start_time:
            duration_seconds = time.monotonic() - self.start_time

        log.info("Ending interview %s — generating summary...", interview_id)

        try:
            # Mark interview as ended
            await asyncio.to_thread(memory.end_interview, interview_id)

            # Get transcript and generate summary
            transcript = await asyncio.to_thread(memory.get_interview_transcript, interview_id)
            if transcript:
                summary = await asyncio.to_thread(self.brain.generate_summary, transcript)
                if summary:
                    await asyncio.to_thread(memory.save_interview_summary, interview_id, summary)
                    log.info("Interview %s summary: %s", interview_id, summary[:100])
        except Exception as e:
            log.error("Failed to finalize interview %s: %s", interview_id, e)

        self.interview_id = None

    async def _send_state(self):
        """Send current interview state to browser."""
        data = {"type": "state", "state": self.state.value}
        # Include elapsed time if interview is active
        if self.start_time:
            elapsed = time.monotonic() - self.start_time
            data["elapsed"] = round(elapsed, 1)
        await self._send(data)

    async def _send(self, data: dict):
        """Send JSON message to the connected guest."""
        if self.ws:
            try:
                await self.ws.send(json.dumps(data))
            except websockets.ConnectionClosed:
                pass


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------

def _preload_models():
    """Preload Voxtral and Piper so first request is instant."""
    _get_voxtral()
    _get_piper()


session = InterviewSession()


async def handler(ws, path: str = ""):
    await session.handle_connection(ws)


async def main():
    log.info("Starting Anky Interview Engine on %s:%d", HOST, PORT)

    # Preload models so first request is fast
    log.info("Preloading Voxtral STT model...")
    await asyncio.to_thread(_preload_models)
    log.info("All models ready.")

    # Start the compositor / RTMP stream
    session.compositor.start_stream()

    # Start WebSocket server
    async with websockets.serve(handler, HOST, PORT, max_size=10 * 1024 * 1024):
        log.info("WebSocket server ready. Waiting for guest...")
        stop = asyncio.get_event_loop().create_future()

        def on_signal():
            stop.set_result(None)

        loop = asyncio.get_event_loop()
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, on_signal)

        await stop

    session.compositor.stop_stream()
    log.info("Shutdown complete.")


if __name__ == "__main__":
    asyncio.run(main())
