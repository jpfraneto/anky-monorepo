use crate::db::queries;
use crate::error::AppError;
use crate::services::twitter;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar};

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
        let db = state.db.lock().await;
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

    let code = query.code.as_deref().ok_or_else(|| AppError::BadRequest("missing code".into()))?;
    let oauth_state = query.state.as_deref().ok_or_else(|| AppError::BadRequest("missing state".into()))?;

    // Look up and delete PKCE state
    let (verifier, redirect_to) = {
        let db = state.db.lock().await;
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
        let db = state.db.lock().await;
        let existing = queries::get_x_user_by_x_id(&db, &user.id)?;
        let uid = match existing {
            Some(xu) => xu.user_id,
            None => {
                // Check if there's an existing anky_user_id cookie to link
                let uid = jar
                    .get("anky_user_id")
                    .map(|c| c.value().to_string())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, Some(&user.id), &expires_at)?;
    }

    state.emit_log("INFO", "auth", &format!("X login: @{} ({})", user.username, &user_id[..8]));

    // Set cookies
    let session_cookie = Cookie::build(("anky_session", session_token))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(30))
        .build();

    let user_cookie = Cookie::build(("anky_user_id", user_id))
        .path("/")
        .http_only(false)
        .secure(true)
        .max_age(time::Duration::days(365))
        .build();

    let jar = jar.add(session_cookie).add(user_cookie);
    let redirect = redirect_to.as_deref().unwrap_or("/");

    Ok((jar, Redirect::temporary(redirect)))
}

/// GET /auth/x/logout — delete auth session and cookies
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(token) = jar.get("anky_session") {
        let db = state.db.lock().await;
        let _ = queries::delete_auth_session(&db, token.value());
    }

    let jar = jar
        .remove(Cookie::build("anky_session").path("/").build())
        .remove(Cookie::build("anky_user_id").path("/").build());

    Ok((jar, Redirect::temporary("/")))
}

/// Helper: extract auth context from cookies or Privy token for template rendering.
pub struct AuthUser {
    pub user_id: String,
    pub x_user_id: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub profile_image_url: Option<String>,
    pub wallet_address: Option<String>,
}

pub async fn get_auth_user(state: &AppState, jar: &CookieJar) -> Option<AuthUser> {
    // Try cookie-based session first
    if let Some(cookie) = jar.get("anky_session") {
        let token = cookie.value();
        let db = state.db.lock().await;
        if let Ok(Some((user_id, x_user_id))) = queries::get_auth_session(&db, token) {
            if let Some(ref xid) = x_user_id {
                if let Ok(Some(xu)) = queries::get_x_user_by_x_id(&db, xid) {
                    return Some(AuthUser {
                        user_id: xu.user_id,
                        x_user_id: Some(xu.x_user_id),
                        username: Some(xu.username),
                        display_name: xu.display_name,
                        profile_image_url: xu.profile_image_url,
                        wallet_address: None,
                    });
                }
            }
            return Some(AuthUser {
                user_id,
                x_user_id,
                username: None,
                display_name: None,
                profile_image_url: None,
                wallet_address: None,
            });
        }
    }

    // Try Privy token from anky_privy_token cookie (set by frontend after Privy login)
    if let Some(cookie) = jar.get("anky_privy_token") {
        let token = cookie.value();
        if let Some(user) = get_auth_user_from_privy_token(state, token).await {
            return Some(user);
        }
    }

    None
}

/// Extract auth user from Privy auth token by looking up the session we created during verify.
async fn get_auth_user_from_privy_token(state: &AppState, token: &str) -> Option<AuthUser> {
    let db = state.db.lock().await;
    // The token is our session token created during privy verify
    if let Ok(Some((user_id, _))) = queries::get_auth_session(&db, token) {
        let wallet = queries::get_user_wallet(&db, &user_id).ok().flatten();
        let username = queries::get_user_username(&db, &user_id).ok().flatten();
        return Some(AuthUser {
            user_id,
            x_user_id: None,
            username,
            display_name: None,
            profile_image_url: None,
            wallet_address: wallet,
        });
    }
    None
}

/// POST /auth/privy/verify — verify Privy auth token and create session.
/// Frontend calls this after successful Privy login.
/// Verifies JWT locally using ES256 public key, then looks up or creates user.
#[derive(serde::Deserialize)]
pub struct PrivyVerifyRequest {
    pub auth_token: String,
}

#[derive(serde::Deserialize)]
struct PrivyClaims {
    sub: String,        // Privy DID (e.g. "did:privy:...")
    iss: Option<String>,
    aud: Option<String>,
}

pub async fn privy_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<PrivyVerifyRequest>,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    let app_id = &state.config.privy_app_id;
    let app_secret = &state.config.privy_app_secret;
    let verification_key = &state.config.privy_verification_key;

    if app_id.is_empty() || app_secret.is_empty() {
        return Err(AppError::Internal("Privy not configured".into()));
    }

    // Step 1: Verify JWT locally using ES256 public key
    let privy_did = if !verification_key.is_empty() {
        let decoding_key = jsonwebtoken::DecodingKey::from_ec_pem(verification_key.as_bytes())
            .map_err(|e| AppError::Internal(format!("Invalid Privy verification key: {}", e)))?;

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
        validation.set_issuer(&["privy.io"]);
        validation.set_audience(&[app_id.as_str()]);

        let token_data = jsonwebtoken::decode::<PrivyClaims>(
            &req.auth_token,
            &decoding_key,
            &validation,
        )
        .map_err(|e| {
            tracing::warn!("Privy JWT verification failed: {}", e);
            AppError::BadRequest("invalid privy token".into())
        })?;

        token_data.claims.sub
    } else {
        // Fallback: verify via Privy API if no local key configured
        let client = reqwest::Client::new();
        let resp = client
            .post("https://auth.privy.io/api/v1/sessions/verify")
            .header("privy-app-id", app_id.as_str())
            .basic_auth(app_id, Some(app_secret))
            .json(&serde_json::json!({ "auth_token": req.auth_token }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Privy verification request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("Privy verify failed ({}): {}", status, body);
            return Err(AppError::BadRequest("invalid privy token".into()));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("Privy response parse failed: {}", e)))?;

        body.get("user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Internal("no user id in Privy response".into()))?
    };

    // Step 2: Check if we already know this Privy user (fast path — no API call)
    {
        let db = state.db.lock().await;
        if let Some(user_id) = queries::get_user_by_privy_did(&db, &privy_did)? {
            let wallet = queries::get_user_wallet(&db, &user_id)?;
            let session_token = uuid::Uuid::new_v4().to_string();
            let expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::days(30))
                .unwrap()
                .to_rfc3339();
            queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;

            let short = wallet.as_deref().map(|a| format!("{}...{}", &a[..6.min(a.len())], &a[a.len().saturating_sub(4)..])).unwrap_or_else(|| privy_did[..12.min(privy_did.len())].to_string());
            state.emit_log("INFO", "auth", &format!("Privy login (returning): {} ({})", short, &user_id[..8]));

            let session_cookie = Cookie::build(("anky_privy_token", session_token))
                .path("/").http_only(true).secure(true)
                .max_age(time::Duration::days(30)).build();
            let user_cookie = Cookie::build(("anky_user_id", user_id.clone()))
                .path("/").http_only(false).secure(true)
                .max_age(time::Duration::days(365)).build();
            let jar = jar.add(session_cookie).add(user_cookie);

            return Ok((jar, Json(serde_json::json!({
                "ok": true,
                "user_id": user_id,
                "wallet_address": wallet,
            }))));
        }
    }

    // Step 3: New user — call Privy API to get wallet address from linked_accounts
    let client = reqwest::Client::new();
    let encoded_did = urlencoding::encode(&privy_did);
    let resp = client
        .get(format!("https://auth.privy.io/api/v1/users/{}", encoded_did))
        .header("privy-app-id", app_id.as_str())
        .basic_auth(app_id, Some(app_secret))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Privy user fetch failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("Privy user fetch failed ({}): {}", status, body);
        return Err(AppError::Internal("failed to fetch Privy user details".into()));
    }

    let user_data: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Privy user parse failed: {}", e)))?;

    // Extract wallet address from linked_accounts
    let wallet_address = user_data
        .get("linked_accounts")
        .and_then(|la| la.as_array())
        .and_then(|accounts| {
            accounts.iter().find_map(|acc| {
                if acc.get("type").and_then(|t| t.as_str()) == Some("wallet") {
                    acc.get("address").and_then(|a| a.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| AppError::BadRequest("no wallet found in privy account".into()))?;

    // Step 4: Create user with wallet + privy_did
    let user_id = {
        let db = state.db.lock().await;
        // Check wallet first (user might exist from raw ethereum connect)
        if let Some(uid) = queries::get_user_by_wallet(&db, &wallet_address)? {
            // Link privy_did to existing user
            let _ = queries::set_privy_did(&db, &uid, &privy_did);
            uid
        } else {
            let uid = jar
                .get("anky_user_id")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            queries::create_user_with_wallet_and_privy(&db, &uid, &wallet_address, &privy_did)?;
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
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    let short_addr = format!("{}...{}", &wallet_address[..6], &wallet_address[wallet_address.len()-4..]);
    state.emit_log("INFO", "auth", &format!("Privy login (new): {} ({})", short_addr, &user_id[..8]));

    // Set cookies
    let session_cookie = Cookie::build(("anky_privy_token", session_token))
        .path("/").http_only(true).secure(true)
        .max_age(time::Duration::days(30)).build();
    let user_cookie = Cookie::build(("anky_user_id", user_id.clone()))
        .path("/").http_only(false).secure(true)
        .max_age(time::Duration::days(365)).build();
    let jar = jar.add(session_cookie).add(user_cookie);

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "user_id": user_id,
            "wallet_address": wallet_address,
        })),
    ))
}

/// POST /auth/privy/logout — clear Privy session cookies
pub async fn privy_logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(token) = jar.get("anky_privy_token") {
        let db = state.db.lock().await;
        let _ = queries::delete_auth_session(&db, token.value());
    }

    let jar = jar
        .remove(Cookie::build("anky_privy_token").path("/").build())
        .remove(Cookie::build("anky_user_id").path("/").build());

    Ok((jar, Redirect::temporary("/")))
}
