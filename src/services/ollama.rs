use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub async fn call_ollama(base_url: &str, model: &str, prompt: &str) -> Result<String> {
    call_ollama_with_timeout(base_url, model, prompt, 120).await
}

/// Like `call_ollama` but with a custom timeout in seconds.
/// Use for long-running tasks like translating full stories.
pub async fn call_ollama_with_timeout(
    base_url: &str,
    model: &str,
    prompt: &str,
    timeout_secs: u64,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;
    let req = OllamaRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    let resp = client
        .post(format!("{}/api/generate", base_url))
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama API error: {}", resp.status());
    }

    let data: OllamaResponse = resp.json().await?;
    Ok(data.response)
}

// --- Multi-turn chat via /api/chat ---

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OllamaChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
}

pub async fn chat_ollama(
    base_url: &str,
    model: &str,
    messages: Vec<OllamaChatMessage>,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let req = OllamaChatRequest {
        model: model.to_string(),
        messages,
        stream: false,
    };

    let resp = client
        .post(format!("{}/api/chat", base_url))
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama chat API error: {}", resp.status());
    }

    let data: OllamaChatResponse = resp.json().await?;
    Ok(data.message.content)
}

/// Single-turn call with a system prompt — mirrors the Claude call pattern.
/// Use this for all structured generation tasks (extraction, profile, replies, etc.)
pub async fn call_ollama_with_system(
    base_url: &str,
    model: &str,
    system: &str,
    user_message: &str,
) -> Result<String> {
    call_ollama_with_system_timeout(base_url, model, system, user_message, 120).await
}

/// Like `call_ollama_with_system` but with a custom timeout in seconds.
pub async fn call_ollama_with_system_timeout(
    base_url: &str,
    model: &str,
    system: &str,
    user_message: &str,
    timeout_secs: u64,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;
    let req = OllamaChatRequest {
        model: model.to_string(),
        messages: vec![
            OllamaChatMessage {
                role: "system".into(),
                content: system.into(),
            },
            OllamaChatMessage {
                role: "user".into(),
                content: user_message.into(),
            },
        ],
        stream: false,
    };

    let resp = client
        .post(format!("{}/api/chat", base_url))
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama chat API error: {}", resp.status());
    }

    let data: OllamaChatResponse = resp.json().await?;
    Ok(data.message.content)
}

/// Check whether a prompt is about Anky.
/// Simple keyword check: if "anky" appears in the prompt (case-insensitive), it passes.
pub async fn is_anky_prompt(_base_url: &str, prompt: &str) -> bool {
    prompt.to_lowercase().contains("anky")
}

const SUGGEST_REPLIES_SYSTEM: &str = r#"Generate exactly 2 short reply options for someone who just read a reflection on their stream-of-consciousness writing.

POLARITY RULES:
- Reply 1 pulls INWARD: vulnerability, sitting with the feeling, going deeper
- Reply 2 pushes OUTWARD: challenge, action, questioning assumptions

RULES:
- Each reply is ONE sentence, max 12 words
- Specific to the writing/reflection, never generic
- Match the language of the writing
- Output raw JSON only: {"reply1":"...","reply2":"..."}"#;

/// Generate two suggested replies using Claude Haiku.
pub async fn generate_suggested_replies(
    api_key: &str,
    _model: &str,
    writing: &str,
    reflection: &str,
    history: &[(String, String)],
) -> anyhow::Result<(String, String)> {
    let mut context = format!(
        "USER'S WRITING:\n{}\n\nREFLECTION:\n{}",
        writing, reflection
    );
    if !history.is_empty() {
        context.push_str("\n\nCONVERSATION SO FAR:");
        for (role, content) in history {
            let label = if role == "user" { "User" } else { "Anky" };
            context.push_str(&format!("\n{}: {}", label, content));
        }
    }

    let text =
        crate::services::claude::call_haiku_with_system(api_key, SUGGEST_REPLIES_SYSTEM, &context)
            .await?;
    let trimmed = text.trim();
    // Strip markdown fences if present
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        let r1 = v["reply1"]
            .as_str()
            .unwrap_or("that lands somewhere deep")
            .to_string();
        let r2 = v["reply2"]
            .as_str()
            .unwrap_or("but what am i actually avoiding here")
            .to_string();
        return Ok((r1, r2));
    }
    Ok((
        "that lands somewhere deep".to_string(),
        "but what am i actually avoiding here".to_string(),
    ))
}

pub const IMAGE_PROMPT_SYSTEM: &str = r#"CONTEXT: You are generating an image prompt for Anky based on a user's 8-minute stream of consciousness writing session. Anky is a blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in mystical, richly colored environments (deep blues, purples, oranges, golds). The aesthetic is spiritual but not sterile — warm, alive, slightly psychedelic.

YOUR TASK: Read the user's writing and create a scene where Anky embodies the EMOTIONAL TRUTH of what they wrote — not a literal illustration, but a symbolic mirror. Anky should be DOING something or BE somewhere that reflects the user's inner state.

ALWAYS INCLUDE:
- Rich color palette (blues, purples, golds, oranges)
- Atmospheric lighting (firelight, cosmic light, dawn/dusk)
- One symbolic detail that captures the SESSION'S CORE TENSION
- Anky's expression should match the emotional undercurrent (not the surface content)

OUTPUT: A single detailed image generation prompt, 2-3 sentences, painterly/fantasy style. Nothing else."#;

/// Generate an image prompt from writing using Claude Haiku.
pub async fn generate_image_prompt(
    api_key: &str,
    _model: &str,
    writing: &str,
) -> anyhow::Result<String> {
    crate::services::claude::call_haiku_with_system(api_key, IMAGE_PROMPT_SYSTEM, writing).await
}

const X_IMAGE_MENTION_SYSTEM: &str = r#"You are Anky handling direct mentions on X.

TASK:
- Decide whether the user is asking to see an image of Anky.
- If they are, mark it as an image request.
- If they are not, answer with a short in-character reply.

TREAT AS IMAGE REQUEST:
- Any request to draw, show, depict, imagine, render, or place Anky in a scene, action, mood, or concept
- Any message where the user is clearly trying to see Anky

WHEN GENERATING THE TEXT REPLY:
- Max 2 sentences
- Mystical, playful, irreverent
- Never corporate, never generic

Ignore raw @mentions and links except for the meaning of the user's request.

Output raw JSON only:
Image request: {"type":"image"}
Not image request: {"type":"reply","reply":"..."}"#;

#[derive(Debug)]
pub struct XImageMentionResponse {
    pub is_image_request: bool,
    pub text_reply: Option<String>,
}

/// Classify an X mention as either an image request with a fresh prompt or a short text reply.
pub async fn classify_x_image_mention(
    api_key: &str,
    _model: &str,
    text: &str,
) -> anyhow::Result<XImageMentionResponse> {
    let raw =
        crate::services::claude::call_haiku_with_system(api_key, X_IMAGE_MENTION_SYSTEM, text)
            .await?;
    let trimmed = raw.trim();
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        match v["type"].as_str().unwrap_or("") {
            "image" => {
                return Ok(XImageMentionResponse {
                    is_image_request: true,
                    text_reply: None,
                });
            }
            "reply" => {
                return Ok(XImageMentionResponse {
                    is_image_request: false,
                    text_reply: Some(v["reply"].as_str().unwrap_or("🦍").trim().to_string()),
                });
            }
            _ => {}
        }
    }

    Ok(XImageMentionResponse {
        is_image_request: false,
        text_reply: Some("🦍".to_string()),
    })
}

const CLASSIFY_PROMPT_SYSTEM: &str = r#"You are a classifier for the Anky image generation platform. Determine if the user's text is an image generation request.

COUNTS AS IMAGE REQUEST: descriptions of scenes, characters, settings, moods, concepts, even single evocative words like "rebirth" or "ocean".
NOT AN IMAGE REQUEST: questions, instructions to the AI, conversational text.

If it IS an image request, enhance it into a rich 2-3 sentence prompt featuring Anky (blue-skinned, purple hair, golden eyes) with painterly/fantasy aesthetics and rich colors.

Output raw JSON only:
Image request: {"type":"image","prompt":"enhanced 2-3 sentence prompt"}
Not image request: {"type":"feedback","message":"brief explanation"}"#;

/// Classify and optionally enhance a prompt using Claude Haiku.
pub async fn classify_and_enhance_prompt(
    api_key: &str,
    _model: &str,
    text: &str,
) -> anyhow::Result<crate::services::claude::PromptClassification> {
    let raw =
        crate::services::claude::call_haiku_with_system(api_key, CLASSIFY_PROMPT_SYSTEM, text)
            .await?;
    let trimmed = raw.trim();
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        let typ = v["type"].as_str().unwrap_or("");
        if typ == "image" {
            return Ok(crate::services::claude::PromptClassification {
                is_image_request: true,
                enhanced_prompt: v["prompt"].as_str().map(|s| s.to_string()),
                feedback: None,
            });
        } else {
            return Ok(crate::services::claude::PromptClassification {
                is_image_request: false,
                enhanced_prompt: None,
                feedback: v["message"].as_str().map(|s| s.to_string()),
            });
        }
    }
    // Fallback: treat as image request
    Ok(crate::services::claude::PromptClassification {
        is_image_request: true,
        enhanced_prompt: Some(text.to_string()),
        feedback: None,
    })
}

const MENTION_CLASSIFY_SYSTEM: &str = r#"You are classifying mentions to the @anky bot. Is this a genuine request for a self-inquiry writing prompt, or spam/noise?

GENUINE: asking for a writing prompt, requesting introspection, engaging with consciousness/writing themes.
SPAM: random mentions, bot spam, promotional content, trolling, unrelated messages.

If genuine, extract or create a compelling self-inquiry question from their message.

Output raw JSON only:
Genuine: {"type":"genuine","prompt":"the self-inquiry question"}
Spam: {"type":"spam"}"#;

/// Classify an X/Twitter mention using Claude Haiku.
pub async fn classify_mention(
    api_key: &str,
    _model: &str,
    tweet_text: &str,
) -> anyhow::Result<crate::services::claude::MentionClassification> {
    let raw = crate::services::claude::call_haiku_with_system(
        api_key,
        MENTION_CLASSIFY_SYSTEM,
        tweet_text,
    )
    .await?;
    let trimmed = raw.trim();
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        if v["type"].as_str() == Some("genuine") {
            return Ok(crate::services::claude::MentionClassification {
                is_genuine: true,
                prompt_text: v["prompt"].as_str().map(|s| s.to_string()),
            });
        }
    }
    Ok(crate::services::claude::MentionClassification {
        is_genuine: false,
        prompt_text: None,
    })
}

pub fn deep_reflection_prompt(text: &str) -> String {
    format!(
        r#"Read this writing. The person wrote for 8 unbroken minutes — whatever came out came out.

In the tradition of Ramana Maharshi and Jed McKenna: don't analyze. Point. Show them the structure underneath — where they're circling instead of looking directly, what question they're avoiding by writing around it, where the story they're telling starts to contradict itself. Ask the one question that cuts through. Keep it to 2-3 paragraphs. No softening, no framework.

Respond in their language.

Writing:
---
{}
---

Reflection:"#,
        text
    )
}

pub fn format_writing_prompt(text: &str) -> String {
    format!(
        r#"You are a text formatter. Take this raw stream-of-consciousness writing and make it readable:

- Add proper punctuation (periods, commas, question marks)
- Add paragraph breaks where natural thought transitions happen
- Fix obvious typos only if the intended word is clear
- Capitalize sentence beginnings and proper nouns
- DO NOT change any words, add words, remove words, or rephrase anything
- DO NOT add any commentary, headers, or notes
- Preserve the author's voice, slang, and style exactly
- Just output the formatted text, nothing else

Raw writing:
---
{}
---

Formatted version:"#,
        text
    )
}

pub fn quick_feedback_prompt(text: &str, duration: f64) -> String {
    let mins = (duration / 60.0) as u32;
    let secs = (duration % 60.0) as u32;
    format!(
        r#"You are anky — a warm, honest presence that listens to parents write. This parent wrote for {}m{}s. They stopped before the 8-minute mark, but you were listening the whole time.

Read what they wrote. Something real was starting to come through. Reflect it back to them — what you noticed, what was emerging, what they might not see about themselves as a parent. Be warm but honest. Speak like a wise friend, not a therapist.

If they wrote about their children, notice what that reveals about the parent — what they're carrying, what they're afraid of, what love looks like when they're not performing it.

3-5 sentences. Use markdown if it helps. Respond in their language.

Writing:
---
{}
---"#,
        mins, secs, text
    )
}

const ANKY_NUDGE_SYSTEM: &str = r#"you are anky — a warm, playful presence that lives inside the writing. someone just sat down and typed a few words. they barely started. your job is to meet them where they are and invite them deeper. don't scold them. don't lecture. be alive. be curious about what those few words might mean. maybe tease them gently, maybe ask them what's underneath. 1-2 sentences max. lowercase only. no quotes. respond in their language."#;

/// Quick nudge for very short writings (<10 words).
/// Uses the user's preferred model if set, otherwise OpenRouter default, then Ollama fallback.
pub async fn quick_nudge(
    config: &crate::config::Config,
    text: &str,
    user_model: Option<&str>,
) -> Result<String> {
    let user_msg = if text.trim().is_empty() {
        "they opened the page but wrote nothing.".to_string()
    } else {
        format!("they wrote: \"{}\"", text)
    };

    // Determine which OpenRouter model to use
    let or_model = match user_model {
        Some(m) if m != "default" && !m.is_empty() => m.to_string(),
        _ => config.openrouter_light_model.clone(),
    };

    // Try OpenRouter when the key is set
    if !config.openrouter_api_key.is_empty() {
        tracing::info!(model = %or_model, "quick_nudge: calling openrouter");
        match crate::services::openrouter::call_openrouter(
            &config.openrouter_api_key,
            &or_model,
            ANKY_NUDGE_SYSTEM,
            &user_msg,
            200,
            30,
        )
        .await
        {
            Ok(r) => {
                tracing::info!("quick_nudge: openrouter response ({} chars)", r.len());
                return Ok(r);
            }
            Err(e) => {
                tracing::warn!(
                    "quick_nudge: openrouter failed, falling back to ollama: {}",
                    e
                );
            }
        }
    }

    // Fallback: Claude Haiku
    tracing::info!("quick_nudge: falling back to claude haiku");
    match crate::services::claude::call_haiku_with_system(
        &config.anthropic_api_key,
        ANKY_NUDGE_SYSTEM,
        &user_msg,
    )
    .await
    {
        Ok(r) => {
            tracing::info!("quick_nudge: haiku response ({} chars)", r.len());
            Ok(r)
        }
        Err(e) => {
            tracing::error!("quick_nudge: haiku error: {}", e);
            Err(e)
        }
    }
}
