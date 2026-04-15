pub mod altar;
pub mod api;
pub mod auth;
pub mod collection;
pub mod dashboard;
pub mod evolve;
pub mod extension_api;
pub mod generations;
pub mod health;
pub mod interview;
pub mod live;
pub mod notification;
pub mod now;
pub mod pages;
pub mod payment;
pub mod payment_helper;
pub mod poiesis;
pub mod prompt;
pub mod qr_auth;
pub mod relay;
pub mod sealed;
pub mod session;
pub mod settings;
pub mod simulations;
pub mod social_context;
pub mod swift;
pub mod training;
pub mod voices;
pub mod webhook_farcaster;
pub mod webhook_x;
pub mod writing;

use crate::middleware;
use crate::state::AppState;
use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

async fn apple_app_site_association() -> ([(axum::http::HeaderName, &'static str); 1], &'static str)
{
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        r#"{"applinks":{"details":[{"appIDs":["84V63LKV45.com.jpfraneto.Anky"],"components":[{"/":"/write*"},{"/":"/seal*"}]}]}}"#,
    )
}

async fn farcaster_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../static/farcaster.json"),
    )
}

async fn agent_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../static/agent.json"),
    )
}

async fn service_worker() -> ([(axum::http::HeaderName, &'static str); 2], &'static str) {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            ("Service-Worker-Allowed".parse().unwrap(), "/"),
        ],
        include_str!("../../static/sw.js"),
    )
}

async fn prompt_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../PROMPT.md"),
    )
}

async fn manifesto_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../MANIFESTO.md"),
    )
}

async fn soul_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../SOUL.md"),
    )
}

async fn terms_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../TERMS_OF_SERVICE.md"),
    )
}

async fn privacy_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../PRIVACY_POLICY.md"),
    )
}

async fn faq_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../FAQ.md"),
    )
}

// ── Station API ─────────────────────────────────────────────────────────

async fn station_steps() -> axum::Json<serde_json::Value> {
    let pipeline_dir = std::path::Path::new("videos/gods/pipeline");
    let mut steps = Vec::new();
    if pipeline_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(pipeline_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
                .collect();
            files.sort_by_key(|e| e.file_name());
            for entry in files {
                let name = entry.file_name().to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                steps.push(serde_json::json!({
                    "name": name,
                    "size": size,
                    "modified": entry.metadata().ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                        .unwrap_or(0),
                }));
            }
        }
    }
    axum::Json(serde_json::json!({ "steps": steps }))
}

async fn station_step_content(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), axum::http::StatusCode> {
    let path = std::path::Path::new("videos/gods/pipeline").join(&name);
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok((
            [(
                axum::http::header::CONTENT_TYPE,
                "text/markdown; charset=utf-8",
            )],
            content,
        )),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// Serve generated assets (images + audio) from videos/gods/* for station preview.
// Path traversal prevented by rejecting ".." segments.
async fn station_asset(
    axum::extract::Path(rel): axum::extract::Path<String>,
) -> Result<([(axum::http::HeaderName, String); 1], Vec<u8>), axum::http::StatusCode> {
    if rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }
    let path = std::path::Path::new("videos/gods").join(&rel);
    let bytes = std::fs::read(&path).map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
    let ct = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "ass" => "text/plain; charset=utf-8",
        "json" => "application/json",
        "html" | "htm" => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    };
    Ok(([(axum::http::header::CONTENT_TYPE, ct.to_string())], bytes))
}

async fn station_run_pipeline() -> axum::Json<serde_json::Value> {
    // Spawn the pipeline in background
    let _ = tokio::process::Command::new("python3")
        .arg("scripts/gods_pipeline.py")
        .current_dir(".")
        .env("PYTHONUNBUFFERED", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    axum::Json(serde_json::json!({ "status": "started" }))
}

async fn station_run_step(
    axum::extract::Path(step): axum::extract::Path<String>,
) -> axum::Json<serde_json::Value> {
    let _ = tokio::process::Command::new("python3")
        .args(["scripts/gods_pipeline.py", "--step", &step])
        .current_dir(".")
        .env("PYTHONUNBUFFERED", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    axum::Json(serde_json::json!({ "status": "started", "step": step }))
}

// ── Story review + prompt evolution ────────────────────────────────────

fn review_data_dir() -> std::path::PathBuf {
    let p = std::path::PathBuf::from("data/review");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn current_god_and_script_path() -> Option<(String, std::path::PathBuf, std::path::PathBuf)> {
    // Pull god name from the combined script index, then return EN and ES paths.
    let index = std::path::Path::new("videos/gods/pipeline/03_script.md");
    let text = std::fs::read_to_string(index).ok()?;
    let mut god = String::from("Unknown");
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("- **God:**") {
            god = rest.trim().to_string();
            break;
        }
    }
    let en = std::path::PathBuf::from("videos/gods/pipeline/03_script_en.md");
    let es = std::path::PathBuf::from("videos/gods/pipeline/03_script_es.md");
    Some((god, en, es))
}

async fn station_review_comments_get(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let episode = q.get("episode").cloned().unwrap_or_default();
    if episode.is_empty() {
        return axum::Json(serde_json::json!({ "comments": [] }));
    }
    let path = review_data_dir().join(format!("{}_comments.json", episode));
    let comments: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!([]));
    axum::Json(serde_json::json!({ "episode": episode, "comments": comments }))
}

async fn station_review_comment_post(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let episode = body.get("episode").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let block_id = body.get("block_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let block_text = body
        .get("block_text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if episode.is_empty() || text.trim().is_empty() {
        return axum::Json(serde_json::json!({ "ok": false, "error": "episode + text required" }));
    }
    let path = review_data_dir().join(format!("{}_comments.json", episode));
    let mut comments: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    comments.push(serde_json::json!({
        "id": id,
        "block_id": block_id,
        "block_text": block_text,
        "text": text,
        "created_at": chrono::Utc::now().to_rfc3339(),
    }));
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&comments).unwrap());
    axum::Json(serde_json::json!({ "ok": true, "comments": comments }))
}

async fn station_review_comment_delete(
    axum::extract::Path((episode, id)): axum::extract::Path<(String, String)>,
) -> axum::Json<serde_json::Value> {
    let path = review_data_dir().join(format!("{}_comments.json", episode));
    let mut comments: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    comments.retain(|c| c.get("id").and_then(|v| v.as_str()) != Some(&id));
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&comments).unwrap());
    axum::Json(serde_json::json!({ "ok": true }))
}

async fn station_review_evolve(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let episode = body.get("episode").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let lang = body.get("lang").and_then(|v| v.as_str()).unwrap_or("en").to_string();
    if episode.is_empty() {
        return axum::Json(serde_json::json!({ "ok": false, "error": "episode required" }));
    }

    // Load comments
    let comments_path = review_data_dir().join(format!("{}_comments.json", episode));
    let comments: Vec<serde_json::Value> = std::fs::read_to_string(&comments_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if comments.is_empty() {
        return axum::Json(serde_json::json!({
            "ok": false, "error": "no comments to evolve from — add some first"
        }));
    }

    // Load the script being reviewed
    let script_path = format!("videos/gods/pipeline/03_script_{}.md", lang);
    let script = std::fs::read_to_string(&script_path).unwrap_or_default();

    // Load current learned notes
    let overrides_path = std::path::Path::new("scripts/prompt_overrides.py");
    let overrides_src = std::fs::read_to_string(overrides_path).unwrap_or_default();
    let current_notes = extract_learned_notes(&overrides_src);

    // Load last 10 history entries for context
    let history_path = std::path::Path::new("data/review_history.jsonl");
    let history_tail: Vec<String> = std::fs::read_to_string(history_path)
        .ok()
        .map(|s| {
            let all: Vec<String> = s.lines().map(|l| l.to_string()).collect();
            let start = all.len().saturating_sub(10);
            all[start..].to_vec()
        })
        .unwrap_or_default();

    // Build meta-prompt for Qwen
    let comments_bullets: String = comments
        .iter()
        .map(|c| {
            let block = c.get("block_text").and_then(|v| v.as_str()).unwrap_or("");
            let text = c.get("text").and_then(|v| v.as_str()).unwrap_or("");
            format!("  - On this line: \"{}\"\n    Feedback: {}", block.chars().take(140).collect::<String>(), text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let history_block = if history_tail.is_empty() {
        String::new()
    } else {
        format!("\nRECENT REVIEW HISTORY (for context, do not duplicate):\n{}\n",
            history_tail.join("\n"))
    };

    let meta = format!(
        r#"You are tuning a bedtime-story generation system. The stories are for 4-year-old children, in the Bluey / Rudolf Steiner / Waldorf register. They should be soft, reverent, relational, emotionally literate. No didactic lessons. The god in each story honours things; it does not teach.

Here is the story that was just generated (in {lang}):

────────────────
{script}
────────────────

Here is the human reviewer's feedback:

{comments_bullets}
{history_block}

Here are the LEARNED STYLE NOTES currently in effect (they've already been applied to the prompts — do not duplicate anything already listed here):

────────────────
{current_notes}
────────────────

Your job: propose 1 to 4 NEW bullet points to ADD to the learned style notes. Each bullet should be concrete, actionable, and addressed to a future story-writing LLM. Bullets should be short (≤20 words each) and specific enough that an LLM can follow them without interpretation.

Good bullets:
  - "Do not have the god deliver aphorisms or truisms. The god speaks plainly, like a grandmother."
  - "Avoid the word 'little one' when the god addresses the child. Use just 'you' or name a small thing."

Bad bullets (too vague):
  - "Make it softer."
  - "Better pacing."

Output JSON only, no other text:
{{"proposed_notes": ["bullet 1", "bullet 2"], "rationale": "one sentence summary of what you changed and why"}}
"#,
        lang = lang,
        script = script,
        comments_bullets = comments_bullets,
        history_block = history_block,
        current_notes = if current_notes.trim().is_empty() { "(empty — first evolution)" } else { current_notes.trim() }
    );

    // Call Qwen
    let client = reqwest::Client::new();
    let llm_resp = client
        .post("http://localhost:8080/v1/chat/completions")
        .json(&serde_json::json!({
            "model": "local",
            "messages": [
                {"role": "system", "content": "You are a precise tuning assistant. You output only JSON."},
                {"role": "user", "content": meta}
            ],
            "temperature": 0.5,
            "max_tokens": 2000,
        }))
        .send()
        .await;

    let proposal = match llm_resp {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(v) => {
                let content = v
                    .pointer("/choices/0/message/content")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                // Strip code fences if present
                let cleaned = content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                // Find first { to last } to be robust to extra prose
                let start = cleaned.find('{');
                let end = cleaned.rfind('}');
                let json_str = match (start, end) {
                    (Some(s), Some(e)) if e > s => &cleaned[s..=e],
                    _ => cleaned,
                };
                serde_json::from_str::<serde_json::Value>(json_str)
                    .unwrap_or_else(|_| serde_json::json!({ "proposed_notes": [], "rationale": content }))
            }
            Err(e) => serde_json::json!({ "error": format!("bad llm response: {}", e) }),
        },
        Err(e) => serde_json::json!({ "error": format!("llm unreachable: {}", e) }),
    };

    axum::Json(serde_json::json!({
        "ok": true,
        "episode": episode,
        "lang": lang,
        "comment_count": comments.len(),
        "proposal": proposal,
    }))
}

fn extract_learned_notes(src: &str) -> String {
    // Pull the string literal that follows LEARNED_STYLE_NOTES =
    let needle = "LEARNED_STYLE_NOTES = \"\"\"";
    if let Some(start) = src.find(needle) {
        let after = &src[start + needle.len()..];
        if let Some(end) = after.find("\"\"\"") {
            return after[..end].to_string();
        }
    }
    String::new()
}

async fn station_review_apply(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let episode = body.get("episode").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let notes_to_add: Vec<String> = body
        .get("notes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let rationale = body.get("rationale").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if notes_to_add.is_empty() {
        return axum::Json(serde_json::json!({ "ok": false, "error": "no notes provided" }));
    }

    // Append to prompt_overrides.py
    let overrides_path = std::path::Path::new("scripts/prompt_overrides.py");
    let src = std::fs::read_to_string(overrides_path).unwrap_or_default();
    let current = extract_learned_notes(&src);
    let mut combined = current.trim_end().to_string();
    for note in &notes_to_add {
        let bullet = note.trim();
        if !bullet.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str("- ");
            combined.push_str(bullet.trim_start_matches("- "));
        }
    }

    let new_src = format!(
        "\"\"\"Learned style notes, grown by the story-review feedback loop.\n\nPrepended to each story-generation prompt in gods_pipeline.py.\nEdited by POST /api/station/review/apply after an evolution cycle.\n\"\"\"\n\n# Bulleted style notes learned from human review across episodes.\n# Each bullet should be concrete and actionable. No vague platitudes.\nLEARNED_STYLE_NOTES = \"\"\"\n{}\n\"\"\"\n",
        combined
    );
    let _ = std::fs::write(overrides_path, new_src);

    // Append to history jsonl
    let history_path = std::path::Path::new("data/review_history.jsonl");
    let _ = std::fs::create_dir_all(history_path.parent().unwrap());
    let entry = serde_json::json!({
        "applied_at": chrono::Utc::now().to_rfc3339(),
        "episode": episode,
        "notes_added": notes_to_add,
        "rationale": rationale,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)
    {
        use std::io::Write;
        let _ = writeln!(f, "{}", entry);
    }

    axum::Json(serde_json::json!({
        "ok": true,
        "notes_added": notes_to_add.len(),
    }))
}

async fn station_review_regenerate() -> axum::Json<serde_json::Value> {
    // Kick step 3 (script) again in background — it picks up the new prompt_overrides.
    let _ = tokio::process::Command::new("python3")
        .args(["scripts/gods_pipeline.py", "--step", "script"])
        .current_dir(".")
        .env("PYTHONUNBUFFERED", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    axum::Json(serde_json::json!({ "ok": true, "status": "regenerating" }))
}

async fn station_tts(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Result<([(axum::http::HeaderName, &'static str); 1], Vec<u8>), axum::http::StatusCode> {
    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let voice_id = body
        .get("voice_id")
        .and_then(|v| v.as_str())
        .unwrap_or("cgSgspJ2msm6clMCkdW9");
    let model_id = body
        .get("model_id")
        .and_then(|v| v.as_str())
        .unwrap_or("eleven_v3");
    // v3 stability: API expects numeric 0.0..1.0. Map named presets to numeric.
    // creative = 0.3 (expressive, tags on), natural = 0.5 (balanced), robust = 0.75 (stable).
    let stability: f64 = match body.get("stability") {
        Some(v) if v.is_string() => match v.as_str().unwrap_or("") {
            "creative" => 0.3,
            "robust" => 0.75,
            _ => 0.5, // natural / unknown
        },
        Some(v) if v.is_number() => v.as_f64().unwrap_or(0.5),
        _ => 0.5,
    };
    let similarity_boost = body
        .get("similarity_boost")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.75);

    let api_key = std::env::var("ELEVENLABS_API_KEY").unwrap_or_default();
    if api_key.is_empty() || text.is_empty() {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{voice_id}"
        ))
        .header("xi-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "text": text,
            "model_id": model_id,
            "voice_settings": {
                "stability": stability,
                "similarity_boost": similarity_boost
            }
        }))
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %body, voice_id = %voice_id, "ElevenLabs TTS error");
        return Err(axum::http::StatusCode::BAD_GATEWAY);
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "audio/mpeg")],
        bytes.to_vec(),
    ))
}

async fn poiesis_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../POIESIS.md"),
    )
}

async fn pitch_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../static/pitch.md"),
    )
}

async fn protocol_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../static/protocol.md"),
    )
}

async fn spec_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../SPEC.md"),
    )
}

async fn skills() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("../../skills.md"),
    )
}

async fn skill_md() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        include_str!("../../agent-skills/anky/SKILL.md"),
    )
}

async fn skills_redirect() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/skills")
}

/// GET /prompts/{id} — serve markdown prompt files from prompts/ directory.
/// id must be exactly 4 digits (e.g. 0001).
async fn serve_prompt(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<([(axum::http::HeaderName, String); 1], String), axum::http::StatusCode> {
    // Validate: exactly 4 digits
    if id.len() != 4 || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(axum::http::StatusCode::NOT_FOUND);
    }
    let path = std::path::PathBuf::from("prompts").join(format!("{}.md", id));
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok((
            [(
                axum::http::header::CONTENT_TYPE,
                "text/markdown; charset=utf-8".to_string(),
            )],
            content,
        )),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

async fn anky_skill_bundle() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        "Anky installable skill bundle\n\n\
Bundle URL: https://anky.app/agent-skills/anky\n\
Manifest: https://anky.app/agent-skills/anky/manifest.json\n\
Entrypoint: https://anky.app/agent-skills/anky/SKILL.md\n\
\n\
Supporting files:\n\
- https://anky.app/agent-skills/anky/references/api.md\n\
- https://anky.app/agent-skills/anky/references/automation.md\n\
- https://anky.app/agent-skills/anky/references/quality.md\n\
- https://anky.app/agent-skills/anky/scripts/anky_session.py\n\
- https://anky.app/agent-skills/anky/agents/openai.yaml\n\
\n\
Session replay endpoint:\n\
- https://anky.app/api/v1/session/{session_id}/events (requires X-API-Key)\n\
- https://anky.app/api/v1/session/{session_id}/result (requires X-API-Key)\n\
\n\
Canonical practice doc: https://anky.app/skills\n",
    )
}

async fn anky_skill_bundle_manifest() -> ([(axum::http::HeaderName, &'static str); 1], &'static str)
{
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("../../agent-skills/anky/manifest.json"),
    )
}

async fn anky_skill_bundle_entry_redirect() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/agent-skills/anky/SKILL.md")
}

pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin([
            "https://anky.app".parse::<HeaderValue>().unwrap(),
            "https://www.anky.app".parse::<HeaderValue>().unwrap(),
            "https://pitch.anky.app".parse::<HeaderValue>().unwrap(),
            "https://ankycoin.com".parse::<HeaderValue>().unwrap(),
            "https://www.ankycoin.com".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            "x-api-key".parse().unwrap(),
            "payment-signature".parse().unwrap(),
            "x-payment".parse().unwrap(),
            "x-wallet".parse().unwrap(),
        ])
        .expose_headers([
            "payment-required".parse::<header::HeaderName>().unwrap(),
            "payment-response".parse::<header::HeaderName>().unwrap(),
        ])
        .allow_credentials(false);

    // Mobile CORS — allow any origin for the /swift/* routes (native apps + testing)
    let mobile_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(false);

    // Paid generate routes (optional API key — payment handled in handler)
    let generate_routes = Router::new()
        .route(
            "/api/v1/generate",
            axum::routing::post(api::generate_anky_paid),
        )
        .route(
            "/api/v1/prompt",
            axum::routing::post(prompt::create_prompt_api),
        )
        .route(
            "/api/v1/prompt/create",
            axum::routing::post(prompt::create_prompt_api),
        )
        .route(
            "/api/v1/prompt/quick",
            axum::routing::post(prompt::create_prompt_quick),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::optional_api_key,
        ));

    // Studio upload route (needs large body limit for video)
    let studio_routes = Router::new()
        .route(
            "/api/v1/studio/upload",
            axum::routing::post(api::upload_studio_video),
        )
        .layer(axum::extract::DefaultBodyLimit::max(512 * 1024 * 1024)); // 512MB

    // Media factory routes (large body limit for base64 image uploads)
    let media_factory_routes = Router::new()
        .route(
            "/api/v1/media-factory/video",
            axum::routing::post(api::media_factory_video),
        )
        .route(
            "/api/v1/media-factory/image",
            axum::routing::post(api::media_factory_image),
        )
        .route(
            "/api/v1/media-factory/flux",
            axum::routing::post(api::media_factory_flux),
        )
        .layer(axum::extract::DefaultBodyLimit::max(20 * 1024 * 1024)); // 20MB

    // Extension API routes (optional API key — payment handled in handler)
    let extension_routes = Router::new()
        .route(
            "/api/v1/transform",
            axum::routing::post(extension_api::transform),
        )
        .route(
            "/api/v1/balance",
            axum::routing::get(extension_api::balance),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::api_auth::optional_api_key,
        ));

    // Swift / Mobile API routes under /swift/v1/
    let swift_routes = Router::new()
        // Auth
        .route(
            "/swift/v1/auth/privy",
            axum::routing::post(swift::auth_privy),
        )
        .route(
            "/swift/v2/auth/challenge",
            axum::routing::post(swift::auth_seed_challenge),
        )
        .route(
            "/swift/v2/auth/verify",
            axum::routing::post(swift::auth_seed_verify),
        )
        .route(
            "/swift/v1/auth/session",
            axum::routing::delete(swift::auth_logout),
        )
        .route(
            "/swift/v2/auth/session",
            axum::routing::delete(swift::auth_logout),
        )
        // Me
        .route("/swift/v1/me", axum::routing::get(swift::get_me))
        .route("/swift/v2/me", axum::routing::get(swift::get_me))
        // Writings
        .route(
            "/swift/v1/writings",
            axum::routing::get(swift::list_writings),
        )
        .route(
            "/swift/v2/writings",
            axum::routing::get(swift::list_writings),
        )
        .route(
            "/swift/v1/write",
            axum::routing::post(swift::submit_writing_unified),
        )
        .route(
            "/swift/v2/write",
            axum::routing::post(swift::submit_writing_unified),
        )
        .route(
            "/swift/v2/writing/{sessionId}/status",
            axum::routing::get(swift::get_writing_status),
        )
        .route(
            "/swift/v2/writing/{sessionId}/retry-reflection",
            axum::routing::post(swift::retry_reflection),
        )
        // Children
        .route(
            "/swift/v2/children",
            axum::routing::get(swift::list_children).post(swift::create_child_profile),
        )
        .route(
            "/swift/v2/children/{childId}",
            axum::routing::get(swift::get_child_profile),
        )
        // Cuentacuentos
        .route(
            "/swift/v2/cuentacuentos/ready",
            axum::routing::get(swift::cuentacuentos_ready),
        )
        .route(
            "/swift/v2/cuentacuentos/history",
            axum::routing::get(swift::cuentacuentos_history),
        )
        .route(
            "/swift/v2/cuentacuentos/{id}/complete",
            axum::routing::post(swift::complete_cuentacuentos),
        )
        .route(
            "/swift/v2/cuentacuentos/{id}/assign",
            axum::routing::post(swift::assign_cuentacuentos),
        )
        // Prompt by ID (for deep links)
        .route(
            "/swift/v2/prompt/{id}",
            axum::routing::get(swift::get_prompt_by_id),
        )
        // Next Prompt
        .route(
            "/swift/v2/next-prompt",
            axum::routing::get(swift::get_next_prompt),
        )
        // Chat Prompt (opening message for new session)
        .route(
            "/swift/v2/chat/prompt",
            axum::routing::get(swift::get_chat_prompt),
        )
        // You (profile)
        .route("/swift/v2/you", axum::routing::get(swift::get_you))
        .route(
            "/swift/v2/you/ankys",
            axum::routing::get(swift::get_you_ankys),
        )
        // Items
        .route(
            "/swift/v2/you/items",
            axum::routing::get(swift::get_you_items),
        )
        // Mirror mint (iOS app)
        .route(
            "/swift/v2/mirror/mint",
            axum::routing::post(swift::swift_mirror_mint),
        )
        // Device Token (legacy path)
        .route(
            "/swift/v2/device-token",
            axum::routing::post(swift::register_device),
        )
        // Devices (new spec path)
        .route(
            "/swift/v2/devices",
            axum::routing::post(swift::register_device).delete(swift::delete_device),
        )
        // Settings
        .route(
            "/swift/v2/settings",
            axum::routing::get(swift::get_settings).patch(swift::patch_settings),
        )
        // Minting
        .route(
            "/swift/v2/writing/{sessionId}/prepare-mint",
            axum::routing::post(swift::prepare_mint),
        )
        .route(
            "/swift/v2/writing/{sessionId}/confirm-mint",
            axum::routing::post(swift::confirm_mint),
        )
        // Solana mirror mint (iOS raw path)
        .route(
            "/swift/v2/mint-mirror",
            axum::routing::post(swift::mint_raw_mirror),
        )
        // Alias: iOS app calls POST /mirror/mint
        .route("/mirror/mint", axum::routing::post(swift::mint_raw_mirror))
        // Sealed sessions
        .route(
            "/swift/v2/sealed-sessions",
            axum::routing::get(sealed::list_sealed_sessions),
        )
        // Admin
        .route(
            "/swift/v1/admin/premium",
            axum::routing::post(swift::set_premium),
        )
        .layer(mobile_cors);

    Router::new()
        // Pages
        .route("/", axum::routing::get(pages::home))
        .route("/altar", axum::routing::get(pages::altar_page))
        .route(
            "/profile-testing",
            axum::routing::get(pages::profile_testing_page),
        )
        .route("/write", axum::routing::get(pages::write_page))
        .route("/stories", axum::routing::get(pages::stories_page))
        .route("/ankys", axum::routing::get(pages::ankys_page))
        .route("/you", axum::routing::get(pages::you_page))
        .route("/test", axum::routing::get(pages::test_page))
        .route("/gallery", axum::routing::get(pages::gallery))
        .route(
            "/gallery/dataset-round-two",
            axum::routing::get(pages::dataset_round_two),
        )
        .route(
            "/gallery/dataset-round-two/og-image",
            axum::routing::get(pages::dataset_og_image),
        )
        .route(
            "/gallery/dataset-round-two/eliminate",
            axum::routing::post(pages::dataset_eliminate),
        )
        .route("/video-gallery", axum::routing::get(pages::videos_gallery))
        .route("/feed", axum::routing::get(pages::feed_page))
        .route("/help", axum::routing::get(pages::help))
        .route("/mobile", axum::routing::get(pages::mobile))
        .route("/dca", axum::routing::get(pages::dca_dashboard))
        .route("/dca-bot-code", axum::routing::get(pages::dca_bot_code))
        .route("/login", axum::routing::get(pages::login_page))
        .route("/seal", axum::routing::get(pages::seal_bridge_page))
        .route("/ankycoin", axum::routing::get(pages::ankycoin_page))
        .route("/leaderboard", axum::routing::get(pages::leaderboard))
        .route("/pitch", axum::routing::get(pages::pitch))
        .route("/station", axum::routing::get(pages::station))
        .route("/api/station/steps", axum::routing::get(station_steps))
        .route(
            "/api/station/step/{name}",
            axum::routing::get(station_step_content),
        )
        .route(
            "/api/station/run",
            axum::routing::post(station_run_pipeline),
        )
        .route(
            "/api/station/run/{step}",
            axum::routing::post(station_run_step),
        )
        .route("/api/station/tts", axum::routing::post(station_tts))
        .route(
            "/api/station/asset/{*rel}",
            axum::routing::get(station_asset),
        )
        .route(
            "/api/station/review/comments",
            axum::routing::get(station_review_comments_get)
                .post(station_review_comment_post),
        )
        .route(
            "/api/station/review/comments/{episode}/{id}",
            axum::routing::delete(station_review_comment_delete),
        )
        .route(
            "/api/station/review/evolve",
            axum::routing::post(station_review_evolve),
        )
        .route(
            "/api/station/review/apply",
            axum::routing::post(station_review_apply),
        )
        .route(
            "/api/station/review/regenerate",
            axum::routing::post(station_review_regenerate),
        )
        .route("/generate", axum::routing::get(pages::generate_page))
        .route(
            "/create-videos",
            axum::routing::get(pages::create_videos_page),
        )
        .route(
            "/generate/video",
            axum::routing::get(pages::video_dashboard),
        )
        .route(
            "/video/pipeline",
            axum::routing::get(pages::video_pipeline_page),
        )
        .route(
            "/video-dashboard",
            axum::routing::get(pages::media_dashboard),
        )
        .route("/sleeping", axum::routing::get(pages::sleeping))
        .route("/feedback", axum::routing::get(pages::feedback))
        .route("/changelog", axum::routing::get(pages::changelog))
        .route("/encoder", axum::routing::get(pages::encoder))
        .route("/api/encoder/data", axum::routing::get(pages::encoder_data))
        .route("/easter", axum::routing::get(pages::easter_gallery))
        // Programming classes
        .route("/classes", axum::routing::get(pages::classes_index))
        .route("/classes/{number}", axum::routing::get(pages::class_page))
        // Simulations — 8-slot inference dashboard
        .route(
            "/simulations",
            axum::routing::get(simulations::simulations_page),
        )
        .route(
            "/api/simulations/slots",
            axum::routing::get(simulations::slots_status),
        )
        .route(
            "/api/simulations/slots/stream",
            axum::routing::get(simulations::slots_stream),
        )
        .route(
            "/api/simulations/slots/demo",
            axum::routing::post(simulations::slots_demo),
        )
        .route("/llm", axum::routing::get(pages::llm))
        .route("/pitch-deck", axum::routing::get(pages::pitch_deck))
        .route("/pitch-deck.pdf", axum::routing::get(pages::pitch_deck_pdf))
        .route(
            "/api/v1/llm/training-status",
            axum::routing::post(api::llm_training_status),
        )
        // Programming classes API
        .route(
            "/api/v1/classes/generate",
            axum::routing::post(api::generate_class),
        )
        .route("/anky/{id}", axum::routing::get(pages::anky_detail))
        // Public story deep link page (no auth)
        .route(
            "/story/{story_id}",
            axum::routing::get(voices::story_deep_link_page),
        )
        // Prompt pages
        .route("/api/og/write", axum::routing::get(api::og_write_svg))
        .route("/prompt", axum::routing::get(prompt::prompt_new_page))
        .route(
            "/prompt/create",
            axum::routing::get(prompt::create_prompt_page),
        )
        .route("/prompt/{id}", axum::routing::get(prompt::prompt_page))
        // Prompt API
        .route(
            "/api/v1/prompt/{id}",
            axum::routing::get(prompt::get_prompt_api),
        )
        .route(
            "/api/v1/prompt/{id}/write",
            axum::routing::post(prompt::submit_prompt_writing),
        )
        .route(
            "/api/v1/prompts",
            axum::routing::get(prompt::list_prompts_api),
        )
        .route(
            "/api/v1/prompts/random",
            axum::routing::get(prompt::random_prompt_api),
        )
        // Settings
        .route("/settings", axum::routing::get(settings::settings_page))
        .route(
            "/api/settings",
            axum::routing::post(settings::save_settings),
        )
        .route(
            "/api/claim-username",
            axum::routing::post(settings::claim_username),
        )
        // Auth
        .route("/auth/x/login", axum::routing::get(auth::login))
        .route("/auth/x/callback", axum::routing::get(auth::callback))
        .route("/auth/x/logout", axum::routing::get(auth::logout))
        .route("/auth/logout", axum::routing::post(auth::logout_json))
        // Farcaster MiniApp auth
        .route(
            "/auth/farcaster/verify",
            axum::routing::post(auth::farcaster_verify),
        )
        // Solana wallet auth (Phantom etc.)
        .route(
            "/auth/solana/verify",
            axum::routing::post(auth::solana_verify),
        )
        // Anky protocol relay (encrypt → Irys → Solana)
        .route("/api/v1/relay", axum::routing::post(relay::relay_session))
        // Writing
        .route("/write", axum::routing::post(writing::process_writing))
        .route("/writings", axum::routing::get(writing::get_writings))
        .route("/writing/{id}", axum::routing::get(writing::view_writing))
        .route(
            "/api/writing/{sessionId}/status",
            axum::routing::get(writing::get_writing_status_web),
        )
        // Collection
        .route(
            "/collection/create",
            axum::routing::post(collection::create_collection),
        )
        .route(
            "/collection/{id}",
            axum::routing::get(collection::get_collection),
        )
        // Payment
        .route(
            "/payment/verify",
            axum::routing::post(payment::verify_payment),
        )
        // Notifications
        .route("/notify/signup", axum::routing::post(notification::signup))
        // API
        .route("/api/ankys", axum::routing::get(api::list_ankys))
        .route("/api/v1/ankys", axum::routing::get(api::list_ankys))
        .route("/api/generate", axum::routing::post(api::generate_anky))
        .route("/api/v1/anky/{id}", axum::routing::get(api::get_anky))
        .route(
            "/api/v1/mind/status",
            axum::routing::get(api::get_mind_status),
        )
        .route(
            "/api/v1/anky/{id}/metadata",
            axum::routing::get(swift::anky_metadata),
        )
        .route(
            "/api/stream-reflection/{id}",
            axum::routing::get(api::stream_reflection),
        )
        .route("/api/warm-context", axum::routing::post(api::warm_context))
        .route("/api/me", axum::routing::get(api::web_me))
        .route(
            "/api/anky/{id}/birth",
            axum::routing::get(api::anky_birth_status),
        )
        .route("/api/my-ankys", axum::routing::get(api::web_my_ankys))
        .route(
            "/api/chat-history",
            axum::routing::get(api::web_chat_history),
        )
        .route(
            "/api/anky-card/{id}",
            axum::routing::get(api::anky_reflection_card_image),
        )
        .route("/api/checkpoint", axum::routing::post(api::save_checkpoint))
        .route(
            "/api/session/paused",
            axum::routing::get(api::get_paused_writing_session),
        )
        .route(
            "/api/session/pause",
            axum::routing::post(api::pause_writing_session),
        )
        .route(
            "/api/session/resume",
            axum::routing::post(api::resume_writing_session),
        )
        .route(
            "/api/session/discard",
            axum::routing::post(api::discard_paused_writing_session),
        )
        .route(
            "/api/prefetch-memory",
            axum::routing::post(api::prefetch_memory),
        )
        .route("/api/cost-estimate", axum::routing::get(api::cost_estimate))
        .route("/api/treasury", axum::routing::get(api::treasury_address))
        .route("/api/mirror", axum::routing::get(api::mirror))
        .route(
            "/api/mirror/gallery",
            axum::routing::get(api::mirror_gallery),
        )
        .route("/api/mirror/chat", axum::routing::post(api::mirror_chat))
        .route(
            "/api/mirror/solana-mint",
            axum::routing::post(api::solana_mint_mirror),
        )
        .route(
            "/api/mirror/raw-mint",
            axum::routing::post(api::raw_mint_mirror),
        )
        .route("/api/mirror/supply", axum::routing::get(api::mirror_supply))
        .route(
            "/api/mirror/collection-metadata",
            axum::routing::get(api::mirror_collection_metadata),
        )
        .route(
            "/api/mirror/metadata/{id}",
            axum::routing::get(api::mirror_metadata),
        )
        .route("/image.png", axum::routing::get(api::mirror_latest_image))
        .route("/splash.png", axum::routing::get(api::mirror_latest_image))
        .route(
            "/api/miniapp/notifications",
            axum::routing::post(api::save_notification_token),
        )
        .route(
            "/api/miniapp/prompt",
            axum::routing::get(api::get_farcaster_prompt),
        )
        .route(
            "/api/webhook",
            axum::routing::post(api::farcaster_miniapp_webhook),
        )
        // Miniapp onboarding
        .route(
            "/api/miniapp/onboarding",
            axum::routing::get(api::miniapp_onboarding_status),
        )
        .route(
            "/api/miniapp/onboard",
            axum::routing::post(api::miniapp_onboard),
        )
        .route(
            "/api/miniapp/images",
            axum::routing::get(api::miniapp_image_list),
        )
        .route(
            "/api/miniapp/stickers",
            axum::routing::get(api::miniapp_sticker_list),
        )
        // Altar
        .route("/api/altar", axum::routing::get(altar::get_altar))
        .route("/api/altar/burn", axum::routing::post(altar::verify_burn))
        .route(
            "/api/altar/checkout",
            axum::routing::post(altar::create_checkout),
        )
        .route(
            "/api/altar/stripe-success",
            axum::routing::get(altar::stripe_success),
        )
        .route(
            "/api/altar/payment-intent",
            axum::routing::post(altar::create_payment_intent),
        )
        .route(
            "/api/altar/apple-pay",
            axum::routing::post(altar::apple_pay_burn),
        )
        // QR Auth (seal from phone)
        .route(
            "/api/auth/qr",
            axum::routing::post(qr_auth::create_challenge),
        )
        .route(
            "/api/auth/qr/seal",
            axum::routing::post(qr_auth::seal_by_token),
        )
        .route(
            "/api/auth/qr/{id}",
            axum::routing::get(qr_auth::poll_challenge),
        )
        .route(
            "/api/auth/qr/{id}/seal",
            axum::routing::post(qr_auth::seal_challenge),
        )
        // Sealed sessions
        .route(
            "/api/sessions/seal",
            axum::routing::post(sealed::seal_session),
        )
        .route(
            "/api/sessions/seal-browser",
            axum::routing::post(sealed::seal_session_browser),
        )
        .route(
            "/api/verify/{session_hash}",
            axum::routing::get(sealed::verify_sealed_session),
        )
        .route(
            "/api/anky/public-key",
            axum::routing::get(sealed::get_enclave_public_key),
        )
        .route(
            "/api/anky/submit",
            axum::routing::post(writing::submit_anky_protocol),
        )
        .route(
            "/api/sealed-write",
            axum::routing::post(sealed::sealed_write),
        )
        .route("/api/feedback", axum::routing::post(api::submit_feedback))
        .route(
            "/api/v1/feedback",
            axum::routing::post(api::submit_feedback),
        )
        .route("/api/chat", axum::routing::post(api::chat_with_anky))
        .route("/api/chat-quick", axum::routing::post(api::chat_quick))
        .route(
            "/api/suggest-replies",
            axum::routing::post(api::suggest_replies),
        )
        .route("/api/retry-failed", axum::routing::post(api::retry_failed))
        .route(
            "/api/v1/generate/video-frame",
            axum::routing::post(api::generate_video_frame),
        )
        .route(
            "/api/v1/generate/video",
            axum::routing::post(api::generate_video),
        )
        .route(
            "/api/v1/create-videos/{id}",
            axum::routing::get(api::get_create_video_card),
        )
        .route(
            "/api/v1/create-videos/image",
            axum::routing::post(api::generate_create_video_image),
        )
        .route(
            "/api/v1/create-videos/video",
            axum::routing::post(api::generate_create_video_clip),
        )
        .route(
            "/api/v1/video/{id}",
            axum::routing::get(api::get_video_project),
        )
        .route(
            "/api/v1/video/{id}/resume",
            axum::routing::post(api::resume_video_project),
        )
        .route(
            "/api/v1/video/pipeline/config",
            axum::routing::get(api::get_video_pipeline_config),
        )
        .route(
            "/api/v1/video/pipeline/config",
            axum::routing::post(api::save_video_pipeline_config),
        )
        .route("/api/v1/purge-cache", axum::routing::post(api::purge_cache))
        .route("/og/video", axum::routing::get(api::og_video_image))
        .route("/og/dca", axum::routing::get(api::og_dca_image))
        .route("/api/v1/feed", axum::routing::get(api::get_feed))
        .route(
            "/api/v1/anky/{id}/like",
            axum::routing::post(api::toggle_like),
        )
        .route("/api/v1/story/test", axum::routing::post(api::story_test))
        // Admin
        .route(
            "/admin/story-tester",
            axum::routing::get(api::admin_story_tester),
        )
        .route(
            "/onboarding-lab",
            axum::routing::get(api::onboarding_lab_page),
        )
        // Flux Lab
        .route("/flux-lab", axum::routing::get(api::flux_lab_page))
        .route(
            "/api/v1/flux-lab/experiments",
            axum::routing::get(api::flux_lab_list_experiments),
        )
        .route(
            "/api/v1/flux-lab/experiments/{name}",
            axum::routing::get(api::flux_lab_get_experiment),
        )
        .route(
            "/api/v1/flux-lab/generate",
            axum::routing::post(api::flux_lab_generate),
        )
        // AnkyCoin image generator
        .route(
            "/api/v1/ankycoin/generate",
            axum::routing::post(api::ankycoin_generate_image),
        )
        .route(
            "/api/v1/ankycoin/latest",
            axum::routing::get(api::ankycoin_latest_image),
        )
        // Media Factory
        .route(
            "/media-factory",
            axum::routing::get(api::media_factory_page),
        )
        .route(
            "/api/v1/media-factory/list",
            axum::routing::get(api::media_factory_list),
        )
        .route(
            "/api/v1/media-factory/video/{request_id}",
            axum::routing::get(api::media_factory_video_poll),
        )
        .route(
            "/api/v1/check-prompt",
            axum::routing::post(api::check_prompt),
        )
        // Farcaster OG embed image
        .route("/api/v1/og-embed", axum::routing::get(api::og_embed_image))
        // Public stories feed (no auth required)
        .route(
            "/api/v1/stories",
            axum::routing::get(swift::list_all_stories),
        )
        .route("/api/v1/stories/{id}", axum::routing::get(swift::get_story))
        // Anky Voices — story recordings
        .route(
            "/api/v1/stories/{story_id}/recordings",
            axum::routing::get(voices::list_recordings).post(voices::create_recording),
        )
        .route(
            "/api/v1/stories/{story_id}/voice",
            axum::routing::get(voices::get_voice),
        )
        .route(
            "/api/v1/stories/{story_id}/recordings/{recording_id}/complete",
            axum::routing::post(voices::complete_listen),
        )
        // Agent registration (no auth required)
        .route(
            "/api/v1/register",
            axum::routing::post(extension_api::register),
        )
        // Chunked writing sessions (agent stream-of-consciousness)
        .route(
            "/api/v1/session/start",
            axum::routing::post(session::start_session),
        )
        .route(
            "/api/v1/session/chunk",
            axum::routing::post(session::send_chunk),
        )
        .route(
            "/api/v1/session/{id}/events",
            axum::routing::get(session::session_events),
        )
        .route(
            "/api/v1/session/{id}/result",
            axum::routing::get(session::session_result),
        )
        .route(
            "/api/v1/session/{id}",
            axum::routing::get(session::session_status),
        )
        // Anky Now
        .route("/api/v1/now", axum::routing::post(now::create_now))
        .route("/api/v1/now/{slug}", axum::routing::get(now::get_now))
        .route(
            "/api/v1/now/{slug}/join",
            axum::routing::post(now::join_now),
        )
        .route(
            "/api/v1/now/{slug}/start",
            axum::routing::post(now::start_now),
        )
        .route(
            "/api/v1/now/{slug}/heartbeat",
            axum::routing::post(now::heartbeat_now),
        )
        .route("/n/{slug}", axum::routing::get(now::now_page))
        // Skills (for agents)
        .route("/manifesto.md", axum::routing::get(manifesto_md))
        .route("/MANIFESTO.md", axum::routing::get(manifesto_md))
        .route("/PROMPT.md", axum::routing::get(prompt_md))
        .route("/SOUL.md", axum::routing::get(soul_md))
        .route("/terms-of-service.md", axum::routing::get(terms_md))
        .route("/privacy-policy.md", axum::routing::get(privacy_md))
        .route("/faq.md", axum::routing::get(faq_md))
        .route("/spec.md", axum::routing::get(spec_md))
        .route("/SPEC.md", axum::routing::get(spec_md))
        .route("/protocol", axum::routing::get(protocol_md))
        .route("/protocol.md", axum::routing::get(protocol_md))
        .route("/PROTOCOL.md", axum::routing::get(protocol_md))
        .route("/pitch.md", axum::routing::get(pitch_md))
        .route("/PITCH.md", axum::routing::get(pitch_md))
        .route("/poiesis.md", axum::routing::get(poiesis_md))
        .route("/POIESIS.md", axum::routing::get(poiesis_md))
        .route("/prompts/{id}", axum::routing::get(serve_prompt))
        .route("/skills", axum::routing::get(skills))
        .route("/skill.md", axum::routing::get(skill_md))
        .route("/skill", axum::routing::get(skills_redirect))
        .route("/skills.md", axum::routing::get(skills_redirect))
        .route("/agent-skills/anky", axum::routing::get(anky_skill_bundle))
        .route("/agent-skills/anky/", axum::routing::get(anky_skill_bundle))
        .route(
            "/agent-skills/anky/skill.md",
            axum::routing::get(anky_skill_bundle_entry_redirect),
        )
        .route(
            "/agent-skills/anky/skills.md",
            axum::routing::get(anky_skill_bundle_entry_redirect),
        )
        .route(
            "/agent-skills/anky/manifest.json",
            axum::routing::get(anky_skill_bundle_manifest),
        )
        // Live streaming — disabled (too slow, not worth it)
        // Routes kept in live.rs but not wired up
        .route("/api/ankys/today", axum::routing::get(live::todays_ankys))
        .route(
            "/api/live-status",
            axum::routing::get(live::live_status_sse),
        )
        // Interview
        .route("/interview", axum::routing::get(interview::interview_page))
        .route(
            "/ws/interview",
            axum::routing::get(interview::ws_interview_proxy),
        )
        .route(
            "/api/interview/start",
            axum::routing::post(interview::interview_start),
        )
        .route(
            "/api/interview/message",
            axum::routing::post(interview::interview_message),
        )
        .route(
            "/api/interview/end",
            axum::routing::post(interview::interview_end),
        )
        .route(
            "/api/interview/history/{user_id}",
            axum::routing::get(interview::interview_history),
        )
        .route(
            "/api/interview/user-context/{user_id}",
            axum::routing::get(interview::interview_user_context),
        )
        // Stream overlay
        .route("/stream/overlay", axum::routing::get(pages::stream_overlay))
        // Generations review + live dashboard
        .route(
            "/generations",
            axum::routing::get(generations::list_batches),
        )
        .route(
            "/generations/{id}",
            axum::routing::get(generations::review_batch),
        )
        .route(
            "/generations/{id}/status",
            axum::routing::post(generations::save_status),
        )
        .route(
            "/generations/{id}/dashboard",
            axum::routing::get(generations::generation_dashboard),
        )
        .route(
            "/generations/{id}/progress",
            axum::routing::get(generations::generation_progress),
        )
        .route(
            "/generations/{id}/tinder",
            axum::routing::get(generations::review_images),
        )
        .route(
            "/generations/{id}/review",
            axum::routing::post(generations::save_review),
        )
        // Training curation
        .route("/training", axum::routing::get(training::training_page))
        .route("/trainings", axum::routing::get(training::trainings_list))
        .route(
            "/trainings/general-instructions",
            axum::routing::get(training::general_instructions),
        )
        .route(
            "/trainings/{date}",
            axum::routing::get(training::training_run_detail),
        )
        .route(
            "/api/training/next",
            axum::routing::get(training::next_image),
        )
        .route("/api/training/vote", axum::routing::post(training::vote))
        .route(
            "/api/training/heartbeat",
            axum::routing::post(training::training_heartbeat),
        )
        .route(
            "/api/training/state",
            axum::routing::get(training::training_state),
        )
        .route(
            "/training/live",
            axum::routing::get(training::training_live),
        )
        .route(
            "/training/live/samples/{filename}",
            axum::routing::get(training::training_sample_image),
        )
        // Memory
        .route(
            "/api/memory/backfill",
            axum::routing::post(api::memory_backfill),
        )
        // Evolution dashboard (public)
        .route("/evolve", axum::routing::get(evolve::evolve_dashboard))
        // Dashboard
        .route("/dashboard", axum::routing::get(dashboard::dashboard))
        .route(
            "/dashboard/logs",
            axum::routing::get(dashboard::dashboard_logs),
        )
        .route(
            "/dashboard/summaries",
            axum::routing::get(dashboard::dashboard_summaries),
        )
        // Apple Universal Links (AASA)
        .route(
            "/.well-known/apple-app-site-association",
            axum::routing::get(apple_app_site_association),
        )
        // Farcaster MiniApp manifest
        .route(
            "/.well-known/farcaster.json",
            axum::routing::get(farcaster_manifest),
        )
        // Agent manifest (8004 registry / OASF)
        .route("/.well-known/agent", axum::routing::get(agent_manifest))
        // Service Worker (served from root for scope)
        .route("/sw.js", axum::routing::get(service_worker))
        // X Account Activity webhook (CRC + events)
        .route("/webhooks/x", axum::routing::get(webhook_x::webhook_crc))
        .route("/webhooks/x", axum::routing::post(webhook_x::webhook_post))
        // Farcaster (Neynar) webhook
        .route(
            "/webhooks/farcaster",
            axum::routing::post(webhook_farcaster::webhook_post),
        )
        // X Webhook live log viewer
        .route(
            "/webhooks/logs",
            axum::routing::get(webhook_x::webhook_logs_page),
        )
        .route(
            "/webhooks/logs/stream",
            axum::routing::get(webhook_x::webhook_logs_stream),
        )
        // Health
        .route("/health", axum::routing::get(health::health_check))
        .route("/api/health", axum::routing::get(health::health_check))
        // Swift / Mobile API
        .merge(swift_routes)
        // Extension API (authed)
        .merge(extension_routes)
        // Paid generate API (optional auth)
        .merge(generate_routes)
        // Studio upload (large body limit)
        .merge(studio_routes)
        // Media factory (large body limit for base64 images)
        .merge(media_factory_routes)
        // Static files
        .nest_service("/agent-skills", ServeDir::new("agent-skills"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service(
            "/data/images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/images")),
        )
        .nest_service(
            "/data/anky-images",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("data/anky-images")),
        )
        .nest_service("/flux", ServeDir::new("flux"))
        .nest_service("/data/writings", ServeDir::new("data/writings"))
        .nest_service("/videos", ServeDir::new("videos"))
        .nest_service("/data/videos", ServeDir::new("data/videos"))
        .nest_service("/gen-images", ServeDir::new("data/generations"))
        .nest_service(
            "/data/training-images",
            ServeDir::new("data/training-images"),
        )
        .nest_service("/data/training-runs", ServeDir::new("data/training_runs"))
        .nest_service("/data/mirrors", ServeDir::new("data/mirrors"))
        .nest_service("/data/classes", ServeDir::new("data/classes"))
        // Middleware layers (applied bottom-up)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(256 * 1024)) // 256KB body limit
        .layer(axum::middleware::from_fn(
            middleware::security_headers::security_headers,
        ))
        .layer(axum::middleware::from_fn(
            middleware::honeypot::honeypot_and_attack_detection,
        ))
        .layer(axum::middleware::from_fn(
            middleware::subdomain::pitch_subdomain,
        ))
        .with_state(state)
}
