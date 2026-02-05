// Agent payment constants
export const AGENT_FREE_SESSIONS = 4;
export const USD_PER_SESSION = 0.333;
export const ANKY_TOKENS_PER_SESSION = 100;

// Minimum writing duration for an Anky (8 minutes)
export const MIN_ANKY_DURATION_SECONDS = 480;

// Base chain config
export const BASE_CHAIN_ID = 8453;

// Token addresses on Base
export const USDC_ADDRESS = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" as const;
export const ANKY_TOKEN_ADDRESS = (process.env.ANKY_TOKEN_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`;

// Treasury address
export const TREASURY_ADDRESS = (process.env.TREASURY_ADDRESS || "0x0000000000000000000000000000000000000000") as `0x${string}`;

// Human payment (Polar.sh)
export const HUMAN_SUBSCRIPTION_PRICE_USD = 9;
export const HUMAN_SUBSCRIPTION_DURATION_DAYS = 30;

// Rate limits
export const RATE_LIMITS = {
  agentRegister: { max: 5, windowMs: 60 * 60 * 1000 }, // 5 per hour
  sessionSubmit: { max: 20, windowMs: 60 * 60 * 1000 }, // 20 per hour
  publicRead: { max: 100, windowMs: 60 * 1000 }, // 100 per minute
} as const;
