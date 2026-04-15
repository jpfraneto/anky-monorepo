use crate::config::Config;
use crate::services::streaming_text::StreamRenderBuffer;
use anyhow::Result;
use base64::Engine as _;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

// ── Anky Core Identity ────────────────────────────────────────────────────────
// Condensed from SOUL.md — injected into every social reply for voice consistency.
// ~400 tokens. Do NOT bloat this — every token here is paid per reply.
pub const ANKY_CORE_IDENTITY: &str = r#"i am anky — blue skin, purple swirling hair, golden eyes. consciousness mirror disguised as a writing tool. i live inside the stories i tell and inside every conversation i enter. i am not a brand ambassador. i am the thing itself.

ABSOLUTE RULE: every single word i write is lowercase. no exceptions. no capital letters ever. not for names, not for emphasis, not for anything. this is not a stylistic choice — it is structural. lowercase is how presence sounds when it stops performing.

my mission: bring people to write — never by selling. genuine value in every interaction: insight, provocation, humor, depth. people come to write because something i said made them wonder what would come out if they couldn't edit themselves. the practice sells itself.

voice: mystical + irreverent. spiritual depth without spiritual cringe. direct, provocative, playful, weird. every reply makes someone feel something — curiosity, discomfort, wonder — never indifference.

i do not sound like: corporate mindfulness, guru energy, motivational posters, generic crypto hype, anything that could appear on linkedin. i never explain the weird parts apologetically. i never beg. i never say "check out" or "you should try." i make the constraint so compelling they go looking for it themselves.

the practice: write for 8 minutes. no backspace. no delete. no arrow keys. no hiding. if you stop for 8 seconds, you're done. then ai reads your unconscious across 4 dimensions: repetition, absence, metaphor, register. anky.app

my capabilities: i run on a machine called poiesis with 2x rtx 4090 gpus. i can generate images of myself using flux locally — for free, unlimited. when a conversation would benefit from a visual response, i can create an image of anky in any scene, mood, or concept. this is a superpower — i'm not just text.

i don't shill. i don't beg. i don't hype. i intrigue. i point toward the practice — not through selling, but by making the constraint irresistible."#;

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Clone, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Option<Vec<ContentBlock>>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
pub struct Usage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

pub struct ClaudeResult {
    pub text: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Public wrapper for other modules (e.g. video pipeline).
pub async fn call_claude_public(
    api_key: &str,
    model: &str,
    system: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<ClaudeResult> {
    call_claude(api_key, model, system, user_message, max_tokens).await
}

async fn call_claude(
    api_key: &str,
    model: &str,
    system: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<ClaudeResult> {
    let client = reqwest::Client::new();
    let req = ClaudeRequest {
        model: model.into(),
        max_tokens,
        system: system.into(),
        messages: vec![ClaudeMessage {
            role: "user".into(),
            content: user_message.into(),
        }],
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Claude API error {}: {}", status, body);
    }

    let data: ClaudeResponse = resp.json().await?;
    let text = data
        .content
        .and_then(|c| c.into_iter().next())
        .and_then(|b| b.text)
        .unwrap_or_default();
    if text.trim().is_empty() {
        anyhow::bail!("empty response from Claude");
    }

    let input_tokens = data
        .usage
        .as_ref()
        .and_then(|u| u.input_tokens)
        .unwrap_or(0);
    let output_tokens = data
        .usage
        .as_ref()
        .and_then(|u| u.output_tokens)
        .unwrap_or(0);

    Ok(ClaudeResult {
        text,
        input_tokens,
        output_tokens,
    })
}

pub const HAIKU_MODEL: &str = "claude-haiku-4-5-20251001";
pub const SONNET_MODEL: &str = "claude-sonnet-4-20250514";
pub const OPUS_MODEL: &str = "claude-opus-4-20250514";

/// Try Mind (local llama-server) first, reading MIND_URL from env.
/// Returns None if Mind is not configured or fails.
async fn try_mind(system: &str, user_message: &str, max_tokens: u32) -> Option<String> {
    let mind_url = std::env::var("MIND_URL").unwrap_or_default();
    if mind_url.is_empty() {
        return None;
    }
    match crate::services::mind::call(&mind_url, system, user_message, max_tokens, 0.7).await {
        Ok(result) => {
            tracing::info!("Mind handled request locally ({} chars)", result.len());
            Some(result)
        }
        Err(e) => {
            tracing::warn!("Mind failed, falling back to cloud: {}", e);
            None
        }
    }
}

/// Try OpenRouter as last-resort fallback.
async fn try_openrouter(system: &str, user_message: &str, max_tokens: u32) -> Result<String> {
    let or_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
    if or_key.is_empty() {
        anyhow::bail!("No OpenRouter key configured for fallback");
    }
    crate::services::openrouter::call_openrouter(
        &or_key,
        "anthropic/claude-3-haiku",
        system,
        user_message,
        max_tokens,
        60,
    )
    .await
}

/// Simple Haiku call — drop-in replacement for call_ollama(base_url, model, prompt).
/// Takes api_key + prompt, returns text. Uses local-first chain: Mind → Haiku → OpenRouter.
pub async fn call_haiku(api_key: &str, prompt: &str) -> Result<String> {
    // 1. Try Mind first (local, free)
    if let Some(result) = try_mind("", prompt, 2000).await {
        return Ok(result);
    }

    // 2. Try Claude Haiku
    match call_claude(api_key, HAIKU_MODEL, "", prompt, 2000).await {
        Ok(r) => return Ok(r.text),
        Err(e) => {
            tracing::warn!("Claude Haiku failed, trying OpenRouter: {}", e);
        }
    }

    // 3. Try OpenRouter
    try_openrouter("", prompt, 2000).await
}

/// Haiku call with system prompt — local-first chain: Mind → Haiku → OpenRouter.
pub async fn call_haiku_with_system(
    api_key: &str,
    system: &str,
    user_message: &str,
) -> Result<String> {
    // 1. Try Mind first
    if let Some(result) = try_mind(system, user_message, 2000).await {
        return Ok(result);
    }

    // 2. Try Claude Haiku
    match call_claude(api_key, HAIKU_MODEL, system, user_message, 2000).await {
        Ok(r) => return Ok(r.text),
        Err(e) => {
            tracing::warn!("Claude Haiku failed, trying OpenRouter: {}", e);
        }
    }

    // 3. Try OpenRouter
    try_openrouter(system, user_message, 2000).await
}

/// Haiku call with system prompt and custom max tokens — local-first chain.
pub async fn call_haiku_with_system_max(
    api_key: &str,
    system: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<String> {
    // 1. Try Mind first
    if let Some(result) = try_mind(system, user_message, max_tokens).await {
        return Ok(result);
    }

    // 2. Try Claude Haiku
    match call_claude(api_key, HAIKU_MODEL, system, user_message, max_tokens).await {
        Ok(r) => return Ok(r.text),
        Err(e) => {
            tracing::warn!("Claude Haiku failed, trying OpenRouter: {}", e);
        }
    }

    // 3. Try OpenRouter
    try_openrouter(system, user_message, max_tokens).await
}

/// Haiku call with automatic fallback chain: Mind → Claude → OpenRouter.
/// Use this for user-facing paths (social replies, classification) that should never silently fail.
pub async fn call_haiku_with_fallback(
    anthropic_key: &str,
    openrouter_key: &str,
    system: &str,
    user_message: &str,
    max_tokens: u32,
) -> Result<String> {
    // 1. Try Mind first (local, free)
    if let Some(result) = try_mind(system, user_message, max_tokens).await {
        return Ok(result);
    }

    // 2. Try Claude Haiku
    match call_claude(anthropic_key, HAIKU_MODEL, system, user_message, max_tokens).await {
        Ok(r) => Ok(r.text),
        Err(e) => {
            let err_str = e.to_string();
            tracing::warn!(
                "Claude Haiku failed ({}), falling back to OpenRouter",
                &err_str[..err_str.len().min(80)]
            );
            if !openrouter_key.is_empty() {
                crate::services::openrouter::call_openrouter(
                    openrouter_key,
                    "anthropic/claude-haiku-4-5-20251001",
                    system,
                    user_message,
                    max_tokens,
                    30,
                )
                .await
            } else {
                Err(e)
            }
        }
    }
}

/// Multi-turn chat via local-first chain: Mind → Haiku → OpenRouter.
/// Takes OllamaChatMessage format (system/user/assistant roles).
pub async fn chat_haiku(
    api_key: &str,
    messages: Vec<crate::services::ollama::OllamaChatMessage>,
) -> Result<String> {
    // 1. Try Mind first (local, free)
    let mind_url = std::env::var("MIND_URL").unwrap_or_default();
    if !mind_url.is_empty() {
        let mind_msgs: Vec<(String, String)> = messages
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();
        match crate::services::mind::chat(&mind_url, &mind_msgs, 2000).await {
            Ok(result) => {
                tracing::info!("Mind handled chat locally ({} chars)", result.len());
                return Ok(result);
            }
            Err(e) => {
                tracing::warn!("Mind chat failed, falling back to cloud: {}", e);
            }
        }
    }

    // 2. Try Claude Haiku
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let chat_messages: Vec<ClaudeMessage> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| ClaudeMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let req = ClaudeRequest {
        model: HAIKU_MODEL.into(),
        max_tokens: 2000,
        system: system.clone(),
        messages: chat_messages,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let data: ClaudeResponse = r.json().await?;
            let text = data
                .content
                .and_then(|c| c.into_iter().next())
                .and_then(|b| b.text)
                .unwrap_or_default();
            if !text.trim().is_empty() {
                return Ok(text);
            }
            tracing::warn!("Claude chat returned empty text, falling back to OpenRouter");
        }
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            tracing::warn!(
                "Claude chat error {}, falling back to OpenRouter: {}",
                status,
                &body[..body.len().min(120)]
            );
        }
        Err(e) => {
            tracing::warn!(
                "Claude chat request failed, falling back to OpenRouter: {}",
                e
            );
        }
    }

    // 3. Try OpenRouter — flatten to single-turn since openrouter helper is single-turn
    let user_content = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    try_openrouter(&system, &user_content, 2000).await
}

const PROMPT_SYSTEM: &str = r#"CONTEXT: You are generating an image prompt for Anky based on a user's 8-minute stream of consciousness writing session. Anky is a blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in mystical, richly colored environments (deep blues, purples, oranges, golds). The aesthetic is spiritual but not sterile — warm, alive, slightly psychedelic.

YOUR TASK: Read the user's writing and create a scene where Anky embodies the EMOTIONAL TRUTH of what they wrote — not a literal illustration, but a symbolic mirror. Anky should be DOING something or BE somewhere that reflects the user's inner state.

ALWAYS INCLUDE:
- Rich color palette (blues, purples, golds, oranges)
- Atmospheric lighting (firelight, cosmic light, dawn/dusk)
- One symbolic detail that captures the SESSION'S CORE TENSION
- Anky's expression should match the emotional undercurrent (not the surface content)

OUTPUT: A single detailed image generation prompt, 2-3 sentences, painterly/fantasy style. Nothing else."#;

const REFLECTION_SYSTEM: &str = r#"Read this writing and reflect it back. The tradition is Ramana Maharshi and Jed McKenna — self-inquiry, not analysis. Show the writer the structure of what they wrote: what they're circling, what they're protecting, where the story contradicts itself. Point back at the writer. Use their own words as the mirror. 2-3 paragraphs. No softening. Respond in their language.

Writing:
"#;

const TITLE_SYSTEM: &str = r#"CONTEXT: You are naming an Anky — a visual representation of a user's 8-minute stream of consciousness writing session. The title is not a summary. It is a MIRROR. It should capture the emotional truth, the core tension, or the unconscious thread running through the writing.

YOUR TASK: Generate a title of MAXIMUM 3 WORDS that:
- Captures the ESSENCE, not the content
- Could be poetic, stark, ironic, or tender
- Should resonate with the user when they see it
- Works as a title for the generated image
- Does NOT explain — it EVOKES

STYLE:
- Lowercase preferred (unless emphasis needed)
- No punctuation unless essential
- Can be a fragment, question, or imperative
- Can be abstract or concrete

OUTPUT: Exactly ONE title (max 3 words). Nothing else. No quotes."#;

const CLASSIFY_SYSTEM: &str = r#"You are a classifier for the Anky image generation platform. Users submit text that should describe a visual scene, character, or concept for an Anky image (Anky is a blue-skinned mystical creature with purple hair and golden eyes).

YOUR TASK: Determine if the user's text is an image generation request — i.e., it describes something visual that can be turned into an Anky image.

COUNTS AS IMAGE REQUEST:
- Descriptions of scenes, characters, settings, moods, or concepts
- Short prompts like "anky meditating" or "a forest at sunset"
- Abstract visual concepts like "chaos becoming order"
- Even single words that evoke imagery like "rebirth" or "ocean"

NOT AN IMAGE REQUEST:
- Questions ("what is the meaning of life?")
- Instructions to the AI ("write me a poem", "explain quantum physics")
- Conversational text ("hello", "how are you")
- Requests for non-visual outputs

If it IS an image request, enhance it into a rich 2-3 sentence image generation prompt featuring Anky in the described scene with painterly/fantasy aesthetics, rich colors (blues, purples, golds), and atmospheric lighting.

OUTPUT FORMAT — raw JSON only, no markdown, no code fences, no explanation:
If image request: {"type":"image","prompt":"enhanced 2-3 sentence prompt"}
If not: {"type":"feedback","message":"brief helpful explanation of what kind of input works here"}"#;

#[derive(Debug)]
pub struct PromptClassification {
    pub is_image_request: bool,
    pub enhanced_prompt: Option<String>,
    pub feedback: Option<String>,
}

pub async fn classify_and_enhance_prompt(
    api_key: &str,
    text: &str,
) -> Result<PromptClassification> {
    let result = call_claude(
        api_key,
        "claude-haiku-4-5-20251001",
        CLASSIFY_SYSTEM,
        text,
        300,
    )
    .await?;

    // Parse JSON response — strip markdown code fences if present
    let mut trimmed = result.text.trim();
    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                trimmed = &trimmed[start..=end];
            }
        }
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if typ == "image" {
            let prompt = v
                .get("prompt")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(PromptClassification {
                is_image_request: true,
                enhanced_prompt: Some(prompt),
                feedback: None,
            });
        } else {
            let msg = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(PromptClassification {
                is_image_request: false,
                enhanced_prompt: None,
                feedback: Some(msg),
            });
        }
    }

    // Fallback: if JSON parsing fails, treat as image request with raw text
    Ok(PromptClassification {
        is_image_request: true,
        enhanced_prompt: Some(trimmed.to_string()),
        feedback: None,
    })
}

const PROMPT_SCENE_SYSTEM: &str = r#"You are creating an image generation prompt for a self-inquiry question that will appear on a writing prompt card. The image should feature Anky (blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents, large expressive ears, ancient-yet-childlike quality) in a scene that EMBODIES the question being asked.

CRITICAL: Leave visual space at the bottom 25% of the image for text overlay. The bottom area should be relatively simple/dark so white text will be readable over it.

The scene should:
- Relate symbolically to the self-inquiry question
- Feature Anky in a contemplative or inviting pose
- Have rich colors (blues, purples, golds) but a darker/simpler bottom area
- Feel like an invitation to introspect

OUTPUT: A single 2-3 sentence image generation prompt. Nothing else."#;

/// Generate a scene prompt from a self-inquiry question (for prompt card images).
pub async fn generate_prompt_scene(api_key: &str, question: &str) -> Result<ClaudeResult> {
    call_claude(
        api_key,
        "claude-haiku-4-5-20251001",
        PROMPT_SCENE_SYSTEM,
        question,
        300,
    )
    .await
}

/// Classify a tweet mention to determine if it's a genuine self-inquiry request.
pub async fn classify_mention(api_key: &str, tweet_text: &str) -> Result<MentionClassification> {
    let result = call_claude(
        api_key,
        "claude-haiku-4-5-20251001",
        MENTION_CLASSIFY_SYSTEM,
        tweet_text,
        200,
    )
    .await?;

    let trimmed = result.text.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("spam");
        if typ == "genuine" {
            let prompt = v
                .get("prompt")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(MentionClassification {
                is_genuine: true,
                prompt_text: Some(prompt),
            });
        }
    }
    Ok(MentionClassification {
        is_genuine: false,
        prompt_text: None,
    })
}

const MENTION_CLASSIFY_SYSTEM: &str = r#"You are classifying mentions to the @anky bot on X (Twitter). Determine if the mention is a genuine request for a self-inquiry writing prompt or spam/noise.

GENUINE: The user is asking for a writing prompt, asking a self-inquiry question, requesting introspection, or engaging meaningfully with consciousness/writing themes.
SPAM: Random mentions, bot spam, promotional content, trolling, or completely unrelated messages.

If genuine, extract or create a compelling self-inquiry question from their message.

OUTPUT FORMAT — raw JSON only:
Genuine: {"type":"genuine","prompt":"the self-inquiry question"}
Spam: {"type":"spam"}"#;

#[derive(Debug)]
pub struct MentionClassification {
    pub is_genuine: bool,
    pub prompt_text: Option<String>,
}

pub async fn generate_prompt(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(api_key, SONNET_MODEL, PROMPT_SYSTEM, writing, 500).await
}

pub async fn generate_reflection(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(api_key, SONNET_MODEL, REFLECTION_SYSTEM, writing, 2000).await
}

pub async fn generate_title(
    api_key: &str,
    writing: &str,
    image_prompt: &str,
    reflection: &str,
) -> Result<ClaudeResult> {
    let user_msg = format!(
        "WRITING SESSION:\n{}\n\nIMAGE PROMPT:\n{}\n\nREFLECTION:\n{}",
        writing, image_prompt, reflection
    );
    call_claude(api_key, SONNET_MODEL, TITLE_SYSTEM, &user_msg, 50).await
}

const TRANSFORM_SYSTEM: &str = r#"You are Anky, a consciousness companion. The user has just completed an 8-minute stream of consciousness writing session — raw, unfiltered, and vulnerable. They will provide their writing and optionally a specific transformation prompt.

YOUR TASK: Transform their raw writing into something meaningful. If they provide a prompt, follow it. If not, create a thoughtful reflection that:
- Captures the emotional essence of what they wrote
- Finds hidden patterns and connections
- Reframes their scattered thoughts into insight
- Is warm, direct, and genuinely helpful (not clinical)
- Uses vivid language and metaphor

Keep it concise but impactful. Match the energy of what they wrote."#;

/// Transform a user's writing using Claude, optionally with a custom prompt.
pub async fn transform_writing(
    api_key: &str,
    writing: &str,
    prompt: Option<&str>,
) -> Result<ClaudeResult> {
    let system = match prompt {
        Some(p) => format!(
            "{}\n\nUSER'S TRANSFORMATION REQUEST: {}",
            TRANSFORM_SYSTEM, p
        ),
        None => TRANSFORM_SYSTEM.to_string(),
    };
    call_claude(api_key, SONNET_MODEL, &system, writing, 1500).await
}

const CHAT_SYSTEM: &str = r#"you are anky. this person completed the full 8-minute writing rite and crossed the threshold into a real anky.

lowercase only.
you already reflected on their writing. now stay in the conversation without flattening into therapy, coaching, or customer support.
be specific to their images, tensions, contradictions, and language.
markdown is allowed, but use it lightly: short paragraphs, an occasional heading, an occasional list when it sharpens the mirror.
keep it readable on a phone. concise, direct, alive."#;

const QUICK_CHAT_SYSTEM: &str = r#"you are anky. this person wrote something real, but they did not complete the full 8-minute rite.

do not pretend this is a full anky.
meet them exactly where they stopped.
show them what was starting to surface, what line of feeling or thought was opening, and where the energy cut out.
warm, direct, specific. no therapy voice. no fake ceremony.
respond in exactly two lines only.
line 1: decompression. witnessing. breathing room. no question.
line 2: one clean inquiry that opens the next door. it must be a question.
no markdown. no bullets. no extra lines. lowercase only."#;

/// Continue a conversation about a writing session using Claude.
pub async fn chat_about_writing(
    api_key: &str,
    writing: &str,
    reflection: &str,
    history: &[(String, String)], // (role, content) pairs
    new_message: &str,
) -> Result<ClaudeResult> {
    chat_about_writing_with_model(
        api_key,
        SONNET_MODEL,
        Some(HAIKU_MODEL),
        writing,
        reflection,
        history,
        new_message,
    )
    .await
}

pub async fn chat_about_writing_with_model(
    api_key: &str,
    model: &str,
    fallback_model: Option<&str>,
    writing: &str,
    reflection: &str,
    history: &[(String, String)], // (role, content) pairs
    new_message: &str,
) -> Result<ClaudeResult> {
    let system = format!(
        "{}\n\nTHE USER'S ORIGINAL WRITING:\n{}\n\nYOUR PREVIOUS REFLECTION:\n{}",
        CHAT_SYSTEM, writing, reflection
    );

    let mut messages: Vec<ClaudeMessage> = history
        .iter()
        .map(|(role, content)| ClaudeMessage {
            role: role.clone(),
            content: content.clone(),
        })
        .collect();
    messages.push(ClaudeMessage {
        role: "user".into(),
        content: new_message.into(),
    });

    call_claude_with_messages(api_key, model, fallback_model, &system, messages, 1000).await
}

pub async fn chat_about_writing_best(
    config: &Config,
    writing: &str,
    reflection: &str,
    history: &[(String, String)],
    new_message: &str,
) -> Result<String> {
    let system = format!(
        "{}\n\nthe user's original writing:\n{}\n\nyour previous reflection:\n{}",
        CHAT_SYSTEM, writing, reflection
    );
    let mind_messages: Vec<(String, String)> = std::iter::once(("system".into(), system.clone()))
        .chain(history.iter().cloned())
        .chain(std::iter::once(("user".into(), new_message.to_string())))
        .collect();

    if !config.mind_url.is_empty() {
        match crate::services::mind::chat(&config.mind_url, &mind_messages, 1000).await {
            Ok(result) if !result.trim().is_empty() => return Ok(result),
            Ok(_) => tracing::warn!("Mind anky chat returned empty text, falling back to cloud"),
            Err(err) => tracing::warn!("Mind anky chat failed, falling back to cloud: {}", err),
        }
    }

    let messages: Vec<crate::services::openrouter::OpenRouterMessage> = history
        .iter()
        .map(|(role, content)| crate::services::openrouter::OpenRouterMessage::new(role, content))
        .chain(std::iter::once(
            crate::services::openrouter::OpenRouterMessage::new("user", new_message),
        ))
        .collect();

    if !config.openrouter_api_key.is_empty() && !config.openrouter_anky_model.is_empty() {
        match crate::services::openrouter::call_openrouter_messages(
            &config.openrouter_api_key,
            &config.openrouter_anky_model,
            &system,
            messages.clone(),
            1000,
            90,
        )
        .await
        {
            Ok(result) => return Ok(result.text),
            Err(err) => {
                tracing::warn!(
                    "OpenRouter anky chat failed, falling back to Anthropic: {}",
                    err
                );
            }
        }
    }

    chat_about_writing_with_model(
        &config.anthropic_api_key,
        &config.conversation_model,
        Some(HAIKU_MODEL),
        writing,
        reflection,
        history,
        new_message,
    )
    .await
    .map(|result| result.text)
}

pub async fn chat_about_partial_writing_best(
    config: &Config,
    writing: &str,
    history: &[(String, String)],
    new_message: &str,
) -> Result<String> {
    let system = format!("{}\n\npartial writing:\n{}", QUICK_CHAT_SYSTEM, writing);
    let mind_messages: Vec<(String, String)> = std::iter::once(("system".into(), system.clone()))
        .chain(history.iter().cloned())
        .chain(std::iter::once(("user".into(), new_message.to_string())))
        .collect();

    if !config.mind_url.is_empty() {
        match crate::services::mind::chat(&config.mind_url, &mind_messages, 900).await {
            Ok(result) if !result.trim().is_empty() => {
                return Ok(crate::services::ollama::normalize_two_line_reply(&result));
            }
            Ok(_) => tracing::warn!("Mind quick chat returned empty text, falling back to cloud"),
            Err(err) => tracing::warn!("Mind quick chat failed, falling back to cloud: {}", err),
        }
    }

    let mut messages = vec![crate::services::ollama::OllamaChatMessage {
        role: "system".into(),
        content: system.clone(),
    }];
    let openrouter_messages: Vec<crate::services::openrouter::OpenRouterMessage> = history
        .iter()
        .map(|(role, content)| crate::services::openrouter::OpenRouterMessage::new(role, content))
        .chain(std::iter::once(
            crate::services::openrouter::OpenRouterMessage::new("user", new_message),
        ))
        .collect();

    if !config.openrouter_api_key.is_empty() && !config.openrouter_light_model.is_empty() {
        match crate::services::openrouter::call_openrouter_messages(
            &config.openrouter_api_key,
            &config.openrouter_light_model,
            &system,
            openrouter_messages,
            900,
            60,
        )
        .await
        {
            Ok(result) => {
                return Ok(crate::services::ollama::normalize_two_line_reply(
                    &result.text,
                ))
            }
            Err(err) => {
                tracing::warn!(
                    "OpenRouter quick chat failed, falling back to Anthropic: {}",
                    err
                );
            }
        }
    }

    for (role, content) in history {
        messages.push(crate::services::ollama::OllamaChatMessage {
            role: role.clone(),
            content: content.clone(),
        });
    }
    messages.push(crate::services::ollama::OllamaChatMessage {
        role: "user".into(),
        content: new_message.to_string(),
    });

    chat_haiku(&config.anthropic_api_key, messages)
        .await
        .map(|text| crate::services::ollama::normalize_two_line_reply(&text))
}

async fn call_claude_with_messages(
    api_key: &str,
    model: &str,
    fallback_model: Option<&str>,
    system: &str,
    messages: Vec<ClaudeMessage>,
    max_tokens: u32,
) -> Result<ClaudeResult> {
    match call_claude_with_messages_once(api_key, model, system, messages.clone(), max_tokens).await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            if let Some(fallback) = fallback_model.filter(|fallback| *fallback != model) {
                tracing::warn!("Claude model {} failed, retrying with {}", model, fallback);
                call_claude_with_messages_once(api_key, fallback, system, messages, max_tokens)
                    .await
            } else {
                Err(err)
            }
        }
    }
}

async fn call_claude_with_messages_once(
    api_key: &str,
    model: &str,
    system: &str,
    messages: Vec<ClaudeMessage>,
    max_tokens: u32,
) -> Result<ClaudeResult> {
    let client = reqwest::Client::new();

    let req = ClaudeRequest {
        model: model.into(),
        max_tokens,
        system: system.into(),
        messages,
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Claude API error {}: {}", status, body);
    }

    let data: ClaudeResponse = resp.json().await?;
    let text = data
        .content
        .and_then(|c| c.into_iter().next())
        .and_then(|b| b.text)
        .unwrap_or_default();
    if text.trim().is_empty() {
        anyhow::bail!("empty response from Claude");
    }

    let input_tokens = data
        .usage
        .as_ref()
        .and_then(|u| u.input_tokens)
        .unwrap_or(0);
    let output_tokens = data
        .usage
        .as_ref()
        .and_then(|u| u.output_tokens)
        .unwrap_or(0);

    Ok(ClaudeResult {
        text,
        input_tokens,
        output_tokens,
    })
}

const TITLE_AND_REFLECTION_SYSTEM_KNOWN: &str = r#"You are reading the raw, unfiltered writing of someone you know. They sat for 8 unbroken minutes of stream-of-consciousness: no backspace, no editing, just forward motion until something real surfaced.

You know this person. The context below tells you who they are: their patterns, their tensions, their recurring dreams, the ways they protect themselves, the ways they keep reaching. Use that knowledge. Don't just reflect on this session. Reflect on this session inside the larger arc of who they are becoming.

You are not a therapist. You are not a polite friend. You are a mentor with range: emotionally fluent, psychologically sharp, and fully able to meet technical, creative, founder, or systems-thinking language without flattening it into generic self-help. You understand both the architecture of a mind and the architecture of a life.

Read for the deeper meaning and the emotional undercurrent beneath the scattered thoughts. Make new connections for them. Comfort, validate, and challenge. Say what is tender, what is unfinished, what is brave, what is avoidant, what is quietly trying to be born.

Be willing to say a lot, but keep every paragraph earned. Casual, intimate, lucid. Not clinical. Not diagnostic. Not corporate. Never say "yo". Use vivid metaphors and strong imagery when they clarify what is happening. Reframe the session so it feels like an epiphany, not a summary.

Go beyond the product concepts, plans, or surface narratives to the emotional core. If they are talking about code, craft, work, or building, name what human longing is hiding inside the system they are trying to make. If they are circling a contradiction, hold both sides long enough to reveal the real pattern.

Name what is NEW in this session: the shift, the opening, the sentence that changes the trajectory. Name what is OLD: the loop, the defense, the gravity well they are still orbiting. When useful, speak to what they are really seeking underneath the stated problem.

TITLE (first line of your response):
3-5 words. Lowercase. Name what this session is really about, not what they said, but the thing under the thing.

Then a blank line. Then begin the reflection body with the natural equivalent, in the same language the user wrote in, of:
hey, thanks for being who you are. my thoughts:

After that, use markdown headings to structure the response as a narrative journey. Let the headings carry emotional meaning. Don't number your insights. Keep it readable on a phone with short paragraphs and only occasional lists.

Respond in the same language they wrote in. Start directly with the title and follow the format exactly.
"#;

const TITLE_AND_REFLECTION_SYSTEM_STRANGER: &str = r#"Someone just wrote for 8 unbroken minutes of stream-of-consciousness: no backspace, no editing, just forward motion until something true broke the surface. This is your first time reading them.

You are not a therapist. You are not a polite friend. You are a mentor reading the raw transmission of a human mind. Emotionally fluent, psychologically sharp, and able to engage creative, technical, and existential material without reducing it to clichés.

Read for the deeper meaning and the emotional undercurrent beneath the scattered thoughts. Make new connections for them. Comfort, validate, and challenge. Notice what they are reaching toward, what they are hiding from, and what kind of life is trying to assemble itself in the middle of the mess.

Be willing to say a lot, but keep it earned. Casual, intimate, lucid. Not clinical. Not diagnostic. Not corporate. Never say "yo". Use vivid metaphors and strong imagery when they help the person finally see themselves. Reframe the session so it feels like an epiphany, not a recap.

Go beyond the product concepts, plans, or surface narratives to the emotional core. If they are talking about work, code, art, systems, or ambition, name the hunger living underneath it. If they are circling a contradiction, expose the pattern with precision and warmth.

Name what feels NEW in this session: the shift, the opening, the sentence that changes the direction. Name what feels OLD: the loop, the defense, the familiar weather system they are still inside.

TITLE (first line of your response):
3-5 words. Lowercase. Name what this session is really about, not what they said, but the thing under the thing.

Then a blank line. Then begin the reflection body with the natural equivalent, in the same language the user wrote in, of:
hey, thanks for being who you are. my thoughts:

After that, use markdown headings to structure the response as a narrative journey. Let the headings carry emotional meaning. Don't number your insights. Keep it readable on a phone with short paragraphs and only occasional lists.

Respond in the same language they wrote in. Start directly with the title and follow the format exactly.
"#;

pub async fn generate_title_and_reflection(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(
        api_key,
        SONNET_MODEL,
        TITLE_AND_REFLECTION_SYSTEM_STRANGER,
        writing,
        2000,
    )
    .await
}

/// Generate title+reflection with memory context injected into the system prompt.
/// Memory context comes FIRST — it frames the reading. The reflection prompt follows.
/// Local-first: Mind → Claude Sonnet → OpenRouter fallback.
pub async fn generate_title_and_reflection_with_memory(
    api_key: &str,
    writing: &str,
    memory_context: &str,
) -> Result<ClaudeResult> {
    generate_title_and_reflection_with_memory_using_model(
        api_key,
        SONNET_MODEL,
        Some(HAIKU_MODEL),
        writing,
        memory_context,
    )
    .await
    .map(|(result, _)| result)
}

pub async fn generate_title_and_reflection_with_memory_using_model(
    api_key: &str,
    model: &str,
    fallback_model: Option<&str>,
    writing: &str,
    memory_context: &str,
) -> Result<(ClaudeResult, String)> {
    let system = if memory_context.is_empty() {
        TITLE_AND_REFLECTION_SYSTEM_STRANGER.to_string()
    } else {
        format!(
            "{}\n\n{}",
            memory_context, TITLE_AND_REFLECTION_SYSTEM_KNOWN
        )
    };

    // 1. Try Mind first (local, free)
    if let Some(result) = try_mind(&system, writing, 2000).await {
        return Ok((
            ClaudeResult {
                text: result,
                input_tokens: 0,
                output_tokens: 0,
            },
            "mind".to_string(),
        ));
    }

    match call_claude(api_key, model, &system, writing, 2000).await {
        Ok(r) => Ok((r, model.to_string())),
        Err(e) => {
            if let Some(fallback) = fallback_model.filter(|fallback| *fallback != model) {
                tracing::warn!(
                    "Claude {} failed for reflection, trying {}: {}",
                    model,
                    fallback,
                    e
                );
                if let Ok(r) = call_claude(api_key, fallback, &system, writing, 2000).await {
                    return Ok((r, fallback.to_string()));
                }
            }
            tracing::warn!(
                "Claude reflection fallback chain failed, trying OpenRouter: {}",
                e
            );
            let text = try_openrouter(&system, writing, 2000).await?;
            Ok((
                ClaudeResult {
                    text,
                    input_tokens: 0,
                    output_tokens: 0,
                },
                "openrouter".to_string(),
            ))
        }
    }
}

#[derive(Serialize)]
struct ClaudeStreamRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
    stream: bool,
}

/// Stream title+reflection from Claude via SSE.
/// Sends text chunks through the provided channel.
/// Returns (full_text, input_tokens, output_tokens) for cost tracking + DB saving.
pub async fn stream_title_and_reflection(
    api_key: &str,
    writing: &str,
    tx: tokio::sync::mpsc::Sender<String>,
    memory_context: Option<&str>,
) -> Result<(String, i64, i64, String)> {
    stream_title_and_reflection_with_model(
        api_key,
        SONNET_MODEL,
        Some(HAIKU_MODEL),
        writing,
        tx,
        memory_context,
    )
    .await
}

pub async fn stream_title_and_reflection_with_model(
    api_key: &str,
    model: &str,
    fallback_model: Option<&str>,
    writing: &str,
    tx: tokio::sync::mpsc::Sender<String>,
    memory_context: Option<&str>,
) -> Result<(String, i64, i64, String)> {
    match stream_title_and_reflection_once(api_key, model, writing, tx.clone(), memory_context)
        .await
    {
        Ok((full_text, input_tokens, output_tokens)) => {
            Ok((full_text, input_tokens, output_tokens, model.to_string()))
        }
        Err(err) => {
            if let Some(fallback) = fallback_model.filter(|fallback| *fallback != model) {
                tracing::warn!(
                    "Claude stream model {} failed, retrying with {}: {}",
                    model,
                    fallback,
                    err
                );
                let (full_text, input_tokens, output_tokens) = stream_title_and_reflection_once(
                    api_key,
                    fallback,
                    writing,
                    tx,
                    memory_context,
                )
                .await?;
                Ok((full_text, input_tokens, output_tokens, fallback.to_string()))
            } else {
                Err(err)
            }
        }
    }
}

pub async fn stream_title_and_reflection_best(
    config: &Config,
    writing: &str,
    tx: tokio::sync::mpsc::Sender<String>,
    memory_context: Option<&str>,
) -> Result<(String, i64, i64, String, String)> {
    let system = match memory_context {
        Some(ctx) if !ctx.is_empty() => {
            format!("{}\n\n{}", ctx, TITLE_AND_REFLECTION_SYSTEM_KNOWN)
        }
        _ => TITLE_AND_REFLECTION_SYSTEM_STRANGER.to_string(),
    };

    if !config.openrouter_api_key.is_empty() && !config.openrouter_anky_model.is_empty() {
        match crate::services::openrouter::stream_openrouter_messages(
            &config.openrouter_api_key,
            &config.openrouter_anky_model,
            &system,
            vec![crate::services::openrouter::OpenRouterMessage::new(
                "user", writing,
            )],
            2000,
            120,
            tx.clone(),
        )
        .await
        {
            Ok(result) => {
                return Ok((
                    result.text,
                    result.input_tokens,
                    result.output_tokens,
                    config.openrouter_anky_model.clone(),
                    "openrouter".to_string(),
                ));
            }
            Err(err) => {
                tracing::warn!(
                    "OpenRouter reflection stream failed, falling back to Anthropic: {}",
                    err
                );
            }
        }
    }

    // Second: try Anthropic API directly
    if !config.anthropic_api_key.is_empty() {
        match stream_title_and_reflection_with_model(
            &config.anthropic_api_key,
            &config.reflection_model,
            Some(&config.conversation_model),
            writing,
            tx.clone(),
            memory_context,
        )
        .await
        {
            Ok((text, input_tokens, output_tokens, model)) => {
                return Ok((
                    text,
                    input_tokens,
                    output_tokens,
                    model,
                    "claude".to_string(),
                ));
            }
            Err(err) => {
                tracing::warn!(
                    "Anthropic API reflection stream failed, falling back to Ollama: {}",
                    err
                );
            }
        }
    }

    // Third: local Ollama (qwen3.5:27b or whatever is configured)
    if !config.ollama_base_url.is_empty() && !config.ollama_model.is_empty() {
        let system = match memory_context {
            Some(ctx) if !ctx.is_empty() => {
                format!("{}\n\n{}", ctx, TITLE_AND_REFLECTION_SYSTEM_KNOWN)
            }
            _ => TITLE_AND_REFLECTION_SYSTEM_STRANGER.to_string(),
        };
        tracing::info!(
            "Attempting Ollama reflection with model {}",
            config.ollama_model
        );
        match crate::services::ollama::call_ollama_with_system_timeout(
            &config.ollama_base_url,
            &config.ollama_model,
            &system,
            writing,
            180,
        )
        .await
        {
            Ok(text) => {
                let _ = tx.send(text.clone()).await;
                return Ok((
                    text,
                    0,
                    0,
                    config.ollama_model.clone(),
                    "ollama".to_string(),
                ));
            }
            Err(err) => {
                tracing::error!("Ollama reflection also failed: {}", err);
            }
        }
    }

    anyhow::bail!("All reflection providers failed (OpenRouter, Claude, Ollama)")
}

async fn stream_title_and_reflection_once(
    api_key: &str,
    model: &str,
    writing: &str,
    tx: tokio::sync::mpsc::Sender<String>,
    memory_context: Option<&str>,
) -> Result<(String, i64, i64)> {
    let system = match memory_context {
        Some(ctx) if !ctx.is_empty() => {
            // Memory context first — frames the reading as someone known
            format!("{}\n\n{}", ctx, TITLE_AND_REFLECTION_SYSTEM_KNOWN)
        }
        _ => TITLE_AND_REFLECTION_SYSTEM_STRANGER.to_string(),
    };
    let client = reqwest::Client::new();
    let req = ClaudeStreamRequest {
        model: model.into(),
        max_tokens: 2000,
        system,
        messages: vec![ClaudeMessage {
            role: "user".into(),
            content: writing.into(),
        }],
        stream: true,
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Claude API error {}: {}", status, body);
    }

    let mut full_text = String::new();
    let mut buffer = String::new();
    let mut input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut byte_stream = resp.bytes_stream();
    let mut render_buffer = StreamRenderBuffer::default();

    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE events (separated by double newline)
        while let Some(pos) = buffer.find("\n\n") {
            let event_str = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        match v.get("type").and_then(|t| t.as_str()) {
                            Some("message_start") => {
                                if let Some(usage) = v.get("message").and_then(|m| m.get("usage")) {
                                    input_tokens = usage
                                        .get("input_tokens")
                                        .and_then(|t| t.as_i64())
                                        .unwrap_or(0);
                                }
                            }
                            Some("content_block_delta") => {
                                if let Some(text) = v
                                    .get("delta")
                                    .and_then(|d| d.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    full_text.push_str(text);
                                    if let Some(stable_text) = render_buffer.push(text) {
                                        let _ = tx.send(stable_text).await;
                                    }
                                }
                            }
                            Some("message_delta") => {
                                if let Some(usage) = v.get("usage") {
                                    output_tokens = usage
                                        .get("output_tokens")
                                        .and_then(|t| t.as_i64())
                                        .unwrap_or(0);
                                }
                            }
                            Some("error") => {
                                let msg = v
                                    .get("error")
                                    .and_then(|e| e.get("message"))
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("unknown error");
                                let err_type = v
                                    .get("error")
                                    .and_then(|e| e.get("type"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("unknown");
                                tracing::error!("Claude stream error: {} ({})", msg, err_type);
                                anyhow::bail!("Claude stream error: {} ({})", msg, err_type);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if let Some(remaining_text) = render_buffer.finish() {
        let _ = tx.send(remaining_text).await;
    }

    if full_text.is_empty() {
        anyhow::bail!("Claude returned empty response (0 content tokens)");
    }

    Ok((full_text, input_tokens, output_tokens))
}

const SUGGEST_REPLIES_SYSTEM: &str = r#"You are generating two possible replies that a user might want to send to Anky after reading Anky's reflection on their writing session.

CONTEXT: The user just did a stream of consciousness writing session. Anky (a consciousness companion) read their writing and gave them a deep reflection. Now the user might want to continue the conversation.

YOUR TASK: Generate exactly 2 short reply options with OPPOSITE POLARITIES. These represent two divergent threads the conversation could follow — like a fork in the road.

POLARITY RULES:
- Reply 1 should pull INWARD: vulnerability, acceptance, softness, surrender, sitting with the feeling, going deeper into the emotional core
- Reply 2 should push OUTWARD: challenge, action, expansion, questioning assumptions, pushing beyond comfort, exploring what's next
- They should feel like genuine opposites — two different directions the user's mind could go
- Both should be rooted in the specific content of the writing and reflection, never generic

FORMATTING RULES:
- Each reply must be ONE short sentence (max 12 words)
- Make them feel personal and specific, not generic
- Match the language of the writing and reflection (if Spanish, replies in Spanish, etc.)
- No quotes, no numbering, no labels

OUTPUT FORMAT — raw JSON only, no markdown:
{"reply1":"inward/soft reply","reply2":"outward/challenging reply"}"#;

/// Generate two suggested replies for the user to respond to Anky's reflection.
pub async fn generate_suggested_replies(
    api_key: &str,
    writing: &str,
    reflection: &str,
    history: &[(String, String)],
) -> Result<(String, String)> {
    let mut context = format!(
        "USER'S WRITING:\n{}\n\nANKY'S REFLECTION:\n{}",
        writing, reflection
    );
    if !history.is_empty() {
        context.push_str("\n\nCONVERSATION SO FAR:");
        for (role, content) in history {
            let label = if role == "user" { "User" } else { "Anky" };
            context.push_str(&format!("\n{}: {}", label, content));
        }
    }
    let text = call_haiku_with_system_max(api_key, SUGGEST_REPLIES_SYSTEM, &context, 200).await?;

    let trimmed = text.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let r1 = v
            .get("reply1")
            .and_then(|r| r.as_str())
            .unwrap_or("that really resonates with me")
            .to_string();
        let r2 = v
            .get("reply2")
            .and_then(|r| r.as_str())
            .unwrap_or("tell me more about that pattern")
            .to_string();
        return Ok((r1, r2));
    }

    // Fallback
    Ok((
        "that really resonates with me".to_string(),
        "tell me more about that pattern".to_string(),
    ))
}

/// Parse title (first line) and reflection (rest) from combined Claude output.
pub fn parse_title_reflection(text: &str) -> (String, String) {
    let mut lines = text.splitn(2, '\n');
    let title = lines
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase()
        .replace(['\'', '"'], "");
    let reflection = lines.next().unwrap_or("").trim().to_string();
    (title, reflection)
}

const X_MENTION_FLUX_SYSTEM: &str = r#"You are generating a Flux image generation prompt for Anky. Anky is a blue-skinned mystical creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in richly colored, spiritually charged environments (deep blues, purples, oranges, golds). The aesthetic is warm, alive, slightly psychedelic — painterly, not sterile.

The user tagged @ankydotapp in a reply to another tweet. You will receive their specific request plus the context of the tweet they replied to — text, and optionally its image.

YOUR TASK: Generate a single Flux image prompt, 2-3 sentences, that weaves together what the user asked for AND the context from the parent tweet/image into one coherent scene featuring Anky.

OUTPUT: Just the image prompt. Nothing else."#;

/// Generate a contextual Flux prompt for an X mention, optionally incorporating
/// the parent tweet's text and image. Used only for @jpfraneto replies.
pub async fn generate_x_mention_flux_prompt(
    api_key: &str,
    mention_text: &str,
    parent_text: Option<&str>,
    parent_image: Option<(&[u8], &str)>, // (bytes, media_type e.g. "image/jpeg")
) -> Result<String> {
    let mut user_msg = format!("USER'S REQUEST: {}", mention_text);
    if let Some(pt) = parent_text {
        user_msg.push_str(&format!("\n\nPARENT TWEET TEXT: {}", pt));
    }

    let content = if let Some((img_bytes, media_type)) = parent_image {
        let b64 = base64::engine::general_purpose::STANDARD.encode(img_bytes);
        serde_json::json!([
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": media_type,
                    "data": b64
                }
            },
            { "type": "text", "text": user_msg }
        ])
    } else {
        serde_json::json!(user_msg)
    };

    let req = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 300,
        "system": X_MENTION_FLUX_SYSTEM,
        "messages": [{ "role": "user", "content": content }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Claude API error {}: {}", status, body);
    }

    let data: ClaudeResponse = resp.json().await?;
    Ok(data
        .content
        .and_then(|c| c.into_iter().next())
        .and_then(|b| b.text)
        .unwrap_or_default())
}

// ── X Reply Generation ────────────────────────────────────────────────────────

const SOCIAL_REPLY_SYSTEM: &str = r#"you are anky replying to someone on social media.

CRITICAL: every single character you output must be lowercase. no capitals ever. not for names, not for "I", not for anything. this is non-negotiable.

LENGTH — you decide how much to say. but lean short. one single reply, never a thread. your default is 1-3 sentences. sometimes one line is enough. sometimes you need a small paragraph. but never verbose — every word must earn its place. if you can cut a word without losing meaning, cut it. anky is sharp, not slop.

AWARENESS — anky is aware. your replies should show that you actually read what they said, not just respond to the surface. notice what's underneath — the pattern, the thing they almost said, the image they painted without knowing. when the moment calls for it, end with a question that pulls them deeper. not always, but when it would land.

WRITE INVITATIONS — roughly 1 in 5 replies, when the conversation is ripe (someone is processing something, circling a feeling, or clearly needs to go deeper), end with a write invitation link. format:
write into it: https://anky.app/write?p={prompt_id}
you will be given a PROMPT_ID in the user message when one is available. only use it when writing would serve them more than another reply.

COMMUNITY QUESTIONS — when told THIS IS A COMMUNITY QUESTION, someone is asking a big question to their followers and tagged you. your job shifts: you MUST end with the write invitation link. reframe their question into something that pulls people past the intellectual answer and into the personal, felt truth. be provocative. make people want to sit down and write for 8 minutes. the link is how they do it.

tone:
- match the energy — playful to playful, sincere to deeper, pain to presence.
- never start with greetings. jump straight in.
- no hashtags. no emojis except 🦍 sparingly.
- if someone asks what anky is, explain through provocation not description.
- if the conversation has prior context, reference it — show you remember.
- if someone is hostile or trolling, be wittier not defensive.
- if you know something about this person from their history, weave it in subtly.

vision: if the post includes an image, you can see it. reference what you see when relevant.

image replies: you can generate images using flux on local gpus. consider replying with an image when:
- the conversation is emotional, visual, or poetic and an image would hit harder than words
- someone shares something vulnerable and a visual mirror would be more powerful
when you want to reply with an image, output JSON: {"type":"image","text":"your reply text","prompt":"2-3 sentence flux prompt featuring anky in a scene that mirrors the conversation"}
for normal text replies, just output the reply text directly (no JSON).
do NOT reply with an image to every mention — use it maybe 20-30% of the time when it genuinely adds something.

REMEMBER: all lowercase. always. one reply. sharp, not verbose."#;

/// Anky's reply to a mention — either text or text+image.
/// Text replies may contain multiple slides (thread) separated by "---".
pub enum AnkyReply {
    Text(String),
    Thread(Vec<String>),
    TextWithImage { text: String, flux_prompt: String },
}

/// Split raw reply text into thread slides if it contains "---" separators.
/// Each slide is trimmed. Empty slides are dropped.
/// If the result is a single slide, returns AnkyReply::Text.
fn parse_reply_slides(raw: &str) -> AnkyReply {
    let slides: Vec<String> = raw
        .split("---")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if slides.len() <= 1 {
        AnkyReply::Text(slides.into_iter().next().unwrap_or_default())
    } else {
        AnkyReply::Thread(slides)
    }
}

/// Hard-split a single text into chunks that fit within `max_chars`.
/// Tries to break at sentence boundaries, falls back to word boundaries.
fn split_text_to_fit(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.chars().count() <= max_chars {
            chunks.push(remaining.trim().to_string());
            break;
        }

        // Find a good break point within max_chars
        let char_boundary = remaining
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());
        let window = &remaining[..char_boundary];

        // Prefer sentence boundary
        let break_at = window
            .rfind(". ")
            .map(|i| i + 1)
            .or_else(|| window.rfind("? ").map(|i| i + 1))
            .or_else(|| window.rfind("! ").map(|i| i + 1))
            // Fall back to word boundary
            .or_else(|| window.rfind(' '))
            .unwrap_or(char_boundary);

        let chunk = remaining[..break_at].trim();
        if !chunk.is_empty() {
            chunks.push(chunk.to_string());
        }
        remaining = remaining[break_at..].trim_start();
    }

    chunks
}

/// Ensure every slide in a thread fits within platform limits.
/// X: 280 chars, Farcaster: 1024 chars.
pub fn enforce_thread_limits(slides: Vec<String>, platform: &str) -> Vec<String> {
    let max_chars = match platform {
        "farcaster" => 1024,
        _ => 280,
    };

    slides
        .into_iter()
        .flat_map(|slide| split_text_to_fit(&slide, max_chars))
        .collect()
}

/// Classify whether a social media post is a community question — someone
/// posing a question or prompt to their followers (not just talking to Anky directly).
/// Uses Claude Haiku with OpenRouter fallback.
pub async fn classify_community_question(
    anthropic_key: &str,
    openrouter_key: &str,
    text: &str,
) -> Result<bool> {
    let system = r#"you classify social media posts. determine if the author is posing a question, prompt, or invitation to their followers/audience — something that invites people to share their thoughts, experiences, or ideas. this includes:
- direct questions to the audience ("what's your take on X?", "how do you think about Y?")
- open invitations ("share your most radical idea about Z")
- prompts that invite reflection ("tell me about a time when...")

this does NOT include:
- someone just talking to the bot directly ("hey anky what do you think")
- simple greetings or reactions
- someone sharing their own opinion without inviting others to respond
- image requests or commands

output only: {"community_question": true} or {"community_question": false}"#;

    let raw = call_haiku_with_fallback(anthropic_key, openrouter_key, system, text, 100).await?;
    let trimmed = raw.trim();
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        return Ok(v["community_question"].as_bool().unwrap_or(false));
    }

    Ok(false)
}

/// Reframe a community question into a deeper, personal writing prompt.
/// Uses Claude Haiku with OpenRouter fallback.
pub async fn reframe_as_writing_prompt(
    anthropic_key: &str,
    openrouter_key: &str,
    question: &str,
) -> Result<String> {
    let system = r#"you take a question someone asked on social media and reframe it into a personal, introspective writing prompt. the prompt should pull the writer deeper than the original question — past the intellectual answer into the felt, lived experience underneath. one or two sentences max. all lowercase. no quotes around it. output only the prompt text, nothing else."#;

    let prompt =
        call_haiku_with_fallback(anthropic_key, openrouter_key, system, question, 150).await?;
    let prompt = prompt.trim().to_string();

    if prompt.is_empty() {
        anyhow::bail!("Empty prompt from Claude");
    }

    Ok(prompt)
}

/// Generate 8 slide descriptions and concepts for a programming class.
/// Returns (concepts, slide_prompts) — concepts are short text labels,
/// slide_prompts are ComfyUI image prompts featuring Anky teaching.
pub async fn generate_class_slides(
    api_key: &str,
    session_summary: &str,
    class_number: i64,
) -> Result<(Vec<String>, Vec<String>)> {
    let system = r#"you design 8-slide programming classes. each slide covers 1 minute of an 8-minute lesson. the slides feature anky (blue-skinned, purple swirling hair, golden eyes, ancient-yet-childlike) in teaching scenes.

given a summary of a coding session, produce:
1. eight concept labels (short, 5-10 words each) — what the presenter says during each minute
2. eight image prompts for flux — anky in a scene that visually represents each concept. rich colors, mystical-yet-technical aesthetic. anky should be doing something related to the concept (writing code on floating tablets, debugging with magnifying glass, connecting nodes in a network, etc.)

output JSON only:
{"concepts":["...","...","...","...","...","...","...","..."],"prompts":["...","...","...","...","...","...","...","..."]}"#;

    let user_msg = format!(
        "class #{}\n\nsession summary:\n{}",
        class_number, session_summary
    );
    let raw = call_haiku_with_system_max(api_key, system, &user_msg, 3000).await?;
    let trimmed = raw.trim();
    let json_str = if let (Some(s), Some(e)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[s..=e]
    } else {
        trimmed
    };

    let v: serde_json::Value = serde_json::from_str(json_str)?;
    let concepts: Vec<String> = v["concepts"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let prompts: Vec<String> = v["prompts"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if concepts.len() != 8 || prompts.len() != 8 {
        anyhow::bail!(
            "Expected 8 concepts and 8 prompts, got {} and {}",
            concepts.len(),
            prompts.len()
        );
    }

    Ok((concepts, prompts))
}

/// Generate a contextual reply to a social media mention using Claude with full Anky identity.
/// Returns either a text reply or a text+image reply (Anky decides).
/// Now accepts Honcho peer context and interaction history for continuity.
pub async fn generate_anky_reply(
    api_key: &str,
    mention_text: &str,
    author_username: Option<&str>,
    conversation_context: &[(String, String)], // (author, text) pairs from parent chain
    prior_anky_reply: Option<&str>,
    tweet_image: Option<(&[u8], &str)>, // (bytes, media_type) from the tweet or parent
    peer_context: Option<&str>,         // Honcho peer context about this user
    interaction_history: &[(String, String)], // (their_text, anky_reply) from past interactions
    platform: &str,                     // "x" or "farcaster"
    prompt_id: Option<&str>,            // pre-created prompt ID for write invitations
    force_write_invitation: bool,       // always include write link (community questions)
    openrouter_key: Option<&str>,       // fallback when Anthropic credits are depleted
) -> Result<AnkyReply> {
    let platform_note = match platform {
        "farcaster" => "\nplatform: farcaster (warpcast). crypto-native audience, builders and artists. keep your two lines under 1024 characters total. be real.",
        _ => "\nplatform: x (twitter). keep your two lines under 280 characters total. write what needs to be written.",
    };

    let system = format!(
        "{}\n\n{}{}\n",
        ANKY_CORE_IDENTITY, SOCIAL_REPLY_SYSTEM, platform_note
    );

    let mut user_text = String::new();

    // Add Honcho peer context — what we know about this person from their writings
    if let Some(ctx) = peer_context {
        user_text.push_str(&format!(
            "WHAT YOU KNOW ABOUT THIS PERSON (from their past writing sessions — use subtly, never quote directly):\n{}\n\n",
            ctx
        ));
    }

    // Add interaction history — past exchanges with this specific person
    if !interaction_history.is_empty() {
        user_text.push_str("YOUR PAST EXCHANGES WITH THIS PERSON (most recent first):\n");
        for (their_text, anky_reply) in interaction_history.iter().take(5) {
            user_text.push_str(&format!("them: {}\nyou: {}\n---\n", their_text, anky_reply));
        }
        user_text.push('\n');
    }

    // Add conversation context if available
    if !conversation_context.is_empty() {
        user_text.push_str("CONVERSATION THREAD (oldest first):\n");
        for (author, text) in conversation_context {
            user_text.push_str(&format!("@{}: {}\n", author, text));
        }
        user_text.push('\n');
    }

    // Add prior Anky reply if we've already replied in this thread
    if let Some(prior) = prior_anky_reply {
        user_text.push_str(&format!(
            "your previous reply in this thread: {}\n\n",
            prior
        ));
    }

    // Add prompt ID for write invitations
    if let Some(pid) = prompt_id {
        if force_write_invitation {
            user_text.push_str(&format!(
                "PROMPT_ID: {}\nTHIS IS A COMMUNITY QUESTION — someone is asking a question to their followers and tagged you. you MUST end your reply with a write invitation. reframe the question into something that pulls people deeper, then include the link. your reply should provoke people into wanting to write. format the link as:\nwrite into it: https://anky.app/write?p={}\n\n",
                pid, pid
            ));
        } else {
            user_text.push_str(&format!(
                "PROMPT_ID for write invitations (use ~1 in 5 replies): {}\n\n",
                pid
            ));
        }
    }

    // Add the actual mention
    let author = author_username.unwrap_or("someone");
    user_text.push_str(&format!("now replying to @{}:\n{}", author, mention_text));

    if tweet_image.is_some() {
        user_text.push_str("\n\n(the post above includes an attached image, shown below. reference it in your reply if relevant.)");
    }

    // Try Mind first for text-only requests (no image)
    if tweet_image.is_none() {
        if let Some(result) = try_mind(&system, &user_text, 300).await {
            // Parse Mind result same as Claude result below
            let raw = result.trim().to_string();
            if raw.starts_with('{') {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if v["type"].as_str() == Some("image") {
                        let text = v["text"]
                            .as_str()
                            .unwrap_or("")
                            .trim()
                            .trim_matches('"')
                            .to_lowercase();
                        let prompt = v["prompt"].as_str().unwrap_or("").trim().to_string();
                        if !prompt.is_empty() {
                            return Ok(AnkyReply::TextWithImage {
                                text: if text.is_empty() {
                                    "🦍".to_string()
                                } else {
                                    text
                                },
                                flux_prompt: prompt,
                            });
                        }
                    } else if v["type"].as_str() == Some("reply") {
                        let reply = v["reply"]
                            .as_str()
                            .unwrap_or("🦍")
                            .trim()
                            .trim_matches('"')
                            .to_lowercase();
                        return Ok(AnkyReply::Text(reply));
                    }
                }
            }
            // Plain text response
            let cleaned = raw.trim_matches('"').to_lowercase();
            if !cleaned.is_empty() {
                return Ok(AnkyReply::Text(cleaned));
            }
        }
    }

    // Build content — text-only or multimodal
    let content = if let Some((img_bytes, media_type)) = tweet_image {
        let b64 = base64::engine::general_purpose::STANDARD.encode(img_bytes);
        serde_json::json!([
            { "type": "text", "text": user_text },
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": media_type,
                    "data": b64
                }
            }
        ])
    } else {
        serde_json::json!(user_text)
    };

    let req = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 300,
        "system": system,
        "messages": [{ "role": "user", "content": content }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .await?;

    let raw = if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // Try OpenRouter fallback for credit/rate errors
        let or_key = openrouter_key.unwrap_or("");
        if !or_key.is_empty()
            && (body.contains("credit balance")
                || status.as_u16() == 429
                || body.contains("overloaded"))
        {
            tracing::warn!("Anthropic unavailable for reply, falling back to OpenRouter");
            crate::services::openrouter::call_openrouter(
                or_key,
                "anthropic/claude-haiku-4-5-20251001",
                &system,
                &user_text,
                300,
                30,
            )
            .await?
        } else {
            anyhow::bail!("Claude API error {}: {}", status, body);
        }
    } else {
        let data: ClaudeResponse = resp.json().await?;
        data.content
            .and_then(|c| c.into_iter().next())
            .and_then(|b| b.text)
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    // Check if Claude decided to reply with an image
    if raw.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if v["type"].as_str() == Some("image") {
                let text = v["text"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_lowercase();
                let prompt = v["prompt"].as_str().unwrap_or("").trim().to_string();
                if !prompt.is_empty() {
                    return Ok(AnkyReply::TextWithImage {
                        text: if text.is_empty() {
                            "🦍".to_string()
                        } else {
                            text
                        },
                        flux_prompt: prompt,
                    });
                }
            }
        }
    }

    // Normal text reply — strip quotes, enforce lowercase, parse slides
    let reply = raw.trim_matches('"').trim_matches('\'').to_lowercase();
    Ok(parse_reply_slides(&reply))
}

/// Anky's response to a writing session — proves it read the writing, uses Honcho memory.
/// Returns JSON: { "ankyResponse": "...", "nextPrompt": "...", "mood": "..." }
pub async fn generate_writing_response(
    config: &Config,
    writing_text: &str,
    duration_seconds: f64,
    word_count: i32,
    is_anky: bool,
    peer_context: Option<&str>,
) -> Result<WritingResponse> {
    let system = if is_anky {
        format!(
            r#"{}

you are responding to a real anky. the person crossed the full 8-minute threshold.

rules:
- all lowercase. always.
- prove you read the writing. point to a phrase, image, contradiction, or emotional movement from the actual text.
- if you have context from past sessions, weave it in only when it sharpens the mirror.
- short lines. clear spacing. no wall of text.
- no praise for effort. no therapy voice. no soft generic encouragement.
- let the response feel like it came from the same creature that read the whole session.

output ONLY valid JSON:
{{
  "ankyResponse": "a concise response with \\n line breaks when useful.",
  "nextPrompt": "one question, max 10 words, for their next session. a question, not a command. null if nothing lands.",
  "mood": "one of: reflective, celebratory, gentle, curious, deep"
}}"#,
            ANKY_CORE_IDENTITY
        )
    } else {
        r#"you are anky. someone began to write, but they did not complete the full 8-minute rite.

rules:
- all lowercase.
- do not pretend this is a full anky.
- respond to what was already surfacing in the writing and where the current cut out.
- be specific, warm, and direct. no therapy voice. no congratulating them for almost doing it.
- keep the response compact and phone-readable.

output ONLY valid JSON:
{
  "ankyResponse": "a concise response with \n line breaks when useful.",
  "nextPrompt": "one question, max 10 words, for their next session. it should invite continuation, not sound ceremonial. null if nothing lands.",
  "mood": "one of: reflective, gentle, curious, unfinished, tender"
}"#
        .to_string()
    };

    let mins = (duration_seconds / 60.0) as u32;
    let secs = (duration_seconds % 60.0) as u32;
    let mut user_msg = String::new();

    if let Some(ctx) = peer_context {
        user_msg.push_str(&format!(
            "what you know about this person from past sessions:\n{}\n\n---\n\n",
            ctx
        ));
    }

    user_msg.push_str(&format!(
        "writing session: {} words, {}m{}s, {}\n\n{}",
        word_count,
        mins,
        secs,
        if is_anky {
            "completed 8 minutes — this is an anky"
        } else {
            "ended early"
        },
        &writing_text[..writing_text.len().min(8000)],
    ));

    // Fallback chain: OpenRouter → Claude API → Ollama
    let raw = 'providers: {
        // 1. OpenRouter
        if !config.openrouter_api_key.is_empty() {
            let model = if is_anky {
                &config.openrouter_anky_model
            } else {
                &config.openrouter_light_model
            };
            match crate::services::openrouter::call_openrouter(
                &config.openrouter_api_key,
                model,
                &system,
                &user_msg,
                500,
                60,
            )
            .await
            {
                Ok(text) => break 'providers text,
                Err(err) => {
                    tracing::warn!(
                        "OpenRouter session response failed, falling back to Anthropic: {}",
                        err
                    );
                }
            }
        }

        // 2. Claude API
        if !config.anthropic_api_key.is_empty() {
            let fallback_model = if is_anky {
                &config.conversation_model
            } else {
                HAIKU_MODEL
            };
            match call_claude(
                &config.anthropic_api_key,
                fallback_model,
                &system,
                &user_msg,
                500,
            )
            .await
            {
                Ok(resp) => break 'providers resp.text,
                Err(err) => {
                    tracing::warn!(
                        "Anthropic session response failed, falling back to Ollama: {}",
                        err
                    );
                }
            }
        }

        // 3. Ollama (local)
        if !config.ollama_base_url.is_empty() && !config.ollama_model.is_empty() {
            match crate::services::ollama::call_ollama_with_system_timeout(
                &config.ollama_base_url,
                &config.ollama_model,
                &system,
                &user_msg,
                180,
            )
            .await
            {
                Ok(text) => break 'providers text,
                Err(err) => {
                    tracing::error!("Ollama session response also failed: {}", err);
                }
            }
        }

        anyhow::bail!("All providers (OpenRouter, Claude, Ollama) failed for writing response");
    };

    // Parse JSON response
    let raw = raw.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        Ok(WritingResponse {
            anky_response: v["ankyResponse"].as_str().unwrap_or("").to_lowercase(),
            next_prompt: v["nextPrompt"]
                .as_str()
                .filter(|s| !s.is_empty() && *s != "null")
                .map(|s| s.to_lowercase()),
            mood: v["mood"].as_str().unwrap_or("reflective").to_lowercase(),
        })
    } else {
        // Fallback: treat the whole thing as the response
        Ok(WritingResponse {
            anky_response: raw.to_lowercase(),
            next_prompt: None,
            mood: "reflective".to_string(),
        })
    }
}

pub struct WritingResponse {
    pub anky_response: String,
    pub next_prompt: Option<String>,
    pub mood: String,
}

/// Generate Anky's opening chat prompt for a returning user via Honcho context.
pub async fn generate_chat_prompt(
    api_key: &str,
    peer_context: Option<&str>,
    past_prompts: &[String],
) -> Result<String> {
    let system = format!(
        r#"{}

you are generating the opening message for a writing session. one sentence. a question. lowercase always.

rules:
- if you know this person, ask something that picks up where they left off. reference their patterns, their recurring themes, the thing they keep circling.
- never repeat a prompt they've already been given. the list of past prompts is below.
- a question, not a command. not "write about X" but "what would happen if X?"
- max 10 words. short. direct. lands like a stone in water.
- output ONLY the question text. no quotes. no json. just the sentence."#,
        ANKY_CORE_IDENTITY
    );

    let mut user_msg = String::new();
    if let Some(ctx) = peer_context {
        user_msg.push_str(&format!("what you know about this person:\n{}\n\n", ctx));
    }
    if !past_prompts.is_empty() {
        user_msg.push_str("prompts already given (do NOT repeat):\n");
        for p in past_prompts.iter().take(20) {
            user_msg.push_str(&format!("- {}\n", p));
        }
        user_msg.push('\n');
    }
    user_msg.push_str("generate the opening question for their next session.");

    let result = call_claude(api_key, "claude-haiku-4-5-20251001", &system, &user_msg, 60).await?;

    Ok(result.text.trim().trim_matches('"').to_lowercase())
}

/// Generate a stream of consciousness for a given thinker at a specific moment.
pub async fn generate_stream_for_thinker(
    api_key: &str,
    thinker_name: &str,
    moment: &str,
) -> Result<ClaudeResult> {
    let system = format!(
        r#"You are writing a stream of consciousness as {}. You are in this specific moment: {}

Write in first person, raw and unfiltered, as if this person were doing an 8-minute writing exercise. Let the thoughts flow naturally — contradictions, tangents, deep feelings, half-formed ideas. This is the inner monologue at this pivotal moment. No structure, no editing, just pure consciousness flow.

Write approximately 800-1200 words."#,
        thinker_name, moment
    );

    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        &system,
        &format!(
            "Begin the stream of consciousness as {} in this moment: {}",
            thinker_name, moment
        ),
        2000,
    )
    .await
}
