use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteRequest {
    pub text: String,
    pub duration: f64,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub session_token: Option<String>,
    #[serde(default)]
    pub keystroke_deltas: Option<Vec<f64>>,
    #[serde(default)]
    pub inquiry_id: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteResponse {
    pub response: String,
    pub duration: f64,
    pub is_anky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anky_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_wait_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
    pub payment_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
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
    pub message: String,
}

// --- Meditation ---
#[derive(Debug, Serialize, Deserialize)]
pub struct StartMeditationResponse {
    pub session_id: String,
    pub duration: i32,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMeditationRequest {
    pub session_id: String,
    pub duration_actual: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteMeditationResponse {
    pub completed: bool,
    pub options: Vec<PostMeditationOption>,
    pub level: i32,
    pub total_completed: i32,
    pub streak: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostMeditationOption {
    pub id: String,
    pub label: String,
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlock_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProgressionInfo {
    pub level: i32,
    pub duration: i32,
    pub total_meditations: i32,
    pub total_completed: i32,
    pub write_unlocked: bool,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub next_level_at: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectResponse {
    pub interaction_id: String,
    pub question: String,
    pub answers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectAnswerRequest {
    pub interaction_id: String,
    pub answer_index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalPromptResponse {
    pub interaction_id: String,
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalSubmitRequest {
    pub interaction_id: String,
    pub entry: String,
}

// --- Interview System ---
#[derive(Debug, Serialize, Deserialize)]
pub struct InterviewStartRequest {
    pub id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default = "default_guest_name")]
    pub guest_name: String,
    #[serde(default = "default_true")]
    pub is_anonymous: bool,
}

fn default_guest_name() -> String {
    "guest".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InterviewMessageRequest {
    pub interview_id: String,
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InterviewEndRequest {
    pub interview_id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub message_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterviewSummary {
    pub id: String,
    pub guest_name: String,
    pub started_at: String,
    pub summary: Option<String>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInterviewContext {
    pub username: Option<String>,
    pub psychological_profile: Option<String>,
    pub core_tensions: Option<String>,
    pub growth_edges: Option<String>,
    pub recent_writings: Vec<String>,
    pub past_interviews: Vec<InterviewSummary>,
}
