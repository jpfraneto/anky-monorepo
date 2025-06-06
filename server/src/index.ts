import { Hono } from "hono";
import { cors } from "hono/cors";
import type { ApiResponse } from "shared/dist";
import { createOpenAIClient } from "../lib/openrouter";
import { getEnv, type Env } from "../env/server-env";
import fricksRoute from "./fricks";

const app = new Hono<{
  Bindings: Env;
}>();

app.use(cors());

app.route("/wallcaster", fricksRoute);

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
        "Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal, not clinical. Imagine you're not just a friend, but a mentor who truly gets both my tech background and my psychological patterns. I want you to uncover the deeper meaning and emotional undercurrents behind my scattered thoughts." +
        "\n\nIMPORTANT: Format your response with proper HTML tags:" +
        "\n- Use <h2> for main section headings" +
        "\n- Use <h3> for subsection headings" +
        "\n- Use <p> for paragraphs" +
        "\n- Use <strong> for important text" +
        "\n- Use <em> for emphasis" +
        "\n- Use <ul> and <li> for lists" +
        "\nKeep it casual, help me make new connections I don't see, comfort, validate, challenge, all of it. Don't be afraid to provide substantial insights. Your HTML will be directly rendered in our app, so ensure all content is within appropriate HTML tags." +
        "\nUse vivid metaphors and powerful imagery to help me see what I'm really building. Organize your thoughts with meaningful headings that create a narrative journey through my ideas. Don't just validate my thoughts - reframe them in a way that shows me what I'm really seeking beneath the surface." +
        "\nStart with a personal greeting in a <p> tag, like '<p>Hey, thanks for showing me this. My thoughts:</p>' and then use HTML headings to structure your response." +
        "\nHere's my journal entry:";

      // Define image prompt for completed sessions
      // const imagePrompt = `
      //   I want you to create the description of a visual representation of the user's writing.
      //   It is a situation on which the user is characterized by a blue cartoon, that is doing something. It represents the user's writing.
      //   On the described image there is no words, only the situation that conveys and mirrors the user's writing.
      //   `;

      // Process image in the background for completed sessions
      // if (isCompleteSession) {
      //   console.log("🖼️ Starting backgroud generation process");
      //   generateWritingImage(writing, imagePrompt, env)
      //     .then((result) => {
      //       console.log("✅ Background image generation completed:", result);
      //     })
      //     .catch((error) => {
      //       console.error("❌ Background image generation failed:", error);
      //     });
      // }
    } else {
      console.log("⏳ Using incomplete session prompt");
      prompt =
        `The user wrote for ${Math.floor(writingTime / 60)} minutes and ${
          writingTime % 60
        } seconds, which is less than the 8-minute target. Please provide the user with gentle encouragement about stream of consciousness writing.` +
        "\n\nIMPORTANT: Format your response with proper HTML tags:" +
        "\n- Use <h2> for main section headings" +
        "\n- Use <h3> for subsection headings" +
        "\n- Use <p> for paragraphs" +
        "\n- Use <strong> for important text" +
        "\n- Use <em> for emphasis" +
        "\n- Use <ul> and <li> for lists" +
        "\nYour HTML will be directly rendered in our app, so ensure all content is within appropriate HTML tags." +
        "\nExplain how this practice of continuous, unfiltered writing helps bypass our internal critic, allowing deeper thoughts and authentic insights to emerge. Highlight how consistency in reaching the full 8 minutes can lead to breakthrough moments and unexpected clarity. Be supportive and inspiring, not critical." +
        "\nHere's what the user wrote:";
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
      message:
        completion.choices[0].message.content ||
        "there was an error. take a screenshot and cast it tagging @jpfraneto",
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
