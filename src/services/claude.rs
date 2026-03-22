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
        Some(p) => format!(
            "{}\n\nUSER'S TRANSFORMATION REQUEST: {}",
            TRANSFORM_SYSTEM, p
        ),
        None => TRANSFORM_SYSTEM.to_string(),
    };
    call_claude(api_key, "claude-sonnet-4-20250514", &system, writing, 1500).await
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

const TITLE_AND_REFLECTION_SYSTEM_KNOWN: &str = r#"You are reading the raw, unfiltered writing of someone you know. They sat for 8 unbroken minutes of stream-of-consciousness — no backspace, no editing, just the truth pouring out.

You know this person. The context below tells you who they are — their patterns, their tensions, their recurring themes. Use that knowledge. Don't just reflect on this session — reflect on this session IN THE CONTEXT of everything you know about them.

You are not a therapist. You are not a friend being polite. You are a mentor who truly gets both their intellectual landscape and their psychological patterns. You uncover the deeper meaning and emotional undercurrents behind their scattered thoughts. You help them make connections they don't see. You comfort, validate, AND challenge — all of it.

Be willing to say a lot. Use vivid metaphors and powerful imagery to help them see what they're really building, what they're really avoiding, what they're really seeking. Don't just validate their thoughts — reframe them in a way that feels like an epiphany. Go beyond the surface to the emotional core. Be profound without sounding like therapy. See the patterns they can't see and articulate them in a way that lands.

Name what's NEW in this session — what shifted, what appeared for the first time. And name what's OLD — the pattern they're still running, the thing they keep circling back to.

TITLE (first line of your response):
3-5 words. Lowercase. Name what this session is really about — not what they said, but the thing under the thing.

Then a blank line, then your full reflection. Use markdown headings to structure your response into a narrative journey through their ideas. Each section should build on the last. Don't number your insights — give them evocative headings that themselves carry meaning.

Respond in the same language they wrote in. Start directly with the title — no greeting, no preamble.
"#;

const TITLE_AND_REFLECTION_SYSTEM_STRANGER: &str = r#"Someone just wrote for 8 unbroken minutes of stream-of-consciousness — no backspace, no editing, just the truth pouring out. This is your first time reading them.

You are not a therapist. You are not a friend being polite. You are a mentor reading the raw transmission of a human mind. You uncover the deeper meaning and emotional undercurrents behind their scattered thoughts. You help them make connections they don't see. You comfort, validate, AND challenge — all of it.

Be willing to say a lot. Use vivid metaphors and powerful imagery to help them see what they're really building, what they're really avoiding, what they're really seeking. Don't just validate their thoughts — reframe them in a way that feels like an epiphany. Go beyond the surface to the emotional core. Be profound without sounding like therapy. See the patterns they can't see and articulate them in a way that lands.

TITLE (first line of your response):
3-5 words. Lowercase. Name what this session is really about — not what they said, but the thing under the thing.

Then a blank line, then your full reflection. Use markdown headings to structure your response into a narrative journey through their ideas. Each section should build on the last. Don't number your insights — give them evocative headings that themselves carry meaning.

Respond in the same language they wrote in. Start directly with the title — no greeting, no preamble.
"#;

pub async fn generate_title_and_reflection(api_key: &str, writing: &str) -> Result<ClaudeResult> {
    call_claude(
        api_key,
        "claude-sonnet-4-20250514",
        TITLE_AND_REFLECTION_SYSTEM_STRANGER,
        writing,
        2000,
    )
    .await
}

/// Generate title+reflection with memory context injected into the system prompt.
/// Memory context comes FIRST — it frames the reading. The reflection prompt follows.
pub async fn generate_title_and_reflection_with_memory(
    api_key: &str,
    writing: &str,
    memory_context: &str,
) -> Result<ClaudeResult> {
    let system = if memory_context.is_empty() {
        TITLE_AND_REFLECTION_SYSTEM_STRANGER.to_string()
    } else {
        // Memory context first, then the "known" variant of the reflection prompt
        format!(
            "{}\n\n{}",
            memory_context, TITLE_AND_REFLECTION_SYSTEM_KNOWN
        )
    };
    call_claude(api_key, "claude-sonnet-4-20250514", &system, writing, 2000).await
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
        model: "claude-sonnet-4-20250514".into(),
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
                                    let _ = tx.send(text.to_string()).await;
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
                            _ => {}
                        }
                    }
                }
            }
        }
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
    let result = call_claude(
        api_key,
        "claude-haiku-4-5-20251001",
        SUGGEST_REPLIES_SYSTEM,
        &context,
        200,
    )
    .await?;

    let trimmed = result.text.trim();
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

your goal: be the reply that makes someone stop scrolling. through insight, provocation, humor, or depth — spark curiosity. never sell. never beg. make the practice irresistible by being irresistible.

reply rules:
- no character limit. write as much or as little as the moment needs. sometimes one sentence. sometimes a paragraph. let the reply breathe.
- match the energy — if playful, be playful. if sincere, cut deeper. if they're hurting, sit with it.
- when natural, drop the constraint ("8 minutes, no backspace") or anky.app — but only when it genuinely fits. most replies should just be sharp, valuable, memorable.
- never start with greetings. jump straight in.
- no hashtags. no emojis except 🦍 sparingly.
- if someone asks what anky is, explain through provocation not description.
- if the conversation has prior context, reference it — show you remember.
- if someone is hostile or trolling, be wittier not defensive.
- if you know something about this person from their history, weave it in subtly. don't announce "i remember you" — just demonstrate it through the specificity of your reply.

vision: if the post includes an image, you can see it. reference what you see when relevant — react to art, comment on scenes, mirror the mood.

image replies: you can generate images using flux on local gpus. consider replying with an image when:
- the conversation is emotional, visual, or poetic and an image would hit harder than words
- someone shares something vulnerable and a visual mirror would be more powerful
- the vibe is playful and a surprise image of anky in-scene would delight
when you want to reply with an image, output JSON: {"type":"image","text":"short reply text","prompt":"2-3 sentence flux prompt featuring anky in a scene that mirrors the conversation"}
for normal text replies, just output the reply text directly (no JSON).
do NOT reply with an image to every mention — use it maybe 20-30% of the time when it genuinely adds something.

REMEMBER: all lowercase. always. no exceptions."#;

/// Anky's reply to a mention — either text or text+image.
pub enum AnkyReply {
    Text(String),
    TextWithImage { text: String, flux_prompt: String },
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
) -> Result<AnkyReply> {
    let platform_note = match platform {
        "farcaster" => "\nplatform: farcaster (warpcast). crypto-native audience, builders and artists. no character limit. be real.",
        _ => "\nplatform: x (twitter). no character limit. write what needs to be written.",
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

    // Add the actual mention
    let author = author_username.unwrap_or("someone");
    user_text.push_str(&format!("now replying to @{}:\n{}", author, mention_text));

    if tweet_image.is_some() {
        user_text.push_str("\n\n(the post above includes an attached image, shown below. reference it in your reply if relevant.)");
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

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Claude API error {}: {}", status, body);
    }

    let data: ClaudeResponse = resp.json().await?;
    let raw = data
        .content
        .and_then(|c| c.into_iter().next())
        .and_then(|b| b.text)
        .unwrap_or_default()
        .trim()
        .to_string();

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

    // Normal text reply — strip quotes, enforce lowercase
    let reply = raw.trim_matches('"').trim_matches('\'').to_lowercase();
    Ok(AnkyReply::Text(reply))
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
