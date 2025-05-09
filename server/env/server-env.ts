import dotenv from "dotenv";
import { z } from "zod";

dotenv.config();

const envSchema = z.object({
  OPENROUTER_API_KEY: z.string({
    required_error: "OPENROUTER_API_KEY is required",
  }),
  SUPABASE_PASSWORD: z.string({
    required_error: "SUPABASE_PASSWORD is required",
  }),
  SUPABASE_URL: z.string({
    required_error: "SUPABASE_URL is required",
  }),
  SUPABASE_API_KEY: z.string({
    required_error: "SUPABASE_API_KEY is required",
  }),
});
export const {
  OPENROUTER_API_KEY,
  SUPABASE_PASSWORD,
  SUPABASE_URL,
  SUPABASE_API_KEY,
} = envSchema.parse(process.env);
