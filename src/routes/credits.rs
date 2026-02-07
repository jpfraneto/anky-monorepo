use axum::extract::State;
use axum::response::{Html, IntoResponse, Json};
use serde_json::json;

use crate::db::queries;
use crate::error::AppError;
use crate::models::{CreateKeyRequest, CreateKeyResponse, CreditPaymentRequest, CreditPaymentResponse};
use crate::state::AppState;

fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let hex_chars: Vec<u8> = (0..16).map(|_| rng.gen::<u8>()).collect();
    format!("anky_{}", hex::encode(hex_chars))
}

pub async fn credits_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("treasury_address", &state.config.treasury_address);
    let html = state.tera.render("credits.html", &ctx)?;
    Ok(Html(html))
}

pub async fn create_key(
    State(state): State<AppState>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let key = generate_api_key();
    let db = state.db.lock().await;
    queries::create_api_key(&db, &key, req.label.as_deref())?;
    Ok(Json(CreateKeyResponse {
        key: key.clone(),
        message: "API key created. fund it with USDC on Base to start transforming.".into(),
    }))
}

pub async fn verify_credit_payment(
    State(state): State<AppState>,
    Json(req): Json<CreditPaymentRequest>,
) -> Result<Json<CreditPaymentResponse>, AppError> {
    // Validate API key exists
    let db = state.db.lock().await;
    let key_record = queries::get_api_key(&db, &req.api_key)?
        .ok_or_else(|| AppError::BadRequest("invalid API key".into()))?;

    if !key_record.is_active {
        return Err(AppError::BadRequest("API key is deactivated".into()));
    }

    // Check tx_hash not already used
    if queries::check_tx_hash_used(&db, &req.tx_hash)? {
        return Err(AppError::BadRequest("transaction already used".into()));
    }

    // Verify the USDC payment on Base chain
    let verified_amount = verify_usdc_transfer(
        &state.config.base_rpc_url,
        &req.tx_hash,
        &state.config.usdc_address,
        &state.config.treasury_address,
    )
    .await?;

    if verified_amount < 0.01 {
        return Err(AppError::BadRequest("payment not found or too small".into()));
    }

    // Credit 1:1 USDC to USD
    let credited = verified_amount;
    let purchase_id = uuid::Uuid::new_v4().to_string();
    queries::insert_credit_purchase(&db, &purchase_id, &req.api_key, &req.tx_hash, verified_amount, credited)?;
    queries::add_balance(&db, &req.api_key, credited)?;

    let new_key = queries::get_api_key(&db, &req.api_key)?
        .ok_or_else(|| AppError::Internal("key vanished".into()))?;

    Ok(Json(CreditPaymentResponse {
        credited,
        new_balance: new_key.balance_usd,
    }))
}

pub async fn usage_stats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let api_key = params
        .get("key")
        .ok_or_else(|| AppError::BadRequest("missing key parameter".into()))?;

    let db = state.db.lock().await;
    let key_record = queries::get_api_key(&db, api_key)?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    let recent = queries::get_recent_transformations(&db, api_key, 20)?;
    let transforms: Vec<serde_json::Value> = recent
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "input_tokens": t.input_tokens,
                "output_tokens": t.output_tokens,
                "cost_usd": t.cost_usd,
                "created_at": t.created_at,
            })
        })
        .collect();

    Ok(Json(json!({
        "balance_usd": key_record.balance_usd,
        "total_spent_usd": key_record.total_spent_usd,
        "total_transforms": key_record.total_transforms,
        "recent_transforms": transforms,
    })))
}

/// Verify a USDC transfer on Base chain by reading the transaction receipt
/// and checking Transfer events to our treasury.
async fn verify_usdc_transfer(
    rpc_url: &str,
    tx_hash: &str,
    usdc_address: &str,
    treasury_address: &str,
) -> Result<f64, AppError> {
    let client = reqwest::Client::new();

    // Get transaction receipt
    let resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("RPC error: {}", e)))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("RPC parse error: {}", e)))?;

    let receipt = data["result"]
        .as_object()
        .ok_or_else(|| AppError::BadRequest("transaction not found".into()))?;

    // Check status (0x1 = success)
    let status = receipt
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("0x0");
    if status != "0x1" {
        return Err(AppError::BadRequest("transaction failed".into()));
    }

    // Look for ERC20 Transfer event to treasury
    // Transfer(address,address,uint256) topic: 0xddf252ad...
    let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    let treasury_padded = format!("0x000000000000000000000000{}", &treasury_address[2..].to_lowercase());
    let usdc_lower = usdc_address.to_lowercase();

    let logs = receipt.get("logs").and_then(|l| l.as_array());
    if let Some(logs) = logs {
        for log in logs {
            let address = log["address"].as_str().unwrap_or("").to_lowercase();
            let topics = log["topics"].as_array();

            if address == usdc_lower {
                if let Some(topics) = topics {
                    if topics.len() >= 3
                        && topics[0].as_str() == Some(transfer_topic)
                        && topics[2].as_str().map(|s| s.to_lowercase()) == Some(treasury_padded.clone())
                    {
                        // Parse amount from data (USDC has 6 decimals)
                        let data_hex = log["data"].as_str().unwrap_or("0x0");
                        let amount_raw = u128::from_str_radix(data_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                        let amount_usdc = amount_raw as f64 / 1_000_000.0;
                        return Ok(amount_usdc);
                    }
                }
            }
        }
    }

    Ok(0.0) // No matching transfer found
}
