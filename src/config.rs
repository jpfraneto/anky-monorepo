use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub ollama_light_model: String,
    pub openrouter_api_key: String,
    pub openrouter_light_model: String,
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
    // OpenAI (embeddings for memory)
    pub openai_api_key: String,
    // Neynar (Farcaster)
    pub neynar_api_key: String,
    pub neynar_signer_uuid: String,
    pub neynar_webhook_secret: String,
    pub farcaster_bot_fid: u64,
    // xAI (Grok video generation)
    pub xai_api_key: String,
    // Cloudflare (cache purge)
    pub cloudflare_api_token: String,
    pub cloudflare_zone_id: String,
    // Training live monitor
    pub training_secret: String,
    // Dataset gallery password
    pub dataset_password: String,
    // ComfyUI (local Flux image generation)
    pub comfyui_url: String,
    // Honcho (user identity modeling)
    pub honcho_api_key: String,
    pub honcho_workspace_id: String,
    pub honcho_base_url: String,
    // TTS (F5-TTS local service)
    pub tts_base_url: String,
    // Cloudflare R2 (audio storage for Anky Voices)
    pub r2_account_id: String,
    pub r2_bucket_name: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_public_url: String,
    // Flux Image Generation API credentials
    pub flux_api_key: String,
    pub flux_secret_key: String,
    // Pinata (IPFS pinning for on-chain metadata)
    pub pinata_jwt: String,
    // Anky mint wallet (EIP-712 signer for birthSoul)
    pub anky_wallet_private_key: String,
    // APNs (push notifications)
    pub apns_key_path: String,
    pub apns_key_id: String,
    pub apns_team_id: String,
    pub apns_bundle_id: String,
    pub apns_environment: String, // "production" or "sandbox"
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
            ollama_model: std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:35b".into()),
            ollama_light_model: std::env::var("OLLAMA_LIGHT_MODEL").unwrap_or_else(|_| {
                std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:35b".into())
            }),
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
            openrouter_light_model: std::env::var("OPENROUTER_LIGHT_MODEL")
                .unwrap_or_else(|_| "meta-llama/llama-4-scout:free".into()),
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
            twitter_bot_bearer_token: std::env::var("X_BEARER_TOKEN").unwrap_or_default(),
            twitter_bot_api_key: std::env::var("X_CONSUMER_KEY").unwrap_or_default(),
            twitter_bot_api_secret: std::env::var("X_CONSUMER_SECRET").unwrap_or_default(),
            twitter_bot_access_token: std::env::var("X_ACCESS_TOKEN").unwrap_or_default(),
            twitter_bot_access_secret: std::env::var("X_ACCESS_TOKEN_SECRET").unwrap_or_default(),
            twitter_bot_user_id: std::env::var("TWITTER_BOT_USER_ID").unwrap_or_default(),
            privy_app_id: std::env::var("PRIVY_APP_ID").unwrap_or_default(),
            privy_app_secret: std::env::var("PRIVY_APP_SECRET").unwrap_or_default(),
            privy_verification_key: std::env::var("PRIVY_VERIFICATION_KEY")
                .unwrap_or_default()
                .replace("\\n", "\n"),
            pumpfun_rtmp_url: std::env::var("PUMPFUN_RTMP_URL").unwrap_or_default(),
            pumpfun_stream_key: std::env::var("PUMPFUN_STREAM_KEY").unwrap_or_default(),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            neynar_api_key: std::env::var("NEYNAR_API_KEY").unwrap_or_default(),
            neynar_signer_uuid: std::env::var("NEYNAR_SIGNER_UUID").unwrap_or_default(),
            neynar_webhook_secret: std::env::var("NEYNAR_WEBHOOK_SECRET").unwrap_or_default(),
            farcaster_bot_fid: std::env::var("FARCASTER_BOT_FID")
                .unwrap_or_else(|_| "0".into())
                .parse()
                .unwrap_or(0),
            xai_api_key: std::env::var("XAI_API_KEY").unwrap_or_default(),
            cloudflare_api_token: std::env::var("CLOUDFLARE_API_TOKEN").unwrap_or_default(),
            cloudflare_zone_id: std::env::var("CLOUDFLARE_ZONE_ID").unwrap_or_default(),
            training_secret: std::env::var("TRAINING_SECRET").unwrap_or_default(),
            dataset_password: std::env::var("DATASET_PASSWORD")
                .unwrap_or_else(|_| "ankyisyou".into()),
            comfyui_url: std::env::var("COMFYUI_URL")
                .unwrap_or_else(|_| "http://localhost:8188".into()),
            tts_base_url: std::env::var("TTS_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5001".into()),
            honcho_api_key: std::env::var("HONCHO_API_KEY").unwrap_or_default(),
            honcho_workspace_id: std::env::var("HONCHO_WORKSPACE_ID")
                .unwrap_or_else(|_| "anky-prod".into()),
            honcho_base_url: std::env::var("HONCHO_BASE_URL")
                .unwrap_or_else(|_| "https://api.honcho.dev/v3".into()),
            r2_account_id: std::env::var("R2_ACCOUNT_ID").unwrap_or_default(),
            r2_bucket_name: std::env::var("R2_BUCKET_NAME")
                .unwrap_or_else(|_| "anky-voices".into()),
            r2_access_key_id: std::env::var("R2_ACCESS_KEY_ID").unwrap_or_default(),
            r2_secret_access_key: std::env::var("R2_SECRET_ACCESS_KEY").unwrap_or_default(),
            r2_public_url: std::env::var("R2_PUBLIC_URL").unwrap_or_default(),
            // Flux Image Generation API credentials
            flux_api_key: std::env::var("FLUX_API_KEY").unwrap_or_default(),
            flux_secret_key: std::env::var("FLUX_SECRET_KEY").unwrap_or_default(),
            pinata_jwt: std::env::var("PINATA_JWT").unwrap_or_default(),
            apns_key_path: std::env::var("APNS_KEY_PATH").unwrap_or_default(),
            apns_key_id: std::env::var("APNS_KEY_ID").unwrap_or_default(),
            apns_team_id: std::env::var("APNS_TEAM_ID").unwrap_or_default(),
            apns_bundle_id: std::env::var("APNS_BUNDLE_ID").unwrap_or_default(),
            apns_environment: std::env::var("APNS_ENVIRONMENT")
                .unwrap_or_else(|_| "production".into()),
            anky_wallet_private_key: std::env::var("ANKY_WALLET_PRIVATE_KEY").unwrap_or_default(),
        })
    }
}
