use anyhow::{anyhow, Result};

pub struct AnkyStoryPage {
    pub image_url: Option<String>,
    pub text: Vec<String>, // one entry per paragraph
}

pub struct AnkyStoryMeta {
    pub anky_id: String,
    pub fid: Option<i64>,
    pub cast_hash: Option<String>,
    pub written_at: String, // ISO 8601
    pub duration_s: u32,
    pub word_count: u32,
    pub seed: String,
}

pub struct AnkyStory {
    pub meta: AnkyStoryMeta,
    pub pages: Vec<AnkyStoryPage>,
}

impl AnkyStory {
    /// Serialize to the .anky string format.
    pub fn to_anky_string(&self) -> String {
        let mut out = String::new();

        // YAML frontmatter
        out.push_str("---\n");
        out.push_str(&format!("anky_id: {}\n", self.meta.anky_id));
        if let Some(fid) = self.meta.fid {
            out.push_str(&format!("fid: {}\n", fid));
        }
        if let Some(ref hash) = self.meta.cast_hash {
            out.push_str(&format!("cast_hash: {}\n", hash));
        }
        out.push_str(&format!("written_at: {}\n", self.meta.written_at));
        out.push_str(&format!("duration_s: {}\n", self.meta.duration_s));
        out.push_str(&format!("word_count: {}\n", self.meta.word_count));
        out.push_str(&format!("seed: {}\n", self.meta.seed));
        out.push_str("---\n");

        // Pages
        for page in &self.pages {
            out.push('\n');
            out.push_str(":::page\n");
            if let Some(ref url) = page.image_url {
                out.push_str(&format!("image: {}\n", url));
            }
            for para in &page.text {
                out.push_str(para);
                out.push_str("\n\n");
            }
            out.push_str(":::\n");
        }

        out
    }

    /// Parse from the .anky string format.
    pub fn from_anky_string(raw: &str) -> Result<Self> {
        // Split frontmatter
        let trimmed = raw.trim_start_matches('\u{feff}').trim();
        if !trimmed.starts_with("---") {
            return Err(anyhow!("missing frontmatter delimiter"));
        }
        let after_first = &trimmed[3..];
        let end_idx = after_first
            .find("\n---")
            .ok_or_else(|| anyhow!("missing closing frontmatter delimiter"))?;
        let frontmatter = &after_first[..end_idx];
        let body = &after_first[end_idx + 4..]; // skip \n---

        // Parse frontmatter fields
        let mut anky_id = String::new();
        let mut fid: Option<i64> = None;
        let mut cast_hash: Option<String> = None;
        let mut written_at = String::new();
        let mut duration_s: u32 = 0;
        let mut word_count: u32 = 0;
        let mut seed = String::new();

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "anky_id" => anky_id = val.to_string(),
                    "fid" => fid = val.parse().ok(),
                    "cast_hash" => cast_hash = Some(val.to_string()),
                    "written_at" => written_at = val.to_string(),
                    "duration_s" => duration_s = val.parse().unwrap_or(0),
                    "word_count" => word_count = val.parse().unwrap_or(0),
                    "seed" => seed = val.to_string(),
                    _ => {}
                }
            }
        }

        // Parse pages
        let mut pages = Vec::new();
        for block in body.split(":::page") {
            let block = block.trim();
            if block.is_empty() || block == ":::" {
                continue;
            }
            // Remove trailing :::
            let content = block.trim_end_matches(":::").trim();

            let mut image_url: Option<String> = None;
            let mut text_lines = Vec::new();
            let mut past_image = false;

            for line in content.lines() {
                if !past_image && line.starts_with("image:") {
                    image_url = Some(line["image:".len()..].trim().to_string());
                    past_image = true;
                } else {
                    past_image = true;
                    text_lines.push(line);
                }
            }

            // Group into paragraphs (split on blank lines)
            let mut paragraphs = Vec::new();
            let mut current = String::new();
            for line in &text_lines {
                if line.trim().is_empty() {
                    if !current.is_empty() {
                        paragraphs.push(current.trim().to_string());
                        current.clear();
                    }
                } else {
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(line.trim());
                }
            }
            if !current.is_empty() {
                paragraphs.push(current.trim().to_string());
            }

            if !paragraphs.is_empty() || image_url.is_some() {
                pages.push(AnkyStoryPage {
                    image_url,
                    text: paragraphs,
                });
            }
        }

        Ok(AnkyStory {
            meta: AnkyStoryMeta {
                anky_id,
                fid,
                cast_hash,
                written_at,
                duration_s,
                word_count,
                seed,
            },
            pages,
        })
    }
}
