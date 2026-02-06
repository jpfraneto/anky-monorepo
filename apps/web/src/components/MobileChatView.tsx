import { useState, useEffect, useRef, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import {
  fetchAPI,
  createAnky,
  type PromptResponse,
  type ReflectionResponse,
  type ImageResponse,
  type TitleResponse,
  type IpfsResponse,
  type ChatResponse,
} from "../api";
import type { SessionData, AnkyData, ChatMessage } from "../types";
import { formatDuration, escapeHtml } from "../utils/helpers";
import { TextMandala } from "./TextMandala";

interface MobileChatViewProps {
  visible: boolean;
  sessionData: SessionData | null;
  ankyData: AnkyData | null;
  chatHistory: ChatMessage[];
  backendSessionId: string | null;
  backendUserId?: string;
  onAnkyDataUpdate: (data: AnkyData, ankyId?: string) => void;
  onChatHistoryUpdate: (history: ChatMessage[]) => void;
  onBack: () => void;
  onMint: () => void;
  isMinting: boolean;
}

export function MobileChatView({
  visible,
  sessionData,
  ankyData,
  chatHistory,
  backendSessionId,
  backendUserId,
  onAnkyDataUpdate,
  onChatHistoryUpdate,
  onBack,
  onMint,
  isMinting,
}: MobileChatViewProps) {
  const [inputValue, setInputValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [statusText, setStatusText] = useState("");
  const [generatedImage, setGeneratedImage] = useState<string | null>(null);
  const [generatedTitle, setGeneratedTitle] = useState<string | null>(null);
  const [expandedWriting, setExpandedWriting] = useState(false);
  const messagesRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = useCallback(() => {
    messagesRef.current?.scrollTo({
      top: messagesRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, []);

  // Generate anky or fetch short session response
  useEffect(() => {
    if (!visible || !sessionData || chatHistory.length > 0 || ankyData) return;

    const generateAnky = async () => {
      setIsLoading(true);

      if (sessionData.isFullSession) {
        try {
          setStatusText("reading your soul...");
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
          const imageResult = await fetchAPI<ImageResponse>("/api/image", {
            prompt: promptResult.prompt,
          });
          setGeneratedImage(imageResult.url);

          setStatusText("naming your anky...");
          const titleResult = await fetchAPI<TitleResponse>("/api/title", {
            writingSession: sessionData.content,
            imagePrompt: promptResult.prompt,
            reflection: reflectionResult.reflection,
          });
          setGeneratedTitle(titleResult.title);

          setStatusText("");
          setIsLoading(false);

          onChatHistoryUpdate([
            { role: "assistant", content: reflectionResult.reflection },
          ]);

          // Try IPFS upload
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

          // Use IPFS gateway URL if available, otherwise fall back to generated URL
          const finalImageUrl = ipfsResult?.imageUrl || imageResult.url;

          // Create Anky record in backend with IPFS hashes
          let ankyId: string | undefined;
          if (backendSessionId) {
            try {
              const anky = await createAnky({
                writingSessionId: backendSessionId,
                userId: backendUserId,
                imagePrompt: promptResult.prompt,
                reflection: reflectionResult.reflection,
                title: titleResult.title,
                imageUrl: finalImageUrl,
                writingIpfsHash: ipfsResult?.writingSessionIpfs,
                imageIpfsHash: ipfsResult?.imageIpfs,
                metadataIpfsHash: ipfsResult?.tokenUri,
                generatedImageId: imageResult.id,
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
          onAnkyDataUpdate(newAnkyData, ankyId);
        } catch (e: unknown) {
          setStatusText("");
          setIsLoading(false);
          const error = e as { message?: string };
          onChatHistoryUpdate([
            {
              role: "assistant",
              content: `Something went wrong: ${error.message}`,
            },
          ]);
        }
      } else {
        try {
          setStatusText("reading...");
          const response = await fetchAPI<ChatResponse>("/api/chat-short", {
            writingSession: sessionData.content,
            duration: sessionData.elapsed,
            wordCount: sessionData.wordCount,
            history: [],
          });
          setStatusText("");
          setIsLoading(false);
          onChatHistoryUpdate([
            { role: "assistant", content: response.response },
          ]);
        } catch (e: unknown) {
          setStatusText("");
          setIsLoading(false);
          const error = e as { message?: string };
          onChatHistoryUpdate([
            { role: "assistant", content: `Error: ${error.message}` },
          ]);
        }
      }
    };

    generateAnky();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, sessionData, chatHistory.length, ankyData, backendSessionId]);

  useEffect(() => {
    scrollToBottom();
  }, [chatHistory, isLoading, scrollToBottom]);

  const sendMessage = async () => {
    if (!inputValue.trim() || !sessionData) return;

    const newHistory = [
      ...chatHistory,
      { role: "user" as const, content: inputValue },
    ];
    onChatHistoryUpdate(newHistory);
    setInputValue("");
    setIsLoading(true);
    setStatusText("thinking...");

    try {
      const endpoint = sessionData.isFullSession
        ? "/api/chat"
        : "/api/chat-short";
      const body = sessionData.isFullSession
        ? {
            writingSession: sessionData.content,
            reflection: ankyData?.reflection || "",
            title: ankyData?.title || "",
            history: newHistory,
          }
        : {
            writingSession: sessionData.content,
            duration: sessionData.elapsed,
            wordCount: sessionData.wordCount,
            history: newHistory,
          };

      const response = await fetchAPI<ChatResponse>(endpoint, body);
      setStatusText("");
      setIsLoading(false);
      onChatHistoryUpdate([
        ...newHistory,
        { role: "assistant", content: response.response },
      ]);
    } catch (e: unknown) {
      setStatusText("");
      setIsLoading(false);
      const error = e as { message?: string };
      onChatHistoryUpdate([
        ...newHistory,
        { role: "assistant", content: `Error: ${error.message}` },
      ]);
    }
  };

  if (!visible) return null;

  return (
    <div className={`chat-view ${visible ? "visible" : ""}`}>
      <div className="chat-header">
        <button
          className="chat-header-back"
          onClick={() => {
            if (confirm("Start over? Your writing will be lost.")) {
              onBack();
            }
          }}
        >
          <span>&larr;</span>
        </button>
        <div className="chat-header-title">anky</div>
        <div className="chat-header-stats">
          {sessionData
            ? `${formatDuration(sessionData.elapsed)} \u00B7 ${sessionData.wordCount} words`
            : ""}
        </div>
      </div>

      <div className="chat-messages" ref={messagesRef}>
        {sessionData && (
          <div
            className={`chat-bubble user writing-bubble ${expandedWriting ? "expanded" : ""}`}
            onClick={() => setExpandedWriting(!expandedWriting)}
          >
            <div
              className="writing-text"
              dangerouslySetInnerHTML={{
                __html: escapeHtml(sessionData.content),
              }}
            />
            {!expandedWriting && <div className="writing-bubble-fade" />}
            <span className="writing-bubble-expand">
              {expandedWriting ? "tap to collapse" : "tap to expand"}
            </span>
          </div>
        )}

        {sessionData?.isFullSession && (isLoading || generatedImage) && (
          <div className="chat-bubble-row">
            <div className="anky-avatar" />
            <div className="chat-bubble anky">
              {generatedImage ? (
                <div className="anky-image-container">
                  <img
                    className="anky-image"
                    src={generatedImage}
                    alt="Your Anky"
                  />
                </div>
              ) : (
                <TextMandala text={sessionData.content} />
              )}
              {generatedTitle ? (
                <div className="anky-title">{generatedTitle}</div>
              ) : (
                <>
                  <div className="skeleton-title">
                    <div className="skeleton-title-word" />
                    <div className="skeleton-title-word" />
                    <div className="skeleton-title-word" />
                  </div>
                  {isLoading && statusText && (
                    <div className="skeleton-status">
                      <div className="typing-dots"><span/><span/><span/></div>
                      <span>{statusText}</span>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        )}

        {isLoading && sessionData?.isFullSession && chatHistory.length === 0 && (
          <div className="chat-bubble-row">
            <div className="anky-avatar" />
            <div className="chat-bubble anky">
              <div className="skeleton-reflection">
                <div className="skeleton-line" />
                <div className="skeleton-line" />
                <div className="skeleton-line" />
                <div className="skeleton-line" />
              </div>
            </div>
          </div>
        )}

        {chatHistory.map((msg, idx) => (
          <div key={idx}>
            {msg.role === "user" ? (
              <div className="chat-bubble user">{msg.content}</div>
            ) : (
              <div className="chat-bubble-row">
                <div className="anky-avatar" />
                <div className="chat-bubble anky markdown-content">
                  <ReactMarkdown>{msg.content}</ReactMarkdown>
                </div>
              </div>
            )}
          </div>
        ))}

        {isLoading && statusText && !sessionData?.isFullSession && (
          <div className="chat-bubble-row">
            <div className="anky-avatar" />
            <div className="status-message">
              <div className="typing-dots"><span/><span/><span/></div>
              <span>{statusText}</span>
            </div>
          </div>
        )}

        {!isLoading && sessionData?.isFullSession && ankyData && (
          <div className="chat-actions">
            <button className="btn btn-secondary" onClick={onBack}>
              write again
            </button>
            <button
              className="btn btn-primary"
              onClick={onMint}
              disabled={isMinting}
            >
              {isMinting ? "minting..." : "mint anky"}
            </button>
          </div>
        )}
      </div>

      <div className="chat-input-area">
        <input
          type="text"
          className="chat-input"
          placeholder="reply to anky..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              sendMessage();
            }
          }}
        />
        <button
          className="chat-send"
          onClick={sendMessage}
          disabled={isLoading || !inputValue.trim()}
        >
          <span>&uarr;</span>
        </button>
      </div>
    </div>
  );
}
