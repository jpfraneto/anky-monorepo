import { useState, useEffect, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import {
  fetchAPI,
  createAnky,
  type PromptResponse,
  type ReflectionResponse,
  type ImageResponse,
  type TitleResponse,
  type IpfsResponse,
} from "../api";
import type { SessionData, AnkyData } from "../types";

type RevealPhase = "generating" | "image" | "title" | "reflection" | "complete";

interface SacredRevealProps {
  sessionData: SessionData;
  backendSessionId: string | null;
  backendUserId?: string;
  onComplete: (ankyData: AnkyData, ankyId?: string) => void;
  onWriteAgain: () => void;
  onMint: () => void;
  onTalkToAnky: () => void;
  isMinting: boolean;
}

export function SacredReveal({
  sessionData,
  backendSessionId,
  backendUserId,
  onComplete,
  onWriteAgain,
  onMint,
  onTalkToAnky,
  isMinting,
}: SacredRevealProps) {
  const [phase, setPhase] = useState<RevealPhase>("generating");
  const [statusText, setStatusText] = useState("reading your soul...");
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [title, setTitle] = useState<string | null>(null);
  const [reflection, setReflection] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ankyData, setAnkyData] = useState<AnkyData | null>(null);

  const generateAnky = useCallback(async () => {
    try {
      setStatusText("reading your soul...");

      // Generate prompt and reflection in parallel
      const [promptResult, reflectionResult] = await Promise.all([
        fetchAPI<PromptResponse>("/api/prompt", {
          writingSession: sessionData.content,
        }),
        fetchAPI<ReflectionResponse>("/api/reflection", {
          writingSession: sessionData.content,
          locale: navigator.language,
        }),
      ]);

      setStatusText("painting your anky...");

      // Generate image
      const imageResult = await fetchAPI<ImageResponse>("/api/image", {
        prompt: promptResult.prompt,
      });
      setImageUrl(imageResult.url);
      setPhase("image");

      // Wait for image reveal animation
      await new Promise((resolve) => setTimeout(resolve, 3000));

      setStatusText("naming your anky...");

      // Generate title
      const titleResult = await fetchAPI<TitleResponse>("/api/title", {
        writingSession: sessionData.content,
        imagePrompt: promptResult.prompt,
        reflection: reflectionResult.reflection,
      });
      setTitle(titleResult.title);
      setPhase("title");

      // Wait for title reveal animation
      await new Promise((resolve) => setTimeout(resolve, 1500));

      setReflection(reflectionResult.reflection);
      setPhase("reflection");

      // Wait for reflection reveal animation
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Upload to IPFS
      let ipfsResult: IpfsResponse | null = null;
      try {
        ipfsResult = await fetchAPI<IpfsResponse>("/api/ipfs", {
          writingSession: sessionData.content,
          imageBase64: imageResult.base64,
          title: titleResult.title,
          reflection: reflectionResult.reflection,
          imagePrompt: promptResult.prompt,
        });
      } catch {
        // IPFS upload failed, continue without it
      }

      // Create Anky record in backend
      let ankyId: string | undefined;
      if (backendSessionId) {
        try {
          const anky = await createAnky({
            writingSessionId: backendSessionId,
            userId: backendUserId,
            imagePrompt: promptResult.prompt,
            reflection: reflectionResult.reflection,
            title: titleResult.title,
            imageBase64: imageResult.base64,
            imageUrl: imageResult.url,
            writingIpfsHash: ipfsResult?.writingSessionIpfs,
            imageIpfsHash: ipfsResult?.imageIpfs,
            metadataIpfsHash: ipfsResult?.tokenUri,
          });
          ankyId = anky.id;
        } catch (e) {
          console.error("Failed to create anky in backend:", e);
        }
      }

      const newAnkyData: AnkyData = {
        ...promptResult,
        ...reflectionResult,
        ...imageResult,
        ...titleResult,
        ...(ipfsResult || {}),
        writingSession: sessionData.content,
      };

      setAnkyData(newAnkyData);
      setPhase("complete");
      onComplete(newAnkyData, ankyId);
    } catch (e: unknown) {
      const err = e as { message?: string };
      setError(err.message || "Something went wrong");
      setPhase("complete");
    }
  }, [sessionData, backendSessionId, backendUserId, onComplete]);

  useEffect(() => {
    generateAnky();
  }, [generateAnky]);

  if (error) {
    return (
      <div className="sacred-reveal">
        <div className="sacred-reveal-error">
          <p>{error}</p>
          <button className="btn btn-secondary" onClick={onWriteAgain}>
            try again
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="sacred-reveal">
      {/* Generating Phase */}
      {phase === "generating" && (
        <div className="sacred-reveal-generating">
          <div className="sacred-reveal-pulse" />
          <p className="sacred-reveal-status">{statusText}</p>
        </div>
      )}

      {/* Image Phase and Beyond */}
      {phase !== "generating" && (
        <div className="sacred-reveal-content">
          {/* Image */}
          {imageUrl && (
            <div className={`sacred-reveal-image-container ${phase === "image" ? "revealing" : "revealed"}`}>
              <img
                src={imageUrl}
                alt="Your Anky"
                className="sacred-reveal-image"
              />
            </div>
          )}

          {/* Title */}
          {title && (phase === "title" || phase === "reflection" || phase === "complete") && (
            <h2 className={`sacred-reveal-title ${phase === "title" ? "revealing" : "revealed"}`}>
              {title}
            </h2>
          )}

          {/* Reflection */}
          {reflection && (phase === "reflection" || phase === "complete") && (
            <div className={`sacred-reveal-reflection markdown-content ${phase === "reflection" ? "revealing" : "revealed"}`}>
              <ReactMarkdown>{reflection}</ReactMarkdown>
            </div>
          )}

          {/* Complete Phase Actions */}
          {phase === "complete" && ankyData && (
            <div className="sacred-reveal-actions">
              <button className="btn btn-secondary" onClick={onWriteAgain}>
                write again
              </button>
              <button
                className="btn btn-primary"
                onClick={onMint}
                disabled={isMinting}
              >
                {isMinting ? "minting..." : "mint anky"}
              </button>
              <button
                className="sacred-reveal-talk-link"
                onClick={onTalkToAnky}
              >
                talk to anky
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
