import { join } from "node:path";
import { existsSync } from "node:fs";
import { readFileSync } from "node:fs";
import { GoogleGenAI } from "@google/genai";

let ANKY_REFERENCES: string[] = [];

// **
//  * Load Anky reference images on startup
//  * Place your reference images in public/references/
//  * - anky-1.png (main character reference - SUBJECT)
//  * - anky-2.png (style/pose reference)
//  * - anky-3.png (additional reference)
//  *
//  * MAX 4 IMAGES
//  */
export function initAnkyReferences(): void {
  console.log("[ANKY REFERENCES] Initializing reference image loader...");
  const referencesDir = join(process.cwd(), "public", "references");
  console.log(`[ANKY REFERENCES] Looking in directory: ${referencesDir}`);

  const referenceFiles = ["anky-1.png", "anky-2.png", "anky-3.png"];

  ANKY_REFERENCES = []; // Reset array

  for (const file of referenceFiles) {
    const filePath = join(referencesDir, file);
    console.log(`[ANKY REFERENCES] Checking for: ${filePath}`);

    if (existsSync(filePath)) {
      try {
        const imageBuffer = readFileSync(filePath);
        const base64Data = imageBuffer.toString("base64");
        ANKY_REFERENCES.push(base64Data);
        const sizeKB = Math.round(imageBuffer.length / 1024);
        console.log(
          `✓ Loaded reference: ${file} (${sizeKB}KB, base64 length: ${base64Data.length})`
        );
      } catch (e) {
        console.warn(`✗ Failed to load ${file}:`, e);
      }
    } else {
      console.log(`  → File not found: ${file}`);
    }
  }

  console.log(
    `[ANKY REFERENCES] Total loaded: ${ANKY_REFERENCES.length} reference images`
  );
  if (ANKY_REFERENCES.length === 0) {
    console.warn("[ANKY REFERENCES] ⚠️  WARNING: No reference images loaded!");
  }
}

export async function generateImageWithReferences(
  prompt: string
): Promise<{ url: string; base64: string }> {
  console.log("[IMAGE GEN] Starting image generation...");
  console.log(`[IMAGE GEN] Input prompt length: ${prompt.length} chars`);
  console.log(`[IMAGE GEN] Available references: ${ANKY_REFERENCES.length}`);

  if (!process.env.GEMINI_API_KEY) {
    console.error("[IMAGE GEN] ❌ GEMINI_API_KEY is missing!");
    throw new Error("GEMINI_API_KEY is required");
  }
  console.log("[IMAGE GEN] ✓ GEMINI_API_KEY found");

  const fullPrompt = `Create a mystical fantasy illustration: ${prompt}
  
  CHARACTER - ANKY (follow exactly):
  - Blue-skinned creature with large expressive pointed ears
  - Purple swirling hair with golden spiral accents
  - Golden/amber glowing eyes
  - Golden jewelry and decorative accents on body
  - Compact body, ancient yet childlike quality
  
  STYLE:
  - Rich colors: deep blues, purples, oranges, golds
  - Painterly, atmospheric, slightly psychedelic
  - Warm mystical lighting
  - Fantasy art style, highly detailed`;

  // Build contents array with reference images and text
  const contents: Array<
    { text: string } | { inlineData: { mimeType: string; data: string } }
  > = [];

  console.log(
    `[IMAGE GEN] Building request with ${ANKY_REFERENCES.length} reference images...`
  );

  // Add reference images first (if any)
  for (let i = 0; i < Math.min(ANKY_REFERENCES.length, 4); i++) {
    const ref = ANKY_REFERENCES[i];
    if (!ref) {
      console.warn(`[IMAGE GEN] ⚠️  Reference ${i + 1} is undefined, skipping`);
      continue;
    }
    const refSize = ref.length;
    console.log(
      `[IMAGE GEN] Adding reference image ${
        i + 1
      }: base64 length = ${refSize} chars`
    );

    contents.push({
      inlineData: {
        mimeType: "image/png",
        data: ref,
      },
    });
  }

  console.log(`[IMAGE GEN] Added ${contents.length} image parts to request`);

  // Add reference context if we have images
  if (ANKY_REFERENCES.length > 0) {
    const contextText =
      "Reference images above show Anky. Create a NEW image matching this character exactly:";
    console.log(`[IMAGE GEN] Adding reference context text`);
    contents.push({ text: contextText });
  }

  // Add the main prompt
  console.log(`[IMAGE GEN] Adding main prompt (${fullPrompt.length} chars)`);
  contents.push({ text: fullPrompt });

  console.log(`[IMAGE GEN] Total parts in request: ${contents.length}`);
  console.log(`[IMAGE GEN] Parts breakdown:`);
  contents.forEach((part, idx) => {
    if ("inlineData" in part) {
      console.log(
        `  [${idx}] Image part (${part.inlineData.data.length} chars base64)`
      );
    } else if ("text" in part) {
      const preview = part.text.substring(0, 100).replace(/\n/g, " ");
      console.log(
        `  [${idx}] Text part: "${preview}..." (${part.text.length} chars)`
      );
    }
  });

  console.log(`[IMAGE GEN] Initializing Google GenAI client...`);
  const ai = new GoogleGenAI({
    apiKey: process.env.GEMINI_API_KEY,
  });

  // Use gemini-2.5-flash-image for speed, or gemini-3-pro-image-preview for higher quality
  const model = "gemini-2.5-flash-image";
  console.log(`[IMAGE GEN] Using model: ${model}`);
  console.log(`[IMAGE GEN] Sending request to Gemini API...`);

  const startTime = Date.now();

  try {
    const response = await ai.models.generateContent({
      model: model,
      contents: contents,
      config: {
        responseModalities: ["TEXT", "IMAGE"],
        imageConfig: {
          aspectRatio: "1:1", // Square aspect ratio for Anky images
        },
      },
    });

    const requestTime = Date.now() - startTime;
    console.log(`[IMAGE GEN] API response received in ${requestTime}ms`);

    const candidates = response.candidates;
    if (!candidates || candidates.length === 0) {
      console.error("[IMAGE GEN] ❌ No candidates in response!");
      throw new Error("No candidates in Gemini response");
    }

    console.log(`[IMAGE GEN] Response parsed successfully`);
    console.log(`[IMAGE GEN] Candidates count: ${candidates.length}`);

    const firstCandidate = candidates[0];
    if (!firstCandidate) {
      console.error("[IMAGE GEN] ❌ First candidate is missing!");
      throw new Error("No candidate in Gemini response");
    }

    const responseParts = firstCandidate.content?.parts;
    if (!responseParts) {
      console.error("[IMAGE GEN] ❌ No parts in candidate content!");
      throw new Error("No parts in Gemini response");
    }

    console.log(`[IMAGE GEN] Response parts count: ${responseParts.length}`);
    responseParts.forEach((part, idx) => {
      if ("inlineData" in part && part.inlineData && part.inlineData.data) {
        console.log(
          `  [${idx}] Image part: ${part.inlineData.mimeType} (${part.inlineData.data.length} chars base64)`
        );
      } else if ("text" in part && part.text) {
        console.log(`  [${idx}] Text part: "${part.text.substring(0, 50)}..."`);
      } else {
        console.log(`  [${idx}] Other part:`, Object.keys(part));
      }
    });

    // Find the image part
    const imagePart = responseParts.find(
      (part) =>
        "inlineData" in part &&
        part.inlineData?.mimeType?.startsWith("image/") &&
        part.inlineData?.data
    );

    if (
      !imagePart ||
      !("inlineData" in imagePart) ||
      !imagePart.inlineData?.data
    ) {
      console.error("[IMAGE GEN] ❌ No image part found in response!");
      console.error(
        "[IMAGE GEN] Response parts:",
        JSON.stringify(responseParts, null, 2)
      );
      throw new Error(
        "No image in Gemini response - model may have returned text only"
      );
    }

    const base64 = imagePart.inlineData.data;
    const mimeType = imagePart.inlineData.mimeType || "image/png";
    const imageSizeKB = Math.round(base64.length / 1024);

    console.log(`[IMAGE GEN] ✓ Image extracted successfully!`);
    console.log(`[IMAGE GEN] Image type: ${mimeType}`);
    console.log(`[IMAGE GEN] Image size: ${imageSizeKB}KB (base64)`);

    return {
      url: `data:${mimeType};base64,${base64}`,
      base64,
    };
  } catch (error) {
    const requestTime = Date.now() - startTime;
    console.error(`[IMAGE GEN] ❌ Error after ${requestTime}ms:`);
    console.error(error);
    if (error instanceof Error) {
      throw new Error(`Gemini API failed: ${error.message}`);
    }
    throw error;
  }
}
