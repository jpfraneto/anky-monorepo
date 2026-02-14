use crate::services::{claude, gemini};
use crate::state::AppState;
use anyhow::Result;

/// Generate a 2:1 prompt image with text overlay.
/// 1. Claude Haiku creates a scene prompt from the self-inquiry question
/// 2. Gemini generates 2:1 base image
/// 3. Rust image processing overlays the prompt text
pub async fn generate_prompt_image(
    state: &AppState,
    prompt_id: &str,
    prompt_text: &str,
) -> Result<String> {
    let api_key = &state.config.anthropic_api_key;
    let gemini_key = &state.config.gemini_api_key;

    if api_key.is_empty() || gemini_key.is_empty() {
        anyhow::bail!("API keys not configured");
    }

    state.emit_log("INFO", "prompt_gen", &format!("Generating prompt image for {}", &prompt_id[..8]));

    // Step 1: Claude Haiku creates a scene prompt
    let scene_result = claude::generate_prompt_scene(api_key, prompt_text).await?;
    let scene_prompt = scene_result.text;
    state.emit_log("INFO", "prompt_gen", &format!("Scene prompt: {}", &scene_prompt[..scene_prompt.len().min(80)]));

    // Step 2: Gemini generates wide base image (16:9 — closest supported to 2:1)
    let references = gemini::load_references(std::path::Path::new("src/public"));
    let image_result = gemini::generate_image_with_aspect(gemini_key, &scene_prompt, &references, "16:9").await?;

    // Decode base64 image
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&image_result.base64)?;

    // Step 3: Overlay prompt text on the image
    let final_bytes = overlay_text_on_image(&image_bytes, prompt_text)?;

    // Save the final image
    let filename = format!("prompt_{}.png", prompt_id);
    let path = std::path::Path::new("data/images").join(&filename);
    std::fs::create_dir_all("data/images")?;
    std::fs::write(&path, &final_bytes)?;

    state.emit_log("INFO", "prompt_gen", &format!("Prompt image saved: {} ({}KB)", filename, final_bytes.len() / 1024));

    Ok(filename)
}

use base64::Engine;

/// Overlay prompt text on an image with a semi-transparent dark band at the top-left.
fn overlay_text_on_image(image_bytes: &[u8], text: &str) -> Result<Vec<u8>> {
    use ab_glyph::{FontRef, PxScale};
    use image::{DynamicImage, Rgba, RgbaImage};
    use imageproc::drawing::draw_text_mut;

    let img = image::load_from_memory(image_bytes)?;
    let (width, height) = (img.width(), img.height());
    let mut canvas: RgbaImage = img.to_rgba8();

    // Load font
    let font_data = std::fs::read("static/fonts/Righteous-Regular.ttf")
        .unwrap_or_else(|_| include_bytes!("../../static/fonts/Righteous-Regular.ttf").to_vec());
    let font = FontRef::try_from_slice(&font_data)?;

    // Calculate font size — scale relative to image width
    let font_size = (width as f32 * 0.04).max(18.0);
    let scale = PxScale::from(font_size);

    // Word-wrap text to fit within 55% of image width (left-aligned, don't span too wide)
    let max_text_width = (width as f32 * 0.55) as usize;
    let lines = word_wrap(text, &font, scale, max_text_width);

    // Calculate text block dimensions for the dark band
    let line_height = (font_size * 1.5) as i32;
    let total_text_height = lines.len() as i32 * line_height;
    let padding = (font_size * 0.8) as u32;
    let text_x = padding as i32;
    let text_start_y = padding as i32;

    // Draw semi-transparent dark band behind text at top-left
    let band_w = max_text_width as u32 + padding * 2;
    let band_h = total_text_height as u32 + padding * 2;
    for y in 0..band_h.min(height) {
        // Fade out at the bottom and right edges
        let y_fade = if y > band_h.saturating_sub(padding / 2) {
            1.0 - (y - band_h.saturating_sub(padding / 2)) as f32 / (padding / 2) as f32
        } else {
            1.0
        };
        for x in 0..band_w.min(width) {
            let x_fade = if x > band_w.saturating_sub(padding / 2) {
                1.0 - (x - band_w.saturating_sub(padding / 2)) as f32 / (padding / 2) as f32
            } else {
                1.0
            };
            let alpha = (160.0 * y_fade * x_fade) as u8;
            let pixel = canvas.get_pixel_mut(x, y);
            let a = alpha as f32 / 255.0;
            let r = ((pixel[0] as f32) * (1.0 - a)) as u8;
            let g = ((pixel[1] as f32) * (1.0 - a)) as u8;
            let b = ((pixel[2] as f32) * (1.0 - a)) as u8;
            *pixel = Rgba([r, g, b, 255]);
        }
    }

    // Draw each line left-aligned at top-left
    let text_color = Rgba([255u8, 255, 255, 255]); // white

    for (i, line) in lines.iter().enumerate() {
        let y = text_start_y + (i as i32 * line_height);
        draw_text_mut(&mut canvas, text_color, text_x, y, scale, &font, line);
    }

    // Encode back to PNG
    let dynamic = DynamicImage::ImageRgba8(canvas);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic.write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

fn measure_text_width(font: &ab_glyph::FontRef, scale: ab_glyph::PxScale, text: &str) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    let mut width = 0.0f32;
    let mut prev = None;
    for c in text.chars() {
        let glyph_id = scaled.glyph_id(c);
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }
    width
}

fn word_wrap(text: &str, font: &ab_glyph::FontRef, scale: ab_glyph::PxScale, max_width: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        let test = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };
        let w = measure_text_width(font, scale, &test);
        if w > max_width as f32 && !current_line.is_empty() {
            lines.push(current_line);
            current_line = word.to_string();
        } else {
            current_line = test;
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}
