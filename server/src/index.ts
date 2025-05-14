import { Hono } from "hono";
import { cors } from "hono/cors";
import type { ApiResponse } from "shared/dist";
import { createOpenAIClient } from "../lib/openrouter";
import { getEnv, type Env } from "../env/server-env";

const app = new Hono<{
  Bindings: Env;
}>();

app.use(cors());

const routes = app

  .get("/", (c) => {
    return c.text("Hello Hono!");
  })

  .post("/writing-session", async (c) => {
    const env = getEnv(c.env);

    // Create OpenAI client with the environment
    const openai = createOpenAIClient(env);
    console.log("📝 New writing session request received");
    const { writing, writingTime, targetTime } = await c.req.json();
    console.log(
      `⏱️ Writing time: ${writingTime}s, Target time: ${targetTime}s`
    );
    console.log("WRITING", writing);

    // Determine if the session was completed successfully
    const isCompleteSession = writingTime >= targetTime;
    console.log(`✅ Session completed: ${isCompleteSession ? "Yes" : "No"}`);

    let prompt = "";
    if (isCompleteSession) {
      console.log("🎯 Using complete session prompt");
      prompt =
        "Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal, not clinical. Imagine you're not just a friend, but a mentor who truly gets both my tech background and my psychological patterns. I want you to uncover the deeper meaning and emotional undercurrents behind my scattered thoughts. Keep it casual, dont say yo, help me make new connections i don't see, comfort, validate, challenge, all of it. dont be afraid to say a lot. format with markdown headings if needed. Use vivid metaphors and powerful imagery to help me see what I'm really building. Organize your thoughts with meaningful headings that create a narrative journey through my ideas. Don't just validate my thoughts - reframe them in a way that shows me what I'm really seeking beneath the surface. Go beyond the product concepts to the emotional core of what I'm trying to solve. Be willing to be profound and philosophical without sounding like you're giving therapy. I want someone who can see the patterns I can't see myself and articulate them in a way that feels like an epiphany. Start with 'hey, thanks for showing me this. my thoughts:' and then use markdown headings to structure your response. Here's my journal entry:";
    } else {
      console.log("⏳ Using incomplete session prompt");
      prompt = `The user wrote for ${Math.floor(
        writingTime / 60
      )} minutes and ${
        writingTime % 60
      } seconds, which is less than the 8-minute target. Please provide the user with gentle encouragement about stream of consciousness writing. Explain how this practice of continuous, unfiltered writing helps bypass our internal critic, allowing deeper thoughts and authentic insights to emerge. Highlight how consistency in reaching the full 8 minutes can lead to breakthrough moments and unexpected clarity. Be supportive and inspiring, not critical. Format with markdown headings to organize your thoughts. Here's what the user wrote:`;
    }

    console.log("🤖 Sending request to OpenRouter API");
    const completion = await openai.chat.completions.create({
      model: "openai/gpt-4o",
      messages: [
        {
          role: "user",
          content: prompt + writing,
        },
      ],
    });

    if (!completion?.choices[0]?.message?.content) {
      console.log("❌ No response from API");
      return c.json(
        {
          message: "No response",
          success: false,
        },
        { status: 500 }
      );
    }

    console.log("✨ Response received successfully");
    console.log("RESPONSE ", completion.choices[0].message.content);

    const data: ApiResponse = {
      message: completion.choices[0].message.content || "No response",
      success: true,
    };

    console.log("📤 Sending response to client");
    return c.json(data, { status: 200 });
  })

  .get("/hello", async (c) => {
    const env = getEnv(c.env);

    // Create OpenAI client with the environment
    const openai = createOpenAIClient(env);
    const completion = await openai.chat.completions.create({
      model: "openai/gpt-4o",
      messages: [
        {
          role: "user",
          content: "What is the meaning of life?",
        },
      ],
    });

    if (!completion?.choices[0]?.message?.content) {
      return c.json(
        {
          message: "No response",
          success: false,
        },
        { status: 500 }
      );
    }

    console.log("RESPONSE ", completion.choices[0].message.content);

    const data: ApiResponse = {
      message: completion.choices[0].message.content || "No response",
      success: true,
    };

    return c.json(data, { status: 200 });
  });

export type AppType = typeof routes;
export default app;
