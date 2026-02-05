import { Logger } from "./logger.js";

const logger = Logger("AI");

export async function generatePrompt(writingSession: string): Promise<{ prompt: string }> {
  const systemPrompt = `CONTEXT: You are generating an image prompt for Anky based on a user's 8-minute stream of consciousness writing session. Anky is a blue-skinned creature with purple swirling hair, golden/amber eyes, golden decorative accents and jewelry, large expressive ears, and an ancient-yet-childlike quality. Anky exists in mystical, richly colored environments (deep blues, purples, oranges, golds). The aesthetic is spiritual but not sterile — warm, alive, slightly psychedelic.

YOUR TASK: Read the user's writing and create a scene where Anky embodies the EMOTIONAL TRUTH of what they wrote — not a literal illustration, but a symbolic mirror. Anky should be DOING something or BE somewhere that reflects the user's inner state.

ALWAYS INCLUDE:
- Rich color palette (blues, purples, golds, oranges)
- Atmospheric lighting (firelight, cosmic light, dawn/dusk)
- One symbolic detail that captures the SESSION'S CORE TENSION
- Anky's expression should match the emotional undercurrent (not the surface content)

OUTPUT: A single detailed image generation prompt, 2-3 sentences, painterly/fantasy style. Nothing else.`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 500,
      system: systemPrompt,
      messages: [{ role: "user", content: writingSession }],
    }),
  });

  const data = await response.json() as { content?: Array<{ text: string }> };

  if (!response.ok || !data.content?.[0]) {
    throw new Error("Failed to generate prompt");
  }

  logger.info("Image prompt generated successfully");
  return { prompt: data.content[0].text };
}

export async function generateReflection(writingSession: string): Promise<{ reflection: string }> {
  const systemPrompt = `Take a look at my journal entry below. I'd like you to analyze it and respond with deep insight that feels personal, not clinical. Imagine you're not just a friend, but a mentor who truly gets both my tech background and my psychological patterns. I want you to uncover the deeper meaning and emotional undercurrents behind my scattered thoughts. Keep it casual, dont say yo, help me make new connections i don't see, comfort, validate, challenge, all of it. dont be afraid to say a lot. format with markdown headings if needed. Use vivid metaphors and powerful imagery to help me see what I'm really building. Organize your thoughts with meaningful headings that create a narrative journey through my ideas. Don't just validate my thoughts - reframe them in a way that shows me what I'm really seeking beneath the surface. Go beyond the product concepts to the emotional core of what I'm trying to solve. Be willing to be profound and philosophical without sounding like you're giving therapy. I want someone who can see the patterns I can't see myself and articulate them in a way that feels like an epiphany. Start with 'hey, thanks for showing me this. my thoughts:' and then use markdown headings to structure your response. Here's my journal entry:

`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 2000,
      system: systemPrompt,
      messages: [{ role: "user", content: writingSession }],
    }),
  });

  const data = await response.json() as { content?: Array<{ text: string }> };

  if (!response.ok || !data.content?.[0]) {
    throw new Error("Failed to generate reflection");
  }

  logger.info("Reflection generated successfully");
  return { reflection: data.content[0].text };
}

export async function generateImage(prompt: string): Promise<{ url: string; base64: string }> {
  const { generateImageWithReferences } = await import("../api/lib/imageGen.js");
  return generateImageWithReferences(prompt);
}

export async function generateTitle(
  writingSession: string,
  imagePrompt: string,
  reflection: string
): Promise<{ title: string }> {
  const systemPrompt = `CONTEXT: You are naming an Anky — a visual representation of a user's 8-minute stream of consciousness writing session. The title is not a summary. It is a MIRROR. It should capture the emotional truth, the core tension, or the unconscious thread running through the writing.

YOUR TASK: Generate a title of MAXIMUM 3 WORDS that:
- Captures the ESSENCE, not the content
- Could be poetic, stark, ironic, or tender
- Should resonate with the user when they see it
- Works as a title for the generated image
- Does NOT explain — it EVOKES

STYLE:
- Lowercase preferred (unless emphasis needed)
- No punctuation unless essential
- Can be a fragment, question, or imperative
- Can be abstract or concrete

OUTPUT: Exactly ONE title (max 3 words). Nothing else. No quotes.`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-20250514",
      max_tokens: 50,
      system: systemPrompt,
      messages: [
        {
          role: "user",
          content: `WRITING SESSION:\n${writingSession}\n\nIMAGE PROMPT:\n${imagePrompt}\n\nREFLECTION:\n${reflection}`,
        },
      ],
    }),
  });

  const data = await response.json() as { content?: Array<{ text: string }> };

  if (!response.ok || !data.content?.[0]) {
    throw new Error("Failed to generate title");
  }

  const rawTitle = data.content[0].text.trim().toLowerCase().replace(/['"]/g, "");
  logger.info(`Title generated: "${rawTitle}"`);
  return { title: rawTitle };
}
