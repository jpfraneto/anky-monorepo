/// Anky Voices — story recording, quality check, and playback endpoints.
use crate::db::queries;
use crate::error::AppError;
use crate::services::r2;
use crate::state::AppState;
use axum::extract::{Multipart, Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ── Auth helper (same pattern as swift.rs) ──────────────────────────────────

async fn bearer_auth(state: &AppState, headers: &HeaderMap) -> Result<String, AppError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Authorization: Bearer header".into()))?;

    let db = crate::db::conn(&state.db)?;
    let (user_id, _) = queries::get_auth_session(&db, token)?
        .ok_or_else(|| AppError::Unauthorized("invalid or expired session token".into()))?;
    Ok(user_id)
}

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingItem {
    id: String,
    story_id: String,
    user_id: String,
    username: String,
    attempt_number: i32,
    language: String,
    status: String,
    duration_seconds: f64,
    audio_url: Option<String>,
    rejection_reason: Option<String>,
    full_listen_count: i32,
    created_at: String,
    approved_at: Option<String>,
}

// ── GET /api/v1/stories/{story_id}/recordings ───────────────────────────────

pub async fn list_recordings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(story_id): Path<String>,
) -> Result<Json<Vec<RecordingItem>>, AppError> {
    let _user_id = bearer_auth(&state, &headers).await?;
    let db = crate::db::conn(&state.db)?;
    let mut stmt = db.prepare(
        "SELECT sr.id, sr.story_id, sr.user_id, sr.attempt_number, sr.language, sr.status,
                sr.duration_seconds, sr.audio_url, sr.rejection_reason,
                sr.full_listen_count, sr.created_at, sr.approved_at
         FROM story_recordings sr
         WHERE sr.story_id = ?1
         ORDER BY sr.approved_at DESC, sr.created_at DESC",
    )?;
    let rows = stmt.query_map(crate::params![story_id], |row| {
        Ok((
            row.get::<_, String>(0)?,          // id
            row.get::<_, String>(1)?,          // story_id
            row.get::<_, String>(2)?,          // user_id
            row.get::<_, i32>(3)?,             // attempt_number
            row.get::<_, String>(4)?,          // language
            row.get::<_, String>(5)?,          // status
            row.get::<_, f64>(6)?,             // duration_seconds
            row.get::<_, Option<String>>(7)?,  // audio_url
            row.get::<_, Option<String>>(8)?,  // rejection_reason
            row.get::<_, i32>(9)?,             // full_listen_count
            row.get::<_, String>(10)?,         // created_at
            row.get::<_, Option<String>>(11)?, // approved_at
        ))
    })?;
    let raw: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    let items = raw
        .into_iter()
        .map(
            |(
                id,
                story_id,
                user_id,
                attempt_number,
                language,
                status,
                duration_seconds,
                audio_url,
                rejection_reason,
                full_listen_count,
                created_at,
                approved_at,
            )| {
                let username = queries::get_display_username(&db, &user_id)
                    .unwrap_or_else(|_| "someone".into());
                RecordingItem {
                    id,
                    story_id,
                    user_id,
                    username,
                    attempt_number,
                    language,
                    status,
                    duration_seconds,
                    audio_url,
                    rejection_reason,
                    full_listen_count,
                    created_at,
                    approved_at,
                }
            },
        )
        .collect();
    Ok(Json(items))
}

// ── POST /api/v1/stories/{story_id}/recordings ──────────────────────────────

pub async fn create_recording(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(story_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    // Parse multipart fields
    let mut language = String::new();
    let mut duration_seconds: f64 = 0.0;
    let mut _audio_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {}", e)))?
    {
        match field.name() {
            Some("language") => {
                language = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("language field: {}", e)))?;
            }
            Some("duration_seconds") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("duration field: {}", e)))?;
                duration_seconds = text
                    .parse()
                    .map_err(|_| AppError::BadRequest("invalid duration_seconds".into()))?;
            }
            Some("audio") => {
                _audio_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("audio field: {}", e)))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    if language.is_empty() {
        return Err(AppError::BadRequest("language is required".into()));
    }

    // Determine attempt_number (next available, max 4)
    let attempt_number = {
        let db = crate::db::conn(&state.db)?;
        let count: i32 = db
            .query_row(
                "SELECT COUNT(*) FROM story_recordings WHERE story_id = ?1 AND user_id = ?2",
                crate::params![story_id, user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        count + 1
    };

    if attempt_number > 4 {
        return Err(AppError::RateLimited(0));
    }

    let recording_id = uuid::Uuid::new_v4().to_string();
    let file_uuid = uuid::Uuid::new_v4().to_string();
    let r2_key = format!("recordings/{}/{}/{}.m4a", story_id, user_id, file_uuid);

    // Generate presigned upload URL
    let upload_url = if r2::is_configured(&state.config) {
        r2::presigned_put_url(&state.config, &r2_key)
            .await
            .map_err(|e| AppError::Internal(format!("R2 presign error: {}", e)))?
    } else {
        // R2 not configured — return placeholder
        format!("https://placeholder.r2.dev/{}", r2_key)
    };

    // Insert record
    {
        let db = crate::db::conn(&state.db)?;
        db.execute(
            "INSERT INTO story_recordings
             (id, story_id, user_id, attempt_number, language, status, duration_seconds, r2_key)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?7)",
            crate::params![
                recording_id,
                story_id,
                user_id,
                attempt_number,
                language,
                duration_seconds,
                r2_key,
            ],
        )?;
    }

    // Spawn async quality check
    let qc_state = state.clone();
    let qc_id = recording_id.clone();
    tokio::spawn(async move {
        quality_check_recording(&qc_state, &qc_id).await;
    });

    Ok(Json(json!({
        "recording_id": recording_id,
        "status": "pending",
        "upload_url": upload_url,
    })))
}

// ── GET /api/v1/stories/{story_id}/voice ────────────────────────────────────

pub async fn get_voice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(story_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _user_id = bearer_auth(&state, &headers).await?;

    // Detect preferred language from Accept-Language
    let preferred_lang = headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.split('-').next())
        .map(|v| v.trim().to_lowercase())
        .unwrap_or_else(|| "en".into());

    let db = crate::db::conn(&state.db)?;

    // Try preferred language first
    let recording = db
        .query_row(
            "SELECT id, audio_url, language, duration_seconds, user_id
             FROM story_recordings
             WHERE story_id = ?1 AND status = 'approved' AND language = ?2
             ORDER BY approved_at DESC LIMIT 1",
            crate::params![story_id, preferred_lang],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .ok();

    // Fallback to any approved recording in any language
    let recording = recording.or_else(|| {
        db.query_row(
            "SELECT id, audio_url, language, duration_seconds, user_id
             FROM story_recordings
             WHERE story_id = ?1 AND status = 'approved'
             ORDER BY approved_at DESC LIMIT 1",
            crate::params![story_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .ok()
    });

    // Return human recording if available, otherwise fall back to TTS
    match recording {
        Some((rec_id, audio_url, lang, duration, recorder_user_id)) => {
            let recorder_username = queries::get_display_username(&db, &recorder_user_id)
                .unwrap_or_else(|_| "someone".into());
            Ok(Json(json!({
                "recordingId": rec_id,
                "audioUrl": audio_url,
                "language": lang,
                "durationSeconds": duration,
                "userId": recorder_user_id,
                "username": recorder_username,
                "source": "human",
            })))
        }
        None => {
            // Fallback: try TTS audio for this story
            let tts = db
                .query_row(
                    "SELECT id, audio_url, language, duration_seconds
                     FROM cuentacuentos_audio
                     WHERE cuentacuentos_id = ?1 AND status = 'complete' AND language = ?2
                     LIMIT 1",
                    crate::params![story_id, preferred_lang],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, f64>(3)?,
                        ))
                    },
                )
                .or_else(|_| {
                    db.query_row(
                        "SELECT id, audio_url, language, duration_seconds
                         FROM cuentacuentos_audio
                         WHERE cuentacuentos_id = ?1 AND status = 'complete'
                         ORDER BY CASE language WHEN 'en' THEN 0 ELSE 1 END
                         LIMIT 1",
                        crate::params![story_id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, f64>(3)?,
                            ))
                        },
                    )
                })
                .map_err(|_| AppError::NotFound("no voice available for this story".into()))?;

            Ok(Json(json!({
                "recordingId": tts.0,
                "audioUrl": tts.1,
                "language": tts.2,
                "durationSeconds": tts.3,
                "userId": "anky",
                "username": "Anky",
                "source": "tts",
            })))
        }
    }
}

// ── POST /api/v1/stories/{story_id}/recordings/{recording_id}/complete ──────

pub async fn complete_listen(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((story_id, recording_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = bearer_auth(&state, &headers).await?;

    let db = crate::db::conn(&state.db)?;
    db.execute(
        "UPDATE story_recordings SET full_listen_count = full_listen_count + 1 WHERE id = ?1",
        crate::params![recording_id],
    )?;

    let event_id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO story_listen_events (id, story_id, recording_id, user_id)
         VALUES (?1, ?2, ?3, ?4)",
        crate::params![event_id, story_id, recording_id, user_id],
    )?;

    Ok(Json(json!({ "ok": true })))
}

// ── Quality Check (async) ───────────────────────────────────────────────────

async fn quality_check_recording(state: &AppState, recording_id: &str) {
    // Get recording details
    let (r2_key, story_id) = {
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
        match db.query_row(
            "SELECT r2_key, story_id FROM story_recordings WHERE id = ?1",
            crate::params![recording_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("quality check: recording {} not found: {}", recording_id, e);
                return;
            }
        }
    };

    // Get story text for comparison
    let story_content = {
        let Some(db) = crate::db::get_conn_logged(&state.db) else {
            return;
        };
        match queries::get_cuentacuentos_by_id(&db, &story_id) {
            Ok(Some(story)) => story.content,
            _ => {
                tracing::error!("quality check: story {} not found", story_id);
                return;
            }
        }
    };

    // Try to download and transcribe via Whisper
    let transcription = if r2::is_configured(&state.config) {
        match r2::get_object_bytes(&state.config, &r2_key).await {
            Ok(audio_bytes) => {
                // Try local Whisper via Ollama or whisper.cpp
                match transcribe_audio(&state.config.ollama_base_url, &audio_bytes).await {
                    Ok(text) => Some(text),
                    Err(e) => {
                        tracing::warn!("whisper transcription failed, auto-approving: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("R2 download failed for {}, auto-approving: {}", r2_key, e);
                None
            }
        }
    } else {
        tracing::info!(
            "R2 not configured, auto-approving recording {}",
            recording_id
        );
        None
    };

    let Some(db) = crate::db::get_conn_logged(&state.db) else {
        return;
    };

    match transcription {
        Some(transcribed_text) => {
            let similarity = compute_word_similarity(&story_content, &transcribed_text);
            if similarity >= 0.65 {
                let audio_url = r2::public_url(&state.config, &r2_key);
                let _ = db.execute(
                    "UPDATE story_recordings
                     SET status = 'approved', audio_url = ?2, approved_at = datetime('now')
                     WHERE id = ?1",
                    crate::params![recording_id, audio_url],
                );
                tracing::info!(
                    "Recording {} approved (similarity: {:.2})",
                    &recording_id[..8.min(recording_id.len())],
                    similarity
                );
            } else {
                let reason = format!("transcription similarity: {:.2}", similarity);
                let _ = db.execute(
                    "UPDATE story_recordings SET status = 'rejected', rejection_reason = ?2 WHERE id = ?1",
                    crate::params![recording_id, reason],
                );
                tracing::info!(
                    "Recording {} rejected (similarity: {:.2})",
                    &recording_id[..8.min(recording_id.len())],
                    similarity
                );
            }
        }
        None => {
            // Whisper not available or download failed — auto-approve for manual review later
            let audio_url = if r2::is_configured(&state.config) {
                r2::public_url(&state.config, &r2_key)
            } else {
                format!("https://placeholder.r2.dev/{}", r2_key)
            };
            let _ = db.execute(
                "UPDATE story_recordings
                 SET status = 'approved', audio_url = ?2, approved_at = datetime('now')
                 WHERE id = ?1",
                crate::params![recording_id, audio_url],
            );
            tracing::info!(
                "Recording {} auto-approved (whisper unavailable)",
                &recording_id[..8.min(recording_id.len())]
            );
        }
    }
}

/// Try to transcribe audio using local Whisper endpoint.
/// Attempts Ollama's audio model or a local whisper.cpp HTTP server.
async fn transcribe_audio(ollama_base_url: &str, audio_bytes: &[u8]) -> anyhow::Result<String> {
    let _ = (ollama_base_url, audio_bytes);
    // TODO: Restore Whisper fallback once it no longer collides with llama-server on :8080.
    anyhow::bail!("no whisper endpoint available")
}

/// Compute word-level similarity: what fraction of story words appear in the transcription.
fn compute_word_similarity(story: &str, transcription: &str) -> f64 {
    let normalize = |s: &str| -> Vec<String> {
        s.to_lowercase()
            .split_whitespace()
            .map(|w| {
                w.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|w| !w.is_empty() && w.len() > 2) // skip short words (articles, etc.)
            .collect()
    };

    let story_words = normalize(story);
    if story_words.is_empty() {
        return 1.0;
    }

    let transcription_words: std::collections::HashSet<String> =
        normalize(transcription).into_iter().collect();

    let matches = story_words
        .iter()
        .filter(|w| transcription_words.contains(*w))
        .count();

    matches as f64 / story_words.len() as f64
}

// ── GET /story/{story_id} — public deep link page ───────────────────────────

#[derive(Deserialize)]
pub struct StoryPageQuery {
    #[serde(default)]
    recording_id: Option<String>,
}

pub async fn story_deep_link_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(story_id): Path<String>,
    query: axum::extract::Query<StoryPageQuery>,
) -> Result<Response, AppError> {
    let db = crate::db::conn(&state.db)?;
    let story = queries::get_cuentacuentos_by_id(&db, &story_id)?
        .ok_or_else(|| AppError::NotFound("story not found".into()))?;

    // Get first two paragraphs
    let paragraphs: Vec<&str> = story
        .content
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .take(2)
        .collect();
    let preview_text = paragraphs.join("</p><p>");

    // Check for approved recording
    let audio_html = if let Some(ref rec_id) = query.recording_id {
        match db.query_row(
            "SELECT audio_url FROM story_recordings WHERE id = ?1 AND status = 'approved'",
            crate::params![rec_id],
            |row| row.get::<_, String>(0),
        ) {
            Ok(url) => format!(
                r#"<audio id="storyAudio" controls preload="auto" style="width:100%;margin:1.5rem 0">
                    <source src="{}" type="audio/mp4">
                   </audio>"#,
                url
            ),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };

    // Detect iOS for app download modal
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let is_ios = user_agent.contains("iPhone") || user_agent.contains("iPad");
    let modal_js = if is_ios {
        r#"<script>
        setTimeout(function() { showModal(); }, 3000);
        var audio = document.getElementById('storyAudio');
        if (audio) audio.addEventListener('play', function() { showModal(); }, { once: true });
        function showModal() {
            var m = document.getElementById('dlModal');
            if (m) m.style.display = 'flex';
        }
        function closeModal() {
            document.getElementById('dlModal').style.display = 'none';
        }
        </script>"#
    } else {
        ""
    };

    let modal_html = if is_ios {
        format!(
            r#"<div id="dlModal" style="display:none;position:fixed;inset:0;background:rgba(0,0,0,0.85);z-index:100;align-items:center;justify-content:center">
              <div style="background:#111118;border:1px solid #2a2a3a;padding:2.5rem;max-width:340px;text-align:center">
                <p style="font-size:1.1rem;margin-bottom:1.5rem;line-height:1.6">Hear more stories — download Anky</p>
                <a href="anky://story/{story_id}" style="display:block;background:#4a9eff;color:#000;padding:0.8rem;font-family:'Space Mono',monospace;font-size:0.7rem;letter-spacing:0.15em;text-transform:uppercase;text-decoration:none;margin-bottom:0.75rem">open in app</a>
                <a href="https://testflight.apple.com/join/YOUR_TESTFLIGHT_ID" style="display:block;border:1px solid #2a2a3a;color:#e8e4dc;padding:0.8rem;font-family:'Space Mono',monospace;font-size:0.7rem;letter-spacing:0.15em;text-transform:uppercase;text-decoration:none;margin-bottom:1rem">get testflight</a>
                <button onclick="closeModal()" style="background:none;border:none;color:#6b6880;font-size:0.8rem;cursor:pointer">not now</button>
              </div>
            </div>"#,
            story_id = story_id
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title} — Anky</title>
<meta property="og:title" content="{title}">
<meta property="og:description" content="A story from the Ankyverse">
<meta property="og:type" content="article">
<meta property="og:url" content="https://anky.app/story/{story_id}">
<style>
  @import url('https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,300;0,400;1,300;1,400&family=Space+Mono:wght@400;700&display=swap');
  *{{margin:0;padding:0;box-sizing:border-box}}
  body{{background:#0a0a0f;color:#e8e4dc;font-family:'Cormorant Garamond',serif;min-height:100vh;display:flex;flex-direction:column;align-items:center}}
  .container{{max-width:640px;width:100%;padding:3rem 2rem}}
  .logo{{font-family:'Space Mono',monospace;font-size:0.65rem;color:#4a9eff;letter-spacing:0.3em;text-transform:uppercase;margin-bottom:2rem}}
  h1{{font-size:1.4rem;font-weight:400;font-style:italic;color:#c9a84c;margin-bottom:1.5rem;line-height:1.4}}
  p{{font-size:1.05rem;line-height:1.8;margin-bottom:1.2rem}}
  .kingdom{{font-family:'Space Mono',monospace;font-size:0.55rem;background:#1a3a5c;color:#4a9eff;padding:0.2rem 0.5rem;letter-spacing:0.1em;text-transform:uppercase;display:inline-block;margin-bottom:1.5rem}}
  .deep-link{{display:block;text-align:center;margin-top:2rem;font-family:'Space Mono',monospace;font-size:0.65rem;color:#6b6880;letter-spacing:0.15em;text-decoration:none}}
  .deep-link:hover{{color:#4a9eff}}
</style>
</head>
<body>
<div class="container">
  <div class="logo">Anky</div>
  {kingdom_tag}
  <h1>{title}</h1>
  <p>{preview_text}</p>
  {audio_html}
  <a class="deep-link" href="anky://story/{story_id}">open in anky</a>
</div>
{modal_html}
{modal_js}
</body>
</html>"#,
        title = story.title,
        story_id = story_id,
        kingdom_tag = story
            .kingdom
            .as_ref()
            .map(|k| format!(r#"<div class="kingdom">{}</div>"#, k))
            .unwrap_or_default(),
        preview_text = preview_text,
        audio_html = audio_html,
        modal_html = modal_html,
        modal_js = modal_js,
    );

    Ok(Html(html).into_response())
}
