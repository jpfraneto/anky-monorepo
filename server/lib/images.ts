import type { Env } from "../env/server-env";
import { createOpenAIClient } from "./openrouter";
import OpenAI, { toFile } from "openai";
import fetch, { type BodyInit } from "node-fetch";

/**
 * Generates an image based on the user's writing in the background
 * @param writing The user's writing content
 * @param imagePrompt The prompt to generate the image
 * @param env Environment variables
 * @returns Promise with the result of the image generation
 */
export async function generateWritingImage(
  writing: string,
  imagePrompt: string,
  env: Env
) {
  console.log("🎨 Generating image based on user's writing");
  try {
    // Create OpenAI client for image description generation
    const openai = createOpenAIClient(env);

    // First, get a description of the image based on the writing
    const descriptionResponse = await openai.chat.completions.create({
      model: "openai/gpt-4o",
      messages: [
        {
          role: "user",
          content:
            imagePrompt + "\n\nHere's the writing to visualize:\n" + writing,
        },
      ],
    });

    const imageDescription = descriptionResponse.choices[0]?.message?.content;
    if (!imageDescription) {
      throw new Error("Failed to generate image description");
    }

    console.log("📝 Generated image description:", imageDescription);

    // Initialize OpenAI client with API key from environment
    if (!env.OPENAI_API_KEY) {
      throw new Error("OpenAI API key not found in environment variables");
    }

    console.log("🤖 OpenAI client initialized for image generation");

    // Fetch the base image
    console.log("🔄 Fetching base image for editing");
    // Extract base64 data from the data URL
    console.log("🔄 Extracting base64 data from image");
    const imageResponse = await fetch("https://s.mj.run/YLJMlMJbo70");
    const imageArrayBuffer = await imageResponse.arrayBuffer();
    const imageBuffer = Buffer.from(imageArrayBuffer);

    console.log("📤 Sending image to OpenAI API...");

    // Create a file object from the buffer
    console.log("📄 Creating file object from buffer");
    const file = await toFile(imageBuffer, "input.png", { type: "image/png" });

    // Send to OpenAI API
    console.log("🔮 Calling OpenAI image edit API");
    const response = await openai.images.edit({
      model: "gpt-image-1",
      image: file,
      prompt: imageDescription,
      n: 1,
      size: "1024x1024",
    });

    console.log("✨ Received response from OpenAI", response);

    if (!response.data || response.data.length === 0) {
      throw new Error("No image data received from OpenAI");
    }

    const imageUrl = response.data[0]?.url;
    if (!imageUrl) {
      throw new Error("No image URL in the response");
    }

    // If PINATA credentials are available, upload to IPFS
    if (env.PINATA_API_JWT) {
      console.log("📌 Pinata credentials found, uploading to IPFS");

      // Download the image from OpenAI
      const imageDataResponse = await fetch(imageUrl);
      const imageData = await imageDataResponse.arrayBuffer();

      // Prepare form data for Pinata
      const formData = new FormData();
      const imageBlob = new Blob([Buffer.from(imageData)], {
        type: "image/png",
      });
      const randomId = Math.random().toString(36).substring(2, 15);
      formData.append("file", imageBlob, `anky-writing-image-${randomId}.png`);

      // Add metadata
      const metadata = JSON.stringify({
        name: `Anky Writing Image ${randomId}`,
        keyvalues: {
          service: "anky",
          type: "writing-image",
        },
      });
      formData.append("pinataMetadata", metadata);

      // Add options
      const pinataOptions = JSON.stringify({
        cidVersion: 1,
      });
      formData.append("pinataOptions", pinataOptions);

      // Upload to Pinata
      const pinataResponse = await fetch(
        "https://api.pinata.cloud/pinning/pinFileToIPFS",
        {
          method: "POST",
          headers: {
            Authorization: `Bearer ${env.PINATA_API_JWT}`,
            // Note: We can't use formData.getHeaders() with the standard FormData
            // as it's not available in this context
          },
          body: formData as unknown as BodyInit,
        }
      );

      if (pinataResponse.ok) {
        const pinataData = await pinataResponse.json();
        const ipfsCid = pinataData.IpfsHash;
        const ipfsUrl = `https://anky.mypinata.cloud/ipfs/${ipfsCid}`;

        console.log("✅ Image uploaded to IPFS:", ipfsUrl);

        return {
          success: true,
          message: "Image generation and IPFS upload completed",
          imageUrl: ipfsUrl,
          ipfsCid: ipfsCid,
          originalUrl: imageUrl,
        };
      } else {
        console.warn("⚠️ Failed to upload to IPFS, returning original URL");
      }
    }

    // Return the original URL if Pinata upload wasn't available or failed
    console.log("🖼️ Image generation process completed");
    return {
      success: true,
      message: "Image generation process completed",
      imageUrl: imageUrl,
    };
  } catch (error) {
    console.error("❌ Error generating image:", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
