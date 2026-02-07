use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteRequest {
    pub text: String,
    pub duration: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteResponse {
    pub response: String,
    pub duration: f64,
    pub is_anky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionCreateRequest {
    pub mega_prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentVerifyRequest {
    pub tx_hash: String,
    pub collection_id: String,
    pub expected_amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentVerifyResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotifySignupRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_chat_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub gpu_status: String,
    pub total_cost_usd: f64,
    pub uptime_seconds: u64,
}

// --- Extension API ---
#[derive(Debug, Serialize, Deserialize)]
pub struct TransformRequest {
    pub writing: String,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransformResponse {
    pub transformed: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub balance_remaining: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub balance_usd: f64,
    pub total_spent_usd: f64,
    pub total_transforms: i32,
    pub recent_transforms: Vec<TransformSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransformSummary {
    pub id: String,
    pub cost_usd: f64,
    pub created_at: String,
}

// --- Credits ---
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateKeyRequest {
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditPaymentRequest {
    pub api_key: String,
    pub tx_hash: String,
    pub amount_usdc: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreditPaymentResponse {
    pub credited: f64,
    pub new_balance: f64,
}

// --- Agent Registration ---
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub agent_id: String,
    pub api_key: String,
    pub free_sessions_remaining: i32,
    pub message: String,
}
