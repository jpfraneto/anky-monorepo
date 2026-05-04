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

/// Miniapp HTML — single self-contained file, baked into the binary.
static MINIAPP_HTML: &str = include_str!("../../templates/miniapp.html");

/// Altar HTML served at /altar for web browsers (Stripe payments).
static ALTAR_HTML: &str = include_str!("../../templates/altar.html");

/// Login bridge HTML served at /login for phone-seal handoff.
static LOGIN_HTML: &str = include_str!("../../templates/login.html");

pub async fn load_newlanding_html() -> String {
    tokio::fs::read_to_string("static/newlanding/index.html")
        .await
        .unwrap_or_else(|_| include_str!("../../static/newlanding/index.html").to_string())
}

pub async fn newlanding_page() -> Html<String> {
    Html(load_newlanding_html().await)
}

#[derive(Deserialize)]
pub struct SealBridgeQuery {
    pub challenge: Option<String>,
}

pub async fn altar_page() -> Html<String> {
    Html(ALTAR_HTML.to_string())
}

pub async fn seal_bridge_page(
    State(state): State<AppState>,
    Query(query): Query<SealBridgeQuery>,
) -> Result<Html<String>, AppError> {
    let challenge = query.challenge.unwrap_or_default();
    let app_scheme_url = if challenge.is_empty() {
        "anky://".to_string()
    } else {
        format!("anky://seal?challenge={}", urlencoding::encode(&challenge))
    };

    let mut ctx = tera::Context::new();
    ctx.insert("app_scheme_url", &app_scheme_url);
    ctx.insert("install_url", &state.config.ios_app_url);
    let html = state.tera.render("seal.html", &ctx)?;
    Ok(Html(html))
}

#[derive(serde::Deserialize, Default)]
pub struct LangQuery {
    pub lang: Option<String>,
}

pub async fn home(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(lang_q): Query<LangQuery>,
) -> Result<(CookieJar, Html<String>), AppError> {
    let jar = crate::routes::auth::ensure_visitor_cookie(jar);
    // Persist `?lang=` as a cookie so the choice sticks
    let jar = set_lang_cookie_if_query(jar, lang_q.lang.as_deref());

    // Farcaster miniapp: serve React build for Warpcast/Farcaster clients or iframe embeds
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let fetch_dest = headers
        .get("sec-fetch-dest")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let is_farcaster =
        ua.contains("Farcaster") || ua.contains("Warpcast") || fetch_dest == "iframe";

    if is_farcaster {
        return Ok((jar, Html(MINIAPP_HTML.to_string())));
    }

    // Normal browser: serve the Tera landing page
    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let logged_in = user.is_some();
    let profile_image_url = user
        .as_ref()
        .and_then(|u| u.profile_image_url.clone())
        .unwrap_or_default();

    let username = user
        .as_ref()
        .and_then(|u| u.username.clone())
        .unwrap_or_default();

    let wallet_address = user
        .as_ref()
        .and_then(|u| u.wallet_address.clone())
        .unwrap_or_default();

    let mut ctx = tera::Context::new();
    crate::i18n::inject_into_context(
        &state.i18n,
        &mut ctx,
        &headers,
        &jar,
        lang_q.lang.as_deref(),
    );
    ctx.insert("logged_in", &logged_in);
    ctx.insert("profile_image_url", &profile_image_url);
    ctx.insert("username", &username);
    ctx.insert("wallet_address", &wallet_address);
    ctx.insert("install_url", &state.config.ios_app_url);
    let html = state.tera.render("landing.html", &ctx)?;
    Ok((jar, Html(html)))
}

/// If `?lang=xx` is present, set the `anky_lang` cookie so future pages pick it up.
fn set_lang_cookie_if_query(jar: CookieJar, lang: Option<&str>) -> CookieJar {
    if let Some(l) = lang {
        let primary = l.split('-').next().unwrap_or(l).trim().to_lowercase();
        if !primary.is_empty()
            && primary.len() <= 8
            && primary.chars().all(|c| c.is_ascii_alphabetic())
        {
            let cookie = axum_extra::extract::cookie::Cookie::build(("anky_lang", primary))
                .path("/")
                .max_age(time::Duration::days(365))
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .build();
            return jar.add(cookie);
        }
    }
    jar
}

#[derive(serde::Deserialize, Default)]
pub struct WriteQuery {
    pub prompt: Option<String>,
    pub p: Option<String>,
}

pub async fn write_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(_query): Query<WriteQuery>,
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

    let jar = crate::routes::auth::ensure_visitor_cookie(jar);

    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let username = user.as_ref().and_then(|u| u.username.clone());
    let logged_in = user.is_some();
    let wallet_address = user
        .as_ref()
        .and_then(|u| u.wallet_address.clone())
        .unwrap_or_default();

    let cookie_user_id = crate::routes::auth::visitor_id_from_jar(&jar);

    let mut ctx = tera::Context::new();
    crate::i18n::inject_into_context(&state.i18n, &mut ctx, &headers, &jar, None);
    if let Some(ref uname) = username {
        ctx.insert("username", uname);
    }
    ctx.insert("logged_in", &logged_in);
    ctx.insert("wallet_address", &wallet_address);
    if let Some(ref uid) = cookie_user_id {
        ctx.insert("user_token", uid);
    }

    // Recent ankys for "your threads" on the write page
    let recent_ankys = if let Some(ref uid) = cookie_user_id {
        let db = crate::db::conn(&state.db)?;
        db.prepare(
            "SELECT a.id, a.title FROM ankys a
             WHERE a.user_id = ?1 AND a.status IN ('complete', 'archived') AND a.title IS NOT NULL
             ORDER BY a.created_at DESC LIMIT 5",
        )
        .and_then(|mut stmt| {
            stmt.query_map(crate::params![uid], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                }))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        })
        .unwrap_or_default()
    } else {
        vec![]
    };
    ctx.insert("recent_ankys", &recent_ankys);
    ctx.insert("active_tab", "home");
    let html = state.tera.render("home.html", &ctx)?;
    Ok((jar, Html(html)))
}

pub async fn gallery(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GalleryQuery>,
) -> Result<Html<String>, AppError> {
    let user_id = crate::routes::auth::visitor_id_from_jar(&jar);
    let has_user = user_id.is_some();

    let tab = match query.tab.as_deref() {
        Some("mine") if has_user => "mine",
        Some("viewed") if has_user => "viewed",
        _ => "all",
    };

    let ankys = {
        let db = crate::db::conn(&state.db)?;
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

pub async fn login_page() -> Html<String> {
    Html(LOGIN_HTML.to_string())
}

pub async fn test_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("test.html", &ctx)?;
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
        let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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
            let db = crate::db::conn(&state.db)?;
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
            let db = crate::db::conn(&state.db)?;
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
        let db = crate::db::conn(&state.db)?;
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

pub async fn stories_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let mut ctx = tera::Context::new();
    ctx.insert("logged_in", &user.is_some());
    ctx.insert("active_tab", "stories");
    if let Some(ref u) = user {
        if let Some(ref uname) = u.username {
            ctx.insert("username", uname);
        }
    }
    let html = state.tera.render("stories.html", &ctx)?;
    Ok(Html(html))
}

pub async fn ankys_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let mut ctx = tera::Context::new();
    ctx.insert("logged_in", &user.is_some());
    ctx.insert("active_tab", "ankys");
    ctx.insert("r2_public_url", &state.config.r2_public_url);
    if let Some(ref u) = user {
        if let Some(ref uname) = u.username {
            ctx.insert("username", uname);
        }
    }
    let html = state.tera.render("ankys.html", &ctx)?;
    Ok(Html(html))
}

pub async fn you_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(lang_q): Query<LangQuery>,
) -> Result<(CookieJar, Html<String>), AppError> {
    let jar = crate::routes::auth::ensure_visitor_cookie(jar);
    let jar = set_lang_cookie_if_query(jar, lang_q.lang.as_deref());
    let user = crate::routes::auth::get_auth_user(&state, &jar).await;
    let mut ctx = tera::Context::new();
    crate::i18n::inject_into_context(
        &state.i18n,
        &mut ctx,
        &headers,
        &jar,
        lang_q.lang.as_deref(),
    );
    ctx.insert("logged_in", &true);
    ctx.insert("active_tab", "you");
    if let Some(ref u) = user {
        ctx.insert("user_id", &u.user_id);
        ctx.insert("username", &u.username.as_deref().unwrap_or("anon"));
        ctx.insert("display_name", &u.display_name.as_deref().unwrap_or("anon"));
        ctx.insert(
            "profile_image_url",
            &u.profile_image_url.as_deref().unwrap_or(""),
        );
        ctx.insert("email", &u.email.as_deref().unwrap_or(""));
    } else if let Some(visitor_id) = crate::routes::auth::visitor_id_from_jar(&jar) {
        ctx.insert("user_id", &visitor_id);
        ctx.insert("username", &"you");
        ctx.insert("display_name", &"YOU");
        ctx.insert("profile_image_url", &"");
        ctx.insert("email", &"");
    }
    let html = state.tera.render("you.html", &ctx)?;
    Ok((jar, Html(html)))
}

pub async fn changelog(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("changelog.html", &ctx)?;
    Ok(Html(html))
}

pub async fn easter_gallery(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut images: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir("static/easter") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".png") {
                images.push(name);
            }
        }
    }
    images.sort();
    let mut ctx = tera::Context::new();
    ctx.insert("images", &images);
    let html = state.tera.render("easter.html", &ctx)?;
    Ok(Html(html))
}

pub async fn pitch_deck(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("pitch-deck.html", &ctx)?;
    Ok(Html(html))
}

pub async fn llm(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let runs = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db.prepare(
            "SELECT run_date, val_bpb, training_seconds, peak_vram_mb, mfu_percent,
                    total_tokens_m, num_steps, num_params_m, depth,
                    corpus_sessions, corpus_words, corpus_tokens, epochs
             FROM llm_training_runs ORDER BY run_date ASC",
        )?;
        let rows = stmt.query_map(crate::params![], |row| {
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
        let db = crate::db::conn(&state.db)?;
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

pub async fn station(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("station.html", &ctx)?;
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
    headers: HeaderMap,
    jar: CookieJar,
    Path(id): Path<String>,
    Query(lang_q): Query<LangQuery>,
) -> Result<(CookieJar, Html<String>), AppError> {
    let jar = set_lang_cookie_if_query(jar, lang_q.lang.as_deref());
    let viewer_id = crate::routes::auth::visitor_id_from_jar(&jar);

    let anky = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_anky_by_id(&db, &id)?
    };

    let anky = anky.ok_or_else(|| AppError::NotFound("anky not found".into()))?;

    // Always collect when a logged-in user views any anky (tracks views)
    if let Some(ref vid) = viewer_id {
        let db = crate::db::conn(&state.db)?;
        let _ = crate::db::queries::collect_anky(&db, vid, &id);
    }

    // Determine if the viewer can see the writing text
    // Having the link means the writer shared it — show everything
    let show_writing = anky.origin != "generated";

    let mut ctx = tera::Context::new();
    crate::i18n::inject_into_context(
        &state.i18n,
        &mut ctx,
        &headers,
        &jar,
        lang_q.lang.as_deref(),
    );
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

    // Fetch associated cuentacuentos (story) for this writing session
    let story = if let Some(ref ws_id) = anky.writing_session_id {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_cuentacuentos_by_writing_id(&db, ws_id)
            .ok()
            .flatten()
    } else {
        None
    };
    if let Some(ref s) = story {
        ctx.insert("story_title", &s.title);
        ctx.insert("story_content", &s.content);
        ctx.insert("story_kingdom", &s.kingdom.as_deref().unwrap_or(""));
        ctx.insert("story_city", &s.city.as_deref().unwrap_or(""));
        ctx.insert("has_story", &true);
    } else {
        ctx.insert("has_story", &false);
        ctx.insert("story_title", &"");
        ctx.insert("story_content", &"");
        ctx.insert("story_kingdom", &"");
        ctx.insert("story_city", &"");
    }

    // Ownership check — enables reply UI
    let is_owner = if let Some(ref vid) = viewer_id {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_anky_owner(&db, &id)
            .ok()
            .flatten()
            .as_deref()
            == Some(vid.as_str())
    } else {
        false
    };
    ctx.insert("is_owner", &is_owner);
    ctx.insert("is_logged_in", &viewer_id.is_some());

    // Conversation history
    let conversation_json = {
        let db = crate::db::conn(&state.db)?;
        crate::db::queries::get_anky_conversation(&db, &id)
            .ok()
            .flatten()
            .unwrap_or_default()
    };
    ctx.insert("conversation_json", &conversation_json);

    let html = state.tera.render("anky.html", &ctx)?;
    Ok((jar, Html(html)))
}

pub async fn videos_gallery(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Html<String>, AppError> {
    let logged_in = crate::routes::auth::get_auth_user(&state, &jar)
        .await
        .is_some();
    let items = {
        let db = crate::db::conn(&state.db)?;
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

// ── Programming classes ────────────────────────────────────────────────────

pub async fn class_page(
    State(state): State<AppState>,
    Path(class_number): Path<i64>,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("class_number", &class_number);

    let db = crate::db::conn(&state.db)?;

    // Fetch the requested class
    let class_row = db.query_row(
        "SELECT title, description, slides_json FROM programming_classes WHERE class_number = ?1",
        crate::params![class_number],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    );

    // Get latest class number for nav
    let latest_class: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(class_number), 0) FROM programming_classes",
            crate::params![],
            |row| row.get(0),
        )
        .unwrap_or(0);

    drop(db);

    ctx.insert("latest_class", &latest_class);

    match class_row {
        Ok((title, _description, slides_json)) => {
            ctx.insert("found", &true);
            ctx.insert("title", &title);

            let slides: Vec<serde_json::Value> =
                serde_json::from_str(&slides_json).unwrap_or_default();
            ctx.insert("slides", &slides);
        }
        Err(_) => {
            ctx.insert("found", &false);
            ctx.insert("title", &"");
            ctx.insert("slides", &Vec::<serde_json::Value>::new());
        }
    }

    let html = state.tera.render("class.html", &ctx)?;
    Ok(Html(html))
}

/// List all classes
pub async fn classes_index(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();

    let classes = {
        let db = crate::db::conn(&state.db)?;
        let mut stmt = db.prepare(
            "SELECT class_number, title, description, created_at FROM programming_classes ORDER BY class_number DESC",
        )?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(crate::params![], |row| {
                Ok(serde_json::json!({
                    "number": row.get::<_, i64>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "description": row.get::<_, String>(2)?,
                    "created_at": row.get::<_, String>(3)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    ctx.insert("classes", &classes);
    let html = state.tera.render("classes_index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn encoder(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.tera.render("encoder.html", &ctx)?;
    Ok(Html(html))
}

/// API: return anky data with keystroke stream for the encoder page
pub async fn encoder_data(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let db = crate::db::conn(&state.db)?;

    // If anky_id provided, fetch that one; otherwise list all available
    if let Some(anky_id) = params.get("id") {
        let row = db.query_row(
            "SELECT a.id, a.title, a.kingdom_name, a.image_path,
                    ws.content, ws.keystroke_deltas, ws.duration_seconds, ws.word_count
             FROM ankys a
             JOIN writing_sessions ws ON ws.id = a.writing_session_id
             WHERE a.id = ?1
               AND ws.keystroke_deltas IS NOT NULL
               AND length(ws.keystroke_deltas) > 10",
            crate::params![anky_id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    "kingdom": row.get::<_, Option<String>>(2)?.unwrap_or("Poiesis".into()),
                    "image_path": row.get::<_, Option<String>>(3)?,
                    "text": row.get::<_, String>(4)?,
                    "keystroke_deltas": row.get::<_, Option<String>>(5)?,
                    "duration": row.get::<_, f64>(6)?,
                    "word_count": row.get::<_, i32>(7)?,
                }))
            },
        );
        match row {
            Ok(data) => Ok(axum::Json(data)),
            Err(_) => Ok(axum::Json(serde_json::json!({"error": "not found"}))),
        }
    } else {
        // List all ankys with keystroke data
        let mut stmt = db.prepare(
            "SELECT a.id, a.title, a.kingdom_name, a.image_path,
                    ws.word_count, ws.duration_seconds
             FROM ankys a
             JOIN writing_sessions ws ON ws.id = a.writing_session_id
             WHERE ws.is_anky = 1
               AND ws.keystroke_deltas IS NOT NULL
               AND length(ws.keystroke_deltas) > 100
               AND a.image_path IS NOT NULL
             ORDER BY ws.created_at DESC",
        )?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(crate::params![], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    "kingdom": row.get::<_, Option<String>>(2)?.unwrap_or("Poiesis".into()),
                    "image_path": row.get::<_, Option<String>>(3)?,
                    "word_count": row.get::<_, i32>(4)?,
                    "duration": row.get::<_, f64>(5)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(axum::Json(serde_json::json!({ "ankys": rows })))
    }
}

/// Classify an anky into one of the 8 ankyverse kingdoms (chakras) via Claude Haiku.
/// Returns (kingdom_name, kingdom_id, reason). Cached per-anky in Postgres.
async fn classify_anky_kingdom(
    state: &AppState,
    anky_id: &str,
    reflection: &str,
    writing: &str,
    title: &str,
) -> (String, i32, String) {
    // Cached?
    if let Ok(db) = crate::db::conn(&state.db) {
        if let Ok((k, id, reason)) = db.query_row(
            "SELECT kingdom_name, kingdom_id, reason FROM anky_kingdom_classification WHERE anky_id = ?1",
            crate::params![anky_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i32>(1)?, r.get::<_, String>(2)?)),
        ) {
            return (k, id, reason);
        }
    }

    // Build the kingdoms block for the prompt.
    let mut kingdom_block = String::new();
    for k in crate::kingdoms::KINGDOMS.iter() {
        kingdom_block.push_str(&format!(
            "- {} ({}, {}): lesson — {}\n",
            k.name, k.chakra, k.element, k.lesson
        ));
    }

    let system = "You are Anky, the oracle of the ankyverse. Your job is to read a single \
writing session and say which of the 8 kingdoms it belongs to. Each kingdom is a chakra — a \
distinct register of consciousness. Read for the underlying current, not the surface topic. \
Return STRICT JSON only: {\"kingdom\":\"<Name>\",\"reason\":\"<one precise sentence, max 24 \
words, explaining why>\"}. The kingdom name MUST be one of: Primordia, Emblazion, Chryseos, \
Eleasis, Voxlumis, Insightia, Claridium, Poiesis.";

    let user_prompt = format!(
        "THE 8 KINGDOMS:\n{}\n\
─────────\n\
TITLE: {}\n\n\
WRITING (raw stream from the person):\n{}\n\n\
REFLECTION (what the oracle said back):\n{}\n\n\
─────────\n\
Now classify. Respond with JSON only.",
        kingdom_block,
        title,
        writing.chars().take(1200).collect::<String>(),
        reflection.chars().take(1500).collect::<String>(),
    );

    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    let fallback = || {
        // Deterministic fallback: hash the anky id into one of 8.
        let k = crate::kingdoms::kingdom_for_session(anky_id);
        (
            k.name.to_string(),
            k.id as i32,
            format!("classified by ankyverse resonance ({}).", k.chakra),
        )
    };

    if api_key.is_empty() {
        return fallback();
    }

    let result = crate::services::claude::call_claude_public(
        &api_key,
        crate::services::claude::HAIKU_MODEL,
        system,
        &user_prompt,
        400,
    )
    .await;

    let (kingdom_name, reason) = match result {
        Ok(r) => {
            // Strip code fences if present.
            let raw = r.text.trim();
            let json_str = raw
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(v) => {
                    let k = v
                        .get("kingdom")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                    let r = v
                        .get("reason")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    match k {
                        Some(k) if crate::kingdoms::KINGDOMS.iter().any(|kk| kk.name == k) => {
                            (k, r)
                        }
                        _ => return fallback(),
                    }
                }
                Err(_) => return fallback(),
            }
        }
        Err(e) => {
            tracing::warn!("classify_anky_kingdom: {}", e);
            return fallback();
        }
    };

    let kingdom_id = crate::kingdoms::KINGDOMS
        .iter()
        .find(|k| k.name == kingdom_name)
        .map(|k| k.id as i32)
        .unwrap_or(0);

    // Cache.
    if let Ok(db) = crate::db::conn(&state.db) {
        let _ = db.execute(
            "INSERT INTO anky_kingdom_classification (anky_id, kingdom_name, kingdom_id, reason) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (anky_id) DO UPDATE SET kingdom_name = EXCLUDED.kingdom_name, \
                 kingdom_id = EXCLUDED.kingdom_id, reason = EXCLUDED.reason",
            crate::params![anky_id, &kingdom_name, &kingdom_id, &reason],
        );
    }

    (kingdom_name, kingdom_id, reason)
}

/// Legacy 5-mood classifier (unused by new `/profile-testing` handler but kept
/// in case future experiments want the keyword fallback).
#[allow(dead_code)]
fn classify_emotional_kingdom(reflection: &str, writing: &str, title: &str) -> &'static str {
    let haystack = format!("{} {} {}", reflection, writing, title).to_lowercase();
    let kingdoms: [(&str, &[&str]); 5] = [
        (
            "Fear",
            &[
                "fear", "afraid", "anxious", "anxiety", "trap", "cage", "contain", "stuck",
                "scared", "panic", "threat", "danger", "dread", "tight", "clench", "hide", "avoid",
            ],
        ),
        (
            "Creativity",
            &[
                "create",
                "creat",
                "art",
                "imagine",
                "imagin",
                "vision",
                "idea",
                "dream",
                "kaleidosc",
                "color",
                "wild",
                "dance",
                "cascad",
                "flow",
                "invent",
                "build",
                "make",
                "craft",
                "play",
                "whirl",
                "spiral",
                "psychedel",
                "ayahuasca",
                "dmt",
            ],
        ),
        (
            "Clarity",
            &[
                "clear",
                "clarity",
                "truth",
                "thread",
                "realize",
                "understand",
                "see clearly",
                "arrival",
                "frame",
                "name it",
                "naming",
                "sharp",
                "precise",
                "crystal",
                "focus",
                "mirror",
                "witness",
                "surface",
                "architecture",
                "simpl",
            ],
        ),
        (
            "Grief",
            &[
                "grief",
                "loss",
                "lost",
                "empty",
                "ache",
                "gone",
                "sad",
                "tear",
                "mourn",
                "absence",
                "distance",
                "ghost",
                "slow",
                "silent",
                "lonel",
                "alone",
                "hermit",
                "purge",
                "throwing up",
                "body",
            ],
        ),
        (
            "Love",
            &[
                "love", "tender", "soft", "heart", "warm", "family", "friend", "together", "kind",
                "care", "belong", "thank", "hug", "plushie", "mother", "father", "home",
            ],
        ),
    ];

    let mut best = ("Creativity", 0i32);
    for (name, keywords) in kingdoms.iter() {
        let mut score = 0i32;
        for kw in keywords.iter() {
            if haystack.contains(kw) {
                score += 1;
            }
        }
        if score > best.1 {
            best = (name, score);
        }
    }
    best.0
}

fn image_public_url(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") || path.starts_with('/') {
        path.to_string()
    } else {
        format!("/data/images/{}", path)
    }
}

/// Generate (or reuse) the 3-4 sentence written portrait of the writer
/// synthesized across ALL their sessions. Cached per user; invalidated when
/// anky_count changes (i.e. they wrote a new one).
async fn generate_user_portrait(
    state: &AppState,
    user_id: &str,
    username: &str,
    sessions: &[serde_json::Value],
) -> String {
    let anky_count = sessions.len() as i32;

    // Cached and still valid?
    if let Ok(db) = crate::db::conn(&state.db) {
        if let Ok((portrait, cached_count)) = db.query_row(
            "SELECT portrait, anky_count FROM anky_user_portrait WHERE user_id = ?1",
            crate::params![user_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i32>(1)?)),
        ) {
            if cached_count == anky_count {
                return portrait;
            }
        }
    }

    // Build the distribution + a compact digest of each session for Claude.
    let mut by_kingdom: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for s in sessions.iter() {
        if let Some(k) = s.get("kingdom").and_then(|v| v.as_str()) {
            *by_kingdom.entry(k.to_string()).or_insert(0) += 1;
        }
    }
    let mut dist: Vec<(String, i32)> = by_kingdom.into_iter().collect();
    dist.sort_by(|a, b| b.1.cmp(&a.1));
    let dist_line = dist
        .iter()
        .map(|(k, c)| format!("{} ({})", k, c))
        .collect::<Vec<_>>()
        .join(", ");

    let mut sessions_block = String::new();
    for (i, s) in sessions.iter().enumerate() {
        let kingdom = s.get("kingdom").and_then(|v| v.as_str()).unwrap_or("");
        let date = s.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let reason = s
            .get("kingdom_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let reflection = s.get("reflection").and_then(|v| v.as_str()).unwrap_or("");
        sessions_block.push_str(&format!(
            "[{}] {} — {} ({})\n  why: {}\n  oracle said: {}\n\n",
            i + 1,
            date,
            kingdom,
            title,
            reason,
            reflection.chars().take(280).collect::<String>(),
        ));
    }

    let system = "You are Anky, the oracle. You have read everything this person has ever \
written on the ankyverse. Your task is to write a single PORTRAIT of the writer as a whole — \
who this person is, where they are right now, what the pattern of their writing reveals. \
\
Hard rules: \
\
1) Exactly 3 or 4 sentences. No more, no less. \
2) Second person. Speak TO the writer (\"you\"), not about them. \
3) Reference their actual kingdom distribution by name — e.g. \"you keep returning to Eleasis\" \
   or \"most of your writing lives in the heart kingdom\". Be specific to the shape of THEIR map. \
4) Reference a recurring theme or motif you notice across sessions (an image, a word, a posture). \
5) No generic language. Forbidden phrases include: \"you are a thoughtful writer\", \"your \
   writing shows\", \"introspective\", \"reflective\", \"on a journey\", \"finding yourself\". \
6) Slightly uncanny. Like a close friend who has read every single one of their ankys. \
7) Plain prose. No markdown, no headings, no quotes around it.";

    let user_prompt = format!(
        "WRITER: {}\n\
TOTAL ANKYS: {}\n\
KINGDOM DISTRIBUTION: {}\n\n\
ALL THEIR SESSIONS (newest first):\n\n{}\n\n\
Write the portrait now. 3-4 sentences. Plain text only.",
        username, anky_count, dist_line, sessions_block,
    );

    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return format!(
            "your writing has landed mostly in {}. {} sessions in, a shape is forming.",
            dist.first().map(|(k, _)| k.as_str()).unwrap_or("Poiesis"),
            anky_count
        );
    }

    let result = crate::services::claude::call_claude_public(
        &api_key,
        crate::services::claude::SONNET_MODEL,
        system,
        &user_prompt,
        400,
    )
    .await;

    let portrait = match result {
        Ok(r) => {
            let mut t = r.text.trim().to_string();
            // strip wrapping quotes / code fences just in case
            t = t
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
                .to_string();
            if t.starts_with('"') && t.ends_with('"') && t.len() > 2 {
                t = t[1..t.len() - 1].to_string();
            }
            t
        }
        Err(e) => {
            tracing::warn!("generate_user_portrait: {}", e);
            format!(
                "your writing has landed mostly in {}. {} sessions in, a shape is forming.",
                dist.first().map(|(k, _)| k.as_str()).unwrap_or("Poiesis"),
                anky_count
            )
        }
    };

    if let Ok(db) = crate::db::conn(&state.db) {
        let _ = db.execute(
            "INSERT INTO anky_user_portrait (user_id, portrait, anky_count) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT (user_id) DO UPDATE SET portrait = EXCLUDED.portrait, \
                 anky_count = EXCLUDED.anky_count, \
                 generated_at = to_char(now() AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')",
            crate::params![user_id, &portrait, &anky_count],
        );
    }

    portrait
}

/// GET /profile-testing — prototype of the new profile map.
/// Loads the most prolific user (≥8 ankys), classifies each session into
/// one of 8 ankyverse kingdoms, and renders the world-map profile.
pub async fn profile_testing_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let db = crate::db::conn(&state.db)?;

    // Pick the user with the most real ankys that have both reflection + image.
    let user_id: String = db
        .query_row(
            "SELECT a.user_id
               FROM ankys a
               JOIN writing_sessions ws ON ws.id = a.writing_session_id
              WHERE a.reflection IS NOT NULL
                AND a.image_path IS NOT NULL
              GROUP BY a.user_id
             HAVING COUNT(*) >= 8
              ORDER BY COUNT(*) DESC
              LIMIT 1",
            crate::params![],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "api-user".to_string());

    let username: String = db
        .query_row(
            "SELECT COALESCE(username, display_name, id) FROM users WHERE id = ?1",
            crate::params![&user_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| user_id.clone());

    // Pull the latest 12 complete ankys for this user.
    #[derive(Debug)]
    struct Row {
        id: String,
        title: String,
        reflection: String,
        image_path: String,
        content: String,
        word_count: i64,
        duration_seconds: f64,
        created_at: String,
    }

    let rows: Vec<Row> = {
        let mut stmt = db.prepare(
            "SELECT a.id,
                    COALESCE(a.title, 'untitled'),
                    COALESCE(a.reflection, ''),
                    COALESCE(a.image_webp, a.image_path, ''),
                    COALESCE(ws.content, ''),
                    COALESCE(ws.word_count, 0),
                    COALESCE(ws.duration_seconds, 0.0),
                    a.created_at
               FROM ankys a
               JOIN writing_sessions ws ON ws.id = a.writing_session_id
              WHERE a.user_id = ?1
                AND a.reflection IS NOT NULL
                AND a.image_path IS NOT NULL
              ORDER BY a.created_at DESC
              LIMIT 12",
        )?;
        let iter = stmt.query_map(crate::params![&user_id], |r| {
            Ok(Row {
                id: r.get(0)?,
                title: r.get(1)?,
                reflection: r.get(2)?,
                image_path: r.get(3)?,
                content: r.get(4)?,
                word_count: r.get(5)?,
                duration_seconds: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;
        iter.filter_map(|r| r.ok()).collect()
    };

    // Classify each anky in parallel via Claude Haiku (cached per-anky in DB).
    let mut classifications: Vec<(String, i32, String)> = Vec::with_capacity(rows.len());
    {
        let mut futs = Vec::with_capacity(rows.len());
        for r in rows.iter() {
            futs.push(classify_anky_kingdom(
                &state,
                &r.id,
                &r.reflection,
                &r.content,
                &r.title,
            ));
        }
        let results = futures::future::join_all(futs).await;
        classifications.extend(results);
    }

    // Build serializable sessions.
    let sessions: Vec<serde_json::Value> = rows
        .iter()
        .zip(classifications.into_iter())
        .map(|(r, (kingdom, kingdom_id, reason))| {
            let secs = r.duration_seconds as i64;
            let mins = secs / 60;
            let rem = secs % 60;
            let duration = format!("{}:{:02}", mins, rem);

            // "apr 11" style. created_at is stored as "YYYY-MM-DD HH:MM:SS" (no TZ).
            let date = chrono::NaiveDateTime::parse_from_str(&r.created_at, "%Y-%m-%d %H:%M:%S")
                .ok()
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc3339(&r.created_at)
                        .ok()
                        .map(|d| d.naive_utc())
                })
                .map(|d| d.format("%b %-d").to_string().to_lowercase())
                .unwrap_or_else(|| r.created_at.clone());

            // Trim the reflection: strip the "hey, thanks for being who you are. my thoughts:" preamble
            // and any leading markdown heading so the quoted snippet reads cleanly.
            let cleaned = r
                .reflection
                .replace("hey, thanks for being who you are. my thoughts:", "")
                .trim()
                .to_string();
            // Strip leading "## ..." heading line
            let after_heading = cleaned
                .lines()
                .skip_while(|l| l.trim().is_empty() || l.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            let trimmed = after_heading.trim();
            // Take first ~2 sentences / 320 chars
            let snippet: String = if trimmed.len() > 320 {
                let cut = trimmed[..320]
                    .rfind(|c: char| c == '.' || c == '!' || c == '?')
                    .map(|i| i + 1)
                    .unwrap_or(320);
                trimmed[..cut].to_string()
            } else {
                trimmed.to_string()
            };

            let k_meta = crate::kingdoms::KINGDOMS
                .iter()
                .find(|k| k.name == kingdom)
                .unwrap_or(&crate::kingdoms::KINGDOMS[7]);
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "kingdom": kingdom,
                "kingdom_id": kingdom_id,
                "kingdom_chakra": k_meta.chakra,
                "kingdom_element": k_meta.element,
                "kingdom_lesson": k_meta.lesson,
                "kingdom_reason": reason,
                "date": date,
                "words": r.word_count,
                "duration": duration,
                "image_url": image_public_url(&r.image_path),
                "reflection": snippet,
                "sealed": r.duration_seconds >= 480.0,
            })
        })
        .collect();

    // Dominant kingdom
    let mut counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for s in &sessions {
        if let Some(k) = s.get("kingdom").and_then(|v| v.as_str()) {
            *counts.entry(k.to_string()).or_insert(0) += 1;
        }
    }
    let dominant = counts
        .iter()
        .max_by_key(|(_, c)| **c)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| "Poiesis".to_string());

    // Hero image = latest anky's image
    let hero_image = sessions
        .first()
        .and_then(|s| s.get("image_url").and_then(|v| v.as_str()))
        .unwrap_or("/static/icon-192.png")
        .to_string();

    // Kingdom metadata for the front end (all 8, ordered by chakra)
    let kingdoms_meta: Vec<serde_json::Value> = crate::kingdoms::KINGDOMS
        .iter()
        .map(|k| {
            serde_json::json!({
                "id": k.id,
                "name": k.name,
                "chakra": k.chakra,
                "element": k.element,
                "lesson": k.lesson,
            })
        })
        .collect();

    let kingdom_counts: Vec<serde_json::Value> = crate::kingdoms::KINGDOMS
        .iter()
        .filter_map(|k| {
            counts.get(k.name).map(|c| {
                serde_json::json!({
                    "id": k.id,
                    "name": k.name,
                    "chakra": k.chakra,
                    "count": c,
                })
            })
        })
        .collect();

    let portrait = generate_user_portrait(&state, &user_id, &username, &sessions).await;

    let mut ctx = tera::Context::new();
    ctx.insert("username", &username);
    ctx.insert("sessions", &sessions);
    ctx.insert("dominant_kingdom", &dominant);
    ctx.insert("hero_image", &hero_image);
    ctx.insert("kingdom_counts", &kingdom_counts);
    ctx.insert("kingdoms_meta", &kingdoms_meta);
    ctx.insert("portrait", &portrait);

    let html = state.tera.render("profile_testing.html", &ctx)?;
    Ok(Html(html))
}
