use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

const LIVE_TEXT_PATH: &str = "/tmp/anky_live.txt";
const FONT_PATH: &str = "/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf";
const WRAP_WIDTH: usize = 40;

/// Write wrapped text to the live text file atomically (write .tmp, then rename).
pub fn write_live_text(text: &str) {
    let wrapped = wrap_text(text, WRAP_WIDTH);
    let tmp = format!("{}.tmp", LIVE_TEXT_PATH);
    if std::fs::write(&tmp, &wrapped).is_ok() {
        let _ = std::fs::rename(&tmp, LIVE_TEXT_PATH);
    }
}

/// Write the idle screen text.
pub fn write_idle_text() {
    write_live_text("waiting for a writer...\nyou can be that person on\nhttps://anky.app");
}

/// Write a full frame with life bar, auto-scrolled text, stats, and progress bar.
pub fn write_live_frame(text: &str, words: i64, elapsed_secs: f64, idle_ratio: f64, progress: f64) {
    const BAR_INNER: usize = 38;
    const TEXT_LINES: usize = 33;

    let mut lines = Vec::with_capacity(38);

    // Line 1: life bar (idle countdown)
    let idle = idle_ratio.clamp(0.0, 1.0);
    let filled = (idle * BAR_INNER as f64).round() as usize;
    let empty = BAR_INNER - filled;
    lines.push(format!("[{}{}]", "=".repeat(filled), "-".repeat(empty)));

    // Line 2: blank
    lines.push(String::new());

    // Lines 3-35: auto-scrolled wrapped text (last 33 lines)
    let wrapped = wrap_text(text, WRAP_WIDTH);
    let text_lines: Vec<&str> = wrapped.lines().collect();
    let start = if text_lines.len() > TEXT_LINES { text_lines.len() - TEXT_LINES } else { 0 };
    for line in &text_lines[start..] {
        lines.push(line.to_string());
    }
    // Pad to fill 33 lines
    while lines.len() < 2 + TEXT_LINES {
        lines.push(String::new());
    }

    // Line 36: blank
    lines.push(String::new());

    // Line 37: stats — "N words" left, "M:SS" right, padded to 40 chars
    let mins = (elapsed_secs / 60.0).floor() as u32;
    let secs = (elapsed_secs % 60.0).floor() as u32;
    let left = format!("{} words", words);
    let right = format!("{}:{:02}", mins, secs);
    let pad = if left.len() + right.len() < WRAP_WIDTH {
        WRAP_WIDTH - left.len() - right.len()
    } else {
        1
    };
    lines.push(format!("{}{}{}", left, " ".repeat(pad), right));

    // Line 38: progress bar (8-minute progress)
    let prog = progress.clamp(0.0, 1.0);
    let pfilled = (prog * BAR_INNER as f64).round() as usize;
    let pempty = BAR_INNER - pfilled;
    lines.push(format!("[{}{}]", "=".repeat(pfilled), "-".repeat(pempty)));

    let frame = lines.join("\n");
    let tmp = format!("{}.tmp", LIVE_TEXT_PATH);
    if std::fs::write(&tmp, &frame).is_ok() {
        let _ = std::fs::rename(&tmp, LIVE_TEXT_PATH);
    }
}

/// Word-wrap text to fit within `width` characters per line.
fn wrap_text(text: &str, width: usize) -> String {
    let mut result = String::new();
    for line in text.lines() {
        if line.is_empty() {
            result.push('\n');
            continue;
        }
        let mut col = 0;
        for word in line.split_whitespace() {
            let wlen = word.len();
            if col > 0 && col + 1 + wlen > width {
                result.push('\n');
                col = 0;
            }
            if col > 0 {
                result.push(' ');
                col += 1;
            }
            result.push_str(word);
            col += wlen;
        }
        result.push('\n');
    }
    result
}

/// Spawn the always-on ffmpeg RTMP stream. Restarts on crash with 5s delay.
/// Reads text from `/tmp/anky_live.txt` using drawtext reload=1.
pub async fn spawn_ffmpeg_loop(
    rtmp_url: String,
    stream_key: String,
    live_state: Arc<RwLock<crate::state::LiveState>>,
) {
    // Write initial idle text
    write_idle_text();

    loop {
        let full_url = format!("{}/{}", rtmp_url.trim_end_matches('/'), stream_key);

        tracing::info!("Starting ffmpeg RTMP stream to {}", rtmp_url);

        let drawtext = format!(
            "drawtext=textfile={}:reload=1:fontfile={}:fontsize=32:fontcolor=0xcccccc:x=60:y=100:line_spacing=12",
            LIVE_TEXT_PATH, FONT_PATH
        );

        let result = Command::new("ffmpeg")
            .args([
                "-f", "lavfi",
                "-i", "color=c=0x0a0a12:s=1080x1920:r=10",
                "-vf", &drawtext,
                "-c:v", "libx264",
                "-preset", "ultrafast",
                "-tune", "zerolatency",
                "-b:v", "2500k",
                "-g", "20",
                "-f", "flv",
                &full_url,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                let status = child.wait().await;
                tracing::warn!("ffmpeg exited: {:?}", status);
            }
            Err(e) => {
                tracing::error!("Failed to spawn ffmpeg: {}", e);
            }
        }

        // On crash, make sure live state is reset
        {
            let mut state = live_state.write().await;
            if state.is_live {
                state.is_live = false;
                state.writer_id = None;
            }
        }
        write_idle_text();

        tracing::info!("Restarting ffmpeg in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
