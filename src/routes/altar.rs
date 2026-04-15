use crate::error::AppError;
use crate::services::payment::verify_altar_burn;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct AltarResponse {
    image_url: String,
    treasury_address: String,
    usdc_token_address: String,
    network: String,
    total_burned_usdc: i64,
    total_burns: i64,
    top_burners: Vec<TopBurner>,
    recent_burns: Vec<RecentBurn>,
    stripe_publishable_key: String,
}

#[derive(Serialize)]
struct TopBurner {
    user_identifier: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    total_usdc: i64,
    burn_count: i64,
}

#[derive(Serialize)]
struct RecentBurn {
    display_name: Option<String>,
    amount_usdc: i64,
    created_at: String,
}

#[derive(Deserialize)]
pub struct BurnRequest {
    tx_hash: String,
    user_identifier: String,
    #[serde(default = "default_identifier_type")]
    identifier_type: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    fid: Option<i64>,
}

fn default_identifier_type() -> String {
    "wallet".to_string()
}

fn build_altar_response(state: &AppState) -> Result<AltarResponse, AppError> {
    let db = crate::db::conn(&state.db)?;

    let (total_burned, total_burns) = db
        .query_row(
            "SELECT COALESCE(SUM(amount_usdc), 0), COUNT(*) FROM altar_burns",
            crate::params![],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap_or((0, 0));

    let top_burners = {
        let mut stmt = db.prepare(
            "SELECT user_identifier, MAX(display_name) as display_name, MAX(avatar_url) as avatar_url,
                    SUM(amount_usdc) as total_usdc, COUNT(*) as burn_count
             FROM altar_burns
             GROUP BY user_identifier
             ORDER BY total_usdc DESC
             LIMIT 10",
        )?;
        let rows = stmt.query_map(crate::params![], |row| {
            Ok(TopBurner {
                user_identifier: row.get::<_, String>(0)?,
                display_name: row.get::<_, Option<String>>(1)?,
                avatar_url: row.get::<_, Option<String>>(2)?,
                total_usdc: row.get::<_, i64>(3)?,
                burn_count: row.get::<_, i64>(4)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    let recent_burns = {
        let mut stmt = db.prepare(
            "SELECT display_name, amount_usdc, created_at
             FROM altar_burns
             ORDER BY created_at DESC
             LIMIT 10",
        )?;
        let rows = stmt.query_map(crate::params![], |row| {
            Ok(RecentBurn {
                display_name: row.get::<_, Option<String>>(0)?,
                amount_usdc: row.get::<_, i64>(1)?,
                created_at: row.get::<_, String>(2)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(AltarResponse {
        image_url: "/image.png".to_string(),
        treasury_address: state.config.treasury_address.clone(),
        usdc_token_address: state.config.usdc_address.clone(),
        network: "base".to_string(),
        total_burned_usdc: total_burned,
        total_burns,
        top_burners,
        recent_burns,
        stripe_publishable_key: state.config.stripe_publishable_key.clone(),
    })
}

pub async fn get_altar(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let resp = build_altar_response(&state)?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

pub async fn verify_burn(
    State(state): State<AppState>,
    Json(body): Json<BurnRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.tx_hash.is_empty() || body.user_identifier.is_empty() {
        return Err(AppError::BadRequest(
            "tx_hash and user_identifier required".into(),
        ));
    }

    {
        let db = crate::db::conn(&state.db)?;
        let exists: bool = db
            .query_row(
                "SELECT COUNT(*) > 0 FROM altar_burns WHERE tx_hash = ?1",
                crate::params![body.tx_hash],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if exists {
            return Ok((
                StatusCode::CONFLICT,
                Json(json!({"error": "transaction already recorded"})),
            ));
        }
    }

    let result = verify_altar_burn(
        &state.config.base_rpc_url,
        &body.tx_hash,
        &state.config.treasury_address,
        &state.config.usdc_address,
    )
    .await
    .map_err(|e| AppError::Internal(format!("verification failed: {}", e)))?;

    if !result.valid {
        return Err(AppError::BadRequest(
            result
                .reason
                .unwrap_or_else(|| "invalid transaction".into()),
        ));
    }

    let amount: i64 = result
        .actual_amount
        .as_deref()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);

    {
        let db = crate::db::conn(&state.db)?;
        let insert_result = db.execute(
            "INSERT INTO altar_burns (user_identifier, identifier_type, amount_usdc, tx_hash, display_name, avatar_url, fid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            crate::params![
                body.user_identifier,
                body.identifier_type,
                amount,
                body.tx_hash,
                body.display_name,
                body.avatar_url,
                body.fid
            ],
        );

        if let Err(e) = insert_result {
            let err_str = e.to_string();
            if err_str.contains("unique")
                || err_str.contains("UNIQUE")
                || err_str.contains("duplicate")
            {
                return Ok((
                    StatusCode::CONFLICT,
                    Json(json!({"error": "transaction already recorded"})),
                ));
            }
            return Err(AppError::Internal(format!("insert failed: {}", err_str)));
        }
    }

    let resp = build_altar_response(&state)?;
    Ok((StatusCode::OK, Json(serde_json::to_value(resp).unwrap())))
}

// ── Stripe Checkout ──

#[derive(Deserialize)]
pub struct CheckoutRequest {
    amount_cents: i64, // USD cents (e.g. 500 = $5.00)
    display_name: Option<String>,
}

/// POST /api/altar/checkout — create a Stripe Checkout Session
pub async fn create_checkout(
    State(state): State<AppState>,
    Json(body): Json<CheckoutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.amount_cents < 100 {
        return Err(AppError::BadRequest("minimum $1.00".into()));
    }
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::Unavailable("payments not configured".into()));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(&state.config.stripe_secret_key, Option::<&str>::None)
        .form(&[
            ("mode", "payment"),
            (
                "success_url",
                "https://anky.app/altar?burn=success&session_id={CHECKOUT_SESSION_ID}",
            ),
            ("cancel_url", "https://anky.app/altar"),
            ("line_items[0][price_data][currency]", "usd"),
            (
                "line_items[0][price_data][product_data][name]",
                "offering to the anky altar",
            ),
            (
                "line_items[0][price_data][unit_amount]",
                &body.amount_cents.to_string(),
            ),
            ("line_items[0][quantity]", "1"),
            (
                "metadata[display_name]",
                body.display_name.as_deref().unwrap_or("anon"),
            ),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("stripe request failed: {}", e)))?;

    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("stripe parse failed: {}", e)))?;

    if !status.is_success() {
        let msg = data["error"]["message"].as_str().unwrap_or("stripe error");
        return Err(AppError::Internal(format!("stripe: {}", msg)));
    }

    let url = data["url"].as_str().unwrap_or("");
    let session_id = data["id"].as_str().unwrap_or("");

    Ok(Json(json!({
        "checkout_url": url,
        "session_id": session_id,
    })))
}

#[derive(Deserialize)]
pub struct StripeVerifyQuery {
    session_id: String,
}

/// GET /api/altar/stripe-success?session_id=cs_xxx — verify completed Stripe session and record burn
pub async fn stripe_success(
    State(state): State<AppState>,
    Query(q): Query<StripeVerifyQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::Unavailable("payments not configured".into()));
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.stripe.com/v1/checkout/sessions/{}",
        q.session_id
    );
    let resp = client
        .get(&url)
        .basic_auth(&state.config.stripe_secret_key, Option::<&str>::None)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("stripe request failed: {}", e)))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("stripe parse failed: {}", e)))?;

    let payment_status = data["payment_status"].as_str().unwrap_or("");
    if payment_status != "paid" {
        return Err(AppError::BadRequest("payment not completed".into()));
    }

    let amount_total = data["amount_total"].as_i64().unwrap_or(0); // cents
    let amount_usdc = amount_total * 10000; // cents → USDC micro (1 cent = 10000 micro USDC)
    let display_name = data["metadata"]["display_name"].as_str().unwrap_or("anon");
    let stripe_session_id = data["id"].as_str().unwrap_or(&q.session_id);

    // Insert burn (use stripe session ID as tx_hash for uniqueness)
    {
        let db = crate::db::conn(&state.db)?;
        let insert_result = db.execute(
            "INSERT INTO altar_burns (user_identifier, identifier_type, amount_usdc, tx_hash, display_name)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            crate::params![
                format!("stripe:{}", stripe_session_id),
                "stripe",
                amount_usdc,
                format!("stripe:{}", stripe_session_id),
                display_name.to_string()
            ],
        );

        if let Err(e) = insert_result {
            let err_str = e.to_string();
            if err_str.contains("unique")
                || err_str.contains("UNIQUE")
                || err_str.contains("duplicate")
            {
                // Already recorded — that's fine, just return current state
            } else {
                return Err(AppError::Internal(format!("insert failed: {}", err_str)));
            }
        }
    }

    let stripe_resp = build_altar_response(&state)?;
    Ok(Json(serde_json::to_value(stripe_resp).unwrap()))
}

// ── Apple Pay (iOS app) ──

#[derive(Deserialize)]
pub struct CreatePaymentIntentRequest {
    amount_cents: i64,
}

/// POST /api/altar/payment-intent — create a Stripe PaymentIntent for Apple Pay
/// iOS app calls this to get client_secret before presenting Apple Pay sheet.
pub async fn create_payment_intent(
    State(state): State<AppState>,
    Json(body): Json<CreatePaymentIntentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.amount_cents < 100 {
        return Err(AppError::BadRequest("minimum $1.00".into()));
    }
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::Unavailable("payments not configured".into()));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.stripe.com/v1/payment_intents")
        .basic_auth(&state.config.stripe_secret_key, Option::<&str>::None)
        .form(&[
            ("amount", body.amount_cents.to_string()),
            ("currency", "usd".to_string()),
            ("payment_method_types[]", "card".to_string()),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("stripe request failed: {}", e)))?;

    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("stripe parse failed: {}", e)))?;

    if !status.is_success() {
        let msg = data["error"]["message"].as_str().unwrap_or("stripe error");
        return Err(AppError::Internal(format!("stripe: {}", msg)));
    }

    Ok(Json(json!({
        "client_secret": data["client_secret"],
        "payment_intent_id": data["id"],
    })))
}

#[derive(Deserialize)]
pub struct ApplePayRequest {
    payment_intent_id: String,
    solana_address: String,
    display_name: Option<String>,
}

/// POST /api/altar/apple-pay — verify a Stripe PaymentIntent from Apple Pay and record burn
pub async fn apple_pay_burn(
    State(state): State<AppState>,
    Json(body): Json<ApplePayRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::Unavailable("payments not configured".into()));
    }
    if body.payment_intent_id.is_empty() || body.solana_address.is_empty() {
        return Err(AppError::BadRequest(
            "payment_intent_id and solana_address required".into(),
        ));
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.stripe.com/v1/payment_intents/{}",
        body.payment_intent_id
    );
    let resp = client
        .get(&url)
        .basic_auth(&state.config.stripe_secret_key, Option::<&str>::None)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("stripe request failed: {}", e)))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("stripe parse failed: {}", e)))?;

    let pi_status = data["status"].as_str().unwrap_or("");
    if pi_status != "succeeded" {
        return Err(AppError::BadRequest(format!(
            "payment not succeeded (status: {})",
            pi_status
        )));
    }

    let amount_cents = data["amount"].as_i64().unwrap_or(0);
    let amount_usdc = amount_cents * 10000; // cents -> USDC micro units

    let tx_key = format!("apple:{}", body.payment_intent_id);
    let display_name = body.display_name.as_deref().unwrap_or("anon");

    {
        let db = crate::db::conn(&state.db)?;
        let insert_result = db.execute(
            "INSERT INTO altar_burns (user_identifier, identifier_type, amount_usdc, tx_hash, display_name)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            crate::params![
                body.solana_address,
                "apple",
                amount_usdc,
                tx_key,
                display_name.to_string()
            ],
        );

        if let Err(e) = insert_result {
            let err_str = e.to_string();
            if err_str.contains("unique")
                || err_str.contains("UNIQUE")
                || err_str.contains("duplicate")
            {
                // Already recorded
            } else {
                return Err(AppError::Internal(format!("insert failed: {}", err_str)));
            }
        }
    }

    let apple_resp = build_altar_response(&state)?;
    Ok(Json(serde_json::to_value(apple_resp).unwrap()))
}
