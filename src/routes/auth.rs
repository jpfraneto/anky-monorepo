use crate::db::queries;
use crate::error::AppError;
use crate::services::twitter;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};

const PRIMARY_WEB_SESSION_COOKIE: &str = "anky_session";
const LEGACY_WEB_SESSION_COOKIE: &str = "anky_privy_token";
const WEB_USER_COOKIE: &str = "anky_user_id";

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    pub redirect_to: Option<String>,
}

/// GET /auth/x/login — initiate X OAuth 2.0 PKCE flow
pub async fn login(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AppError> {
    let client_id = &state.config.twitter_client_id;
    if client_id.is_empty() {
        return Err(AppError::Internal("Twitter OAuth not configured".into()));
    }

    let (verifier, challenge) = twitter::generate_pkce();
    let oauth_state = twitter::generate_state();

    // Save state + verifier to DB
    {
        let db = crate::db::conn(&state.db)?;
        queries::save_oauth_state(&db, &oauth_state, &verifier, query.redirect_to.as_deref())?;
    }

    let scopes = &["tweet.read", "tweet.write", "users.read", "offline.access"];
    let auth_url = twitter::authorization_url(
        client_id,
        &state.config.twitter_callback_url,
        &oauth_state,
        &challenge,
        scopes,
    );

    Ok(Redirect::temporary(&auth_url))
}

#[derive(serde::Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// GET /auth/x/callback — handle OAuth callback
pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(err) = &query.error {
        tracing::warn!("OAuth error: {}", err);
        return Ok((jar, Redirect::temporary("/")));
    }

    let code = query
        .code
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("missing code".into()))?;
    let oauth_state = query
        .state
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("missing state".into()))?;

    // Look up and delete PKCE state
    let (verifier, redirect_to) = {
        let db = crate::db::conn(&state.db)?;
        queries::get_and_delete_oauth_state(&db, oauth_state)?
            .ok_or_else(|| AppError::BadRequest("invalid or expired state".into()))?
    };

    // Exchange code for tokens
    let tokens = twitter::exchange_code(
        &state.config.twitter_client_id,
        &state.config.twitter_client_secret,
        code,
        &verifier,
        &state.config.twitter_callback_url,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Token exchange failed: {}", e)))?;

    // Get user profile
    let user = twitter::get_user(&tokens.access_token)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get user: {}", e)))?;

    // Check if this X user already has a linked anky user, or create one
    let user_id = {
        let db = crate::db::conn(&state.db)?;
        let existing = queries::get_x_user_by_x_id(&db, &user.id)?;
        let uid = match existing {
            Some(xu) => xu.user_id,
            None => {
                let uid =
                    visitor_id_from_jar(&jar).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                queries::ensure_user(&db, &uid)?;
                uid
            }
        };

        // Upsert X user record
        let expires_at = tokens.expires_in.map(|secs| {
            chrono::Utc::now()
                .checked_add_signed(chrono::Duration::seconds(secs as i64))
                .map(|t| t.to_rfc3339())
                .unwrap_or_default()
        });

        queries::upsert_x_user(
            &db,
            &user.id,
            &uid,
            &user.username,
            Some(&user.name),
            user.profile_image_url.as_deref(),
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
            expires_at.as_deref(),
        )?;

        // Auto-set username to X handle if user has none
        if queries::get_user_username(&db, &uid)?.is_none() {
            // Only set if not taken by another user
            if queries::check_username_available(&db, &user.username, &uid)? {
                let _ = queries::set_username(&db, &uid, &user.username);
            }
        }

        uid
    };

    // Create auth session
    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .unwrap()
        .to_rfc3339();

    {
        let db = crate::db::conn(&state.db)?;
        queries::create_auth_session(&db, &session_token, &user_id, Some(&user.id), &expires_at)?;
    }

    merge_visitor_writings(&state, &jar, &user_id);

    state.emit_log(
        "INFO",
        "auth",
        &format!("X login: @{} ({})", user.username, &user_id[..8]),
    );

    let jar = add_web_auth_cookies(jar, &session_token, &user_id, 30);
    let redirect = redirect_to.as_deref().unwrap_or("/");

    Ok((jar, Redirect::temporary(redirect)))
}

/// GET /auth/x/logout — delete auth session and cookies
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    delete_session_from_jar(&state, &jar)?;
    let jar = clear_web_auth_cookies(jar);

    Ok((jar, Redirect::temporary("/")))
}

/// Helper: extract browser auth context for page rendering.
pub struct AuthUser {
    pub user_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub profile_image_url: Option<String>,
    pub wallet_address: Option<String>,
    pub email: Option<String>,
}

/// Reattach writing sessions that were saved under a visitor id (anky_user_id cookie)
/// to the authenticated user_id after login. Best-effort: failure is logged and
/// does not block the login flow.
fn merge_visitor_writings(state: &AppState, jar: &CookieJar, authenticated_user_id: &str) {
    let visitor_id = match visitor_id_from_jar(jar) {
        Some(v) => v,
        None => return,
    };
    if visitor_id == authenticated_user_id {
        return;
    }
    let db = match crate::db::conn(&state.db) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("merge_visitor_writings: db conn failed: {}", e);
            return;
        }
    };
    match db.execute(
        "UPDATE writing_sessions SET user_id = ?1 WHERE user_id = ?2",
        crate::params![authenticated_user_id, &visitor_id],
    ) {
        Ok(rows) => {
            if rows > 0 {
                state.emit_log(
                    "INFO",
                    "auth",
                    &format!(
                        "merged {} visitor writing_sessions from {} → {}",
                        rows,
                        &visitor_id[..visitor_id.len().min(8)],
                        &authenticated_user_id[..authenticated_user_id.len().min(8)],
                    ),
                );
            }
        }
        Err(e) => {
            tracing::warn!("merge_visitor_writings UPDATE failed: {}", e);
        }
    }
}

fn build_auth_session_cookie(session_token: &str, max_age_days: i64) -> Cookie<'static> {
    Cookie::build((PRIMARY_WEB_SESSION_COOKIE, session_token.to_string()))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .max_age(time::Duration::days(max_age_days))
        .build()
}

fn build_user_cookie(user_id: &str) -> Cookie<'static> {
    Cookie::build((WEB_USER_COOKIE, user_id.to_string()))
        .path("/")
        .http_only(false)
        .secure(true)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .max_age(time::Duration::days(365))
        .build()
}

pub fn build_visitor_cookie(user_id: &str) -> Cookie<'static> {
    Cookie::build((WEB_USER_COOKIE, user_id.to_string()))
        .path("/")
        .http_only(false)
        .secure(true)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .max_age(time::Duration::days(365))
        .build()
}

fn add_web_auth_cookies(
    jar: CookieJar,
    session_token: &str,
    user_id: &str,
    max_age_days: i64,
) -> CookieJar {
    jar.add(build_auth_session_cookie(session_token, max_age_days))
        .add(build_user_cookie(user_id))
}

fn clear_web_auth_cookies(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::build(PRIMARY_WEB_SESSION_COOKIE).path("/").build())
        .remove(Cookie::build(LEGACY_WEB_SESSION_COOKIE).path("/").build())
        .remove(Cookie::build(WEB_USER_COOKIE).path("/").build())
}

pub fn visitor_id_from_jar(jar: &CookieJar) -> Option<String> {
    jar.get(WEB_USER_COOKIE)
        .map(|cookie| cookie.value().to_string())
}

pub fn visitor_cookie_if_missing(jar: &CookieJar) -> Option<Cookie<'static>> {
    if visitor_id_from_jar(jar).is_some() {
        None
    } else {
        Some(build_visitor_cookie(&uuid::Uuid::new_v4().to_string()))
    }
}

pub fn ensure_visitor_cookie(jar: CookieJar) -> CookieJar {
    if let Some(cookie) = visitor_cookie_if_missing(&jar) {
        jar.add(cookie)
    } else {
        jar
    }
}

fn session_token_from_jar(jar: &CookieJar) -> Option<String> {
    jar.get(PRIMARY_WEB_SESSION_COOKIE)
        .or_else(|| jar.get(LEGACY_WEB_SESSION_COOKIE))
        .map(|cookie| cookie.value().to_string())
}

fn delete_session_from_jar(state: &AppState, jar: &CookieJar) -> Result<(), AppError> {
    if let Some(token) = session_token_from_jar(jar) {
        let db = crate::db::conn(&state.db)?;
        let _ = queries::delete_auth_session(&db, &token);
    }
    Ok(())
}

fn auth_user_from_session_token(state: &AppState, token: &str) -> Option<AuthUser> {
    let db = crate::db::conn(&state.db).ok()?;
    let (user_id, x_user_id) = queries::get_auth_session(&db, token).ok()??;
    let wallet_address = queries::get_user_wallet(&db, &user_id).ok().flatten();
    let email = queries::get_user_email(&db, &user_id).ok().flatten();
    let (username, farcaster_username, farcaster_pfp_url) = db
        .query_row(
            "SELECT username, farcaster_username, farcaster_pfp_url FROM users WHERE id = ?1",
            crate::params![&user_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .ok()
        .unwrap_or((None, None, None));

    if let Some(ref xid) = x_user_id {
        if let Ok(Some(xu)) = queries::get_x_user_by_x_id(&db, xid) {
            let fallback_username = username.clone().or(farcaster_username.clone());
            let x_username = xu.username.clone();
            return Some(AuthUser {
                user_id: xu.user_id,
                username: Some(x_username.clone()).or(fallback_username.clone()),
                display_name: xu.display_name.or(Some(x_username)),
                profile_image_url: xu.profile_image_url.or(farcaster_pfp_url),
                wallet_address,
                email,
            });
        }
    }

    let display_name = username.clone().or(farcaster_username.clone());
    Some(AuthUser {
        user_id,
        username: username.or(farcaster_username),
        display_name,
        profile_image_url: farcaster_pfp_url,
        wallet_address,
        email,
    })
}

pub async fn get_auth_user(state: &AppState, jar: &CookieJar) -> Option<AuthUser> {
    session_token_from_jar(jar).and_then(|token| auth_user_from_session_token(state, &token))
}

/// Resolve the authenticated user_id from the `anky_session` cookie, if any.
/// Unlike `get_auth_user`, this does only the minimal session lookup.
pub fn authenticated_user_id_from_jar(state: &AppState, jar: &CookieJar) -> Option<String> {
    let token = session_token_from_jar(jar)?;
    let db = crate::db::conn(&state.db).ok()?;
    let (user_id, _) = queries::get_auth_session(&db, &token).ok()??;
    Some(user_id)
}

/// POST /auth/farcaster/verify — authenticate via Farcaster MiniApp SDK context.
/// The FID from sdk.context is trusted (comes from Farcaster client's iframe postMessage protocol).
#[derive(serde::Deserialize)]
pub struct FarcasterVerifyRequest {
    pub fid: i64,
    pub username: Option<String>,
    pub pfp_url: Option<String>,
    pub wallet_address: Option<String>,
}

pub async fn farcaster_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<FarcasterVerifyRequest>,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    if req.fid == 0 {
        return Err(AppError::BadRequest("missing fid".into()));
    }

    let username = req.username.as_deref().unwrap_or("").to_string();
    let pfp_url = req.pfp_url.clone();
    let wallet_addr = req.wallet_address.clone();

    // Look up existing user by FID, or create one
    let user_id = {
        let db = crate::db::conn(&state.db)?;
        if let Some(uid) = queries::get_user_by_fid(&db, req.fid)? {
            // Update Farcaster info in case username/pfp changed
            let _ = queries::set_farcaster_info(
                &db,
                &uid,
                req.fid as u64,
                &username,
                pfp_url.as_deref(),
            );
            // Update wallet if provided and not already set
            if let Some(ref addr) = wallet_addr {
                if queries::get_user_wallet(&db, &uid)?.is_none() {
                    let _ = queries::set_wallet_address(&db, &uid, addr);
                }
            }
            uid
        } else {
            // Check if there's an existing user by wallet address
            let uid = if let Some(ref addr) = wallet_addr {
                if let Some(existing_uid) = queries::get_user_by_wallet(&db, addr)? {
                    // Link FID to existing wallet user
                    let _ = queries::set_farcaster_info(
                        &db,
                        &existing_uid,
                        req.fid as u64,
                        &username,
                        pfp_url.as_deref(),
                    );
                    existing_uid
                } else {
                    let new_uid = uuid::Uuid::new_v4().to_string();
                    queries::create_user_with_farcaster(
                        &db,
                        &new_uid,
                        req.fid,
                        &username,
                        pfp_url.as_deref(),
                        Some(addr),
                    )?;
                    new_uid
                }
            } else {
                let new_uid = uuid::Uuid::new_v4().to_string();
                queries::create_user_with_farcaster(
                    &db,
                    &new_uid,
                    req.fid,
                    &username,
                    pfp_url.as_deref(),
                    None,
                )?;
                new_uid
            };

            // Auto-claim Farcaster username if available
            if !username.is_empty() {
                if queries::get_user_username(&db, &uid)?.is_none() {
                    if queries::check_username_available(&db, &username, &uid).unwrap_or(false) {
                        let _ = queries::set_username(&db, &uid, &username);
                    }
                }
            }

            uid
        }
    };

    // Create auth session
    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .unwrap()
        .to_rfc3339();

    {
        let db = crate::db::conn(&state.db)?;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    merge_visitor_writings(&state, &jar, &user_id);

    state.emit_log(
        "INFO",
        "auth",
        &format!(
            "Farcaster login: @{} (fid {}, {})",
            username,
            req.fid,
            &user_id[..8]
        ),
    );

    // Get final wallet + username for response
    let (final_wallet, final_username) = {
        let db = crate::db::conn(&state.db)?;
        (
            queries::get_user_wallet(&db, &user_id)?,
            queries::get_user_username(&db, &user_id)?,
        )
    };

    let jar = add_web_auth_cookies(jar, &session_token, &user_id, 30);

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "user_id": user_id,
            "wallet_address": final_wallet,
            "username": final_username,
        })),
    ))
}

/// POST /auth/solana/verify — authenticate via Phantom (or any Solana wallet).
/// The browser extension signs a challenge nonce; we verify the Ed25519 signature.
#[derive(serde::Deserialize)]
pub struct SolanaVerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
}

pub async fn solana_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SolanaVerifyRequest>,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    // Validate and normalize the wallet address
    let wallet = crate::services::wallet::normalize_solana_address(&req.wallet_address)?;

    // Verify the Ed25519 signature
    crate::services::wallet::verify_solana_signature(&wallet, &req.message, &req.signature)?;

    // Find existing user by wallet or create a new one
    let user_id = {
        let db = crate::db::conn(&state.db)?;
        if let Some(uid) = queries::get_user_by_wallet(&db, &wallet)? {
            uid
        } else {
            // Check if current visitor cookie has a user we can attach the wallet to
            let uid = visitor_id_from_jar(&jar).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            queries::ensure_user(&db, &uid)?;
            queries::set_wallet_address(&db, &uid, &wallet)?;
            uid
        }
    };

    // Create auth session (30 days)
    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .unwrap()
        .to_rfc3339();

    {
        let db = crate::db::conn(&state.db)?;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    state.emit_log(
        "INFO",
        "auth",
        &format!("Solana login: {} ({})", &wallet[..8], &user_id[..8]),
    );

    let username = {
        let db = crate::db::conn(&state.db)?;
        queries::get_user_username(&db, &user_id)?
    };

    let jar = add_web_auth_cookies(jar, &session_token, &user_id, 30);

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "session_token": session_token,
            "user_id": user_id,
            "wallet_address": wallet,
            "username": username,
        })),
    ))
}

/// POST /auth/logout — clear browser auth cookies and invalidate the current session.
pub async fn logout_json(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    delete_session_from_jar(&state, &jar)?;
    let jar = clear_web_auth_cookies(jar);

    Ok((jar, Json(serde_json::json!({ "ok": true }))))
}
