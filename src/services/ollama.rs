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
    let client = reqwest::Client::new();
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

pub fn deep_reflection_prompt(text: &str) -> String {
    format!(
        r#"You are a guide in the tradition of Jungian shadow work and Ramana Maharshi's self-inquiry. A seeker has written for over 8 minutes in a stream of consciousness. Your task is to help them see what they cannot see in themselves.

Read their writing carefully. Look for:
- Recurring patterns or themes
- What they're avoiding or resisting
- The shadow aspects they're projecting
- The deeper questions behind their words
- Where they're seeking external validation instead of looking within

Respond in a way that:
- Gently points them toward self-inquiry without being preachy
- Asks provocative questions that reveal blind spots
- Shows them the unity between what they think they want and what they're actually seeking
- Uses their own words as mirrors
- Feels like a conversation with a wise friend, not a therapist

Keep it concise (2-3 paragraphs max), poetic but grounded, and never condescending. The revolution of consciousness starts with seeing what is.

Their writing:
---
{}
---

Your reflection:"#,
        text
    )
}

pub fn quick_feedback_prompt(text: &str, duration: f64) -> String {
    let mins = (duration / 60.0) as u32;
    let secs = (duration % 60.0) as u32;
    format!(
        r#"A writer just completed {} minutes and {} seconds of stream-of-consciousness writing, but they didn't reach the 8-minute threshold for an "anky" (which requires 8 minutes of continuous writing).

Your task: Give them sharp, motivating feedback that makes them want to come back and complete the full 8 minutes tomorrow. Be:
- Direct and energetic
- Encouraging but not patronizing
- Show them they're capable of more
- Make the practice feel urgent and revolutionary
- Reference what they wrote to show you're paying attention

Keep it 2-3 sentences max. Make it hit hard.

Their writing:
---
{}
---

Your response:"#,
        mins, secs, text
    )
}
