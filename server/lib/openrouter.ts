import OpenAI from "openai";
import { OPENROUTER_API_KEY } from "../env/server-env";

const openai = new OpenAI({
  baseURL: "https://openrouter.ai/api/v1",
  apiKey: OPENROUTER_API_KEY,
  defaultHeaders: {
    "HTTP-Referer": "https://anky.bot", // Optional. Site URL for rankings on openrouter.ai.
    "X-Title": "anky", // Optional. Site title for rankings on openrouter.ai.
  },
});

export default openai;
