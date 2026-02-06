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
import type { SessionData, AnkyData, ChatMessage, WritingSession } from "../types";
import { formatDuration, escapeHtml, ANKY_THRESHOLD } from "../utils/helpers";
import { TextMandala } from "./TextMandala";

interface DesktopDashboardProps {
  visible: boolean;
  sessions: WritingSession[];
  activeSessionIndex: number;
  sessionData: SessionData | null;
  ankyData: AnkyData | null;
  chatHistory: ChatMessage[];
  backendSessionId: string | null;
  backendUserId?: string;
  onSessionSelect: (idx: number) => void;
  onAnkyDataUpdate: (data: AnkyData, ankyId?: string) => void;
  onChatHistoryUpdate: (history: ChatMessage[]) => void;
  onWriteAgain: () => void;
  onMint: () => void;
  isMinting: boolean;
}

export function DesktopDashboard({
  visible,
  sessions,
  activeSessionIndex,
  sessionData,
  ankyData,
  chatHistory,
  backendSessionId,
  backendUserId,
  onSessionSelect,
  onAnkyDataUpdate,
  onChatHistoryUpdate,
  onWriteAgain,
  onMint,
  isMinting,
}: DesktopDashboardProps) {
  const [inputValue, setInputValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [statusText, setStatusText] = useState("");
  const [generatedImage, setGeneratedImage] = useState<string | null>(null);
  const [generatedTitle, setGeneratedTitle] = useState<string | null>(null);
  const [expandedWriting, setExpandedWriting] = useState(false);
  const messagesRef = useRef<HTMLDivElement>(null);
  const hasGeneratedRef = useRef(false);

  const scrollToBottom = useCallback(() => {
    messagesRef.current?.scrollTo({
      top: messagesRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, []);

  // Reset generation state when session changes
  useEffect(() => {
    hasGeneratedRef.current = false;
    setGeneratedImage(ankyData?.url || null);
    setGeneratedTitle(ankyData?.title || null);
  }, [activeSessionIndex, ankyData]);

  // Generate anky or fetch short session response
  useEffect(() => {
    if (
      !visible ||
      !sessionData ||
      hasGeneratedRef.current ||
      chatHistory.length > 0 ||
      ankyData
    )
      return;

    hasGeneratedRef.current = true;

    const generateAnky = async () => {
      setIsLoading(true);

      if (sessionData.isFullSession) {
        try {
          setStatusText("mapping your psyche...");
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
    <div className={`dashboard ${visible ? "visible" : ""}`}>
      {/* Left Sidebar - Sessions List */}
      <div className="sessions-sidebar">
        <div className="sidebar-header">
          <span className="sidebar-title">Sessions</span>
          <button className="new-session-btn" onClick={onWriteAgain}>
            + New
          </button>
        </div>
        <div className="sessions-list">
          {sessions.length === 0 ? (
            <div className="sessions-empty">No sessions yet. Start writing!</div>
          ) : (
            sessions
              .slice()
              .reverse()
              .map((session, idx) => {
                const realIdx = sessions.length - 1 - idx;
                const isAnky =
                  session.duration >= ANKY_THRESHOLD &&
                  (session.content?.length || 0) >= 100;
                const title =
                  session.ankyTitle ||
                  (isAnky ? "anky session" : "writing session");
                const date = new Date(session.timestamp);
                const timeStr = date.toLocaleDateString(undefined, {
                  month: "short",
                  day: "numeric",
                });

                return (
                  <div
                    key={realIdx}
                    className={`session-item ${isAnky ? "is-anky" : ""} ${realIdx === activeSessionIndex ? "active" : ""}`}
                    onClick={() => onSessionSelect(realIdx)}
                  >
                    <div className="session-item-header">
                      <div
                        className={
                          isAnky ? "session-anky-badge" : "session-no-anky"
                        }
                      />
                      <span className="session-title">{title}</span>
                    </div>
                    <div className="session-meta">
                      <span>{timeStr}</span>
                      <span>{formatDuration(session.duration)}</span>
                      <span>{session.stats?.wordCount || 0} words</span>
                    </div>
                    <div className="session-preview">
                      {session.content?.substring(0, 100) || ""}
                    </div>
                  </div>
                );
              })
          )}
        </div>
      </div>

      {/* Right Side - Conversation View */}
      <div className="conversation-main">
        <div className="conversation-header">
          <div className="conversation-avatar" />
          <div className="conversation-info">
            <div className="conversation-title">anky</div>
            <div className="conversation-stats">
              {sessionData
                ? `${formatDuration(sessionData.elapsed)} \u00B7 ${sessionData.wordCount} words`
                : ""}
            </div>
          </div>
        </div>

        <div className="conversation-messages" ref={messagesRef}>
          {sessionData && (
            <div
              className={`desktop-bubble user desktop-writing-bubble ${expandedWriting ? "expanded" : ""}`}
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
                {expandedWriting ? "click to collapse" : "click to expand"}
              </span>
            </div>
          )}

          {sessionData?.isFullSession && (isLoading || generatedImage) && (
            <div className="desktop-bubble-row">
              <div className="desktop-bubble-avatar" />
              <div className="desktop-bubble anky">
                {generatedImage ? (
                  <div className="desktop-anky-image-container">
                    <img
                      className="desktop-anky-image"
                      src={generatedImage}
                      alt="Your Anky"
                    />
                  </div>
                ) : (
                  <TextMandala text={sessionData.content} />
                )}
                {generatedTitle ? (
                  <div className="desktop-anky-title">{generatedTitle}</div>
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
            <div className="desktop-bubble-row">
              <div className="desktop-bubble-avatar" />
              <div className="desktop-bubble anky">
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
                <div className="desktop-bubble user">{msg.content}</div>
              ) : (
                <div className="desktop-bubble-row">
                  <div className="desktop-bubble-avatar" />
                  <div className="desktop-bubble anky markdown-content">
                    <ReactMarkdown>{msg.content}</ReactMarkdown>
                  </div>
                </div>
              )}
            </div>
          ))}

          {isLoading && statusText && !sessionData?.isFullSession && (
            <div className="desktop-bubble-row">
              <div className="desktop-bubble-avatar" />
              <div className="desktop-status-message">
                <div className="typing-dots"><span/><span/><span/></div>
                <span>{statusText}</span>
              </div>
            </div>
          )}

          {!isLoading && sessionData?.isFullSession && ankyData && !sessions[activeSessionIndex]?.minted && (
            <div className="desktop-chat-actions">
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
            </div>
          )}
          {!isLoading && sessionData?.isFullSession && sessions[activeSessionIndex]?.minted && (
            <div className="desktop-chat-actions">
              <button className="btn btn-secondary" onClick={onWriteAgain}>
                write again
              </button>
              <span className="minted-badge">minted</span>
            </div>
          )}
        </div>

        <div className="conversation-input-area">
          <input
            type="text"
            className="conversation-input"
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
            className="conversation-send"
            onClick={sendMessage}
            disabled={isLoading || !inputValue.trim()}
          >
            <span>&uarr;</span>
          </button>
        </div>
      </div>
    </div>
  );
}
