import { Logger } from "./lib/logger";

const logger = Logger("Config");

export const getConfig = () => ({
  identifier: "ANKY API",
  version: process.env.VERSION || "1.0.0",
  isProduction: process.env.NODE_ENV === "production",

  runtime: {
    port: parseInt(process.env.PORT || "3000", 10),
    host: process.env.HOST || "localhost",
  },

  db: {
    url: process.env.DATABASE_URL ? "✓ Connected" : "✗ Missing",
    supabaseUrl: process.env.SUPABASE_URL ? "✓ Connected" : "✗ Missing",
  },

  auth: {
    privyConfigured: !!(
      process.env.PRIVY_APP_ID && process.env.PRIVY_APP_SECRET
    ),
  },

  ai: {
    openai: process.env.OPENAI_API_KEY ? "✓ Configured" : "✗ Missing",
    anthropic: process.env.ANTHROPIC_API_KEY ? "✓ Configured" : "✗ Missing",
    gemini: process.env.GEMINI_API_KEY ? "✓ Configured" : "✗ Missing",
  },

  ipfs: {
    pinata: !!process.env.PINATA_JWT ? "✓ Configured" : "✗ Missing",
  },

  cors: {
    origins: [
      "http://localhost:5173",
      "http://localhost:3000",
      "http://localhost:4173",
      "https://miniapp.anky.app",
      "https://anky.app",
      "https://www.anky.app",
      "https://anky-v8.orbiter.website",
    ],
  },
});

export function printStartupBanner() {
  const config = getConfig();
  const c = {
    reset: "\x1b[0m",
    bright: "\x1b[1m",
    dim: "\x1b[2m",
    green: "\x1b[32m",
    yellow: "\x1b[33m",
    cyan: "\x1b[36m",
    magenta: "\x1b[35m",
    red: "\x1b[31m",
    gray: "\x1b[90m",
  };

  const check = (ok: boolean | string) => {
    const isOk = typeof ok === "string" ? ok.includes("✓") : ok;
    return isOk ? `${c.green}✓${c.reset}` : `${c.red}✗${c.reset}`;
  };

  console.log(`
${c.cyan}${c.bright}
     █████╗ ███╗   ██╗██╗  ██╗██╗   ██╗
    ██╔══██╗████╗  ██║██║ ██╔╝╚██╗ ██╔╝
    ███████║██╔██╗ ██║█████╔╝  ╚████╔╝
    ██╔══██║██║╚██╗██║██╔═██╗   ╚██╔╝
    ██║  ██║██║ ╚████║██║  ██╗   ██║
    ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ╚═╝
${c.reset}
${c.gray}════════════════════════════════════════════════════════════════════════${c.reset}
${c.bright}                          ANKY BACKEND API${c.reset}
${c.gray}════════════════════════════════════════════════════════════════════════${c.reset}

  ${c.bright}Version:${c.reset}     ${config.version}
  ${c.bright}Environment:${c.reset} ${config.isProduction ? `${c.yellow}PRODUCTION${c.reset}` : `${c.cyan}DEVELOPMENT${c.reset}`}
  ${c.bright}Runtime:${c.reset}     Bun + Hono

${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}
  ${c.bright}SERVICES${c.reset}
${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}

  ${check(config.auth.privyConfigured)} Privy Auth       ${config.auth.privyConfigured ? "Configured" : "Not configured"}
  ${check(config.db.supabaseUrl)} Supabase         ${config.db.supabaseUrl}
  ${check(config.db.url)} Database         ${config.db.url}

${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}
  ${c.bright}AI PROVIDERS${c.reset}
${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}

  ${check(config.ai.openai)} OpenAI           ${config.ai.openai}
  ${check(config.ai.anthropic)} Anthropic        ${config.ai.anthropic}
  ${check(config.ai.gemini)} Gemini           ${config.ai.gemini}
  ${check(config.ipfs.pinata)} Pinata IPFS      ${config.ipfs.pinata}

${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}
  ${c.bright}ENDPOINTS${c.reset}
${c.gray}────────────────────────────────────────────────────────────────────────${c.reset}

  ${c.green}PUBLIC${c.reset}
    GET  /api/                    Health check
    GET  /api/db/status           Database status
    GET  /api/ankys               Public gallery feed
    GET  /api/feed                Public anky feed
    GET  /api/s/:shareId          Get shared session

  ${c.cyan}AI GENERATION${c.reset}
    POST /api/prompt              Generate image prompt
    POST /api/reflection          Generate reflection
    POST /api/image               Generate image
    POST /api/title               Generate title
    POST /api/chat                AI conversation

  ${c.magenta}USER (Privy Auth)${c.reset}
    GET  /api/me                  Current user info
    POST /api/sessions            Create writing session
    POST /api/ankys               Create anky
    GET  /api/users/:id/sessions  User's sessions
    GET  /api/users/:id/ankys     User's ankys

${c.gray}════════════════════════════════════════════════════════════════════════${c.reset}

  ${c.green}${c.bright}🚀 Server running at http://${config.runtime.host}:${config.runtime.port}${c.reset}

${c.gray}════════════════════════════════════════════════════════════════════════${c.reset}
`);
}

export default getConfig;
