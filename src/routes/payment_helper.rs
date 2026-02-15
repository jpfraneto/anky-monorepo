use crate::db::queries;
use crate::middleware::api_auth::ApiKeyInfo;
use crate::middleware::x402;
use crate::state::AppState;
use axum::http::HeaderMap;

pub struct PaymentResult {
    pub method: String,
    pub tx_hash: Option<String>,
    pub api_key: Option<String>,
    pub agent_id: Option<String>,
}

/// Validate payment from headers or API key info.
/// Returns Ok(PaymentResult) on success, Err(PaymentError) on failure.
pub async fn validate_payment(
    state: &AppState,
    headers: &HeaderMap,
    api_key_info: &Option<axum::Extension<ApiKeyInfo>>,
) -> Result<PaymentResult, PaymentError> {
    let mut payment_method = String::new();
    let mut tx_hash: Option<String> = None;
    let mut api_key_str: Option<String> = None;
    let mut agent_id: Option<String> = None;

    if let Some(axum::Extension(ref key_info)) = api_key_info {
        api_key_str = Some(key_info.key.clone());

        // Check if this is an agent with free sessions
        let db = state.db.lock().await;
        if let Ok(Some(agent)) = queries::get_agent_by_key(&db, &key_info.key) {
            if agent.free_sessions_remaining > 0 {
                queries::decrement_free_session(&db, &agent.id)?;
                payment_method = "free_session".into();
                agent_id = Some(agent.id);
                drop(db);
            } else {
                drop(db);
            }
        } else {
            drop(db);
        }
    }

    // If no API key payment, check for payment header
    if payment_method.is_empty() {
        if let Some(sig) = headers
            .get("payment-signature")
            .or_else(|| headers.get("x-payment"))
            .and_then(|v| v.to_str().ok())
        {
            let sig = sig.trim();
            if sig.starts_with("0x") && sig.len() == 66 && sig[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                state.emit_log("INFO", "payment", &format!("Direct wallet payment: {}", sig));
                tx_hash = Some(sig.to_string());
                payment_method = "wallet".into();
            } else {
                let facilitator = &state.config.x402_facilitator_url;
                if facilitator.is_empty() {
                    return Err(PaymentError::ConfigError("x402 facilitator not configured".into()));
                }
                match x402::verify_x402_payment(facilitator, sig, "https://anky.app/api/v1/generate").await {
                    Ok(hash) => {
                        tx_hash = Some(hash);
                        payment_method = "x402".into();
                    }
                    Err(reason) => {
                        return Err(PaymentError::VerificationFailed(reason));
                    }
                }
            }
        }
    }

    // No payment at all
    if payment_method.is_empty() {
        return Err(PaymentError::Required);
    }

    Ok(PaymentResult {
        method: payment_method,
        tx_hash,
        api_key: api_key_str,
        agent_id,
    })
}

#[derive(Debug)]
pub enum PaymentError {
    Required,
    VerificationFailed(String),
    ConfigError(String),
    DbError(anyhow::Error),
}

impl From<anyhow::Error> for PaymentError {
    fn from(e: anyhow::Error) -> Self {
        PaymentError::DbError(e)
    }
}
