use super::swift;
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

    state.emit_log(
        "INFO",
        "auth",
        &format!("X login: @{} ({})", user.username, &user_id[..8]),
    );

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
    pub email: Option<String>,
}

pub async fn get_auth_user(state: &AppState, jar: &CookieJar) -> Option<AuthUser> {
    // Try cookie-based session first
    if let Some(cookie) = jar.get("anky_session") {
        let token = cookie.value();
        let db = state.db.lock().await;
        if let Ok(Some((user_id, x_user_id))) = queries::get_auth_session(&db, token) {
            if let Some(ref xid) = x_user_id {
                if let Ok(Some(xu)) = queries::get_x_user_by_x_id(&db, xid) {
                    let email = queries::get_user_email(&db, &xu.user_id).ok().flatten();
                    return Some(AuthUser {
                        user_id: xu.user_id,
                        x_user_id: Some(xu.x_user_id),
                        username: Some(xu.username),
                        display_name: xu.display_name,
                        profile_image_url: xu.profile_image_url,
                        wallet_address: None,
                        email,
                    });
                }
            }
            let email = queries::get_user_email(&db, &user_id).ok().flatten();
            return Some(AuthUser {
                user_id,
                x_user_id,
                username: None,
                display_name: None,
                profile_image_url: None,
                wallet_address: None,
                email,
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
        let email = queries::get_user_email(&db, &user_id).ok().flatten();
        return Some(AuthUser {
            user_id,
            x_user_id: None,
            username,
            display_name: None,
            profile_image_url: None,
            wallet_address: wallet,
            email,
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
    sub: String, // Privy DID (e.g. "did:privy:...")
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

        let token_data =
            jsonwebtoken::decode::<PrivyClaims>(&req.auth_token, &decoding_key, &validation)
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

        let body: serde_json::Value = resp
            .json()
            .await
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
            let email = queries::get_user_email(&db, &user_id)?;
            let wallet = queries::get_user_wallet(&db, &user_id)?;
            let username = queries::get_user_username(&db, &user_id)?;
            let session_token = uuid::Uuid::new_v4().to_string();
            let expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::days(30))
                .unwrap()
                .to_rfc3339();
            queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;

            let display = username
                .as_deref()
                .or(email.as_deref())
                .unwrap_or(&privy_did[..12.min(privy_did.len())]);
            state.emit_log(
                "INFO",
                "auth",
                &format!("Privy login (returning): {} ({})", display, &user_id[..8]),
            );

            let session_cookie = Cookie::build(("anky_privy_token", session_token))
                .path("/")
                .http_only(true)
                .secure(true)
                .max_age(time::Duration::days(30))
                .build();
            let user_cookie = Cookie::build(("anky_user_id", user_id.clone()))
                .path("/")
                .http_only(false)
                .secure(true)
                .max_age(time::Duration::days(365))
                .build();
            let jar = jar.add(session_cookie).add(user_cookie);

            return Ok((
                jar,
                Json(serde_json::json!({
                    "ok": true,
                    "user_id": user_id,
                    "email": email,
                    "wallet_address": wallet,
                    "username": username,
                })),
            ));
        }
    }

    // Step 3: New user — call Privy API to get linked accounts (email, wallet, etc.)
    let client = reqwest::Client::new();
    let encoded_did = urlencoding::encode(&privy_did);
    let resp = client
        .get(format!(
            "https://auth.privy.io/api/v1/users/{}",
            encoded_did
        ))
        .header("privy-app-id", app_id.as_str())
        .basic_auth(app_id, Some(app_secret))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Privy user fetch failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("Privy user fetch failed ({}): {}", status, body);
        return Err(AppError::Internal(
            "failed to fetch Privy user details".into(),
        ));
    }

    let user_data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Privy user parse failed: {}", e)))?;

    // Extract email and wallet from linked_accounts (both optional)
    let linked_accounts = user_data
        .get("linked_accounts")
        .and_then(|la| la.as_array());

    let email_address = linked_accounts.and_then(|accounts| {
        accounts.iter().find_map(|acc| {
            let acct_type = acc.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if acct_type == "email" {
                acc.get("address")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
    });

    // Also check for Google/Apple email
    let email_address = email_address.or_else(|| {
        linked_accounts.and_then(|accounts| {
            accounts.iter().find_map(|acc| {
                let acct_type = acc.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if acct_type == "google_oauth" || acct_type == "apple_oauth" {
                    acc.get("email")
                        .and_then(|a| a.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
    });

    let wallet_address = linked_accounts.and_then(|accounts| {
        accounts.iter().find_map(|acc| {
            let acct_type = acc.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if acct_type == "wallet" {
                acc.get("address")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
    });

    // Step 4: Create or find user
    let user_id = {
        let db = state.db.lock().await;

        // Try to find existing user by email first
        let existing_uid = if let Some(ref email) = email_address {
            queries::get_user_by_email(&db, email)?
        } else {
            None
        };

        // Then try by wallet
        let existing_uid = existing_uid.or(if let Some(ref addr) = wallet_address {
            queries::get_user_by_wallet(&db, addr)?
        } else {
            None
        });

        if let Some(uid) = existing_uid {
            // Link privy_did to existing user
            let _ = queries::set_privy_did(&db, &uid, &privy_did);
            if let Some(ref email) = email_address {
                let _ = queries::set_email(&db, &uid, email);
            }
            if let Some(ref addr) = wallet_address {
                if queries::get_user_wallet(&db, &uid)?.is_none() {
                    let _ = queries::set_wallet_address(&db, &uid, addr);
                }
            }
            uid
        } else {
            let uid = jar
                .get("anky_user_id")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            if let Some(ref email) = email_address {
                queries::create_user_with_email_and_privy(&db, &uid, email, &privy_did)?;
            } else if let Some(ref addr) = wallet_address {
                queries::create_user_with_wallet_and_privy(&db, &uid, addr, &privy_did)?;
            } else {
                queries::ensure_user(&db, &uid)?;
                let _ = queries::set_privy_did(&db, &uid, &privy_did);
            }

            if let Some(ref addr) = wallet_address {
                if email_address.is_some() {
                    let _ = queries::set_wallet_address(&db, &uid, addr);
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
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

    let display = email_address
        .as_deref()
        .or(wallet_address.as_deref().map(|a| &a[..6.min(a.len())]))
        .unwrap_or(&privy_did[..12.min(privy_did.len())]);
    state.emit_log(
        "INFO",
        "auth",
        &format!("Privy login (new): {} ({})", display, &user_id[..8]),
    );

    // Set cookies
    let session_cookie = Cookie::build(("anky_privy_token", session_token))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(30))
        .build();
    let user_cookie = Cookie::build(("anky_user_id", user_id.clone()))
        .path("/")
        .http_only(false)
        .secure(true)
        .max_age(time::Duration::days(365))
        .build();
    let jar = jar.add(session_cookie).add(user_cookie);

    // Get final username
    let final_username = {
        let db = state.db.lock().await;
        queries::get_user_username(&db, &user_id)?
    };

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "user_id": user_id,
            "email": email_address,
            "wallet_address": wallet_address,
            "username": final_username,
        })),
    ))
}

/// POST /auth/farcaster/verify — authenticate via Farcaster MiniApp SDK context.
/// The FID from sdk.context is trusted (comes from Farcaster client's iframe postMessage protocol).
#[derive(serde::Deserialize)]
pub struct FarcasterVerifyRequest {
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
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
        let db = state.db.lock().await;
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
        let db = state.db.lock().await;
        queries::create_auth_session(&db, &session_token, &user_id, None, &expires_at)?;
    }

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
        let db = state.db.lock().await;
        (
            queries::get_user_wallet(&db, &user_id)?,
            queries::get_user_username(&db, &user_id)?,
        )
    };

    // Set cookies
    let session_cookie = Cookie::build(("anky_privy_token", session_token))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(30))
        .build();
    let user_cookie = Cookie::build(("anky_user_id", user_id.clone()))
        .path("/")
        .http_only(false)
        .secure(true)
        .max_age(time::Duration::days(365))
        .build();
    let jar = jar.add(session_cookie).add(user_cookie);

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

/// POST /auth/seed/verify — Web-side seed identity verification.
/// Same logic as /swift/v2/auth/verify but sets browser cookies.
pub async fn seed_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<swift::SeedAuthVerifyRequest>,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    let result = swift::auth_seed_verify_inner(&state, &req).await?;

    let session_cookie = Cookie::build(("anky_session", result.session_token.clone()))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::days(90))
        .build();
    let user_cookie = Cookie::build(("anky_user_id", result.user_id.clone()))
        .path("/")
        .http_only(false)
        .secure(true)
        .max_age(time::Duration::days(365))
        .build();
    let jar = jar.add(session_cookie).add(user_cookie);

    state.emit_log(
        "INFO",
        "auth",
        &format!(
            "Seed login (web): {} ({})",
            &result.user_id[..8],
            result.wallet_address.as_deref().unwrap_or("?")
        ),
    );

    Ok((
        jar,
        Json(serde_json::json!({
            "ok": true,
            "user_id": result.user_id,
            "wallet_address": result.wallet_address,
            "username": result.username,
            "session_token": result.session_token,
        })),
    ))
}

/// POST /auth/seed/logout — clear seed session cookies and invalidate session.
pub async fn seed_logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    if let Some(token) = jar.get("anky_session") {
        let db = state.db.lock().await;
        let _ = queries::delete_auth_session(&db, token.value());
    }

    let jar = jar
        .remove(Cookie::build("anky_session").path("/").build())
        .remove(Cookie::build("anky_user_id").path("/").build());

    Ok((jar, Json(serde_json::json!({ "ok": true }))))
}
