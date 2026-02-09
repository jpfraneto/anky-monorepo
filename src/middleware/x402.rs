use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde_json::json;

const GENERATE_PRICE_USD: &str = "0.10";
const USDC_DECIMALS: u32 = 6;

/// Build a 402 Payment Required response with x402-compatible headers.
/// The PAYMENT-REQUIRED header contains a base64-encoded JSON payload
/// describing how to pay.
pub fn payment_required_response(treasury: &str, resource_url: &str) -> Response {
    let amount_minor = 100_000u64; // $0.10 in USDC (6 decimals)

    let payload = json!({
        "x402Version": 1,
        "accepts": [{
            "scheme": "exact",
            "network": "base",
            "maxAmountRequired": amount_minor.to_string(),
            "resource": resource_url,
            "description": format!("Generate an anky ({})", GENERATE_PRICE_USD),
            "mimeType": "application/json",
            "payTo": treasury,
            "requiredDeadlineSeconds": 300,
            "outputSchema": serde_json::Value::Null,
            "extra": {
                "name": "USDC",
                "decimals": USDC_DECIMALS,
                "token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
            }
        }]
    });

    let encoded = base64::engine::general_purpose::STANDARD.encode(payload.to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        "payment-required",
        HeaderValue::from_str(&encoded).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    (StatusCode::PAYMENT_REQUIRED, headers, "Payment Required").into_response()
}

/// Verify a payment signature by forwarding it to the Coinbase x402 facilitator.
/// Returns Ok(tx_hash) on success, Err(reason) on failure.
pub async fn verify_x402_payment(
    facilitator_url: &str,
    payment_header: &str,
    resource_url: &str,
) -> Result<String, String> {
    let body = json!({
        "x402Version": 1,
        "paymentPayload": payment_header,
        "resource": resource_url,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/verify", facilitator_url.trim_end_matches('/')))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("facilitator request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("facilitator returned {status}: {text}"));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("invalid facilitator response: {e}"))?;

    if result.get("valid").and_then(|v| v.as_bool()) == Some(true) {
        let tx_hash = result
            .get("txHash")
            .or_else(|| result.get("transaction_hash"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(tx_hash)
    } else {
        let reason = result
            .get("error")
            .or_else(|| result.get("reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("payment invalid")
            .to_string();
        Err(reason)
    }
}
