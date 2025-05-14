import OpenAI from "openai";
import type { Env } from "../env/server-env";

// Factory function to create an OpenAI client with the provided API key
export function createOpenAIClient(env: Env) {
  return new OpenAI({
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: env.OPENROUTER_API_KEY,
    defaultHeaders: {
      "HTTP-Referer": "https://anky.bot", // Optional. Site URL for rankings on openrouter.ai.
      "X-Title": "anky", // Optional. Site title for rankings on openrouter.ai.
    },
  });
}
