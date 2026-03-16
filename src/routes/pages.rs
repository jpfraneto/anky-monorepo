use crate::error::AppError;
use crate::state::{AppState, GpuStatus};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path as FsPath, PathBuf};
use std::time::SystemTime;

fn video_public_url(path: &str) -> String {
    if let Some(rel) = path.strip_prefix("videos/") {
        format!("/videos/{}", rel)
    } else if let Some(rel) = path.strip_prefix("data/videos/") {
        format!("/data/videos/{}", rel)
    } else {
        format!("/videos/{}", path.trim_start_matches('/'))
    }
}

fn default_inquiry_for_lang(lang: &str) -> &'static str {
    match lang {
        "es" => "¿Qué estás evitando sentir ahora mismo?",
        "fr" => "Qu'est-ce que tu évites de ressentir en ce moment ?",
        "pt" => "O que você está evitando sentir agora?",
        "de" => "Was vermeidest du gerade zu fühlen?",
        "it" => "Cosa stai evitando di sentire adesso?",
        "ja" => "今、何を感じることを避けていますか？",
        "ko" => "지금 어떤 감정을 피하고 있나요?",
        "zh" => "你现在在回避什么感受？",
        _ => "What are you avoiding feeling right now?",
    }
}

fn parse_accept_language(header: Option<&str>) -> String {
    if let Some(val) = header {
        // Parse first language code from Accept-Language header
        // e.g. "es-MX,es;q=0.9,en;q=0.8" → "es"
        if let Some(first) = val.split(',').next() {
            let lang = first.split(';').next().unwrap_or("en").trim();
            // Extract primary subtag: "es-MX" → "es"
            return lang.split('-').next().unwrap_or("en").to_lowercase();
        }
    }
    "en".to_string()
}

#[derive(Serialize, Clone)]
struct LandingCollageMedia {
    url: String,
    kind: String,
    aspect_ratio: String,
}

fn collect_media_files(
    dir: &str,
    public_prefix: &str,
    exts: &[&str],
    limit: usize,
) -> Vec<(i64, String)> {
    let mut files: Vec<(i64, String)> = Vec::new();
    let root = FsPath::new(dir);
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if !exts.iter().any(|e| *e == ext) {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(v) => v.to_string(),
                None => continue,
            };
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            files.push((mtime, format!("{}/{}", public_prefix, file_name)));
        }
    }

    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().take(limit).collect()
}

fn collect_optimized_webp_files(limit: usize) -> Vec<(i64, String)> {
    let mut grouped: HashMap<String, (i64, i32, String)> = HashMap::new();
    let images_dir = FsPath::new("data/images");
    if let Ok(entries) = std::fs::read_dir(images_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(v) => v.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".webp") {
                continue;
            }

            let is_thumb = file_name.ends_with("_thumb.webp");
            let key = if is_thumb {
                file_name.trim_end_matches("_thumb.webp").to_string()
            } else {
                file_name.trim_end_matches(".webp").to_string()
            };

            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // Prefer lightweight _thumb.webp when present.
            let priority = if is_thumb { 2 } else { 1 };
            let current = grouped.get(&key).cloned();
            let next = (mtime, priority, format!("/data/images/{}", file_name));
            if let Some(existing) = current {
                if next.1 > existing.1 || (next.1 == existing.1 && next.0 > existing.0) {
                    grouped.insert(key, next);
                }
            } else {
                grouped.insert(key, next);
            }
        }
    }

    let mut files: Vec<(i64, String)> = grouped
        .into_values()
        .map(|(mtime, _priority, url)| (mtime, url))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().take(limit).collect()
}

fn load_landing_collage_media(image_limit: usize, video_limit: usize) -> Vec<LandingCollageMedia> {
    let image_files = collect_optimized_webp_files(image_limit);
    let gif_files = collect_media_files(
        "data/images/landing_gifs",
        "/data/images/landing_gifs",
        &["gif"],
        video_limit,
    );
    let video_files = collect_media_files("videos", "/videos", &["mp4"], video_limit);

    let mut media = Vec::with_capacity(image_files.len() + gif_files.len() + video_files.len());
    let mut image_idx = 0usize;
    let mut gif_idx = 0usize;
    let mut video_idx = 0usize;

    // Interleave static images with gif and video motion tiles.
    while image_idx < image_files.len()
        || gif_idx < gif_files.len()
        || video_idx < video_files.len()
    {
        if image_idx < image_files.len() {
            media.push(LandingCollageMedia {
                url: image_files[image_idx].1.clone(),
                kind: "image".to_string(),
                aspect_ratio: "1 / 1".to_string(),
            });
            image_idx += 1;
        }
        if image_idx % 5 == 0 && gif_idx < gif_files.len() {
            media.push(LandingCollageMedia {
                url: gif_files[gif_idx].1.clone(),
                kind: "image".to_string(),
                aspect_ratio: "4 / 5".to_string(),
            });
            gif_idx += 1;
        }
        if image_idx % 8 == 0 && video_idx < video_files.len() {
            media.push(LandingCollageMedia {
                url: video_files[video_idx].1.clone(),
                kind: "video".to_string(),
                aspect_ratio: "16 / 9".to_string(),
            });
            video_idx += 1;
        }
    }

    if media.is_empty() {
        media = vec![
            LandingCollageMedia {
                url: "/static/references/anky-1.png".to_string(),
                kind: "image".to_string(),
                aspect_ratio: "1 / 1".to_string(),
            },
            LandingCollageMedia {
                url: "/static/references/anky-2.png".to_string(),
                kind: "image".to_string(),
                aspect_ratio: "1 / 1".to_string(),
            },
            LandingCollageMedia {
                url: "/static/references/anky-3.png".to_string(),
                kind: "image".to_string(),
                aspect_ratio: "1 / 1".to_string(),
            },
        ];
    }
    media
}

#[derive(Deserialize)]
pub struct GalleryQuery {
    pub tab: Option<String>,
}

pub async fn home(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Html<String>), AppError> {
    // If training, redirect to sleeping page
    {
        let gpu = state.gpu_status.read().await;
        if matches!(*gpu, GpuStatus::Training { .. }) {
            let mut ctx = tera::Context::new();
            ctx.insert("gpu_status", &gpu.to_string());
            let html = state.tera.render("sleeping.html", &ctx)?;
            return Ok((jar, Html(html)));
        }
    }

    // Set anonymous cookie on first visit so /write doesn't reject them
    let jar = if jar.get("anky_user_id").is_none() {
        let id = uuid::Uuid::new_v4().to_string();
        let cookie = axum_extra::extract::cookie::Cookie::build(("anky_user_id", id))
            .max_age(time::Duration::days(365))
            .http_only(false)
            .same_site(tower_cookies::cookie::SameSite::Lax)
            .path("/")
            .build();
        jar.add(cookie)
    } else {
        jar
    };

    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let username = user.as_ref().and_then(|u| u.username.clone());
    let logged_in = user.is_some();

    let cookie_user_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    // Get or create inquiry for user
    let lang = parse_accept_language(headers.get("accept-language").and_then(|v| v.to_str().ok()));
    let (inquiry_id, inquiry_question) = if let Some(ref uid) = cookie_user_id {
        let db = state.db.lock().await;
        match crate::db::queries::get_current_inquiry(&db, uid) {
            Ok(Some((id, question))) => (id, question),
            _ => {
                let question = default_inquiry_for_lang(&lang).to_string();
                let id = crate::db::queries::create_inquiry(&db, uid, &question, &lang)
                    .unwrap_or_default();
                (id, question)
            }
        }
    } else {
        (String::new(), default_inquiry_for_lang(&lang).to_string())
    };

    let mut ctx = tera::Context::new();
    if let Some(ref uname) = username {
        ctx.insert("username", uname);
    }
    ctx.insert("logged_in", &logged_in);
    if let Some(ref uid) = cookie_user_id {
        ctx.insert("user_token", uid);
    }
    ctx.insert("inquiry_id", &inquiry_id);
    ctx.insert("inquiry_question", &inquiry_question);
    // Keep landing lightweight: image/gif-first and no MP4 tiles on initial route load.
    let all_media = load_landing_collage_media(72, 0);
    let initial_count = 14usize.min(all_media.len());
    let (initial, deferred) = all_media.split_at(initial_count);
    let deferred_vec: Vec<LandingCollageMedia> = deferred.to_vec();
    ctx.insert("landing_collage_media_initial", &initial);
    ctx.insert("landing_collage_media_deferred", &deferred_vec);
    let html = state.tera.render("home.html", &ctx)?;
    Ok((jar, Html(html)))
}

pub async fn gallery(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GalleryQuery>,
) -> Result<Html<String>, AppError> {
    let user_id = jar.get("anky_user_id").map(|c| c.value().to_string());
    let has_user = user_id.is_some();

    let tab = match query.tab.as_deref() {
        Some("mine") if has_user => "mine",
        Some("viewed") if has_user => "viewed",
        _ => "all",
    };

    let ankys = {
        let db = state.db.lock().await;
        match tab {
            "mine" => crate::db::queries::get_user_ankys(&db, user_id.as_deref().unwrap())?,
            "viewed" => {
                crate::db::queries::get_user_viewed_ankys(&db, user_id.as_deref().unwrap())?
            }
            _ => {
                // "all" tab: only show non-written ankys (writing sessions are private)
                let all = crate::db::queries::get_all_complete_ankys(&db)?;
                all.into_iter().filter(|a| a.origin != "written").collect()
            }
        }
    };

    let mut ctx = tera::Context::new();
    ctx.insert("active_tab", tab);
    ctx.insert("has_user", &has_user);
    ctx.insert(
        "ankys",
        &serde_json::to_value(
            ankys
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "title": a.title.as_deref().unwrap_or("untitled"),
                        "image_path": a.image_path.as_deref().unwrap_or(""),
                        "image_webp": a.image_webp.as_deref().unwrap_or(""),
                        "image_prompt": a.image_prompt.as_deref().unwrap_or(""),
                        "reflection": a.reflection.as_deref().unwrap_or(""),
                        "thinker_name": a.thinker_name.as_deref().unwrap_or(""),
                        "status": a.status,
                        "created_at": a.created_at,
                        "origin": a.origin,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("gallery.html", &ctx)?;
    Ok(Html(html))
}

pub async fn help(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("help.html", &ctx)?;
    Ok(Html(html))
}

pub async fn mobile(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("mobile.html", &ctx)?;
    Ok(Html(html))
}

pub async fn dca_bot_code(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert(
        "dca_bot_one_liner",
        &"curl -fsSL https://anky.app/static/dca-bot/install.sh | bash",
    );
    ctx.insert(
        "dca_bot_github_url",
        &"https://github.com/jpfraneto/anky-monorepo",
    );
    let html = state.tera.render("dca_bot_code.html", &ctx)?;
    Ok(Html(html))
}

fn read_key_value_env(path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let content = fs::read_to_string(path).unwrap_or_default();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

fn tail_lines(path: &str, max_lines: usize) -> String {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<&str> = content.lines().collect();
    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    lines.join("\n")
}

fn file_modified_epoch(path: &str) -> i64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

async fn fetch_sol_balance(rpc_url: &str, wallet: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet, {"commitment": "confirmed"}]
    });
    let res = client
        .post(rpc_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let lamports = v["result"]["value"].as_i64().unwrap_or(0) as f64;
    Ok(lamports / 1_000_000_000.0)
}

async fn fetch_token_balance(rpc_url: &str, wallet: &str, mint: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            {"mint": mint},
            {"encoding": "jsonParsed", "commitment": "confirmed"}
        ]
    });
    let res = client
        .post(rpc_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let mut total = 0.0f64;
    if let Some(accounts) = v["result"]["value"].as_array() {
        for account in accounts {
            let ui = &account["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"];
            if let Some(amount) = ui.as_f64() {
                total += amount;
            }
        }
    }
    Ok(total)
}

pub async fn dca_dashboard(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let env = read_key_value_env(".secrets/anky_dca.env");
    let wallet = env
        .get("DCA_WALLET_PUBKEY")
        .cloned()
        .unwrap_or_else(|| "not configured".to_string());
    let rpc_url = env
        .get("SOLANA_RPC_URL")
        .cloned()
        .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());
    let anky_mint = env
        .get("ANKY_TOKEN_MINT")
        .cloned()
        .unwrap_or_else(|| "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump".to_string());
    let buy_per_run = env
        .get("ANKY_BUY_SOL_PER_RUN")
        .cloned()
        .unwrap_or_else(|| "0.00002".to_string());
    let slippage = env
        .get("ANKY_SLIPPAGE_BPS")
        .cloned()
        .unwrap_or_else(|| "300".to_string());
    let reserve = env
        .get("ANKY_MIN_SOL_RESERVE")
        .cloned()
        .unwrap_or_else(|| "0.02".to_string());
    let dry_run = env
        .get("ANKY_DRY_RUN")
        .cloned()
        .unwrap_or_else(|| "true".to_string());

    let logs = tail_lines("logs/anky_dca.log", 180);
    let og_version = file_modified_epoch("logs/anky_dca.log");
    let sol_balance = if wallet == "not configured" {
        Err("DCA_WALLET_PUBKEY missing in .secrets/anky_dca.env".to_string())
    } else {
        fetch_sol_balance(&rpc_url, &wallet).await
    };
    let anky_balance = if wallet == "not configured" {
        Err("DCA_WALLET_PUBKEY missing in .secrets/anky_dca.env".to_string())
    } else {
        fetch_token_balance(&rpc_url, &wallet, &anky_mint).await
    };

    let mut ctx = tera::Context::new();
    ctx.insert("dca_wallet", &wallet);
    ctx.insert("dca_rpc_url", &rpc_url);
    ctx.insert("dca_anky_mint", &anky_mint);
    ctx.insert("dca_buy_per_run", &buy_per_run);
    ctx.insert("dca_slippage_bps", &slippage);
    ctx.insert("dca_min_sol_reserve", &reserve);
    ctx.insert("dca_dry_run", &dry_run);
    ctx.insert("dca_logs", &logs);
    ctx.insert("dca_og_version", &og_version);
    ctx.insert(
        "dca_bot_one_liner",
        &"curl -fsSL https://anky.app/static/dca-bot/install.sh | bash",
    );
    ctx.insert("dca_bot_code_url", &"/dca-bot-code");
    match sol_balance {
        Ok(v) => {
            ctx.insert("dca_sol_balance", &format!("{:.6}", v));
            ctx.insert("dca_sol_error", &"");
        }
        Err(e) => {
            ctx.insert("dca_sol_balance", &"n/a");
            ctx.insert("dca_sol_error", &e);
        }
    }
    match anky_balance {
        Ok(v) => {
            ctx.insert("dca_anky_balance", &format!("{:.6}", v));
            ctx.insert("dca_anky_error", &"");
        }
        Err(e) => {
            ctx.insert("dca_anky_balance", &"n/a");
            ctx.insert("dca_anky_error", &e);
        }
    }

    let html = state.tera.render("dca.html", &ctx)?;
    Ok(Html(html))
}

pub async fn ankycoin_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("ankycoin.html", &ctx)?;
    Ok(Html(html))
}

pub async fn login_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("login.html", &ctx)?;
    Ok(Html(html))
}

pub async fn generate_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("generate.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct FeedPageQuery {
    pub page: Option<i32>,
}

pub async fn feed_page(
    State(state): State<AppState>,
    Query(query): Query<FeedPageQuery>,
) -> Result<Html<String>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = 30;

    let (stats, ankys) = {
        let db = state.db.lock().await;
        let s = crate::db::queries::get_feed_stats_24h(&db)?;
        let a = crate::db::queries::get_feed_ankys(&db, per_page, (page - 1) * per_page)?;
        (s, a)
    };

    let mut ctx = tera::Context::new();
    ctx.insert("page", &page);
    ctx.insert("stats_sessions", &stats.total_sessions_24h);
    ctx.insert("stats_ankys", &stats.total_ankys_24h);
    ctx.insert("stats_writers", &stats.unique_writers_24h);
    ctx.insert("stats_minutes", &(stats.total_minutes_24h.round() as i32));
    ctx.insert("stats_words", &stats.total_words_24h);
    ctx.insert(
        "ankys",
        &serde_json::to_value(
            ankys
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "title": a.title.as_deref().unwrap_or("untitled"),
                        "image_path": a.image_path.as_deref().unwrap_or(""),
                        "image_webp": a.image_webp.as_deref().unwrap_or(""),
                        "thinker_name": a.thinker_name.as_deref().unwrap_or("someone"),
                        "origin": a.origin,
                        "created_at": a.created_at,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );
    ctx.insert("has_more", &(ankys.len() == per_page as usize));

    let html = state.tera.render("feed.html", &ctx)?;
    Ok(Html(html))
}

pub async fn sleeping(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let gpu = state.gpu_status.read().await;
    let mut ctx = tera::Context::new();
    ctx.insert("gpu_status", &gpu.to_string());
    let html = state.tera.render("sleeping.html", &ctx)?;
    Ok(Html(html))
}

pub async fn feedback(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let entries = {
        let db = state.db.lock().await;
        crate::db::queries::get_all_feedback(&db)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert(
        "entries",
        &serde_json::to_value(
            entries
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "id": f.id,
                        "source": f.source,
                        "author": f.author.as_deref().unwrap_or("anonymous"),
                        "content": f.content,
                        "status": f.status,
                        "created_at": f.created_at,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("feedback.html", &ctx)?;
    Ok(Html(html))
}

pub async fn video_dashboard(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();

    let auth_user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let logged_in = auth_user.is_some();
    ctx.insert("logged_in", &logged_in);

    if let Some(ref user) = auth_user {
        ctx.insert("user_id", &user.user_id);
        ctx.insert("username", &user.username);

        // Get user's ankys with writing text
        let ankys = {
            let db = state.db.lock().await;
            crate::db::queries::get_user_anky_writings(&db, &user.user_id).unwrap_or_default()
        };
        ctx.insert(
            "ankys",
            &serde_json::to_value(
                ankys
                    .iter()
                    .map(|(id, title, excerpt, image)| {
                        serde_json::json!({
                            "id": id,
                            "title": title,
                            "excerpt": excerpt,
                            "image": image,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        );

        // Get user's video projects
        let projects = {
            let db = state.db.lock().await;
            crate::db::queries::get_user_video_projects(&db, &user.user_id).unwrap_or_default()
        };
        ctx.insert(
            "projects",
            &serde_json::to_value(
                projects
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "status": p.status,
                            "video_url": p.video_path.as_ref().map(|path| video_public_url(path)),
                            "total_scenes": p.total_scenes,
                            "completed_scenes": p.completed_scenes,
                            "created_at": p.created_at,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        );
    }

    let html = state.tera.render("video.html", &ctx)?;
    Ok(Html(html))
}

pub async fn create_videos_page(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    let auth_user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let logged_in = auth_user.is_some();
    ctx.insert("logged_in", &logged_in);
    if let Some(user) = auth_user {
        ctx.insert("user_id", &user.user_id);
        ctx.insert("username", &user.username);
    }

    let cards = crate::create_videos::load_cards()?;
    ctx.insert("cards", &cards);

    let html = state.tera.render("create_videos.html", &ctx)?;
    Ok(Html(html))
}

pub async fn video_pipeline_page(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    let auth_user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let logged_in = auth_user.is_some();
    ctx.insert("logged_in", &logged_in);
    if let Some(user) = auth_user {
        ctx.insert("user_id", &user.user_id);
        ctx.insert("username", &user.username);
    }
    let html = state.tera.render("video_pipeline.html", &ctx)?;
    Ok(Html(html))
}

pub async fn stream_overlay(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ankys = {
        let db = state.db.lock().await;
        crate::db::queries::get_todays_ankys(&db)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert(
        "contract_address",
        "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump",
    );
    ctx.insert(
        "ankys",
        &serde_json::to_value(
            ankys
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "title": a.title.as_deref().unwrap_or("untitled"),
                        "image_url": a.image_path.as_deref().unwrap_or(""),
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("stream_overlay.html", &ctx)?;
    Ok(Html(html))
}

pub async fn changelog(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("changelog.html", &ctx)?;
    Ok(Html(html))
}

pub async fn pitch_deck(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("pitch-deck.html", &ctx)?;
    Ok(Html(html))
}

pub async fn llm(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let runs = {
        let db = state.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT run_date, val_bpb, training_seconds, peak_vram_mb, mfu_percent,
                    total_tokens_m, num_steps, num_params_m, depth,
                    corpus_sessions, corpus_words, corpus_tokens, epochs
             FROM llm_training_runs ORDER BY run_date ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "run_date": row.get::<_, String>(0)?,
                "val_bpb": row.get::<_, f64>(1)?,
                "training_seconds": row.get::<_, f64>(2)?,
                "peak_vram_mb": row.get::<_, f64>(3)?,
                "mfu_percent": row.get::<_, f64>(4)?,
                "total_tokens_m": row.get::<_, f64>(5)?,
                "num_steps": row.get::<_, i64>(6)?,
                "num_params_m": row.get::<_, f64>(7)?,
                "depth": row.get::<_, i64>(8)?,
                "corpus_sessions": row.get::<_, i64>(9)?,
                "corpus_words": row.get::<_, i64>(10)?,
                "corpus_tokens": row.get::<_, i64>(11)?,
                "epochs": row.get::<_, i64>(12)?,
            }))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    let mut ctx = tera::Context::new();
    ctx.insert("runs", &runs);
    ctx.insert(
        "runs_json",
        &serde_json::to_string(&runs).unwrap_or_default(),
    );
    let html = state.tera.render("llm.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct MediaDashboardQuery {
    pub order: Option<String>,
    pub kind: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Serialize, Clone)]
struct MediaEntry {
    kind: &'static str,
    url: String,
    relative_path: String,
    filename: String,
    modified_epoch: i64,
    modified_at: String,
    size_bytes: u64,
}

fn is_supported_media_file(path: &FsPath) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())?;
    if ["png", "jpg", "jpeg", "gif", "webp"].contains(&ext.as_str()) {
        Some("image")
    } else if ["mp4", "mov", "avi", "mkv", "webm"].contains(&ext.as_str()) {
        Some("video")
    } else {
        None
    }
}

fn collect_media_recursive(
    dir: &FsPath,
    base_dir: &FsPath,
    url_prefix: &str,
    out: &mut Vec<MediaEntry>,
) -> Result<(), std::io::Error> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            collect_media_recursive(&path, base_dir, url_prefix, out)?;
            continue;
        }

        let Some(kind) = is_supported_media_file(&path) else {
            continue;
        };

        let rel = path
            .strip_prefix(base_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let modified_epoch = modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let modified_at = chrono::DateTime::<chrono::Local>::from(modified)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let url = format!("{}/{}", url_prefix.trim_end_matches('/'), rel);

        out.push(MediaEntry {
            kind,
            url,
            relative_path: rel,
            filename,
            modified_epoch,
            modified_at,
            size_bytes: metadata.len(),
        });
    }

    Ok(())
}

fn collect_generated_media() -> Result<Vec<MediaEntry>, std::io::Error> {
    let mut out = Vec::new();
    let video_root = if FsPath::new("videos").exists() {
        (PathBuf::from("videos"), "/videos")
    } else {
        (PathBuf::from("data/videos"), "/data/videos")
    };
    let roots: [(PathBuf, &str); 2] = [(PathBuf::from("data/images"), "/data/images"), video_root];

    for (root, url_prefix) in roots {
        collect_media_recursive(&root, &root, url_prefix, &mut out)?;
    }

    Ok(out)
}

pub async fn media_dashboard(
    State(state): State<AppState>,
    Query(query): Query<MediaDashboardQuery>,
) -> Result<Html<String>, AppError> {
    let order = match query.order.as_deref() {
        Some("asc") => "asc",
        _ => "desc",
    };
    let kind = match query.kind.as_deref() {
        Some("image") => "image",
        Some("video") => "video",
        _ => "all",
    };
    let per_page = query.per_page.unwrap_or(150).clamp(20, 500);
    let mut page = query.page.unwrap_or(1).max(1);

    let mut items = collect_generated_media()?;
    if kind != "all" {
        items.retain(|m| m.kind == kind);
    }

    if order == "asc" {
        items.sort_by_key(|m| m.modified_epoch);
    } else {
        items.sort_by(|a, b| b.modified_epoch.cmp(&a.modified_epoch));
    }

    let total_count = items.len();
    let total_pages = total_count.div_ceil(per_page).max(1);
    if page > total_pages {
        page = total_pages;
    }

    let start = (page - 1) * per_page;
    let end = (start + per_page).min(total_count);
    let page_items = if start < end {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };

    let mut ctx = tera::Context::new();
    ctx.insert("items", &page_items);
    ctx.insert("order", &order);
    ctx.insert("kind", &kind);
    ctx.insert("page", &page);
    ctx.insert("per_page", &per_page);
    ctx.insert("total_count", &total_count);
    ctx.insert("total_pages", &total_pages);
    ctx.insert("has_prev", &(page > 1));
    ctx.insert("has_next", &(page < total_pages));
    ctx.insert("prev_page", &page.saturating_sub(1));
    ctx.insert("next_page", &(page + 1));

    let html = state.tera.render("media_dashboard.html", &ctx)?;
    Ok(Html(html))
}

pub async fn leaderboard(
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Html<String>, AppError> {
    let sort = query.sort.as_deref().unwrap_or("flow");
    let entries = {
        let db = state.db.lock().await;
        crate::db::queries::get_leaderboard(&db, sort, 50)?
    };

    let mut ctx = tera::Context::new();
    ctx.insert("sort", sort);
    ctx.insert(
        "entries",
        &serde_json::to_value(
            entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "rank": e.rank,
                        "username": e.username,
                        "best_flow_score": e.best_flow_score,
                        "avg_flow_score": (e.avg_flow_score * 10.0).round() / 10.0,
                        "total_ankys": e.total_ankys,
                        "total_words": e.total_words,
                        "current_streak": e.current_streak,
                        "longest_streak": e.longest_streak,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );

    let html = state.tera.render("leaderboard.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    pub sort: Option<String>,
}

pub async fn pitch(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("pitch.html", &ctx)?;
    Ok(Html(html))
}

/// Serve the auto-generated pitch deck PDF.
pub async fn pitch_deck_pdf() -> Result<impl axum::response::IntoResponse, AppError> {
    let path = std::path::Path::new("static/pitch-deck.pdf");
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| AppError::Internal(format!("pitch-deck.pdf not found: {e}")))?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/pdf"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "inline; filename=\"anky-pitch-deck.pdf\"",
            ),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        bytes,
    ))
}

/// Handler for pitch.anky.app subdomain — redirects everything to the PDF.
pub async fn pitch_subdomain_redirect() -> axum::response::Redirect {
    axum::response::Redirect::temporary("/pitch-deck.pdf")
}

pub async fn anky_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let viewer_id = jar.get("anky_user_id").map(|c| c.value().to_string());

    let anky = {
        let db = state.db.lock().await;
        crate::db::queries::get_anky_by_id(&db, &id)?
    };

    let anky = anky.ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    // Always collect when a logged-in user views any anky (tracks views)
    if let Some(ref vid) = viewer_id {
        let db = state.db.lock().await;
        let _ = crate::db::queries::collect_anky(&db, vid, &id);
    }

    // Determine if the viewer can see the writing text
    // Having the link means the writer shared it — show everything
    let show_writing = anky.origin != "generated";

    let mut ctx = tera::Context::new();
    ctx.insert("id", &anky.id);
    ctx.insert("title", &anky.title.as_deref().unwrap_or("untitled"));
    ctx.insert("image_path", &anky.image_path.as_deref().unwrap_or(""));
    ctx.insert("image_webp", &anky.image_webp.as_deref().unwrap_or(""));
    ctx.insert("reflection", &anky.reflection.as_deref().unwrap_or(""));
    ctx.insert("image_prompt", &anky.image_prompt.as_deref().unwrap_or(""));
    ctx.insert("thinker_name", &anky.thinker_name.as_deref().unwrap_or(""));
    ctx.insert(
        "thinker_moment",
        &anky.thinker_moment.as_deref().unwrap_or(""),
    );
    ctx.insert("status", &anky.status);
    ctx.insert("created_at", &anky.created_at);
    ctx.insert("origin", &anky.origin);
    ctx.insert("prompt_id", &anky.prompt_id.as_deref().unwrap_or(""));
    ctx.insert("prompt_text", &anky.prompt_text.as_deref().unwrap_or(""));

    if show_writing {
        ctx.insert("writing", &anky.writing_text.as_deref().unwrap_or(""));
        ctx.insert(
            "formatted_writing",
            &anky.formatted_writing.as_deref().unwrap_or(""),
        );
    } else {
        ctx.insert("writing", &"");
        ctx.insert("formatted_writing", &"");
    }

    let html = state.tera.render("anky.html", &ctx)?;
    Ok(Html(html))
}

pub async fn videos_gallery(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let logged_in = crate::routes::auth::get_auth_user(&state, &jar)
        .await
        .is_some();
    let items = {
        let db = state.db.lock().await;
        crate::db::queries::get_all_complete_video_projects(&db)?
    };
    let mut ctx = tera::Context::new();
    ctx.insert("logged_in", &logged_in);
    ctx.insert(
        "videos",
        &serde_json::to_value(
            items
                .iter()
                .map(|v| {
                    let image = v
                        .image_webp
                        .as_deref()
                        .or(v.image_path.as_deref())
                        .unwrap_or("");
                    let thumb = v
                        .image_thumb
                        .as_deref()
                        .or(v.image_webp.as_deref())
                        .or(v.image_path.as_deref())
                        .unwrap_or("");
                    serde_json::json!({
                        "id": v.project_id,
                        "video_url": video_public_url(&v.video_path),
                        "created_at": v.created_at,
                        "title": v.anky_title,
                        "image": if image.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(format!("/data/images/{}", image)) },
                        "thumb": if thumb.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(format!("/data/images/{}", thumb)) },
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );
    let html = state.tera.render("videos.html", &ctx)?;
    Ok(Html(html))
}

pub async fn dataset_og_image() -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let cache_path = "data/og-dataset-round-two.jpg";

    // Regenerate if missing
    if !std::path::Path::new(cache_path).exists() {
        let _ = std::process::Command::new("python3")
            .args([
                "-c",
                r#"
from PIL import Image
import os, json, random

thumbs = []
gi_dir = 'data/images/thumbs'
if os.path.isdir(gi_dir):
    thumbs += sorted(os.path.join(gi_dir, f) for f in os.listdir(gi_dir) if f.endswith('.webp'))
ti_dir = 'data/training-images/thumbs'
if os.path.isdir(ti_dir):
    thumbs += sorted(os.path.join(ti_dir, f) for f in os.listdir(ti_dir) if f.endswith('.webp'))
gens_dir = 'data/generations'
if os.path.isdir(gens_dir):
    for batch in sorted(os.listdir(gens_dir)):
        review_path = f'{gens_dir}/{batch}/review.json'
        if not os.path.exists(review_path): continue
        with open(review_path) as f: data = json.load(f)
        approved = sorted(k for k,v in data.items() if v.get('decision') == 'approved')
        for img_id in approved:
            p = f'{gens_dir}/{batch}/thumbs/{img_id}.webp'
            if os.path.exists(p): thumbs.append(p)
cell, cols, rows = 200, 6, 4
W, H = cols * cell, 630
random.seed(42)
sample = thumbs[:8] + random.sample(thumbs[8:], min(len(thumbs)-8, cols*rows*2))
sample = sample[:cols*rows]
canvas = Image.new('RGB', (W, H), (10, 10, 10))
for i, path in enumerate(sample):
    col, row = i % cols, i // cols
    try:
        img = Image.open(path).convert('RGB').resize((cell, cell))
        canvas.paste(img, (col * cell, row * cell))
    except: pass
canvas.save('data/og-dataset-round-two.jpg', 'JPEG', quality=88)
import shutil; shutil.copy('data/og-dataset-round-two.jpg', 'static/og-dataset-round-two.jpg')
"#,
            ])
            .output();
    }

    match fs::read(cache_path) {
        Ok(bytes) => ([(axum::http::header::CONTENT_TYPE, "image/jpeg")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn make_thumb_200(src: &PathBuf, thumb: &PathBuf) {
    let _ = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            src.to_str().unwrap_or(""),
            "-vf",
            "scale=200:200:force_original_aspect_ratio=increase,crop=200:200",
            "-quality",
            "85",
            thumb.to_str().unwrap_or(""),
            "-loglevel",
            "quiet",
        ])
        .output();
}

pub async fn dataset_round_two(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut images: Vec<serde_json::Value> = Vec::new();

    // Source 1: gallery images (all ankys generated by the system)
    let gi_dir = FsPath::new("data/images");
    let gi_thumb_dir = PathBuf::from("data/images/thumbs");
    let _ = fs::create_dir_all(&gi_thumb_dir);
    if let Ok(entries) = fs::read_dir(gi_dir) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                if n.ends_with(".png") {
                    Some(n[..n.len() - 4].to_string())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        for id in names {
            let src = PathBuf::from(format!("data/images/{}.png", id));
            let thumb = gi_thumb_dir.join(format!("{}.webp", id));
            if src.exists() && !thumb.exists() {
                make_thumb_200(&src, &thumb);
            }
            images.push(serde_json::json!({
                "source": "gallery-images",
                "batch_id": "",
                "image_id": id,
                "thumb_url": format!("/data/images/thumbs/{}.webp", id),
                "full_url": format!("/data/images/{}.png", id),
            }));
        }
    }

    // Source 2: original training-images (first Flux fine-tune)
    let ti_dir = FsPath::new("data/training-images");
    let ti_thumb_dir = PathBuf::from("data/training-images/thumbs");
    let _ = fs::create_dir_all(&ti_thumb_dir);
    if let Ok(entries) = fs::read_dir(ti_dir) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                if n.ends_with(".png") {
                    Some(n[..n.len() - 4].to_string())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        for id in names {
            let src = PathBuf::from(format!("data/training-images/{}.png", id));
            let thumb = ti_thumb_dir.join(format!("{}.webp", id));
            if src.exists() && !thumb.exists() {
                make_thumb_200(&src, &thumb);
            }
            images.push(serde_json::json!({
                "source": "training-images",
                "batch_id": "",
                "image_id": id,
                "thumb_url": format!("/data/training-images/thumbs/{}.webp", id),
                "full_url": format!("/data/training-images/{}.png", id),
            }));
        }
    }

    // Source 3: tinder-approved generation images
    let gens_dir = FsPath::new("data/generations");
    if let Ok(batches) = fs::read_dir(gens_dir) {
        let mut batch_dirs: Vec<PathBuf> = batches
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        batch_dirs.sort();

        for batch_dir in batch_dirs {
            let review_path = batch_dir.join("review.json");
            if !review_path.exists() {
                continue;
            }
            let batch_id = batch_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if let Ok(content) = fs::read_to_string(&review_path) {
                if let Ok(map) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = map.as_object() {
                        let mut approved: Vec<&String> = obj
                            .iter()
                            .filter(|(_, v)| {
                                v.get("decision").and_then(|d| d.as_str()) == Some("approved")
                            })
                            .map(|(k, _)| k)
                            .collect();
                        approved.sort();
                        for image_id in approved {
                            let src = PathBuf::from(format!(
                                "data/generations/{}/images/{}.png",
                                batch_id, image_id
                            ));
                            let thumb_dir =
                                PathBuf::from(format!("data/generations/{}/thumbs", batch_id));
                            let thumb = thumb_dir.join(format!("{}.webp", image_id));
                            if src.exists() && !thumb.exists() {
                                let _ = fs::create_dir_all(&thumb_dir);
                                make_thumb_200(&src, &thumb);
                            }
                            images.push(serde_json::json!({
                                "source": "generation",
                                "batch_id": batch_id,
                                "image_id": image_id,
                                "thumb_url": format!("/gen-images/{}/thumbs/{}.webp", batch_id, image_id),
                                "full_url": format!("/gen-images/{}/images/{}.png", batch_id, image_id),
                            }));
                        }
                    }
                }
            }
        }
    }

    let mut ctx = tera::Context::new();
    ctx.insert("total", &images.len());
    ctx.insert("images", &images);
    let html = state.tera.render("dataset_round_two.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct EliminateForm {
    password: String,
    // JSON array: [{"batch_id":"...","image_id":"..."},...]
    selections: String,
}

pub async fn dataset_eliminate(
    State(state): State<AppState>,
    axum::extract::Form(form): axum::extract::Form<EliminateForm>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    if form.password != state.config.dataset_password {
        return (StatusCode::FORBIDDEN, "wrong password").into_response();
    }

    #[derive(serde::Deserialize)]
    struct Sel {
        source: String,
        batch_id: String,
        image_id: String,
    }

    let selections: Vec<Sel> = match serde_json::from_str(&form.selections) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid selections").into_response(),
    };

    let mut moved = 0usize;
    for sel in &selections {
        if sel.batch_id.contains("..") || sel.image_id.contains("..") {
            continue;
        }

        if sel.source == "gallery-images" {
            let src_dir = PathBuf::from("data/images");
            let dst_dir = PathBuf::from("data/images/rejected");
            let _ = fs::create_dir_all(&dst_dir);
            for ext in &["png", "webp"] {
                let src = src_dir.join(format!("{}.{}", sel.image_id, ext));
                let dst = dst_dir.join(format!("{}.{}", sel.image_id, ext));
                if src.exists() {
                    let _ = fs::rename(&src, &dst);
                }
            }
            let thumb_src = src_dir.join(format!("thumbs/{}.webp", sel.image_id));
            let thumb_dst = dst_dir.join(format!("{}.webp", sel.image_id));
            if thumb_src.exists() {
                let _ = fs::rename(&thumb_src, &thumb_dst);
            }
            moved += 1;
            continue;
        }

        if sel.source == "training-images" {
            // First fine-tune images: move to data/training-images/rejected/
            let src_dir = PathBuf::from("data/training-images");
            let dst_dir = PathBuf::from("data/training-images/rejected");
            let _ = fs::create_dir_all(&dst_dir);
            for ext in &["png", "txt"] {
                let src = src_dir.join(format!("{}.{}", sel.image_id, ext));
                let dst = dst_dir.join(format!("{}.{}", sel.image_id, ext));
                if src.exists() {
                    let _ = fs::rename(&src, &dst);
                }
            }
            let thumb_src = src_dir.join(format!("thumbs/{}.webp", sel.image_id));
            let thumb_dst = dst_dir.join(format!("{}.webp", sel.image_id));
            if thumb_src.exists() {
                let _ = fs::rename(&thumb_src, &thumb_dst);
            }
            moved += 1;
            continue;
        }

        let src_dir = PathBuf::from(format!("data/generations/{}/images", sel.batch_id));
        let dst_dir = PathBuf::from(format!("data/generations/{}/rejected", sel.batch_id));
        let _ = fs::create_dir_all(&dst_dir);

        for ext in &["png", "txt"] {
            let src = src_dir.join(format!("{}.{}", sel.image_id, ext));
            let dst = dst_dir.join(format!("{}.{}", sel.image_id, ext));
            if src.exists() {
                let _ = fs::rename(&src, &dst);
            }
        }
        let thumb_src = PathBuf::from(format!(
            "data/generations/{}/thumbs/{}.webp",
            sel.batch_id, sel.image_id
        ));
        let thumb_dst = dst_dir.join(format!("{}.webp", sel.image_id));
        if thumb_src.exists() {
            let _ = fs::rename(&thumb_src, &thumb_dst);
        }

        let review_path = PathBuf::from(format!("data/generations/{}/review.json", sel.batch_id));
        if let Ok(content) = fs::read_to_string(&review_path) {
            if let Ok(mut map) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(entry) = map.get_mut(&sel.image_id) {
                    entry["decision"] = serde_json::Value::String("rejected".to_string());
                    let _ = fs::write(
                        &review_path,
                        serde_json::to_string(&map).unwrap_or_default(),
                    );
                }
            }
        }

        moved += 1;
    }

    (StatusCode::OK, format!("eliminated {} images", moved)).into_response()
}
