use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub anthropic_api_key: String,
    pub gemini_api_key: String,
    pub base_rpc_url: String,
    pub usdc_address: String,
    pub treasury_address: String,
    pub x402_facilitator_url: String,
    // X OAuth (user login)
    pub twitter_client_id: String,
    pub twitter_client_secret: String,
    pub twitter_callback_url: String,
    // X Bot (app-level credentials)
    pub twitter_bot_bearer_token: String,
    pub twitter_bot_api_key: String,
    pub twitter_bot_api_secret: String,
    pub twitter_bot_access_token: String,
    pub twitter_bot_access_secret: String,
    pub twitter_bot_user_id: String,
    // Privy (wallet auth)
    pub privy_app_id: String,
    pub privy_app_secret: String,
    pub privy_verification_key: String,
    // Livestream (pump.fun)
    pub pumpfun_rtmp_url: String,
    pub pumpfun_stream_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8889".into())
                .parse()
                .context("PORT must be a number")?,
            ollama_base_url: std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            ollama_model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "qwen2.5:72b".into()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            gemini_api_key: std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            base_rpc_url: std::env::var("BASE_RPC_URL")
                .unwrap_or_else(|_| "https://mainnet.base.org".into()),
            usdc_address: std::env::var("USDC_ADDRESS")
                .unwrap_or_else(|_| "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into()),
            treasury_address: std::env::var("TREASURY_ADDRESS").unwrap_or_default(),
            x402_facilitator_url: std::env::var("X402_FACILITATOR_URL")
                .unwrap_or_else(|_| "https://x402.org/facilitator".into()),
            twitter_client_id: std::env::var("TWITTER_CLIENT_ID").unwrap_or_default(),
            twitter_client_secret: std::env::var("TWITTER_CLIENT_SECRET").unwrap_or_default(),
            twitter_callback_url: std::env::var("TWITTER_CALLBACK_URL")
                .unwrap_or_else(|_| "https://anky.app/auth/x/callback".into()),
            twitter_bot_bearer_token: std::env::var("TWITTER_BOT_BEARER_TOKEN").unwrap_or_default(),
            twitter_bot_api_key: std::env::var("TWITTER_BOT_API_KEY").unwrap_or_default(),
            twitter_bot_api_secret: std::env::var("TWITTER_BOT_API_SECRET").unwrap_or_default(),
            twitter_bot_access_token: std::env::var("TWITTER_BOT_ACCESS_TOKEN").unwrap_or_default(),
            twitter_bot_access_secret: std::env::var("TWITTER_BOT_ACCESS_SECRET").unwrap_or_default(),
            twitter_bot_user_id: std::env::var("TWITTER_BOT_USER_ID").unwrap_or_default(),
            privy_app_id: std::env::var("PRIVY_APP_ID").unwrap_or_default(),
            privy_app_secret: std::env::var("PRIVY_APP_SECRET").unwrap_or_default(),
            privy_verification_key: std::env::var("PRIVY_VERIFICATION_KEY")
                .unwrap_or_default()
                .replace("\\n", "\n"),
            pumpfun_rtmp_url: std::env::var("PUMPFUN_RTMP_URL").unwrap_or_default(),
            pumpfun_stream_key: std::env::var("PUMPFUN_STREAM_KEY").unwrap_or_default(),
        })
    }
}
