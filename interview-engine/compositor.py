"""Frame compositor — renders 1920x1080 horizontal frames and streams via RTMP.

Layout (horizontal):
┌─────────────────────────────────────────────────┐
│  $ANKY                                          │
│  ┌──────────┐                    ┌──────────┐   │
│  │          │   "who are you,   │          │   │
│  │   ANKY   │    really?"        │  GUEST   │   │
│  │ (artwork)│                    │  (pfp)   │   │
│  │          │   conversation     │          │   │
│  └──────────┘   transcript       └──────────┘   │
│                                   anky.app      │
└─────────────────────────────────────────────────┘

Audio: TTS WAV is piped to ffmpeg via a named FIFO for the stream audio track.
"""

import glob
import math
import os
import random
import subprocess
import struct
import tempfile
import threading
import time
import logging

from PIL import Image, ImageDraw, ImageFont

log = logging.getLogger(__name__)

# Layout constants (1920x1080 horizontal)
WIDTH = 1920
HEIGHT = 1080
BG_COLOR = (10, 10, 15)
SQUARE_SIZE = 500

# Anky square: left side
ANKY_X = 80
ANKY_Y = (HEIGHT - SQUARE_SIZE) // 2

# Guest square: right side
GUEST_X = WIDTH - 80 - SQUARE_SIZE
GUEST_Y = (HEIGHT - SQUARE_SIZE) // 2

# Transcript: center column between the two squares
TEXT_X = ANKY_X + SQUARE_SIZE + 60
TEXT_W = GUEST_X - TEXT_X - 60
TEXT_Y = 160
TEXT_H = HEIGHT - 220

ANKY_GLOW_COLOR = (160, 80, 255)   # purple
GUEST_GLOW_COLOR = (220, 220, 255)  # white-blue

ANKY_IMAGES_DIR = os.path.expanduser("~/anky/data/images")
ASSETS_DIR = os.path.join(os.path.dirname(__file__), "assets")

# RTMP config
RTMP_URL = os.environ.get("PUMPFUN_RTMP_URL", "")
STREAM_KEY = os.environ.get("PUMPFUN_STREAM_KEY", "")

# Audio: 16000Hz mono 16-bit (matches Voxtral STT input + browser PCM)
AUDIO_SAMPLE_RATE = 16000
AUDIO_CHANNELS = 1
AUDIO_SAMPLE_WIDTH = 2  # 16-bit

# Silence: 1 second of silence at 16000Hz mono 16-bit
SILENCE_CHUNK = b"\x00" * (AUDIO_SAMPLE_RATE * AUDIO_CHANNELS * AUDIO_SAMPLE_WIDTH)


def _load_font(size: int):
    for path in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
    ]:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


FONT_HEADER = _load_font(48)
FONT_TEXT = _load_font(28)
FONT_FOOTER = _load_font(24)
FONT_LABEL = _load_font(26)
FONT_STATUS = _load_font(30)


ANKY_ROTATE_SECONDS = 15  # rotate Anky image every N seconds


class Compositor:
    def __init__(self):
        self.anky_image: Image.Image | None = None
        self.guest_image: Image.Image | None = None
        self._anky_images: list[Image.Image] = []
        self._anky_idx: int = 0
        self._anky_last_rotate: float = 0.0
        self._load_anky_gallery()
        self._load_default_guest()

        self.active_speaker: str = "none"  # "anky", "guest", "none"
        self.transcript_lines: list[dict] = []  # [{role, text}]
        self.glow_phase: float = 0.0
        self.guest_name: str = "guest"
        self.has_guest: bool = False
        self.stream_status: str = "idle"  # "idle", "listening", "transcribing", "thinking", "speaking"
        self._status_phase: float = 0.0

        # ffmpeg process
        self._ffmpeg_proc: subprocess.Popen | None = None
        self._running = False
        self._render_thread: threading.Thread | None = None

        # Audio FIFO for piping TTS to ffmpeg
        self._audio_fifo_path: str | None = None
        self._audio_fifo_fd = None
        self._audio_thread: threading.Thread | None = None
        self._audio_queue: list[bytes] = []  # queue of raw PCM chunks
        self._audio_lock = threading.Lock()

    @staticmethod
    def _fit_image(img: Image.Image, box_w: int, box_h: int) -> Image.Image:
        """Resize image to fit within box while preserving aspect ratio.

        Returns a box_w x box_h RGBA image with the source centered.
        """
        src_w, src_h = img.size
        scale = min(box_w / src_w, box_h / src_h)
        new_w = int(src_w * scale)
        new_h = int(src_h * scale)
        resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
        canvas = Image.new("RGBA", (box_w, box_h), (0, 0, 0, 0))
        x = (box_w - new_w) // 2
        y = (box_h - new_h) // 2
        canvas.paste(resized, (x, y), resized if resized.mode == "RGBA" else None)
        return canvas

    def _load_anky_gallery(self):
        """Load all Anky images, shuffle them, and set the first one."""
        pngs = glob.glob(os.path.join(ANKY_IMAGES_DIR, "*.png"))
        pngs = [p for p in pngs if "_thumb" not in p]
        random.shuffle(pngs)
        for path in pngs[:50]:
            try:
                img = Image.open(path).convert("RGBA")
                fitted = self._fit_image(img, SQUARE_SIZE, SQUARE_SIZE)
                self._anky_images.append(fitted)
            except Exception as e:
                log.warning("Failed to load %s: %s", path, e)
        if self._anky_images:
            self.anky_image = self._anky_images[0]
            log.info("Loaded %d Anky images for rotation", len(self._anky_images))
        else:
            self.anky_image = Image.new("RGBA", (SQUARE_SIZE, SQUARE_SIZE), ANKY_GLOW_COLOR)
            log.warning("No Anky images found")
        self._anky_last_rotate = time.monotonic()

    def _load_default_guest(self):
        default_path = os.path.join(ASSETS_DIR, "guest_default.png")
        if os.path.exists(default_path):
            img = Image.open(default_path).convert("RGBA")
            self.guest_image = self._fit_image(img, SQUARE_SIZE, SQUARE_SIZE)
        else:
            self.guest_image = Image.new("RGBA", (SQUARE_SIZE, SQUARE_SIZE), (60, 60, 70))

    def set_guest_image(self, image_bytes: bytes):
        try:
            from io import BytesIO
            img = Image.open(BytesIO(image_bytes)).convert("RGBA")
            self.guest_image = self._fit_image(img, SQUARE_SIZE, SQUARE_SIZE)
            log.info("Guest PFP updated")
        except Exception as e:
            log.warning("Failed to load guest PFP: %s", e)

    def add_transcript(self, role: str, text: str):
        self.transcript_lines.append({"role": role, "text": text})
        if len(self.transcript_lines) > 6:
            self.transcript_lines = self.transcript_lines[-6:]

    def queue_tts_audio(self, wav_bytes: bytes):
        """Queue TTS WAV audio to be piped to ffmpeg's audio track."""
        if len(wav_bytes) <= 44:
            return
        # Strip WAV header (44 bytes), keep raw PCM
        pcm = wav_bytes[44:]
        with self._audio_lock:
            self._audio_queue.append(pcm)

    def queue_raw_pcm(self, pcm_bytes: bytes):
        """Queue raw PCM int16 16000Hz mono directly to the stream."""
        if len(pcm_bytes) < 32:
            return
        with self._audio_lock:
            self._audio_queue.append(pcm_bytes)

    def render_frame(self) -> Image.Image:
        # Rotate Anky image periodically
        now = time.monotonic()
        if self._anky_images and (now - self._anky_last_rotate) >= ANKY_ROTATE_SECONDS:
            self._anky_idx = (self._anky_idx + 1) % len(self._anky_images)
            self.anky_image = self._anky_images[self._anky_idx]
            self._anky_last_rotate = now

        frame = Image.new("RGBA", (WIDTH, HEIGHT), BG_COLOR + (255,))
        draw = ImageDraw.Draw(frame)

        self.glow_phase += 0.15
        self._status_phase += 0.12
        glow_intensity = 0.5 + 0.5 * math.sin(self.glow_phase)

        # Header
        draw.text((ANKY_X, 30), "$ANKY", font=FONT_HEADER,
                   fill=(200, 160, 255), anchor="lt")

        # Status indicator (top center)
        self._draw_status_indicator(draw)

        # Anky square (left)
        self._draw_square_with_glow(
            frame, draw, ANKY_X, ANKY_Y, self.anky_image,
            ANKY_GLOW_COLOR, glow_intensity if self.active_speaker == "anky" else 0.0,
        )
        draw.text((ANKY_X + SQUARE_SIZE // 2, ANKY_Y + SQUARE_SIZE + 20),
                   "anky", font=FONT_LABEL, fill=ANKY_GLOW_COLOR, anchor="mt")

        # Guest square (right)
        self._draw_square_with_glow(
            frame, draw, GUEST_X, GUEST_Y, self.guest_image,
            GUEST_GLOW_COLOR, glow_intensity if self.active_speaker == "guest" else 0.0,
        )
        draw.text((GUEST_X + SQUARE_SIZE // 2, GUEST_Y + SQUARE_SIZE + 20),
                   self.guest_name, font=FONT_LABEL, fill=GUEST_GLOW_COLOR, anchor="mt")

        # Transcript (center)
        self._draw_transcript(draw)

        # CTA message when no guest is connected
        if not self.has_guest:
            cta = "be the guest for anky. go to anky.app/interview and join"
            draw.text((WIDTH // 2, HEIGHT - 70), cta, font=FONT_TEXT,
                       fill=(200, 180, 255), anchor="mm")

        # Footer
        draw.text((WIDTH - 80, HEIGHT - 30), "anky.app", font=FONT_FOOTER,
                   fill=(120, 120, 140), anchor="rb")

        return frame

    def _draw_status_indicator(self, draw):
        """Draw an animated status indicator at the top-center of the frame."""
        if self.stream_status == "idle" and not self.has_guest:
            return  # CTA at bottom is enough

        cx = WIDTH // 2
        y = 40
        pulse = 0.5 + 0.5 * math.sin(self._status_phase)

        if self.stream_status == "listening":
            # Green pulsing dot + text
            dot_r = int(6 + 3 * pulse)
            green = (80, 200, 120)
            draw.ellipse([cx - 140 - dot_r, y + 8 - dot_r, cx - 140 + dot_r, y + 8 + dot_r], fill=green)
            draw.text((cx - 120, y), f"{self.guest_name} is speaking", font=FONT_STATUS,
                       fill=green, anchor="lm")

        elif self.stream_status == "transcribing":
            # Animated dots
            dots = "." * (1 + int(self._status_phase) % 3)
            color = (200, 160, 255)
            draw.text((cx, y), f"transcribing{dots}", font=FONT_STATUS,
                       fill=color, anchor="mm")

        elif self.stream_status == "thinking":
            dots = "." * (1 + int(self._status_phase) % 3)
            # Purple pulse
            alpha = int(150 + 105 * pulse)
            color = (160, 80 + int(40 * pulse), 255)
            draw.text((cx, y), f"anky is thinking{dots}", font=FONT_STATUS,
                       fill=color, anchor="mm")

        elif self.stream_status == "speaking":
            color = (200, 160, 255)
            # Animated sound waves
            wave = "))) " if pulse > 0.5 else "))  "
            draw.text((cx, y), f"{wave} anky is speaking {wave[::-1]}", font=FONT_STATUS,
                       fill=color, anchor="mm")

        elif self.stream_status == "idle" and self.has_guest:
            color = (100, 100, 120)
            draw.text((cx, y), f"{self.guest_name} joined — interview starting",
                       font=FONT_STATUS, fill=color, anchor="mm")

    def _draw_square_with_glow(self, frame, draw, x, y, image, glow_color, intensity):
        border = 6
        if intensity > 0.05:
            glow_width = int(12 + 8 * intensity)
            r, g, b = glow_color
            for i in range(glow_width, 0, -2):
                draw.rectangle(
                    [x - i, y - i, x + SQUARE_SIZE + i, y + SQUARE_SIZE + i],
                    outline=(r, g, b), width=2,
                )
        else:
            draw.rectangle(
                [x - border, y - border, x + SQUARE_SIZE + border, y + SQUARE_SIZE + border],
                outline=(40, 40, 50), width=border,
            )
        if image:
            frame.paste(image, (x, y), image if image.mode == "RGBA" else None)

    def _draw_transcript(self, draw):
        y = TEXT_Y
        for entry in self.transcript_lines[-4:]:
            role = entry["role"]
            text = entry["text"]
            color = ANKY_GLOW_COLOR if role == "anky" else (200, 200, 210)
            label = "anky" if role == "anky" else self.guest_name
            prefix = f"{label}: "

            lines = self._wrap_text(prefix + text, FONT_TEXT, TEXT_W)
            for line in lines:
                if y > TEXT_Y + TEXT_H - 30:
                    break
                draw.text((TEXT_X, y), line, font=FONT_TEXT, fill=color)
                y += 34
            y += 10

    def _wrap_text(self, text, font, max_width):
        words = text.split()
        lines = []
        current = ""
        for word in words:
            test = f"{current} {word}".strip()
            bbox = font.getbbox(test)
            w = bbox[2] - bbox[0]
            if w <= max_width:
                current = test
            else:
                if current:
                    lines.append(current)
                current = word
        if current:
            lines.append(current)
        return lines or [""]

    # ------------------------------------------------------------------
    # RTMP streaming with audio
    # ------------------------------------------------------------------

    def start_stream(self):
        if not RTMP_URL or not STREAM_KEY:
            log.warning("No RTMP credentials — streaming disabled.")
            self._running = True
            self._render_thread = threading.Thread(target=self._render_loop_no_stream, daemon=True)
            self._render_thread.start()
            return

        # Create audio FIFO
        fifo_dir = tempfile.mkdtemp(prefix="anky_audio_")
        self._audio_fifo_path = os.path.join(fifo_dir, "audio.pcm")
        os.mkfifo(self._audio_fifo_path)
        log.info("Audio FIFO: %s", self._audio_fifo_path)

        rtmp_dest = f"{RTMP_URL}/{STREAM_KEY}"
        cmd = [
            "ffmpeg", "-y",
            # Video input: raw RGBA frames from stdin
            "-thread_queue_size", "1024",
            "-f", "rawvideo",
            "-pixel_format", "rgba",
            "-video_size", f"{WIDTH}x{HEIGHT}",
            "-framerate", "10",
            "-i", "pipe:0",
            # Audio input: raw PCM from FIFO
            "-thread_queue_size", "4096",
            "-f", "s16le",
            "-ar", str(AUDIO_SAMPLE_RATE),
            "-ac", str(AUDIO_CHANNELS),
            "-i", self._audio_fifo_path,
            # Map
            "-map", "0:v",
            "-map", "1:a",
            # Video encoding
            "-c:v", "h264_nvenc",
            "-preset", "p1",
            "-tune", "ll",
            "-pix_fmt", "yuv420p",
            "-b:v", "2500k",
            "-g", "20",
            # Audio encoding
            "-c:a", "aac",
            "-b:a", "128k",
            "-ar", "44100",
            "-ac", "2",
            # Output
            "-f", "flv",
            rtmp_dest,
        ]

        log.info("Starting ffmpeg RTMP stream to %s", rtmp_dest)
        # Let ffmpeg see all GPUs for nvenc (parent process restricts to GPU 1 for Voxtral)
        ffmpeg_env = os.environ.copy()
        ffmpeg_env.pop("CUDA_VISIBLE_DEVICES", None)
        self._ffmpeg_proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            env=ffmpeg_env,
        )

        # Start audio feeder thread (must open FIFO before ffmpeg blocks on it)
        self._running = True
        self._audio_thread = threading.Thread(target=self._audio_feed_loop, daemon=True)
        self._audio_thread.start()

        # Give ffmpeg a moment
        time.sleep(1.0)
        if self._ffmpeg_proc.poll() is not None:
            stderr = self._ffmpeg_proc.stderr.read().decode(errors="replace")[-500:]
            log.error("ffmpeg failed to start: %s", stderr)
            self._ffmpeg_proc = None
            self._render_thread = threading.Thread(target=self._render_loop_no_stream, daemon=True)
            self._render_thread.start()
            return

        self._render_thread = threading.Thread(target=self._render_loop, daemon=True)
        self._render_thread.start()
        log.info("RTMP stream started.")

    def stop_stream(self):
        self._running = False
        if self._ffmpeg_proc:
            try:
                self._ffmpeg_proc.stdin.close()
            except Exception:
                pass
            try:
                self._ffmpeg_proc.wait(timeout=5)
            except Exception:
                self._ffmpeg_proc.kill()
            self._ffmpeg_proc = None
        if self._audio_fifo_path:
            try:
                os.unlink(self._audio_fifo_path)
                os.rmdir(os.path.dirname(self._audio_fifo_path))
            except Exception:
                pass
        log.info("Stream stopped.")

    def _audio_feed_loop(self):
        """Continuously feed audio to ffmpeg via the FIFO.

        When TTS audio is queued, write it. Otherwise write silence
        so ffmpeg doesn't stall waiting for audio data.
        """
        try:
            fd = open(self._audio_fifo_path, "wb", buffering=0)
        except Exception as e:
            log.error("Failed to open audio FIFO: %s", e)
            return

        log.info("Audio FIFO opened for writing")
        try:
            while self._running:
                chunk = None
                with self._audio_lock:
                    if self._audio_queue:
                        chunk = self._audio_queue.pop(0)

                if chunk:
                    try:
                        fd.write(chunk)
                    except (BrokenPipeError, OSError):
                        break
                else:
                    # Write silence to keep audio stream flowing
                    try:
                        fd.write(SILENCE_CHUNK)
                    except (BrokenPipeError, OSError):
                        break
                    time.sleep(1.0)  # silence is 1 second
        finally:
            try:
                fd.close()
            except Exception:
                pass

    def _render_loop(self):
        fps = 10
        interval = 1.0 / fps
        while self._running:
            if self._ffmpeg_proc is None or self._ffmpeg_proc.poll() is not None:
                log.warning("ffmpeg not running, restarting in 5s...")
                time.sleep(5)
                self._restart_ffmpeg()
                continue

            t0 = time.monotonic()
            try:
                frame = self.render_frame()
                raw = frame.tobytes()
                self._ffmpeg_proc.stdin.write(raw)
                self._ffmpeg_proc.stdin.flush()
            except (BrokenPipeError, OSError):
                stderr = ""
                try:
                    stderr = self._ffmpeg_proc.stderr.read().decode(errors="replace")[-500:]
                except Exception:
                    pass
                log.error("ffmpeg pipe broken, will restart: %s", stderr)
                self._ffmpeg_proc = None
                continue
            except Exception as e:
                log.error("Render error: %s", e)

            elapsed = time.monotonic() - t0
            sleep_time = interval - elapsed
            if sleep_time > 0:
                time.sleep(sleep_time)

    def _restart_ffmpeg(self):
        """Restart ffmpeg and the audio FIFO."""
        # Clean up old FIFO
        if self._audio_fifo_path:
            try:
                os.unlink(self._audio_fifo_path)
            except Exception:
                pass

        # Create new FIFO
        fifo_dir = tempfile.mkdtemp(prefix="anky_audio_")
        self._audio_fifo_path = os.path.join(fifo_dir, "audio.pcm")
        os.mkfifo(self._audio_fifo_path)

        rtmp_dest = f"{RTMP_URL}/{STREAM_KEY}"
        cmd = [
            "ffmpeg", "-y",
            "-thread_queue_size", "1024",
            "-f", "rawvideo",
            "-pixel_format", "rgba",
            "-video_size", f"{WIDTH}x{HEIGHT}",
            "-framerate", "10",
            "-i", "pipe:0",
            "-thread_queue_size", "4096",
            "-f", "s16le",
            "-ar", str(AUDIO_SAMPLE_RATE),
            "-ac", str(AUDIO_CHANNELS),
            "-i", self._audio_fifo_path,
            "-map", "0:v", "-map", "1:a",
            "-c:v", "h264_nvenc", "-preset", "p1", "-tune", "ll",
            "-pix_fmt", "yuv420p", "-b:v", "2500k", "-g", "20",
            "-c:a", "aac", "-b:a", "128k", "-ar", "44100", "-ac", "2",
            "-f", "flv", rtmp_dest,
        ]

        ffmpeg_env = os.environ.copy()
        ffmpeg_env.pop("CUDA_VISIBLE_DEVICES", None)
        self._ffmpeg_proc = subprocess.Popen(
            cmd, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE, env=ffmpeg_env,
        )

        # Restart audio feeder for new FIFO
        self._audio_thread = threading.Thread(target=self._audio_feed_loop, daemon=True)
        self._audio_thread.start()

        time.sleep(1)
        if self._ffmpeg_proc.poll() is not None:
            stderr = self._ffmpeg_proc.stderr.read().decode(errors="replace")[-300:]
            log.error("ffmpeg restart failed: %s", stderr)
            self._ffmpeg_proc = None
        else:
            log.info("ffmpeg RTMP stream restarted.")

    def _render_loop_no_stream(self):
        while self._running:
            self.render_frame()
            time.sleep(1.0 / 10)
