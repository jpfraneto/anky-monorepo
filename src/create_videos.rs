use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const PROMPTS_JSON: &str = include_str!("../static/create_videos_prompts.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoPrompt {
    pub id: String,
    pub title: String,
    pub duration_seconds: u32,
    pub image_prompt: String,
    pub video_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoState {
    pub prompt_id: String,
    pub image_status: String,
    pub image_path: Option<String>,
    pub image_url: Option<String>,
    pub image_jpeg_path: Option<String>,
    pub video_status: String,
    pub video_path: Option<String>,
    pub video_url: Option<String>,
    pub video_request_id: Option<String>,
    pub image_error: Option<String>,
    pub video_error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateVideoCard {
    pub id: String,
    pub title: String,
    pub duration_seconds: u32,
    pub image_prompt: String,
    pub video_prompt: String,
    pub image_status: String,
    pub image_url: Option<String>,
    pub video_status: String,
    pub video_url: Option<String>,
    pub image_error: Option<String>,
    pub video_error: Option<String>,
}

impl CreateVideoState {
    pub fn new(prompt_id: &str) -> Self {
        Self {
            prompt_id: prompt_id.to_string(),
            image_status: "idle".to_string(),
            image_path: None,
            image_url: None,
            image_jpeg_path: None,
            video_status: "locked".to_string(),
            video_path: None,
            video_url: None,
            video_request_id: None,
            image_error: None,
            video_error: None,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn to_card(&self, prompt: &CreateVideoPrompt) -> CreateVideoCard {
        CreateVideoCard {
            id: prompt.id.clone(),
            title: prompt.title.clone(),
            duration_seconds: prompt.duration_seconds,
            image_prompt: prompt.image_prompt.clone(),
            video_prompt: prompt.video_prompt.clone(),
            image_status: self.image_status.clone(),
            image_url: self.image_url.clone(),
            video_status: self.video_status.clone(),
            video_url: self.video_url.clone(),
            image_error: self.image_error.clone(),
            video_error: self.video_error.clone(),
        }
    }
}

pub fn prompt_catalog() -> Vec<CreateVideoPrompt> {
    serde_json::from_str(PROMPTS_JSON).expect("invalid create_videos_prompts.json")
}

pub fn get_prompt(prompt_id: &str) -> Option<CreateVideoPrompt> {
    prompt_catalog()
        .into_iter()
        .find(|prompt| prompt.id == prompt_id)
}

pub fn load_state(prompt_id: &str) -> Result<CreateVideoState> {
    let path = state_path(prompt_id);
    if !path.exists() {
        return Ok(CreateVideoState::new(prompt_id));
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read create-videos state {}", path.display()))?;
    let state = serde_json::from_str::<CreateVideoState>(&raw)
        .with_context(|| format!("failed to parse create-videos state {}", path.display()))?;
    Ok(state)
}

pub fn save_state(state: &CreateVideoState) -> Result<()> {
    let path = state_path(&state.prompt_id);
    ensure_state_dir()?;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&tmp_path, json)
        .with_context(|| format!("failed to write temp state {}", tmp_path.display()))?;
    fs::rename(&tmp_path, &path)
        .with_context(|| format!("failed to move state into place {}", path.display()))?;
    Ok(())
}

pub fn load_cards() -> Result<Vec<CreateVideoCard>> {
    prompt_catalog()
        .into_iter()
        .map(|prompt| {
            let state = load_state(&prompt.id)?;
            Ok(state.to_card(&prompt))
        })
        .collect()
}

pub fn asset_stem(prompt_id: &str) -> String {
    format!("create-video-{}", prompt_id)
}

pub fn image_public_url(filename: &str) -> String {
    format!("/data/images/{}", filename)
}

pub fn image_absolute_url(filename: &str) -> String {
    format!("https://anky.app/data/images/{}", filename)
}

pub fn video_public_url(filename: &str) -> String {
    format!("/data/videos/{}", filename)
}

pub fn video_filename(prompt_id: &str) -> String {
    format!("{}.mp4", asset_stem(prompt_id))
}

pub fn video_output_path(prompt_id: &str) -> String {
    format!("data/videos/{}", video_filename(prompt_id))
}

fn ensure_state_dir() -> Result<()> {
    fs::create_dir_all(state_dir()).context("failed to create data/create_videos")?;
    Ok(())
}

fn state_dir() -> PathBuf {
    Path::new("data").join("create_videos")
}

fn state_path(prompt_id: &str) -> PathBuf {
    state_dir().join(format!("{}.json", prompt_id))
}
