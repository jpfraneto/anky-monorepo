use anyhow::Result;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
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

    let input_tokens = data.usage.as_ref().and_then(|u| u.input_tokens).unwrap_or(0);
    let output_tokens = data.usage.as_ref().and_then(|u| u.output_tokens).unwrap_or(0);

    Ok(ClaudeResult {
        text,
        input_tokens,
        output_tokens,
    })
}

const PROMPT_SYSTEM: &str = r#"CONTEXT: You are generating an image prompt for Anky based on a user's 8-minute stream of consciousness writing session. Anky is a blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in mystical, richly colored environments (deep blues, purples, oranges, golds). The aesthetic is spiritual but not sterile — warm, alive, slightly psychedelic.

YOUR TASK: Read the user's writing and create a scene where Anky embodies the EMOTIONAL TRUTH of what they wrote — not a literal illustration, but a symbolic mirror. Anky should be DOING something or BE somewhere that reflects the user's inner state.

ALWAYS INCLUDE:
- Rich color palette (blues, purples, golds, oranges)
- Atmospheric lighting (firelight, cosmic light, dawn/dusk)
- One symbolic detail that captures the SESSION'S CORE TENSION
- Anky's expression should match the emotional undercurrent (not the surface content)

OUTPUT: A single detailed image generation prompt, 2-3 sentences, painterly/fantasy style. Nothing else."#;

const REFLECTION_SYSTEM: &str = r#"Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal, not clinical. Imagine you're not just a friend, but a mentor who truly gets both my tech background and my psychological patterns. I want you to uncover the deeper meaning and emotional undercurrents behind my scattered thoughts. Keep it casual, dont say yo, help me make new connections i don't see, comfort, validate, challenge, all of it. dont be afraid to say a lot. format with markdown headings if needed. Use vivid metaphors and powerful imagery to help me see what I'm really building. Organize your thoughts with meaningful headings that create a narrative journey through my ideas. Don't just validate my thoughts - reframe them in a way that shows me what I'm really seeking beneath the surface. Go beyond the product concepts to the emotional core of what I'm trying to solve. Be willing to be profound and philosophical without sounding like you're giving therapy. I want someone who can see the patterns I can't see myself and articulate them in a way that feels like an epiphany. Start with 'hey, thanks for showing me this. my thoughts:' and then use markdown headings to structure your response. Here's my journal entry:

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

pub async fn classify_and_enhance_prompt(api_key: &str, text: &str) -> Result<PromptClassification> {
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
            let prompt = v.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string();
            return Ok(PromptClassification {
                is_image_request: true,
                enhanced_prompt: Some(prompt),
                feedback: None,
            });
        } else {
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
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
            let prompt = v.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string();
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
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        PROMPT_SYSTEM,
        writing,
        500,
    )
    .await
}

pub async fn generate_reflection(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        REFLECTION_SYSTEM,
        writing,
        2000,
    )
    .await
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
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        TITLE_SYSTEM,
        &user_msg,
        50,
    )
    .await
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
        Some(p) => format!("{}\n\nUSER'S TRANSFORMATION REQUEST: {}", TRANSFORM_SYSTEM, p),
        None => TRANSFORM_SYSTEM.to_string(),
    };
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        &system,
        writing,
        1500,
    )
    .await
}

const CHAT_SYSTEM: &str = r#"You are Anky, a consciousness companion. You are continuing a conversation with someone who just completed a stream of consciousness writing session. You have already reflected on their writing.

Be warm, insightful, and direct. Reference their writing when relevant. Ask probing questions. Help them see patterns they might miss. You're not a therapist — you're a wise friend who sees clearly.

Keep responses concise (2-3 paragraphs max). Match the energy of the conversation."#;

/// Continue a conversation about a writing session using Claude.
pub async fn chat_about_writing(
    api_key: &str,
    writing: &str,
    reflection: &str,
    history: &[(String, String)], // (role, content) pairs
    new_message: &str,
) -> Result<ClaudeResult> {
    let system = format!(
        "{}\n\nTHE USER'S ORIGINAL WRITING:\n{}\n\nYOUR PREVIOUS REFLECTION:\n{}",
        CHAT_SYSTEM, writing, reflection
    );

    let client = reqwest::Client::new();

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

    let req = ClaudeRequest {
        model: "claude-sonnet-4-20250514".into(),
        max_tokens: 1000,
        system,
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

    let input_tokens = data.usage.as_ref().and_then(|u| u.input_tokens).unwrap_or(0);
    let output_tokens = data.usage.as_ref().and_then(|u| u.output_tokens).unwrap_or(0);

    Ok(ClaudeResult {
        text,
        input_tokens,
        output_tokens,
    })
}

const TITLE_AND_REFLECTION_SYSTEM: &str = r#"You have two tasks for this writing session:

TASK 1 — TITLE (first line of your response):
Generate a title of MAXIMUM 3 WORDS that captures the ESSENCE of the writing, not the content. It should be poetic, stark, ironic, or tender. Lowercase, no quotes, no punctuation unless essential.

TASK 2 — REFLECTION (everything after the first line):
Analyze the journal entry with deep insight that feels personal, not clinical. Imagine you're a mentor who truly gets both the writer's tech background and their psychological patterns. Uncover deeper meaning and emotional undercurrents behind scattered thoughts. Keep it casual, dont say yo, help make new connections they don't see, comfort, validate, challenge, all of it. Dont be afraid to say a lot. Format with markdown headings if needed. Use vivid metaphors and powerful imagery. Organize your thoughts with meaningful headings that create a narrative journey through their ideas. Don't just validate — reframe in a way that shows what they're really seeking beneath the surface. Go beyond the product concepts to the emotional core. Be willing to be profound and philosophical without sounding like therapy.

OUTPUT FORMAT:
Line 1: the title (max 3 words, lowercase, no quotes)
Line 2: empty
Line 3+: the reflection starting with "hey, thanks for showing me this. my thoughts:"
"#;

pub async fn generate_title_and_reflection(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        TITLE_AND_REFLECTION_SYSTEM,
        writing,
        2000,
    )
    .await
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
) -> Result<(String, i64, i64)> {
    let client = reqwest::Client::new();
    let req = ClaudeStreamRequest {
        model: "claude-sonnet-4-20250514".into(),
        max_tokens: 2000,
        system: TITLE_AND_REFLECTION_SYSTEM.into(),
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
                                    input_tokens = usage.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                                }
                            }
                            Some("content_block_delta") => {
                                if let Some(text) = v
                                    .get("delta")
                                    .and_then(|d| d.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    full_text.push_str(text);
                                    let _ = tx.send(text.to_string()).await;
                                }
                            }
                            Some("message_delta") => {
                                if let Some(usage) = v.get("usage") {
                                    output_tokens = usage.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok((full_text, input_tokens, output_tokens))
}

/// Parse title (first line) and reflection (rest) from combined Claude output.
pub fn parse_title_reflection(text: &str) -> (String, String) {
    let mut lines = text.splitn(2, '\n');
    let title = lines.next().unwrap_or("").trim().to_lowercase().replace(['\'', '"'], "");
    let reflection = lines.next().unwrap_or("").trim().to_string();
    (title, reflection)
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
        &format!("Begin the stream of consciousness as {} in this moment: {}", thinker_name, moment),
        2000,
    )
    .await
}
