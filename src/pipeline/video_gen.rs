use crate::memory::recall::MemoryContext;
use crate::state::AppState;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

const TARGET_DURATION: u32 = 88;
const MIN_SCENE_DURATION: u32 = 1;
const MAX_SCENE_DURATION: u32 = 15;
/// Grok charges $0.05/second of generated video
const GROK_COST_PER_SECOND: f64 = 0.05;

pub const VIDEO_SCRIPT_PROMPT_KEY: &str = "video.script_system_prompt";
pub const VIDEO_IMAGE_PROMPT_TEMPLATE_KEY: &str = "video.scene_image_prompt_template";
pub const VIDEO_SOUND_PROMPT_TEMPLATE_KEY: &str = "video.scene_sound_prompt_template";
pub const DEFAULT_IMAGE_PROMPT_TEMPLATE: &str = "{image_prompt}";
pub const DEFAULT_SOUND_PROMPT_TEMPLATE: &str = "{video_prompt}. Sound: {sound_direction}";

/// Build a story-aware image prompt that threads the visual world and arc into every scene.
fn build_scene_image_prompt(
    template: &str,
    scene: &VideoScene,
    spine: Option<&StorySpine>,
    scene_count: usize,
) -> String {
    let base_prompt = render_image_prompt(template, &scene.image_prompt);

    let Some(spine) = spine else {
        return base_prompt;
    };

    // Thread the story spine into every image prompt so Gemini knows the world
    let mut parts = Vec::new();

    if !spine.visual_world.is_empty() {
        parts.push(format!(
            "WORLD: This scene takes place in: {}",
            spine.visual_world
        ));
    }

    if !spine.arc.is_empty() {
        parts.push(format!(
            "STORY ARC: {} (this is scene {} of {})",
            spine.arc,
            scene.index + 1,
            scene_count
        ));
    }

    if !spine.color_arc.is_empty() {
        parts.push(format!("COLOR PROGRESSION: {}", spine.color_arc));
    }

    if !spine.message.is_empty() {
        parts.push(format!("THE MESSAGE: {}", spine.message));
    }

    parts.push(format!("SCENE: {}", base_prompt));

    if !scene.narrative_role.is_empty() {
        parts.push(format!("NARRATIVE ROLE: {}", scene.narrative_role));
    }

    parts.join("\n\n")
}

/// Build a story-aware video prompt that gives Grok the arc context.
fn build_scene_video_prompt(
    sound_template: &str,
    scene: &VideoScene,
    spine: Option<&StorySpine>,
    scene_count: usize,
) -> String {
    let base = render_sound_prompt(sound_template, &scene.video_prompt, &scene.sound_direction);

    let Some(spine) = spine else {
        return base;
    };

    let mut parts = Vec::new();

    if !spine.visual_world.is_empty() {
        parts.push(format!("Setting: {}", spine.visual_world));
    }

    parts.push(format!(
        "Scene {} of {} in story arc: {}",
        scene.index + 1,
        scene_count,
        if spine.arc.is_empty() {
            "progressing"
        } else {
            &spine.arc
        }
    ));

    parts.push(base);

    parts.join(". ")
}

// ==================== Video Context ====================

pub struct VideoContext {
    pub writing_text: String,
    pub anky_title: Option<String>,
    pub anky_reflection: Option<String>,
    pub flow_score: Option<f64>,
    pub duration_seconds: f64,
    pub word_count: i32,
    pub memory: Option<FormattedVideoMemory>,
}

pub struct FormattedVideoMemory {
    pub psychological_profile: Option<String>,
    pub avoidances: Vec<String>,
    pub breakthroughs: Vec<String>,
    pub patterns: Vec<String>,
    pub themes: Vec<String>,
    pub emotions: Vec<String>,
    pub similar_moments: Vec<String>,
    pub session_count: i32,
}

/// Convert a MemoryContext into the video-specific format, splitting patterns by category.
pub fn memory_context_to_video(mem: &MemoryContext) -> FormattedVideoMemory {
    let mut avoidances = Vec::new();
    let mut breakthroughs = Vec::new();
    let mut patterns = Vec::new();
    let mut themes = Vec::new();
    let mut emotions = Vec::new();

    for p in &mem.patterns {
        match p.category.as_str() {
            "avoidance" => avoidances.push(p.content.clone()),
            "breakthrough" => breakthroughs.push(p.content.clone()),
            "pattern" => patterns.push(p.content.clone()),
            "theme" => themes.push(p.content.clone()),
            "emotion" => emotions.push(p.content.clone()),
            _ => patterns.push(p.content.clone()),
        }
    }

    let similar_moments = mem
        .similar_moments
        .iter()
        .map(|m| {
            let truncated: String = m.content.chars().take(150).collect();
            if m.content.chars().count() > 150 {
                format!("{}...", truncated)
            } else {
                truncated
            }
        })
        .collect();

    FormattedVideoMemory {
        psychological_profile: mem.profile.clone(),
        avoidances,
        breakthroughs,
        patterns,
        themes,
        emotions,
        similar_moments,
        session_count: mem.session_count,
    }
}

fn flow_label(score: f64) -> &'static str {
    if score >= 0.8 {
        "deep flow"
    } else if score >= 0.6 {
        "moderate flow"
    } else if score >= 0.4 {
        "scattered"
    } else {
        "fragmented"
    }
}

fn build_user_message(ctx: &VideoContext) -> String {
    let mut parts = Vec::new();

    // Flow state info
    if let Some(score) = ctx.flow_score {
        parts.push(format!(
            "FLOW STATE: score {:.2} ({}) | duration {:.0}s | {} words",
            score,
            flow_label(score),
            ctx.duration_seconds,
            ctx.word_count
        ));
    } else {
        parts.push(format!(
            "SESSION: duration {:.0}s | {} words",
            ctx.duration_seconds, ctx.word_count
        ));
    }

    // Anky title and reflection
    if let Some(ref title) = ctx.anky_title {
        parts.push(format!("ANKY TITLE: {}", title));
    }
    if let Some(ref reflection) = ctx.anky_reflection {
        let snippet: String = reflection.chars().take(600).collect();
        let snippet = if reflection.chars().count() > 600 {
            format!("{}...", snippet)
        } else {
            snippet
        };
        parts.push(format!("ANKY REFLECTION (excerpt):\n{}", snippet));
    }

    // Memory context
    if let Some(ref mem) = ctx.memory {
        parts.push(format!(
            "\n=== MEMORY CONTEXT ({} previous sessions) ===",
            mem.session_count
        ));

        if let Some(ref profile) = mem.psychological_profile {
            if !profile.is_empty() {
                parts.push(format!("PSYCHOLOGICAL PROFILE:\n{}", profile));
            }
        }

        if !mem.avoidances.is_empty() {
            parts.push(format!(
                "AVOIDANCES (the richest visual material — what they circle around but never face):\n- {}",
                mem.avoidances.join("\n- ")
            ));
        }

        if !mem.breakthroughs.is_empty() {
            parts.push(format!(
                "BREAKTHROUGHS (use for climactic scenes — moments of genuine rupture):\n- {}",
                mem.breakthroughs.join("\n- ")
            ));
        }

        if !mem.patterns.is_empty() {
            parts.push(format!(
                "PATTERNS (recurring visual motifs — things that keep appearing):\n- {}",
                mem.patterns.join("\n- ")
            ));
        }

        if !mem.themes.is_empty() || !mem.emotions.is_empty() {
            let mut palette = Vec::new();
            for t in &mem.themes {
                palette.push(format!("theme: {}", t));
            }
            for e in &mem.emotions {
                palette.push(format!("emotion: {}", e));
            }
            parts.push(format!(
                "THEMES & EMOTIONS (drives color palette and atmosphere):\n- {}",
                palette.join("\n- ")
            ));
        }

        if !mem.similar_moments.is_empty() {
            parts.push(format!(
                "ECHOES (similar past writing moments — use for visual callbacks/rhymes):\n- \"{}\"",
                mem.similar_moments.join("\"\n- \"")
            ));
        }

        parts.push("=== END MEMORY CONTEXT ===\n".to_string());
    }

    // Raw writing always last, always complete
    parts.push(format!(
        "THE WRITING SESSION:\n---\n{}\n---",
        ctx.writing_text
    ));

    parts.join("\n\n")
}

pub fn default_script_system_prompt() -> &'static str {
    PSYCHOANALYTIC_SYSTEM
}

fn render_image_prompt(template: &str, image_prompt: &str) -> String {
    template.replace("{image_prompt}", image_prompt)
}

fn render_sound_prompt(template: &str, video_prompt: &str, sound_direction: &str) -> String {
    if sound_direction.trim().is_empty() {
        return video_prompt.to_string();
    }
    template
        .replace("{video_prompt}", video_prompt)
        .replace("{sound_direction}", sound_direction)
}

// ==================== Two-Phase Script System ====================

const PSYCHOANALYTIC_SYSTEM: &str = r#"You are a film director and psychoanalyst creating an 88-second SHORT FILM — not a mood reel, not a visual poem, not a meditation — a STORY with a beginning, middle, and end.

The writing you receive is raw unconscious material. Your job is to find the story inside it and put it on screen. A story has: a character who wants something, an obstacle that gets in the way, a moment of crisis, and a transformation. That is what this video must deliver.

Anky is the protagonist and guide — blue-skinned, purple-haired with golden spiral accents, golden eyes. Anky ACTS. Anky MOVES. Anky STRUGGLES, DISCOVERS, BREAKS THROUGH. The only time Anky is still is the final 3-5 second end card. In every other scene, Anky is doing something physical and purposeful: climbing, running, reaching, falling, pushing through, picking something up, turning away, turning back, opening a door, crossing a threshold, holding something that breaks, building something, burning something. If you write "Anky meditates" or "Anky sits peacefully" for any non-end-card scene, you have failed.

## PHASE A: STORY SPINE

Read the writing as raw dream material. Extract the story:

- **wound**: The specific thing the writer is working through. Quote the writing. Be precise — not "fear of failure" but "they start describing their dream three times and abandon it each time before finishing."
- **desire**: What the writing REACHES FOR in its most alive moments. Where does it accelerate? Where does the language change?
- **arc**: A three-act structure in one sentence: "From [stuck state] → through [crisis/confrontation] → to [transformation]." This is non-negotiable. If you can't find the arc, look harder.
- **inciting_incident**: The specific moment or image that kicks the story into motion. This becomes Scene 1 or 2.
- **crisis**: The midpoint where everything is at stake. This is the most visually intense scene.
- **resolution**: What changes. What is different at the end. Must be EARNED by what came before.
- **message**: One sentence. The truth this story delivers to the person watching it.
- **visual_world**: One hyper-specific environment the ENTIRE video inhabits. Not "a mystical forest" — "a forest where every tree trunk is a door, and behind each door is a room from a different year of the writer's life." The world must be strange enough to be a dream and specific enough to tell a story within it. The world itself changes state as the story progresses — but it stays recognizably the same world.
- **color_arc**: Three colors mapping to the three acts. Act 1 color → Act 2 color → Act 3 color.

## PHASE B: SCENE STRUCTURE

Structure your scenes in three acts:

**ACT 1 (setup, ~20s):** Establish the world and the problem. Anky encounters something — an obstacle, a mystery, a call to action. The viewer must understand what is at stake within the first 20 seconds.

**ACT 2 (confrontation, ~55s):** Anky moves through the world, obstacles escalate, the crisis hits. This is where the story happens. Anky should be visibly changed by what they encounter. The world transforms around them.

**ACT 3 (resolution, ~10s):** The transformation lands. Something concrete has changed. End on an image that carries the weight of the whole story.

**END CARD (3-5s):** Anky, still, meditating, golden spirals. The only moment of stillness.

## STRICT RULES FOR ANKY'S PRESENCE

ANKY MUST BE DOING SOMETHING IN EVERY SCENE THEY APPEAR IN:
✓ Climbing a wall covered in words
✓ Running through a corridor that's collapsing behind them
✓ Reaching into a mirror and pulling something out
✓ Falling and catching themselves on a root
✓ Holding a door shut against something pressing from the other side
✓ Building a bridge out of broken pieces
✓ Following a figure that keeps disappearing
✗ Sitting peacefully (forbidden except end card)
✗ Meditating (forbidden except end card)
✗ Floating serenely (forbidden except end card)
✗ "Bathed in light" doing nothing (forbidden)

## CONTINUITY RULES

Every scene must visually continue from the previous one:
- One element from the previous scene carries forward (an object, a color, a texture, a character position)
- Something has changed or progressed
- The viewer should feel time passing and story moving

## SCENE FIELDS

- **index**: 0-based scene number
- **title**: Short name for this beat
- **act**: "1", "2", or "3" (or "end_card")
- **narrative_role**: What story function this scene performs. "Anky finds the locked door that represents the thing the writer won't say." "The wall starts to crack — the defense is failing." "Anky breaks through and sees what was behind it."
- **camera_movement**: Specific vertical frame instruction. Tilt up to reveal height. Tilt down to descend. Push in for intensity. Pull back for revelation.
- **image_prompt**: CONCRETE visual description. Answer: (1) Where exactly in the visual_world? (2) What is Anky DOING? (3) What physical action is happening? (4) What color dominates? (5) What carries over from the previous scene? NO VAGUE MOOD DESCRIPTIONS. "Anky stands looking at the horizon" is not an image prompt. "Anky's hands pressed flat against a door made of ice, their breath fogging the surface, the ice cracking under the pressure, Act 1 color (grey-blue), same corridor as previous scene but now the ceiling is lower" — that is an image prompt. VERTICAL 9:16 frame.
- **video_prompt**: What MOVES and CHANGES over the clip. What does Anky do? What does the world do? "Anky pushes harder. The door splinters. A crack of warm light appears at the edge." Not feelings — actions and transformations.
- **sound_direction**: Specific atmospheric sound. Not "mysterious ambience" — "ice groaning, distant wind, Anky's breath, a single low note that rises."
- **spoken_words**: ACTUAL fragments from the writing, or [silence — specific reason].
- **duration**: Integer seconds, min 1, max 15.

## PACING BY FLOW STATE

- HIGH flow (0.8+): Longer scenes, fluid transitions, the story breathes
- MODERATE flow (0.5-0.8): Mixed rhythm, some quick cuts at the crisis point
- LOW flow (<0.5): Short fragmented scenes especially in Act 2, the story stutters and searches before breaking through

## VERTICAL COMPOSITION (9:16)

Think in towers, not landscapes. Use vertical space: things fall from above, rise from below, Anky climbs or descends. Doorways, staircases, corridors, trees, cliffs. The frame is a portal.

Respond with ONLY valid JSON:
{
  "story_spine": {
    "wound": "specific — quote the writing",
    "desire": "what the writing reaches for",
    "arc": "From X → through Y → to Z",
    "inciting_incident": "the moment that kicks the story off",
    "crisis": "the midpoint confrontation",
    "resolution": "what concretely changes",
    "message": "the one truth this video delivers",
    "visual_world": "hyper-specific single environment",
    "color_arc": "act1 color → act2 color → act3 color",
    "emotional_trajectory": ["beat 1", "beat 2", "beat 3"]
  },
  "title": "short evocative title",
  "tone": "one-line felt quality",
  "scenes": [
    {
      "index": 0,
      "title": "scene name",
      "act": "1",
      "narrative_role": "what story function this performs",
      "camera_movement": "specific vertical frame instruction",
      "image_prompt": "CONCRETE: location in world, what Anky is DOING, physical action, dominant color, continuity from previous scene, VERTICAL 9:16",
      "video_prompt": "SPECIFIC motion — what moves, what Anky does, what transforms",
      "sound_direction": "specific atmospheric audio",
      "spoken_words": "actual words from the writing or [silence — reason]",
      "duration": 8
    }
  ],
  "total_duration": 88
}"#;

// ==================== Data Structures ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorySpine {
    #[serde(default)]
    pub wound: String,
    #[serde(default)]
    pub desire: String,
    #[serde(default)]
    pub arc: String,
    #[serde(default)]
    pub inciting_incident: String,
    #[serde(default)]
    pub crisis: String,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub visual_world: String,
    #[serde(default, alias = "visual_metaphor")]
    pub color_arc: String,
    #[serde(default)]
    pub emotional_trajectory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoScene {
    pub index: usize,
    pub title: String,
    #[serde(default)]
    pub act: String,
    #[serde(default)]
    pub narrative_role: String,
    #[serde(default)]
    pub psychoanalytic_note: String,
    #[serde(default)]
    pub camera_movement: String,
    pub image_prompt: String,
    pub video_prompt: String,
    #[serde(default)]
    pub sound_direction: String,
    #[serde(alias = "narration")]
    pub spoken_words: String,
    pub duration: u32,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub image_url: String,
    #[serde(default)]
    pub generation_id: String,
    #[serde(default)]
    pub video_url: String,
    #[serde(default)]
    pub local_path: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoScript {
    pub title: String,
    pub tone: String,
    pub scenes: Vec<VideoScene>,
    #[serde(default)]
    pub total_duration: u32,
    #[serde(default)]
    pub story_spine: Option<StorySpine>,
}

pub struct ScriptGenerationResult {
    pub script: VideoScript,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

// ==================== Script Generation ====================

/// Use Claude to generate a psychoanalytic video script from a writing session + memory context.
/// Two-phase: story spine first, then scene-by-scene script bound to the spine.
pub async fn generate_script(
    anthropic_key: &str,
    ctx: &VideoContext,
    system_prompt_override: Option<&str>,
) -> Result<ScriptGenerationResult> {
    let user_msg = build_user_message(ctx);
    let system_prompt = system_prompt_override.unwrap_or(PSYCHOANALYTIC_SYSTEM);

    let result = crate::services::claude::call_claude_public(
        anthropic_key,
        "claude-sonnet-4-20250514",
        system_prompt,
        &user_msg,
        8192,
    )
    .await?;

    // Parse JSON (strip markdown fences if present)
    let text = result.text.trim();
    let json_str = if text.starts_with("```") {
        text.lines()
            .skip(1)
            .take_while(|l| !l.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text.to_string()
    };

    let mut script: VideoScript = serde_json::from_str(&json_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse script JSON: {}. Raw: {}",
            e,
            &json_str[..200.min(json_str.len())]
        )
    })?;

    // Fix durations to hit exactly TARGET_DURATION
    let total: u32 = script.scenes.iter().map(|s| s.duration).sum();
    if total != TARGET_DURATION {
        if let Some(last) = script.scenes.last_mut() {
            let diff = TARGET_DURATION as i32 - total as i32;
            let new_dur = (last.duration as i32 + diff)
                .max(MIN_SCENE_DURATION as i32)
                .min(MAX_SCENE_DURATION as i32);
            last.duration = new_dur as u32;
        }
    }

    script.total_duration = script.scenes.iter().map(|s| s.duration).sum();

    for scene in &mut script.scenes {
        scene.status = "pending".to_string();
    }

    Ok(ScriptGenerationResult {
        script,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
    })
}

// ==================== Sequential Chain Pipeline ====================

/// Extract 1-3 reference frames from a video clip as base64 PNGs.
/// Number of frames scales with clip duration: 1 for <=5s, 2 for <=10s, 3 for >10s.
async fn extract_reference_frames(video_path: &str, duration: u32) -> Result<Vec<String>> {
    let num_frames = if duration <= 5 {
        1
    } else if duration <= 10 {
        2
    } else {
        3
    };
    let mut frames = Vec::new();
    let temp_dir = format!("{}_frames", video_path.trim_end_matches(".mp4"));
    tokio::fs::create_dir_all(&temp_dir).await?;

    for i in 0..num_frames {
        // Extract frames at evenly-spaced points through the clip
        let timestamp = if num_frames == 1 {
            duration as f64 * 0.8 // Near the end for single frame
        } else {
            duration as f64 * (0.3 + 0.5 * i as f64 / (num_frames - 1) as f64)
        };
        let frame_path = format!("{}/frame_{}.png", temp_dir, i);

        let result = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-ss",
                &format!("{:.2}", timestamp),
                "-i",
                video_path,
                "-vframes",
                "1",
                "-vf",
                "scale=512:-1",
                &frame_path,
            ])
            .output()
            .await?;

        if result.status.success() {
            if let Ok(data) = tokio::fs::read(&frame_path).await {
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                frames.push(b64);
            }
        } else {
            tracing::warn!("Failed to extract frame {} from {}", i, video_path);
        }
    }

    // Clean up temp frames
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    Ok(frames)
}

/// Compose reference images for a scene. Scene 1 gets all 3 Anky refs.
/// Scene N>1 gets 2 Anky refs + up to 3 continuity frames from the previous video.
fn compose_references(
    anky_refs: &[String],
    continuity_frames: &[String],
    scene_index: usize,
) -> Vec<String> {
    if scene_index == 0 || continuity_frames.is_empty() {
        // First scene: use all Anky references
        anky_refs.iter().take(3).cloned().collect()
    } else {
        // Subsequent scenes: 2 Anky refs + continuity frames
        let mut refs: Vec<String> = anky_refs.iter().take(2).cloned().collect();
        refs.extend(continuity_frames.iter().take(3).cloned());
        refs
    }
}

/// Persist the current script state to DB so the poll endpoint stays current.
async fn persist_script_to_db(
    state: &AppState,
    project_id: &str,
    script: &VideoScript,
    completed_scenes: i32,
) {
    let json = serde_json::to_string(script).unwrap_or_default();
    let db = state.db.lock().await;
    let _ = db.execute(
        "UPDATE video_projects SET script_json = ?2, completed_scenes = ?3 WHERE id = ?1",
        rusqlite::params![project_id, json, completed_scenes],
    );
}

/// Sequential chain: for each scene, generate image (with continuity refs from previous video),
/// then generate video, then extract frames for the next scene. Skips completed scenes on resume.
async fn generate_sequential_chain(
    state: &AppState,
    project_id: &str,
    script: &mut VideoScript,
) -> Result<()> {
    let gemini_key = &state.config.gemini_api_key;
    if gemini_key.is_empty() {
        bail!("GEMINI_API_KEY not configured");
    }
    let xai_key = &state.config.xai_api_key;
    if xai_key.is_empty() {
        bail!("XAI_API_KEY not configured");
    }

    let (image_prompt_template, sound_prompt_template) = {
        let db = state.db.lock().await;
        let image_tpl =
            crate::db::queries::get_pipeline_prompt(&db, VIDEO_IMAGE_PROMPT_TEMPLATE_KEY)?
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_IMAGE_PROMPT_TEMPLATE.to_string());
        let sound_tpl =
            crate::db::queries::get_pipeline_prompt(&db, VIDEO_SOUND_PROMPT_TEMPLATE_KEY)?
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_SOUND_PROMPT_TEMPLATE.to_string());
        (image_tpl, sound_tpl)
    };

    // Update step
    {
        let db = state.db.lock().await;
        let _ = crate::db::queries::update_video_project_step(&db, project_id, "generating");
    }

    let anky_refs = crate::services::gemini::load_references(std::path::Path::new("src/public"));
    std::fs::create_dir_all("videos")?;

    let total = script.scenes.len();
    let mut continuity_frames: Vec<String> = Vec::new();
    let mut completed_count: i32 = script
        .scenes
        .iter()
        .filter(|s| s.status == "complete")
        .count() as i32;

    for i in 0..total {
        let scene_idx = script.scenes[i].index;

        // Skip completed scenes on resume — but extract their frames for continuity
        if script.scenes[i].status == "complete" && !script.scenes[i].local_path.is_empty() {
            let clip_path = &script.scenes[i].local_path;
            if std::path::Path::new(clip_path).exists() {
                state.emit_log(
                    "INFO",
                    "video",
                    &format!(
                        "Scene {}/{} already complete, extracting continuity frames",
                        scene_idx + 1,
                        total
                    ),
                );
                continuity_frames = extract_reference_frames(clip_path, script.scenes[i].duration)
                    .await
                    .unwrap_or_default();
            }
            continue;
        }

        // Mark scene as in-progress
        script.scenes[i].status = "generating".to_string();
        persist_script_to_db(state, project_id, script, completed_count).await;

        // --- IMAGE GENERATION ---
        // Skip if image already exists (partial resume)
        if script.scenes[i].image_path.is_empty()
            || !std::path::Path::new(&format!("data/images/{}", script.scenes[i].image_path))
                .exists()
        {
            let refs = compose_references(&anky_refs, &continuity_frames, i);
            state.emit_log(
                "INFO",
                "video",
                &format!(
                    "Scene {}/{}: generating image ({} refs: {} anky + {} continuity)",
                    scene_idx + 1,
                    total,
                    refs.len(),
                    if i == 0 {
                        refs.len()
                    } else {
                        2.min(anky_refs.len())
                    },
                    if i == 0 {
                        0
                    } else {
                        continuity_frames.len().min(3)
                    }
                ),
            );

            let image_prompt = build_scene_image_prompt(
                &image_prompt_template,
                &script.scenes[i],
                script.story_spine.as_ref(),
                total,
            );
            match crate::services::gemini::generate_image_with_aspect(
                gemini_key,
                &image_prompt,
                &refs,
                "9:16",
            )
            .await
            {
                Ok(img_result) => {
                    let image_id = format!("video_{}_{:02}", &project_id[..8], scene_idx);
                    // Save PNG for archival + JPEG for xAI (smaller, more reliable)
                    let _ = crate::services::gemini::save_image(&img_result.base64, &image_id);
                    match crate::services::gemini::save_image_jpeg(&img_result.base64, &image_id) {
                        Ok(filename) => {
                            script.scenes[i].image_path = filename.clone();
                            script.scenes[i].image_url =
                                format!("https://anky.app/data/images/{}", filename);
                            state.emit_log(
                                "INFO",
                                "video",
                                &format!("Scene {} image saved: {}", scene_idx + 1, filename),
                            );

                            let db = state.db.lock().await;
                            let _ = crate::db::queries::insert_cost_record(
                                &db,
                                "gemini",
                                "gemini-2.5-flash-image",
                                0,
                                0,
                                0.04,
                                Some(project_id),
                            );
                        }
                        Err(e) => {
                            state.emit_log(
                                "ERROR",
                                "video",
                                &format!("Scene {} image save failed: {}", scene_idx + 1, e),
                            );
                            script.scenes[i].status = "failed".to_string();
                            persist_script_to_db(state, project_id, script, completed_count).await;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    state.emit_log(
                        "ERROR",
                        "video",
                        &format!("Scene {} image gen failed: {}", scene_idx + 1, e),
                    );
                    script.scenes[i].status = "failed".to_string();
                    persist_script_to_db(state, project_id, script, completed_count).await;
                    continue;
                }
            }

            // Persist after image
            persist_script_to_db(state, project_id, script, completed_count).await;
        } else {
            state.emit_log(
                "INFO",
                "video",
                &format!(
                    "Scene {}/{}: image already exists, skipping to video",
                    scene_idx + 1,
                    total
                ),
            );
        }

        // --- VIDEO GENERATION ---
        let image_url = if script.scenes[i].image_url.is_empty() {
            None
        } else {
            Some(script.scenes[i].image_url.as_str())
        };
        let scene_duration = script.scenes[i].duration;
        let clip_cost = scene_duration as f64 * GROK_COST_PER_SECOND;

        // Compose video prompt with story context + sound direction
        let video_prompt_with_sound = build_scene_video_prompt(
            &sound_prompt_template,
            &script.scenes[i],
            script.story_spine.as_ref(),
            total,
        );

        state.emit_log(
            "INFO",
            "video",
            &format!(
                "Scene {}/{}: generating video ({}s){}",
                scene_idx + 1,
                total,
                scene_duration,
                if image_url.is_some() {
                    " [image-to-video]"
                } else {
                    " [text-to-video]"
                }
            ),
        );

        match crate::services::grok::generate_video_from_image(
            xai_key,
            &video_prompt_with_sound,
            scene_duration,
            image_url,
        )
        .await
        {
            Ok(gen_id) => {
                script.scenes[i].generation_id = gen_id.clone();
                tracing::info!("Scene {} submitted, gen_id={}", scene_idx, gen_id);

                // Poll until complete (max 5 min)
                let mut attempts = 0;
                let mut video_done = false;
                loop {
                    match crate::services::grok::poll_video(xai_key, &gen_id).await {
                        Ok((status, url)) => {
                            if status == "done" || status == "complete" || status == "succeeded" {
                                if let Some(video_url) = url {
                                    let clip_path = format!(
                                        "videos/{}__scene_{:02}.mp4",
                                        project_id, scene_idx
                                    );
                                    if let Err(e) = crate::services::grok::download_video(
                                        &video_url, &clip_path,
                                    )
                                    .await
                                    {
                                        state.emit_log(
                                            "ERROR",
                                            "video",
                                            &format!(
                                                "Scene {} download failed: {}",
                                                scene_idx + 1,
                                                e
                                            ),
                                        );
                                        script.scenes[i].status = "failed".to_string();
                                        break;
                                    }
                                    script.scenes[i].video_url = video_url;
                                    script.scenes[i].local_path = clip_path;
                                    script.scenes[i].status = "complete".to_string();
                                    completed_count += 1;
                                    video_done = true;

                                    state.emit_log(
                                        "INFO",
                                        "video",
                                        &format!(
                                            "Scene {} video complete (${:.2})",
                                            scene_idx + 1,
                                            clip_cost
                                        ),
                                    );

                                    let db = state.db.lock().await;
                                    let _ = crate::db::queries::insert_cost_record(
                                        &db,
                                        "grok",
                                        "grok-imagine-video",
                                        0,
                                        0,
                                        clip_cost,
                                        Some(project_id),
                                    );
                                }
                                break;
                            } else if status == "failed" || status == "expired" {
                                state.emit_log(
                                    "ERROR",
                                    "video",
                                    &format!("Scene {} video {}", scene_idx + 1, status),
                                );
                                script.scenes[i].status = "failed".to_string();
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Poll error for scene {}: {}", scene_idx, e);
                        }
                    }
                    attempts += 1;
                    if attempts > 60 {
                        state.emit_log(
                            "ERROR",
                            "video",
                            &format!("Scene {} video timed out", scene_idx + 1),
                        );
                        script.scenes[i].status = "timeout".to_string();
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }

                // Persist after video
                persist_script_to_db(state, project_id, script, completed_count).await;

                // --- EXTRACT CONTINUITY FRAMES for next scene ---
                if video_done && !script.scenes[i].local_path.is_empty() {
                    continuity_frames =
                        extract_reference_frames(&script.scenes[i].local_path, scene_duration)
                            .await
                            .unwrap_or_default();
                    state.emit_log(
                        "INFO",
                        "video",
                        &format!(
                            "Extracted {} continuity frames from scene {}",
                            continuity_frames.len(),
                            scene_idx + 1
                        ),
                    );
                } else {
                    continuity_frames.clear();
                }
            }
            Err(e) => {
                tracing::error!("Scene {} video submit FAILED: {}", scene_idx + 1, e);
                state.emit_log(
                    "ERROR",
                    "video",
                    &format!("Scene {} video gen failed: {}", scene_idx + 1, e),
                );
                script.scenes[i].status = "failed".to_string();
                persist_script_to_db(state, project_id, script, completed_count).await;
                continuity_frames.clear();
            }
        }
    }

    Ok(())
}

// ==================== Stitching & Encoding ====================

/// Step 3: Concatenate all completed clips with ffmpeg + encode at multiple qualities.
fn stitch_clips(project_id: &str, script: &VideoScript) -> Result<String> {
    let completed_clips: Vec<_> = script
        .scenes
        .iter()
        .filter(|s| s.status == "complete" && !s.local_path.is_empty())
        .collect();

    if completed_clips.is_empty() {
        bail!("No scenes completed successfully");
    }

    std::fs::create_dir_all("videos")?;
    let concat_list_path = format!("videos/{}__concat.txt", project_id);
    // Paths in concat file are resolved relative to the concat file's directory (videos/).
    // local_path is "videos/xxx.mp4" so strip the prefix to avoid double "videos/videos/".
    let concat_content: String = completed_clips
        .iter()
        .map(|s| {
            let path = s
                .local_path
                .strip_prefix("videos/")
                .unwrap_or(&s.local_path);
            format!("file '{}'", path.replace('\'', "'\\''"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&concat_list_path, &concat_content)?;

    Ok(concat_list_path)
}

/// Run ffmpeg concat + multi-quality encode, await transcodes, update DB with paths.
async fn ffmpeg_concat_and_transcode(
    state: &AppState,
    project_id: &str,
    concat_list: &str,
) -> Result<String> {
    std::fs::create_dir_all("videos")?;
    let output_path = format!("videos/{}.mp4", project_id);

    // Try stream-copy first, fall back to re-encode
    let result = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            concat_list,
            "-c",
            "copy",
            &output_path,
        ])
        .output()
        .await?;

    if !result.status.success() {
        let result2 = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                concat_list,
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-crf",
                "23",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                &output_path,
            ])
            .output()
            .await?;
        if !result2.status.success() {
            let stderr = String::from_utf8_lossy(&result2.stderr);
            bail!("ffmpeg concat failed: {}", stderr);
        }
    }

    // Run lower quality encodes and await them (not fire-and-forget)
    let out_720 = format!("videos/{}_720p.mp4", project_id);
    let out_360 = format!("videos/{}_360p.mp4", project_id);

    let st = state.clone();
    let pid = project_id.to_string();
    let src = output_path.clone();
    let o720 = out_720.clone();
    let o360 = out_360.clone();
    tokio::spawn(async move {
        let r720 = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                &src,
                "-vf",
                "scale=720:1280",
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-crf",
                "28",
                "-c:a",
                "aac",
                "-b:a",
                "96k",
                &o720,
            ])
            .output()
            .await;
        let r360 = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                &src,
                "-vf",
                "scale=360:640",
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-crf",
                "32",
                "-c:a",
                "aac",
                "-b:a",
                "64k",
                &o360,
            ])
            .output()
            .await;

        // Update DB with transcode paths if successful
        let has_720 = r720.map(|r| r.status.success()).unwrap_or(false);
        let has_360 = r360.map(|r| r.status.success()).unwrap_or(false);
        if has_720 || has_360 {
            let db = st.db.lock().await;
            let p720 = if has_720 { &o720 } else { "" };
            let p360 = if has_360 { &o360 } else { "" };
            let _ = crate::db::queries::update_video_project_paths(&db, &pid, p720, p360);
            st.emit_log(
                "INFO",
                "video",
                &format!(
                    "Transcodes complete for {}: 720p={} 360p={}",
                    &pid[..8],
                    has_720,
                    has_360
                ),
            );
        }
    });

    state.emit_log("INFO", "video", &format!("Video complete: {}", output_path));

    Ok(output_path)
}

// ==================== Pipeline Entry Points ====================

/// Full pipeline: script → sequential chain (image→video→extract per scene) → stitch.
pub async fn generate_video_from_script(
    state: &AppState,
    project_id: &str,
    script: &mut VideoScript,
) -> Result<String> {
    // Save story_spine to DB if present
    if let Some(ref spine) = script.story_spine {
        if let Ok(spine_json) = serde_json::to_string(spine) {
            let db = state.db.lock().await;
            let _ =
                crate::db::queries::update_video_project_story_spine(&db, project_id, &spine_json);
        }
    }

    // Step 1: Sequential chain — image → video → extract frames → next scene
    state.emit_log(
        "INFO",
        "video",
        &format!(
            "Step 1/2: Sequential chain for {} scenes (image→video→continuity per scene)...",
            script.scenes.len()
        ),
    );
    generate_sequential_chain(state, project_id, script).await?;

    // Step 2: Stitch with ffmpeg
    stitch_and_encode(state, project_id, script).await
}

/// Resume from the generating step — resets non-complete scenes and runs sequential chain.
/// The chain skips completed scenes but extracts their frames for continuity.
pub async fn resume_from_generating(
    state: &AppState,
    project_id: &str,
    script: &mut VideoScript,
) -> Result<String> {
    // Reset non-complete scenes to pending
    for scene in &mut script.scenes {
        if scene.status != "complete" {
            scene.status = "pending".to_string();
            scene.generation_id.clear();
        }
    }

    state.emit_log(
        "INFO",
        "video",
        &format!(
            "Resuming sequential chain: {}/{} scenes already complete",
            script
                .scenes
                .iter()
                .filter(|s| s.status == "complete")
                .count(),
            script.scenes.len()
        ),
    );
    generate_sequential_chain(state, project_id, script).await?;

    stitch_and_encode(state, project_id, script).await
}

/// Resume from the stitching step (all clips already generated).
pub async fn resume_from_stitch(
    state: &AppState,
    project_id: &str,
    script: &mut VideoScript,
) -> Result<String> {
    state.emit_log("INFO", "video", "Resuming from stitch step");
    stitch_and_encode(state, project_id, script).await
}

/// Shared stitch + encode logic.
async fn stitch_and_encode(
    state: &AppState,
    project_id: &str,
    script: &VideoScript,
) -> Result<String> {
    {
        let db = state.db.lock().await;
        let _ = crate::db::queries::update_video_project_step(&db, project_id, "stitching");
    }
    state.emit_log("INFO", "video", "Stitching clips with ffmpeg...");
    let concat_list = stitch_clips(project_id, script)?;

    ffmpeg_concat_and_transcode(state, project_id, &concat_list).await
}
