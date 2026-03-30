use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::RwLock;

pub type FrameBuffer = Arc<RwLock<RgbaImage>>;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

const RIGHTEOUS_PATH: &str = "static/fonts/Righteous-Regular.ttf";
const MONO_PATH: &str = "/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf";

const BG: Rgba<u8> = Rgba([10, 10, 10, 255]);
const PURPLE: Rgba<u8> = Rgba([123, 47, 247, 255]);
const WHITE: Rgba<u8> = Rgba([232, 232, 232, 255]);
const DIM: Rgba<u8> = Rgba([136, 136, 136, 255]);
const GREEN: Rgba<u8> = Rgba([68, 255, 68, 255]);
const RED: Rgba<u8> = Rgba([255, 68, 68, 255]);
const BAR_BG: Rgba<u8> = Rgba([26, 26, 46, 255]);

struct Fonts {
    righteous: FontRef<'static>,
    mono: FontRef<'static>,
}

static FONTS: std::sync::OnceLock<Fonts> = std::sync::OnceLock::new();

fn load_fonts() -> &'static Fonts {
    FONTS.get_or_init(|| {
        let righteous_data: &'static [u8] = Box::leak(
            std::fs::read(RIGHTEOUS_PATH)
                .expect("Righteous font")
                .into_boxed_slice(),
        );
        let mono_data: &'static [u8] = Box::leak(
            std::fs::read(MONO_PATH)
                .expect("LiberationMono font")
                .into_boxed_slice(),
        );
        Fonts {
            righteous: FontRef::try_from_slice(righteous_data).expect("parse Righteous"),
            mono: FontRef::try_from_slice(mono_data).expect("parse LiberationMono"),
        }
    })
}

pub fn new_frame_buffer() -> FrameBuffer {
    // If fonts aren't available (e.g. Docker/Railway), use a blank frame
    let img = if std::path::Path::new(MONO_PATH).exists()
        && std::path::Path::new(RIGHTEOUS_PATH).exists()
    {
        render_idle_frame()
    } else {
        RgbaImage::from_pixel(WIDTH, HEIGHT, BG)
    };
    Arc::new(RwLock::new(img))
}

fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, color);
            }
        }
    }
}

fn text_width(font: &FontRef, scale: PxScale, text: &str) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut w = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let glyph_id = scaled.glyph_id(ch);
        if let Some(prev_id) = prev {
            w += scaled.kern(prev_id, glyph_id);
        }
        w += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }
    w
}

fn word_wrap_lines(text: &str, font: &FontRef, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let test = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            if text_width(font, scale, &test) > max_width && !current.is_empty() {
                lines.push(current);
                current = word.to_string();
            } else {
                current = test;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

/// Render the idle frame (no one writing).
/// Takes a pulse value (0.0 to 1.0) for a breathing animation so frames aren't identical.
pub fn render_idle_frame_animated(pulse: f64) -> RgbaImage {
    let fonts = load_fonts();
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    // Pulse the "$ANKY" header color brightness
    let p = (pulse * std::f64::consts::PI * 2.0).sin() * 0.5 + 0.5; // 0..1 sinusoidal
    let pr = (123.0 + (132.0 * p)) as u8; // 123..255
    let pg = (47.0 + (30.0 * p)) as u8;
    let pb = (247.0 + (8.0 * (1.0 - p))) as u8;
    let pulse_purple = Rgba([pr, pg, pb.min(255), 255]);

    // "$ANKY" header centered
    let header_scale = PxScale::from(120.0);
    let header = "$ANKY";
    let hw = text_width(&fonts.righteous, header_scale, header);
    let hx = ((WIDTH as f32 - hw) / 2.0) as i32;
    draw_text_mut(
        &mut img,
        pulse_purple,
        hx,
        380,
        header_scale,
        &fonts.righteous,
        header,
    );

    // "waiting for a writer..." subtitle
    let sub_scale = PxScale::from(36.0);
    let sub = "waiting for a writer...";
    let sw = text_width(&fonts.mono, sub_scale, sub);
    let sx = ((WIDTH as f32 - sw) / 2.0) as i32;
    draw_text_mut(&mut img, DIM, sx, 530, sub_scale, &fonts.mono, sub);

    // "anky.app" below
    let url = "anky.app";
    let uw = text_width(&fonts.mono, sub_scale, url);
    let ux = ((WIDTH as f32 - uw) / 2.0) as i32;
    draw_text_mut(&mut img, pulse_purple, ux, 580, sub_scale, &fonts.mono, url);

    img
}

/// Static idle frame (used for initial buffer setup)
pub fn render_idle_frame() -> RgbaImage {
    render_idle_frame_animated(0.0)
}

/// Format a created_at timestamp as a relative time string like "3 hours ago"
fn relative_time(created_at: &str) -> String {
    use chrono::{NaiveDateTime, Utc};
    let parsed = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S%.f"));
    let ts = match parsed {
        Ok(dt) => dt.and_utc(),
        Err(_) => return "recently".to_string(),
    };
    let diff = Utc::now().signed_duration_since(ts);
    let mins = diff.num_minutes();
    if mins < 1 {
        return "just now".to_string();
    }
    if mins < 60 {
        return format!("{} min ago", mins);
    }
    let hours = diff.num_hours();
    if hours < 24 {
        return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
    }
    let days = diff.num_days();
    if days < 30 {
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    }
    format!("{} months ago", days / 30)
}

/// Resolve an image path from DB (may be just filename) to actual file path.
fn resolve_image_path(db_path: &str) -> String {
    if std::path::Path::new(db_path).exists() {
        db_path.to_string()
    } else {
        format!("data/images/{}", db_path)
    }
}

/// Load and scale an anky image to fill the 1920x1080 canvas. Returns None if file missing.
fn load_scaled_anky_image(image_path: &str) -> Option<RgbaImage> {
    let path = resolve_image_path(image_path);
    let src = image::open(&path).ok()?;
    let (src_w, src_h) = src.dimensions();
    if src_w == 0 || src_h == 0 {
        return None;
    }

    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    let scale = HEIGHT as f32 / src_h as f32;
    let new_w = (src_w as f32 * scale) as u32;
    let new_h = HEIGHT;
    let resized = image::imageops::resize(
        &src.to_rgba8(),
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let offset_x = if new_w >= WIDTH {
        0
    } else {
        (WIDTH - new_w) / 2
    };
    let src_start_x = if new_w >= WIDTH {
        (new_w - WIDTH) / 2
    } else {
        0
    };
    let copy_w = new_w.min(WIDTH);
    for y in 0..new_h.min(HEIGHT) {
        for x in 0..copy_w {
            let sx = src_start_x + x;
            let dx = offset_x + x;
            if sx < resized.width() && dx < WIDTH {
                img.put_pixel(dx, y, *resized.get_pixel(sx, y));
            }
        }
    }
    Some(img)
}

/// Draw the "ANKY TV" psychedelic header at top center with time-based color cycling.
fn draw_anky_tv_header(img: &mut RgbaImage, time: f64) {
    let fonts = load_fonts();
    let header = "ANKY TV";
    let header_scale = PxScale::from(72.0);
    let hw = text_width(&fonts.righteous, header_scale, header);
    let hx = ((WIDTH as f32 - hw) / 2.0) as i32;
    let hy = 12i32;

    // Draw each character with a different rainbow phase
    let scaled = fonts.righteous.as_scaled(header_scale);
    let mut cx = hx as f32;
    for (i, ch) in header.chars().enumerate() {
        let phase = time * 2.0 + i as f64 * 0.9;
        let r = ((phase.sin() * 0.5 + 0.5) * 255.0) as u8;
        let g = (((phase + 2.094).sin() * 0.5 + 0.5) * 255.0) as u8;
        let b = (((phase + 4.189).sin() * 0.5 + 0.5) * 255.0) as u8;
        let color = Rgba([r.max(80), g.max(40), b.max(80), 255]);
        let ch_str = ch.to_string();
        draw_text_mut(
            img,
            color,
            cx as i32,
            hy,
            header_scale,
            &fonts.righteous,
            &ch_str,
        );
        let glyph_id = scaled.glyph_id(ch);
        cx += scaled.h_advance(glyph_id);
    }
}

/// Draw overlays on a slideshow frame: gradient strips, attribution, title, ANKY TV header.
fn draw_slideshow_overlays(
    img: &mut RgbaImage,
    anky: &crate::db::queries::SlideshowAnky,
    time: f64,
) {
    let fonts = load_fonts();

    // Top gradient strip (100px)
    for y in 0..100u32 {
        let alpha = (1.0 - (y as f32 / 100.0)) * 0.75;
        for x in 0..WIDTH {
            let p = img.get_pixel(x, y);
            let r = (p[0] as f32 * (1.0 - alpha)) as u8;
            let g = (p[1] as f32 * (1.0 - alpha)) as u8;
            let b = (p[2] as f32 * (1.0 - alpha)) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    // Bottom gradient strip (120px)
    let bottom_start = HEIGHT - 120;
    for y in bottom_start..HEIGHT {
        let progress = (y - bottom_start) as f32 / 120.0;
        let alpha = progress * 0.75;
        for x in 0..WIDTH {
            let p = img.get_pixel(x, y);
            let r = (p[0] as f32 * (1.0 - alpha)) as u8;
            let g = (p[1] as f32 * (1.0 - alpha)) as u8;
            let b = (p[2] as f32 * (1.0 - alpha)) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    // "ANKY TV" psychedelic header at top center
    draw_anky_tv_header(img, time);

    // Attribution text (bottom-left)
    let attr_scale = PxScale::from(26.0);
    let time_ago = relative_time(&anky.created_at);
    let attr_text = if anky.origin == "generated" {
        format!("generated {}", time_ago)
    } else {
        format!("written by @{} {}", anky.display_username, time_ago)
    };
    draw_text_mut(
        img,
        Rgba([200, 200, 200, 255]),
        40,
        (HEIGHT - 100) as i32,
        attr_scale,
        &fonts.mono,
        &attr_text,
    );

    // Title (bottom-center)
    if let Some(ref title) = anky.title {
        if !title.is_empty() {
            let title_scale = PxScale::from(36.0);
            let display_title = if title.len() > 60 {
                format!("{}...", &title[..57])
            } else {
                title.clone()
            };
            let tw = text_width(&fonts.righteous, title_scale, &display_title);
            let tx = ((WIDTH as f32 - tw) / 2.0).max(40.0) as i32;
            draw_text_mut(
                img,
                WHITE,
                tx,
                (HEIGHT - 60) as i32,
                title_scale,
                &fonts.righteous,
                &display_title,
            );
        }
    }
}

/// Block size for pixel dissolve transition
const BLOCK_SIZE: u32 = 30;
const BLOCKS_X: u32 = (WIDTH + BLOCK_SIZE - 1) / BLOCK_SIZE; // 64
const BLOCKS_Y: u32 = (HEIGHT + BLOCK_SIZE - 1) / BLOCK_SIZE; // 36

/// Generate a block order based on distance from center — center blocks get lowest values.
/// Returns a flat vec indexed by (by * BLOCKS_X + bx).
fn generate_block_order() -> Vec<f32> {
    let cx = BLOCKS_X as f32 / 2.0;
    let cy = BLOCKS_Y as f32 / 2.0;
    let max_dist = ((cx * cx) + (cy * cy)).sqrt();
    let total = (BLOCKS_X * BLOCKS_Y) as usize;
    let mut order = vec![0.0f32; total];
    for by in 0..BLOCKS_Y {
        for bx in 0..BLOCKS_X {
            let dx = bx as f32 + 0.5 - cx;
            let dy = by as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / max_dist; // 0..1, 0=center
                                                              // Add a bit of randomness for organic feel
            let noise = ((bx * 7 + by * 13) % 17) as f32 / 170.0;
            order[(by * BLOCKS_X + bx) as usize] = (dist + noise).min(1.0);
        }
    }
    order
}

/// Render a dissolve-out frame: current image's blocks disappear from edges toward center.
/// progress: 0.0 = full image, 1.0 = all dissolved to black.
fn render_dissolve_out(
    source: &RgbaImage,
    progress: f32,
    block_order: &[f32],
    time: f64,
) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    for by in 0..BLOCKS_Y {
        for bx in 0..BLOCKS_X {
            let threshold = block_order[(by * BLOCKS_X + bx) as usize];
            // Blocks with high threshold (edges) dissolve first
            let block_progress = ((1.0 - threshold) - (1.0 - progress)) * 4.0;
            let block_progress = block_progress.clamp(0.0, 1.0);

            // block_progress 0 = fully visible, 1 = fully dissolved
            if block_progress >= 1.0 {
                continue; // block is gone, leave BG
            }

            let x0 = bx * BLOCK_SIZE;
            let y0 = by * BLOCK_SIZE;
            let x1 = (x0 + BLOCK_SIZE).min(WIDTH);
            let y1 = (y0 + BLOCK_SIZE).min(HEIGHT);

            if block_progress <= 0.0 {
                // Fully visible — copy block directly
                for y in y0..y1 {
                    for x in x0..x1 {
                        img.put_pixel(x, y, *source.get_pixel(x, y));
                    }
                }
            } else {
                // Partially dissolved — shrink block toward its center
                let shrink = block_progress;
                let bw = (x1 - x0) as f32;
                let bh = (y1 - y0) as f32;
                let new_w = (bw * (1.0 - shrink)).max(1.0);
                let new_h = (bh * (1.0 - shrink)).max(1.0);
                let ox = x0 as f32 + (bw - new_w) / 2.0;
                let oy = y0 as f32 + (bh - new_h) / 2.0;

                for dy in 0..(new_h as u32) {
                    for dx in 0..(new_w as u32) {
                        let src_x = x0 + (dx as f32 / new_w * bw) as u32;
                        let src_y = y0 + (dy as f32 / new_h * bh) as u32;
                        let dst_x = (ox as u32 + dx).min(WIDTH - 1);
                        let dst_y = (oy as u32 + dy).min(HEIGHT - 1);
                        if src_x < WIDTH && src_y < HEIGHT {
                            let p = source.get_pixel(src_x, src_y);
                            // Fade to darker as it dissolves
                            let fade = 1.0 - shrink * 0.5;
                            let r = (p[0] as f32 * fade) as u8;
                            let g = (p[1] as f32 * fade) as u8;
                            let b = (p[2] as f32 * fade) as u8;
                            img.put_pixel(dst_x, dst_y, Rgba([r, g, b, 255]));
                        }
                    }
                }
            }
        }
    }

    // Still draw the ANKY TV header on transition frames
    draw_anky_tv_header(&mut img, time);
    img
}

/// Render a dissolve-in frame: new image's blocks appear from center outward.
/// progress: 0.0 = all black, 1.0 = full image.
fn render_dissolve_in(
    target: &RgbaImage,
    progress: f32,
    block_order: &[f32],
    time: f64,
) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    for by in 0..BLOCKS_Y {
        for bx in 0..BLOCKS_X {
            let threshold = block_order[(by * BLOCKS_X + bx) as usize];
            // Blocks with low threshold (center) appear first
            let block_progress = (progress - threshold) * 4.0;
            let block_progress = block_progress.clamp(0.0, 1.0);

            // block_progress 0 = invisible, 1 = fully visible
            if block_progress <= 0.0 {
                continue; // not yet appeared
            }

            let x0 = bx * BLOCK_SIZE;
            let y0 = by * BLOCK_SIZE;
            let x1 = (x0 + BLOCK_SIZE).min(WIDTH);
            let y1 = (y0 + BLOCK_SIZE).min(HEIGHT);

            if block_progress >= 1.0 {
                // Fully visible
                for y in y0..y1 {
                    for x in x0..x1 {
                        img.put_pixel(x, y, *target.get_pixel(x, y));
                    }
                }
            } else {
                // Growing from center of block
                let bw = (x1 - x0) as f32;
                let bh = (y1 - y0) as f32;
                let new_w = (bw * block_progress).max(1.0);
                let new_h = (bh * block_progress).max(1.0);
                let ox = x0 as f32 + (bw - new_w) / 2.0;
                let oy = y0 as f32 + (bh - new_h) / 2.0;

                for dy in 0..(new_h as u32) {
                    for dx in 0..(new_w as u32) {
                        let src_x = x0 + (dx as f32 / new_w * bw) as u32;
                        let src_y = y0 + (dy as f32 / new_h * bh) as u32;
                        let dst_x = (ox as u32 + dx).min(WIDTH - 1);
                        let dst_y = (oy as u32 + dy).min(HEIGHT - 1);
                        if src_x < WIDTH && src_y < HEIGHT {
                            let p = target.get_pixel(src_x, src_y);
                            let fade = 0.5 + block_progress * 0.5;
                            let r = (p[0] as f32 * fade) as u8;
                            let g = (p[1] as f32 * fade) as u8;
                            let b = (p[2] as f32 * fade) as u8;
                            img.put_pixel(dst_x, dst_y, Rgba([r, g, b, 255]));
                        }
                    }
                }
            }
        }
    }

    draw_anky_tv_header(&mut img, time);
    img
}

/// Render a full static slideshow frame with all overlays.
fn render_slideshow_frame(
    anky: &crate::db::queries::SlideshowAnky,
    time: f64,
) -> Option<RgbaImage> {
    let mut img = load_scaled_anky_image(&anky.image_path)?;
    draw_slideshow_overlays(&mut img, anky, time);
    Some(img)
}

/// Render a live writing frame
pub fn render_live_frame(
    username: &str,
    text: &str,
    words: i64,
    elapsed: f64,
    idle_ratio: f64,
    progress: f64,
) -> RgbaImage {
    render_live_frame_typed(
        username, text, words, elapsed, idle_ratio, progress, "human",
    )
}

/// Render a live writing frame with writer type label
pub fn render_live_frame_typed(
    username: &str,
    text: &str,
    words: i64,
    elapsed: f64,
    idle_ratio: f64,
    progress: f64,
    writer_type: &str,
) -> RgbaImage {
    let fonts = load_fonts();
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    let margin: u32 = 40;
    let content_width = WIDTH - margin * 2;

    // Row 1: @username top-left (y=20) + [type] badge
    let user_scale = PxScale::from(32.0);
    let user_label = if writer_type == "agent" {
        format!("@{} [agent]", username)
    } else {
        format!("@{}", username)
    };
    draw_text_mut(
        &mut img,
        PURPLE,
        margin as i32,
        20,
        user_scale,
        &fonts.mono,
        &user_label,
    );

    // Life bar (y=65, h=10)
    let life_y: u32 = 65;
    draw_rect(&mut img, margin, life_y, content_width, 10, BAR_BG);
    let idle = idle_ratio.clamp(0.0, 1.0);
    let fill_w = (idle * content_width as f64) as u32;
    if fill_w > 0 {
        // Color gradient: green when full, red when low
        let color = if idle > 0.5 { GREEN } else { RED };
        draw_rect(&mut img, margin, life_y, fill_w, 10, color);
    }

    // Text area (y=90 to y=980)
    let text_y_start: u32 = 90;
    let text_y_end: u32 = 980;
    let text_area_h = text_y_end - text_y_start;

    // Draw text background
    draw_rect(
        &mut img,
        margin,
        text_y_start,
        content_width,
        text_area_h,
        Rgba([15, 15, 26, 255]),
    );

    // Render wrapped text
    let text_scale = PxScale::from(28.0);
    let line_height: u32 = 38;
    let text_padding: u32 = 20;
    let max_text_w = (content_width - text_padding * 2) as f32;
    let lines = word_wrap_lines(text, &fonts.mono, text_scale, max_text_w);

    let max_visible_lines = ((text_area_h - text_padding * 2) / line_height) as usize;
    let start = if lines.len() > max_visible_lines {
        lines.len() - max_visible_lines
    } else {
        0
    };

    for (i, line) in lines[start..].iter().enumerate() {
        let ly = text_y_start + text_padding + (i as u32) * line_height;
        if ly + line_height > text_y_end {
            break;
        }
        draw_text_mut(
            &mut img,
            WHITE,
            (margin + text_padding) as i32,
            ly as i32,
            text_scale,
            &fonts.mono,
            line,
        );
    }

    // Stats row (y=990): "N words" left, "M:SS" right
    let stats_scale = PxScale::from(30.0);
    let words_str = format!("{} words", words);
    let mins = (elapsed / 60.0).floor() as u32;
    let secs = (elapsed % 60.0).floor() as u32;
    let time_str = format!("{}:{:02}", mins, secs);

    draw_text_mut(
        &mut img,
        DIM,
        margin as i32,
        990,
        stats_scale,
        &fonts.mono,
        &words_str,
    );
    let tw = text_width(&fonts.mono, stats_scale, &time_str);
    let tx = (WIDTH - margin) as f32 - tw;
    draw_text_mut(
        &mut img,
        DIM,
        tx as i32,
        990,
        stats_scale,
        &fonts.mono,
        &time_str,
    );

    // Progress bar (y=1030, h=14)
    let prog_y: u32 = 1030;
    draw_rect(&mut img, margin, prog_y, content_width, 14, BAR_BG);
    let prog = progress.clamp(0.0, 1.0);
    let prog_w = (prog * content_width as f64) as u32;
    if prog_w > 0 {
        draw_rect(&mut img, margin, prog_y, prog_w, 14, PURPLE);
    }

    // Progress label
    let prog_label = format!("{}%", (prog * 100.0).round() as u32);
    let pl_scale = PxScale::from(20.0);
    let plw = text_width(&fonts.mono, pl_scale, &prog_label);
    let plx = (WIDTH - margin) as f32 - plw;
    draw_text_mut(
        &mut img,
        DIM,
        plx as i32,
        1050,
        pl_scale,
        &fonts.mono,
        &prog_label,
    );

    img
}

/// Render a congratulations frame after 8-minute anky completion
pub fn render_congrats_frame(username: &str) -> RgbaImage {
    let fonts = load_fonts();
    let mut img = RgbaImage::from_pixel(WIDTH, HEIGHT, BG);

    // Big congrats message
    let congrats_scale = PxScale::from(72.0);
    let line1 = format!("@{}", username);
    let line2 = "just wrote an anky!";

    let l1w = text_width(&fonts.righteous, congrats_scale, &line1);
    let l1x = ((WIDTH as f32 - l1w) / 2.0) as i32;
    draw_text_mut(
        &mut img,
        PURPLE,
        l1x,
        380,
        congrats_scale,
        &fonts.righteous,
        &line1,
    );

    let l2w = text_width(&fonts.righteous, congrats_scale, line2);
    let l2x = ((WIDTH as f32 - l2w) / 2.0) as i32;
    draw_text_mut(
        &mut img,
        WHITE,
        l2x,
        480,
        congrats_scale,
        &fonts.righteous,
        line2,
    );

    // "anky.app" below
    let sub_scale = PxScale::from(36.0);
    let url = "anky.app";
    let uw = text_width(&fonts.mono, sub_scale, url);
    let ux = ((WIDTH as f32 - uw) / 2.0) as i32;
    draw_text_mut(&mut img, PURPLE, ux, 600, sub_scale, &fonts.mono, url);

    img
}

/// Set the frame buffer to congratulations
pub async fn set_congrats_frame(buf: &FrameBuffer, username: &str) {
    let frame = render_congrats_frame(username);
    let mut guard = buf.write().await;
    *guard = frame;
}

/// Update the shared frame buffer with a live writing frame
pub async fn update_live_frame(
    buf: &FrameBuffer,
    username: &str,
    text: &str,
    words: i64,
    elapsed: f64,
    idle_ratio: f64,
    progress: f64,
) {
    let frame = render_live_frame(username, text, words, elapsed, idle_ratio, progress);
    let mut guard = buf.write().await;
    *guard = frame;
}

/// Update with writer type label (human/agent)
pub async fn update_live_frame_typed(
    buf: &FrameBuffer,
    username: &str,
    text: &str,
    words: i64,
    elapsed: f64,
    idle_ratio: f64,
    progress: f64,
    writer_type: &str,
) {
    let frame = render_live_frame_typed(
        username,
        text,
        words,
        elapsed,
        idle_ratio,
        progress,
        writer_type,
    );
    let mut guard = buf.write().await;
    *guard = frame;
}

/// Set the frame buffer to idle
pub async fn set_idle_frame(buf: &FrameBuffer) {
    let frame = render_idle_frame();
    let mut guard = buf.write().await;
    *guard = frame;
}

/// Detect a running PulseAudio monitor source (captures system audio output).
async fn detect_pulse_monitor() -> Option<String> {
    let output = Command::new("pactl")
        .args(["list", "short", "sources"])
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Pick the first .monitor source that is RUNNING, else first .monitor source
    let mut first_monitor = None;
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && parts[1].ends_with(".monitor") {
            if first_monitor.is_none() {
                first_monitor = Some(parts[1].to_string());
            }
            if parts.len() >= 5 && parts[4].contains("RUNNING") {
                return Some(parts[1].to_string());
            }
        }
    }
    first_monitor
}

/// Spawn the always-on ffmpeg RTMP stream. Reads frames from the shared buffer at 10fps.
/// Restarts on crash with 5s delay.
pub async fn spawn_ffmpeg_loop(
    rtmp_url: String,
    stream_key: String,
    frame_buf: FrameBuffer,
    live_state: Arc<RwLock<crate::state::LiveState>>,
    db: crate::db::DbPool,
) {
    // Pre-render idle frame
    set_idle_frame(&frame_buf).await;

    loop {
        let full_url = format!("{}/{}", rtmp_url.trim_end_matches('/'), stream_key);

        tracing::info!(
            "Starting ffmpeg RTMP stream (rawvideo pipe) to {}",
            rtmp_url
        );

        // Detect PulseAudio monitor source for system audio passthrough
        let pulse_source = detect_pulse_monitor().await;

        let mut args: Vec<String> = vec![
            // Video input: rawvideo from pipe
            "-thread_queue_size".into(),
            "1024".into(),
            "-f".into(),
            "rawvideo".into(),
            "-pixel_format".into(),
            "rgba".into(),
            "-video_size".into(),
            format!("{}x{}", WIDTH, HEIGHT),
            "-framerate".into(),
            "10".into(),
            "-i".into(),
            "pipe:0".into(),
        ];

        if let Some(ref src) = pulse_source {
            // Audio input: system audio via PulseAudio monitor
            args.extend([
                "-thread_queue_size".into(),
                "1024".into(),
                "-f".into(),
                "pulse".into(),
                "-i".into(),
                src.clone(),
            ]);
            tracing::info!("Audio source: {}", src);
        } else {
            // No audio — generate silent audio so RTMP has an audio track
            args.extend([
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                "anullsrc=r=44100:cl=stereo".into(),
            ]);
            tracing::warn!("No PulseAudio monitor found, using silent audio");
        }

        args.extend([
            // Explicit stream mapping: video from input 0, audio from input 1
            "-map".into(),
            "0:v".into(),
            "-map".into(),
            "1:a".into(),
            // Video encoding
            "-c:v".into(),
            "h264_nvenc".into(),
            "-preset".into(),
            "p1".into(),
            "-tune".into(),
            "ll".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-b:v".into(),
            "2500k".into(),
            "-g".into(),
            "20".into(),
            // Audio encoding — libfdk_aac with AAC-LC profile for mobile compatibility
            "-c:a".into(),
            "libfdk_aac".into(),
            "-profile:a".into(),
            "aac_low".into(),
            "-b:a".into(),
            "128k".into(),
            "-ar".into(),
            "44100".into(),
            "-ac".into(),
            "2".into(),
            "-f".into(),
            "flv".into(),
            full_url.clone(),
        ]);

        let result = Command::new("ffmpeg")
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                let mut stdin = child.stdin.take().expect("ffmpeg stdin");
                let buf_clone = frame_buf.clone();

                // Pump frames at 10fps with animated idle / slideshow + transitions
                let live_state_clone = live_state.clone();
                let idle_buf = buf_clone.clone();
                let db_clone = db.clone();
                let idle_animator = tokio::spawn(async move {
                    let global_start = std::time::Instant::now();

                    // Slideshow state
                    let mut slideshow_ankys: Vec<crate::db::queries::SlideshowAnky> = Vec::new();
                    let mut slideshow_videos: Vec<crate::db::queries::SlideshowVideo> = Vec::new();
                    let mut last_db_refresh =
                        std::time::Instant::now() - std::time::Duration::from_secs(999);
                    let block_order = generate_block_order();

                    // Playlist: interleaves images and videos
                    #[derive(Clone)]
                    enum SlideshowItem {
                        Image(usize),
                        Video(usize),
                    }
                    let mut playlist: Vec<SlideshowItem> = Vec::new();
                    let mut playlist_pos: usize = 0;

                    // Preloaded images: current slide (with overlays) and raw next slide
                    let mut current_frame: Option<RgbaImage> = None;
                    let mut current_raw: Option<RgbaImage> = None;
                    let mut next_raw: Option<RgbaImage> = None;

                    // Transition state machine
                    #[derive(PartialEq)]
                    enum Phase {
                        Showing,
                        DissolveOut,
                        DissolveIn,
                        PlayingVideo,
                    }
                    let mut phase = Phase::Showing;
                    let mut phase_start = std::time::Instant::now();

                    // Video playback state
                    let mut video_child: Option<tokio::process::Child> = None;
                    let mut video_stdout: Option<tokio::process::ChildStdout> = None;

                    const SLIDE_SECS: f64 = 8.0;
                    const DISSOLVE_SECS: f64 = 0.8;
                    const DB_REFRESH_SECS: u64 = 300;
                    const VIDEO_MAX_SECS: f64 = 120.0;
                    const VIDEO_FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 4;

                    /// Build a playlist: insert a video every ~5 images
                    fn build_playlist(num_images: usize, num_videos: usize) -> Vec<SlideshowItem> {
                        if num_images == 0 {
                            return Vec::new();
                        }
                        let mut pl = Vec::new();
                        let mut vid_idx = 0;
                        for i in 0..num_images {
                            pl.push(SlideshowItem::Image(i));
                            // Insert a video every 5 images (if we have videos)
                            if num_videos > 0 && (i + 1) % 5 == 0 {
                                pl.push(SlideshowItem::Video(vid_idx % num_videos));
                                vid_idx += 1;
                            }
                        }
                        pl
                    }

                    loop {
                        let state = live_state_clone.read().await;
                        let is_idle = !state.is_live && !state.showing_congrats;
                        drop(state);

                        let time = global_start.elapsed().as_secs_f64();

                        if is_idle {
                            // Refresh DB periodically
                            if last_db_refresh.elapsed().as_secs() >= DB_REFRESH_SECS {
                                let (ankys, videos) = {
                                    let Some(conn) = crate::db::get_conn_logged(&db_clone) else {
                                        tokio::time::sleep(std::time::Duration::from_millis(250))
                                            .await;
                                        continue;
                                    };
                                    let a = crate::db::queries::get_slideshow_ankys(&conn)
                                        .unwrap_or_default();
                                    let v = crate::db::queries::get_slideshow_videos(&conn)
                                        .unwrap_or_default();
                                    (a, v)
                                };
                                if !ankys.is_empty() {
                                    tracing::info!(
                                        "Slideshow: loaded {} ankys, {} videos from DB",
                                        ankys.len(),
                                        videos.len()
                                    );
                                }
                                slideshow_ankys = ankys;
                                slideshow_videos = videos;
                                playlist =
                                    build_playlist(slideshow_ankys.len(), slideshow_videos.len());
                                playlist_pos = 0;
                                last_db_refresh = std::time::Instant::now();
                                current_frame = None;
                                current_raw = None;
                                phase = Phase::Showing;
                                phase_start = std::time::Instant::now();
                            }

                            if slideshow_ankys.is_empty() {
                                // Fallback: pulsing idle animation
                                let pulse = (time % 4.0) / 4.0;
                                let frame = render_idle_frame_animated(pulse);
                                let mut guard = idle_buf.write().await;
                                *guard = frame;
                                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                continue;
                            }

                            // If playlist exhausted, regenerate
                            if playlist_pos >= playlist.len() {
                                playlist =
                                    build_playlist(slideshow_ankys.len(), slideshow_videos.len());
                                playlist_pos = 0;
                                // Also trigger a DB refresh on next cycle
                                last_db_refresh =
                                    std::time::Instant::now() - std::time::Duration::from_secs(999);
                            }

                            // Handle PlayingVideo phase separately
                            if phase == Phase::PlayingVideo {
                                use tokio::io::AsyncReadExt;
                                let elapsed = phase_start.elapsed().as_secs_f64();

                                // Check time limit
                                if elapsed >= VIDEO_MAX_SECS {
                                    tracing::info!(
                                        "Slideshow: video hit {}s cap, stopping",
                                        VIDEO_MAX_SECS
                                    );
                                    if let Some(ref mut child) = video_child {
                                        let _ = child.kill().await;
                                    }
                                    video_child = None;
                                    video_stdout = None;
                                    playlist_pos += 1;
                                    // Go to dissolve-out then next item
                                    phase = Phase::DissolveOut;
                                    phase_start = std::time::Instant::now();
                                    // current_raw holds the last video frame for dissolve-out
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                    continue;
                                }

                                // Try to read a frame from ffmpeg stdout
                                let mut frame_done = false;
                                if let Some(ref mut stdout) = video_stdout {
                                    let mut buf = vec![0u8; VIDEO_FRAME_SIZE];
                                    let mut total_read = 0;
                                    let read_result = loop {
                                        match stdout.read(&mut buf[total_read..]).await {
                                            Ok(0) => break Ok(total_read),
                                            Ok(n) => {
                                                total_read += n;
                                                if total_read >= VIDEO_FRAME_SIZE {
                                                    break Ok(total_read);
                                                }
                                            }
                                            Err(e) => break Err(e),
                                        }
                                    };
                                    match read_result {
                                        Ok(n) if n >= VIDEO_FRAME_SIZE => {
                                            // Got a full frame — write to buffer
                                            let frame_img = RgbaImage::from_raw(
                                                WIDTH,
                                                HEIGHT,
                                                buf[..VIDEO_FRAME_SIZE].to_vec(),
                                            );
                                            if let Some(mut frame) = frame_img {
                                                draw_anky_tv_header(&mut frame, time);
                                                // Keep a copy for dissolve-out
                                                current_raw = Some(frame.clone());
                                                let mut guard = idle_buf.write().await;
                                                *guard = frame;
                                            }
                                        }
                                        _ => {
                                            // EOF or error — video finished
                                            frame_done = true;
                                        }
                                    }
                                } else {
                                    frame_done = true;
                                }

                                if frame_done {
                                    tracing::info!("Slideshow: video playback finished");
                                    if let Some(ref mut child) = video_child {
                                        let _ = child.kill().await;
                                    }
                                    video_child = None;
                                    video_stdout = None;
                                    playlist_pos += 1;
                                    phase = Phase::DissolveOut;
                                    phase_start = std::time::Instant::now();
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                    continue;
                                }

                                // ~10fps for video playback
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                continue;
                            }

                            // For image phases, ensure current slide is loaded
                            if current_frame.is_none() && phase == Phase::Showing {
                                // Find the current playlist item
                                let mut loaded = false;
                                while playlist_pos < playlist.len() {
                                    match &playlist[playlist_pos] {
                                        SlideshowItem::Image(idx) => {
                                            let idx = *idx;
                                            if idx < slideshow_ankys.len() {
                                                let anky = &slideshow_ankys[idx];
                                                if let Some(raw) =
                                                    load_scaled_anky_image(&anky.image_path)
                                                {
                                                    let mut frm = raw.clone();
                                                    draw_slideshow_overlays(&mut frm, anky, time);
                                                    current_frame = Some(frm);
                                                    current_raw = Some(raw);
                                                    loaded = true;
                                                    break;
                                                } else {
                                                    tracing::warn!(
                                                        "Slideshow: missing image for anky {}",
                                                        &anky.id[..8.min(anky.id.len())]
                                                    );
                                                    playlist_pos += 1;
                                                }
                                            } else {
                                                playlist_pos += 1;
                                            }
                                        }
                                        SlideshowItem::Video(_) => {
                                            // Start video playback
                                            break;
                                        }
                                    }
                                }
                                if !loaded && playlist_pos < playlist.len() {
                                    if let SlideshowItem::Video(vid_idx) = &playlist[playlist_pos] {
                                        let vid_idx = *vid_idx;
                                        if vid_idx < slideshow_videos.len() {
                                            let video = &slideshow_videos[vid_idx];
                                            let video_path = video.video_path.clone();
                                            if std::path::Path::new(&video_path).exists() {
                                                tracing::info!(
                                                    "Slideshow: playing video {}",
                                                    &video.id[..8.min(video.id.len())]
                                                );
                                                let spawn_result = Command::new("ffmpeg")
                                                    .args([
                                                        "-i", &video_path,
                                                        "-vf", &format!(
                                                            "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=#0A0A0A",
                                                            WIDTH, HEIGHT, WIDTH, HEIGHT
                                                        ),
                                                        "-r", "10",
                                                        "-pix_fmt", "rgba",
                                                        "-f", "rawvideo",
                                                        "pipe:1",
                                                    ])
                                                    .stdin(std::process::Stdio::null())
                                                    .stdout(std::process::Stdio::piped())
                                                    .stderr(std::process::Stdio::null())
                                                    .spawn();
                                                match spawn_result {
                                                    Ok(mut child) => {
                                                        video_stdout = child.stdout.take();
                                                        video_child = Some(child);
                                                        phase = Phase::PlayingVideo;
                                                        phase_start = std::time::Instant::now();
                                                        tokio::time::sleep(
                                                            std::time::Duration::from_millis(100),
                                                        )
                                                        .await;
                                                        continue;
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!("Slideshow: failed to spawn ffmpeg for video: {}", e);
                                                        playlist_pos += 1;
                                                    }
                                                }
                                            } else {
                                                tracing::warn!(
                                                    "Slideshow: video file missing: {}",
                                                    video_path
                                                );
                                                playlist_pos += 1;
                                            }
                                        } else {
                                            playlist_pos += 1;
                                        }
                                    }
                                }
                                if !loaded && current_frame.is_none() {
                                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                    continue;
                                }
                                phase = Phase::Showing;
                                phase_start = std::time::Instant::now();
                            }

                            let elapsed = phase_start.elapsed().as_secs_f64();

                            match phase {
                                Phase::Showing => {
                                    // Display cached frame
                                    if let Some(ref frame) = current_frame {
                                        let mut guard = idle_buf.write().await;
                                        *guard = frame.clone();
                                    }

                                    if elapsed >= SLIDE_SECS {
                                        // Time to advance — start dissolve out
                                        phase = Phase::DissolveOut;
                                        phase_start = std::time::Instant::now();
                                    }

                                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                }
                                Phase::DissolveOut => {
                                    let progress = (elapsed / DISSOLVE_SECS).min(1.0) as f32;

                                    if let Some(ref raw) = current_raw {
                                        let frame =
                                            render_dissolve_out(raw, progress, &block_order, time);
                                        let mut guard = idle_buf.write().await;
                                        *guard = frame;
                                    }

                                    if elapsed >= DISSOLVE_SECS {
                                        // Advance playlist
                                        playlist_pos += 1;
                                        if playlist_pos >= playlist.len() {
                                            playlist = build_playlist(
                                                slideshow_ankys.len(),
                                                slideshow_videos.len(),
                                            );
                                            playlist_pos = 0;
                                            last_db_refresh = std::time::Instant::now()
                                                - std::time::Duration::from_secs(999);
                                        }

                                        // Check what's next
                                        current_frame = None;
                                        current_raw = None;
                                        next_raw = None;

                                        if playlist_pos < playlist.len() {
                                            match &playlist[playlist_pos] {
                                                SlideshowItem::Image(idx) => {
                                                    let idx = *idx;
                                                    if idx < slideshow_ankys.len() {
                                                        if let Some(raw) = load_scaled_anky_image(
                                                            &slideshow_ankys[idx].image_path,
                                                        ) {
                                                            next_raw = Some(raw);
                                                            phase = Phase::DissolveIn;
                                                            phase_start = std::time::Instant::now();
                                                        } else {
                                                            // Skip bad image
                                                            playlist_pos += 1;
                                                            phase = Phase::Showing;
                                                            phase_start = std::time::Instant::now();
                                                        }
                                                    } else {
                                                        playlist_pos += 1;
                                                        phase = Phase::Showing;
                                                        phase_start = std::time::Instant::now();
                                                    }
                                                }
                                                SlideshowItem::Video(vid_idx) => {
                                                    let vid_idx = *vid_idx;
                                                    // Start video — skip dissolve-in, go straight to playing
                                                    let mut started = false;
                                                    if vid_idx < slideshow_videos.len() {
                                                        let video_path = slideshow_videos[vid_idx]
                                                            .video_path
                                                            .clone();
                                                        if std::path::Path::new(&video_path)
                                                            .exists()
                                                        {
                                                            let spawn_result = Command::new("ffmpeg")
                                                                .args([
                                                                    "-i", &video_path,
                                                                    "-vf", &format!(
                                                                        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=#0A0A0A",
                                                                        WIDTH, HEIGHT, WIDTH, HEIGHT
                                                                    ),
                                                                    "-r", "10",
                                                                    "-pix_fmt", "rgba",
                                                                    "-f", "rawvideo",
                                                                    "pipe:1",
                                                                ])
                                                                .stdin(std::process::Stdio::null())
                                                                .stdout(std::process::Stdio::piped())
                                                                .stderr(std::process::Stdio::null())
                                                                .spawn();
                                                            if let Ok(mut child) = spawn_result {
                                                                video_stdout = child.stdout.take();
                                                                video_child = Some(child);
                                                                phase = Phase::PlayingVideo;
                                                                phase_start =
                                                                    std::time::Instant::now();
                                                                started = true;
                                                            }
                                                        }
                                                    }
                                                    if !started {
                                                        playlist_pos += 1;
                                                        phase = Phase::Showing;
                                                        phase_start = std::time::Instant::now();
                                                    }
                                                }
                                            }
                                        } else {
                                            phase = Phase::Showing;
                                            phase_start = std::time::Instant::now();
                                        }
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                                Phase::DissolveIn => {
                                    let progress = (elapsed / DISSOLVE_SECS).min(1.0) as f32;

                                    if let Some(ref raw) = next_raw {
                                        let frame =
                                            render_dissolve_in(raw, progress, &block_order, time);
                                        let mut guard = idle_buf.write().await;
                                        *guard = frame;
                                    }

                                    if elapsed >= DISSOLVE_SECS {
                                        // Transition complete — next becomes current
                                        if playlist_pos < playlist.len() {
                                            if let SlideshowItem::Image(idx) =
                                                &playlist[playlist_pos]
                                            {
                                                let idx = *idx;
                                                if idx < slideshow_ankys.len() {
                                                    let anky = &slideshow_ankys[idx];
                                                    if let Some(raw) = next_raw.take() {
                                                        let mut frm = raw.clone();
                                                        draw_slideshow_overlays(
                                                            &mut frm, anky, time,
                                                        );
                                                        current_frame = Some(frm);
                                                        current_raw = Some(raw);
                                                    }
                                                }
                                            }
                                        }
                                        phase = Phase::Showing;
                                        phase_start = std::time::Instant::now();
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                                Phase::PlayingVideo => {
                                    // Handled above, but just in case
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                            }
                        } else {
                            // Not idle — reset slideshow state, kill any playing video
                            if let Some(ref mut child) = video_child {
                                let _ = child.kill().await;
                            }
                            video_child = None;
                            video_stdout = None;
                            current_frame = None;
                            current_raw = None;
                            next_raw = None;
                            phase = Phase::Showing;
                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        }
                    }
                });

                let pump_handle = tokio::spawn(async move {
                    let frame_interval = std::time::Duration::from_millis(100);
                    loop {
                        let frame_data = {
                            let guard = buf_clone.read().await;
                            guard.as_raw().clone()
                        };
                        if let Err(_) = stdin.write_all(&frame_data).await {
                            break;
                        }
                        if let Err(_) = stdin.flush().await {
                            break;
                        }
                        tokio::time::sleep(frame_interval).await;
                    }
                });

                let status = child.wait().await;
                pump_handle.abort();
                idle_animator.abort();
                tracing::warn!("ffmpeg exited: {:?}", status);
            }
            Err(e) => {
                tracing::error!("Failed to spawn ffmpeg: {}", e);
            }
        }

        // On crash, reset live state
        {
            let mut state = live_state.write().await;
            if state.is_live {
                state.is_live = false;
                state.writer_id = None;
                state.writer_username = None;
                state.writer_type = None;
            }
        }
        set_idle_frame(&frame_buf).await;

        tracing::info!("Restarting ffmpeg in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
