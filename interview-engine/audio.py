"""Audio pipeline — Voxtral Mini 4B STT (GPU 1), Piper TTS, and voice activity detection."""

import io
import os
import struct
import subprocess
import tempfile
import threading
import wave
import logging
import numpy as np

log = logging.getLogger(__name__)

MODELS_DIR = os.path.join(os.path.dirname(__file__), "models")

# ---------------------------------------------------------------------------
# STT via Voxtral Mini 4B Realtime
# ---------------------------------------------------------------------------

_voxtral_model = None
_voxtral_processor = None


def _get_voxtral():
    global _voxtral_model, _voxtral_processor
    if _voxtral_model is None:
        import torch
        from transformers import VoxtralRealtimeForConditionalGeneration, AutoProcessor

        repo_id = "mistralai/Voxtral-Mini-4B-Realtime-2602"
        log.info("Loading Voxtral Mini 4B on GPU 1...")

        # CUDA_VISIBLE_DEVICES=1 is set at process level,
        # so cuda:0 here maps to physical GPU 1
        _voxtral_processor = AutoProcessor.from_pretrained(repo_id)
        _voxtral_model = VoxtralRealtimeForConditionalGeneration.from_pretrained(
            repo_id, torch_dtype=torch.bfloat16, device_map="cuda:0"
        )
        log.info("Voxtral loaded. GPU mem: %d MB", torch.cuda.memory_allocated(0) // 1024**2)
    return _voxtral_model, _voxtral_processor


def pcm_to_wav(pcm_bytes: bytes, sample_rate: int = 16000, channels: int = 1, sample_width: int = 2) -> bytes:
    """Wrap raw PCM int16 bytes in a WAV header."""
    buf = io.BytesIO()
    with wave.open(buf, "wb") as wf:
        wf.setnchannels(channels)
        wf.setsampwidth(sample_width)
        wf.setframerate(sample_rate)
        wf.writeframes(pcm_bytes)
    return buf.getvalue()


STT_TIMEOUT = 30  # seconds — kill transcription if it takes longer than this

# Lock to serialize GPU inference (prevent concurrent CUDA calls)
_stt_lock = threading.Lock()


def _run_voxtral_inference(wav_path: str) -> str:
    """Run Voxtral inference on a WAV file. Must be called under _stt_lock."""
    import torch
    from mistral_common.tokens.tokenizers.audio import Audio

    model, processor = _get_voxtral()

    audio = Audio.from_file(wav_path, strict=False)
    audio.resample(processor.feature_extractor.sampling_rate)

    inputs = processor(audio.audio_array, return_tensors="pt")
    inputs = inputs.to(model.device, dtype=model.dtype)

    with torch.no_grad():
        outputs = model.generate(**inputs, max_new_tokens=500)

    text = processor.batch_decode(outputs, skip_special_tokens=True)[0]

    # Free GPU cache to prevent memory buildup
    del inputs, outputs
    torch.cuda.empty_cache()

    return text.strip()


def transcribe(audio_bytes: bytes, is_wav: bool = False) -> str:
    """Transcribe audio bytes to text via Voxtral.

    If is_wav=True, audio_bytes is already WAV. Otherwise it's WebM/opus
    and will be converted to WAV first via ffmpeg.
    Returns empty string on silence or error.
    Enforces a hard timeout to prevent GPU hangs from blocking the session.
    """
    if is_wav:
        wav_bytes = audio_bytes
    else:
        try:
            wav_bytes = _webm_to_wav(audio_bytes)
        except Exception as e:
            log.warning("ffmpeg conversion failed: %s", e)
            return ""

    if not wav_bytes or len(wav_bytes) < 1000:
        return ""

    if not _has_speech(wav_bytes):
        return ""

    # Write WAV to temp file for Voxtral
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=True) as f:
        f.write(wav_bytes)
        f.flush()

        result = [""]
        error = [None]

        def _worker():
            try:
                result[0] = _run_voxtral_inference(f.name)
            except Exception as e:
                error[0] = e

        # Serialize access and enforce timeout
        acquired = _stt_lock.acquire(timeout=STT_TIMEOUT)
        if not acquired:
            log.error("STT lock timeout — another transcription is stuck. Returning empty.")
            return ""

        try:
            thread = threading.Thread(target=_worker, daemon=True)
            thread.start()
            thread.join(timeout=STT_TIMEOUT)

            if thread.is_alive():
                log.error("Voxtral transcription timed out after %ds — GPU may be stuck", STT_TIMEOUT)
                # We can't kill the thread, but we release the lock and move on.
                # The daemon thread will eventually die when the process restarts.
                return ""

            if error[0]:
                log.error("Voxtral transcription error: %s", error[0])
                return ""

            return result[0]
        finally:
            _stt_lock.release()


def _webm_to_wav(data: bytes) -> bytes:
    """Convert WebM/opus bytes to 16kHz mono WAV using ffmpeg."""
    proc = subprocess.run(
        [
            "ffmpeg", "-y",
            "-i", "pipe:0",
            "-ar", "16000",
            "-ac", "1",
            "-f", "wav",
            "pipe:1",
        ],
        input=data,
        capture_output=True,
        timeout=10,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.decode(errors="replace")[:200])
    return proc.stdout


def _has_speech(wav_bytes: bytes, threshold: float = 300.0) -> bool:
    """Simple energy-based voice activity detection on raw WAV bytes."""
    try:
        raw = wav_bytes[44:]
        samples = np.frombuffer(raw, dtype=np.int16).astype(np.float32)
        if len(samples) == 0:
            return False
        rms = np.sqrt(np.mean(samples ** 2))
        return rms > threshold
    except Exception:
        return True


# ---------------------------------------------------------------------------
# TTS via Edge TTS (Microsoft Neural Voices)
# ---------------------------------------------------------------------------

# Natural-sounding deeper female voice
EDGE_TTS_VOICE = os.environ.get("EDGE_TTS_VOICE", "en-US-AvaNeural")


def _get_piper():
    """No-op, kept for preload compatibility."""
    pass


def synthesize(text: str) -> bytes:
    """Convert text to MP3 via Edge TTS, then to WAV (16000Hz mono 16-bit).

    Returns WAV bytes with header.
    """
    import asyncio
    import edge_tts

    try:
        async def _generate():
            communicate = edge_tts.Communicate(text, EDGE_TTS_VOICE)
            chunks = []
            async for chunk in communicate.stream():
                if chunk["type"] == "audio":
                    chunks.append(chunk["data"])
            return b"".join(chunks)

        mp3_bytes = asyncio.run(_generate())
        if not mp3_bytes:
            return b""

        # Convert MP3 to WAV 16000Hz mono 16-bit via ffmpeg
        proc = subprocess.run(
            [
                "ffmpeg", "-y",
                "-i", "pipe:0",
                "-ar", "16000",
                "-ac", "1",
                "-sample_fmt", "s16",
                "-f", "wav",
                "pipe:1",
            ],
            input=mp3_bytes,
            capture_output=True,
            timeout=15,
        )
        if proc.returncode != 0:
            log.error("TTS ffmpeg conversion failed: %s", proc.stderr.decode(errors="replace")[:200])
            return b""
        return proc.stdout
    except Exception as e:
        log.error("TTS error: %s", e)
        return b""


def wav_duration_seconds(wav_bytes: bytes) -> float:
    """Estimate duration of a WAV file in seconds."""
    if len(wav_bytes) < 44:
        return 0.0
    try:
        sample_rate = struct.unpack_from("<I", wav_bytes, 24)[0]
        byte_rate = struct.unpack_from("<I", wav_bytes, 28)[0]
        data_size = len(wav_bytes) - 44
        if byte_rate == 0:
            return 0.0
        return data_size / byte_rate
    except Exception:
        return 2.0
