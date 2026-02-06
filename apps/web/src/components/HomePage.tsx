import { useState, useEffect, useRef, useCallback } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { createPublicClient, createWalletClient, custom, http } from "viem";
import { base } from "viem/chains";
import { useWritingSession } from "../hooks/useWritingSession";
import { useBackendSessions } from "../hooks/useBackendSessions";
import {
  createSession,
  fetchAPI,
  updateAnky,
  recordAnkyMint,
  type User,
  type UserStats,
  type IpfsResponse,
} from "../api";
import type { SessionData, AnkyData, ChatMessage } from "../types";
import { formatDuration, isMobile, ANKY_THRESHOLD } from "../utils/helpers";
import { MobileChatView } from "./MobileChatView";
import { DesktopDashboard } from "./DesktopDashboard";

const ANKY_CONTRACT = "0xdf4f77b20cdba13f5235e89bcf06f46618979c55";
const ANKY_ABI = [
  {
    type: "function",
    name: "mint",
    inputs: [
      { name: "writer", type: "address" },
      { name: "to", type: "address" },
      { name: "writingSessionIpfs", type: "string" },
      { name: "imageIpfs", type: "string" },
      { name: "imagePrompt", type: "string" },
      { name: "title", type: "string" },
      { name: "tokenUri", type: "string" },
    ],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "nonpayable",
  },
] as const;

interface HomePageProps {
  backendUser: User | null;
  userStats?: UserStats;
  onWritingStateChange?: (isWriting: boolean) => void;
  initialSessionIndex?: number | null;
  onSessionHandled?: () => void;
  onSessionsChange?: () => void;
}

export function HomePage({
  backendUser,
  userStats: _userStats,
  onWritingStateChange,
  initialSessionIndex,
  onSessionHandled,
  onSessionsChange,
}: HomePageProps) {
  const { user, login } = usePrivy();
  const {
    sessions,
    addSession,
    updateSessionAnkyData,
    updateSessionChatHistory,
    markSessionMinted,
  } = useBackendSessions(backendUser?.id);

  const [showChatView, setShowChatView] = useState(false);
  const [showDashboard, setShowDashboard] = useState(false);
  const [mainTransitioning, setMainTransitioning] = useState(false);
  const [currentSessionData, setCurrentSessionData] =
    useState<SessionData | null>(null);
  const [currentAnkyData, setCurrentAnkyData] = useState<AnkyData | null>(null);
  const [chatHistory, setChatHistory] = useState<ChatMessage[]>([]);
  const [activeSessionIndex, setActiveSessionIndex] = useState(-1);
  const [isMinting, setIsMinting] = useState(false);
  const [backendSessionId, setBackendSessionId] = useState<string | null>(null);
  const [backendAnkyId, setBackendAnkyId] = useState<string | null>(null);

  const handleSessionEnd = useCallback(
    async (data: SessionData) => {
      setCurrentSessionData(data);
      setChatHistory([]);
      setCurrentAnkyData(null);
      setBackendSessionId(null);
      setBackendAnkyId(null);

      try {
        // Create session in backend first
        const session = await createSession({
          userId: backendUser?.id,
          content: data.content,
          durationSeconds: data.elapsed,
          wordCount: data.wordCount,
          wordsPerMinute: data.wpm,
        });

        setBackendSessionId(session.id);

        // Add to local state
        const idx = addSession({
          id: session.id,
          shareId: session.shareId,
          content: data.content,
          durationSeconds: data.elapsed,
          wordCount: data.wordCount,
          wpm: data.wpm,
        });
        setActiveSessionIndex(idx);

        // Notify parent that sessions changed
        onSessionsChange?.();
      } catch (e) {
        console.error("Failed to create session in backend:", e);
        // Still show the UI even if backend fails
        setActiveSessionIndex(-1);
      }

      // Smooth crossfade: fade out writing, then show chat/dashboard
      setMainTransitioning(true);
      setTimeout(() => {
        if (isMobile()) {
          setShowChatView(true);
        } else {
          setShowDashboard(true);
        }
        setMainTransitioning(false);
      }, 400);
    },
    [addSession, backendUser?.id, onSessionsChange]
  );

  const {
    isWriting,
    content,
    duration,
    wordCount,
    timerProgress,
    timerVisible,
    isAnky,
    keystrokeStats,
    handleInput,
    handleKeyDown,
    resetSession,
  } = useWritingSession(handleSessionEnd);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  // Notify parent when writing state changes
  useEffect(() => {
    onWritingStateChange?.(isWriting);
  }, [isWriting, onWritingStateChange]);

  // Handle session selection from sidebar
  useEffect(() => {
    if (
      initialSessionIndex !== null &&
      initialSessionIndex !== undefined &&
      initialSessionIndex >= 0
    ) {
      const session = sessions[initialSessionIndex];
      if (session) {
        setActiveSessionIndex(initialSessionIndex);
        setBackendSessionId(session.id || null);
        setBackendAnkyId(null);
        setCurrentSessionData({
          content: session.content,
          elapsed: session.duration,
          wordCount: session.stats?.wordCount || 0,
          wpm: session.stats?.wpm || 0,
          isFullSession:
            session.duration >= ANKY_THRESHOLD &&
            (session.content?.length || 0) >= 100,
        });
        setCurrentAnkyData(session.ankyData || null);
        setChatHistory(session.chatHistory || []);

        if (isMobile()) {
          setShowChatView(true);
        } else {
          setShowDashboard(true);
        }

        onSessionHandled?.();
      }
    }
  }, [initialSessionIndex, sessions, onSessionHandled]);

  const handleWriteAgain = useCallback(() => {
    setShowChatView(false);
    setShowDashboard(false);
    setCurrentSessionData(null);
    setCurrentAnkyData(null);
    setChatHistory([]);
    setActiveSessionIndex(-1);
    resetSession();
    setTimeout(() => textareaRef.current?.focus(), 100);
  }, [resetSession]);

  const handleAnkyDataUpdate = useCallback(
    async (data: AnkyData, ankyId?: string) => {
      setCurrentAnkyData(data);
      if (ankyId) setBackendAnkyId(ankyId);

      // Update local state
      if (activeSessionIndex >= 0) {
        updateSessionAnkyData(activeSessionIndex, data, ankyId, data.title);
      }

      // Update backend with IPFS hashes if available
      if (ankyId && (data.writingSessionIpfs || data.imageIpfs || data.tokenUri)) {
        try {
          await updateAnky(ankyId, {
            writingIpfsHash: data.writingSessionIpfs,
            imageIpfsHash: data.imageIpfs,
            metadataIpfsHash: data.tokenUri,
          });
        } catch (e) {
          console.error("Failed to update anky IPFS hashes:", e);
        }
      }
    },
    [activeSessionIndex, updateSessionAnkyData]
  );

  const handleChatHistoryUpdate = useCallback(
    (history: ChatMessage[]) => {
      setChatHistory(history);
      if (activeSessionIndex >= 0) {
        updateSessionChatHistory(activeSessionIndex, history);
      }
    },
    [activeSessionIndex, updateSessionChatHistory]
  );

  const handleMint = useCallback(async () => {
    if (!user?.wallet?.address) {
      login();
      return;
    }

    if (!currentAnkyData || !currentSessionData) {
      alert("Session data missing");
      return;
    }

    setIsMinting(true);

    // If IPFS data is missing, try to upload first
    let ankyDataForMint = currentAnkyData;
    if (
      !currentAnkyData.writingSessionIpfs ||
      !currentAnkyData.imageIpfs ||
      !currentAnkyData.tokenUri
    ) {
      try {
        const ipfsResult = await fetchAPI<IpfsResponse>("/api/ipfs", {
          writingSession: currentSessionData.content,
          title: currentAnkyData.title || "",
          reflection: currentAnkyData.reflection || "",
          imagePrompt: currentAnkyData.prompt || currentAnkyData.imagePrompt || "",
        });

        ankyDataForMint = {
          ...currentAnkyData,
          writingSessionIpfs: ipfsResult.writingSessionIpfs,
          imageIpfs: ipfsResult.imageIpfs,
          tokenUri: ipfsResult.tokenUri,
        };

        // Update state and backend
        await handleAnkyDataUpdate(ankyDataForMint, backendAnkyId || undefined);
      } catch (e) {
        console.error("IPFS upload failed:", e);
        alert("Failed to upload to IPFS. Please try again.");
        setIsMinting(false);
        return;
      }
    }

    try {
      const publicClient = createPublicClient({
        chain: base,
        transport: http(),
      });

      const walletClient = createWalletClient({
        chain: base,
        transport: custom(window.ethereum!),
        account: user.wallet.address as `0x${string}`,
      });

      try {
        await window.ethereum?.request({
          method: "wallet_switchEthereumChain",
          params: [{ chainId: "0x2105" }],
        });
      } catch (e: unknown) {
        if ((e as { code?: number }).code === 4902) {
          await window.ethereum?.request({
            method: "wallet_addEthereumChain",
            params: [
              {
                chainId: "0x2105",
                chainName: "Base",
                nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 },
                rpcUrls: ["https://mainnet.base.org"],
                blockExplorerUrls: ["https://basescan.org"],
              },
            ],
          });
        }
      }

      const { request } = await publicClient.simulateContract({
        address: ANKY_CONTRACT,
        abi: ANKY_ABI,
        functionName: "mint",
        args: [
          user.wallet.address as `0x${string}`,
          user.wallet.address as `0x${string}`,
          ankyDataForMint.writingSessionIpfs!,
          ankyDataForMint.imageIpfs!,
          ankyDataForMint.imagePrompt || ankyDataForMint.prompt || "",
          ankyDataForMint.title || "",
          ankyDataForMint.tokenUri!,
        ],
        account: user.wallet.address as `0x${string}`,
      });

      const hash = await walletClient.writeContract(request);
      const receipt = await publicClient.waitForTransactionReceipt({ hash });

      // Extract tokenId from logs (assuming standard ERC721 Transfer event)
      const tokenId = receipt.logs[0]?.topics[3]
        ? parseInt(receipt.logs[0].topics[3], 16)
        : 0;

      // Update local state
      if (activeSessionIndex >= 0) {
        markSessionMinted(activeSessionIndex);
      }

      // Update backend
      if (backendAnkyId) {
        try {
          await recordAnkyMint(backendAnkyId, hash, tokenId);
        } catch (e) {
          console.error("Failed to record mint in backend:", e);
        }
      }

      alert("Your Anky has been minted!");
    } catch (e: unknown) {
      const error = e as { shortMessage?: string; message?: string };
      alert("Minting failed: " + (error.shortMessage || error.message));
    } finally {
      setIsMinting(false);
    }
  }, [
    user,
    currentAnkyData,
    currentSessionData,
    activeSessionIndex,
    markSessionMinted,
    handleAnkyDataUpdate,
    backendAnkyId,
    login,
  ]);

  const handleSessionSelect = useCallback(
    (idx: number) => {
      const session = sessions[idx];
      if (!session) return;
      setActiveSessionIndex(idx);
      setBackendSessionId(session.id || null);
      setBackendAnkyId(null);
      setCurrentSessionData({
        content: session.content,
        elapsed: session.duration,
        wordCount: session.stats?.wordCount || 0,
        wpm: session.stats?.wpm || 0,
        isFullSession:
          session.duration >= ANKY_THRESHOLD &&
          (session.content?.length || 0) >= 100,
      });
      setCurrentAnkyData(session.ankyData || null);
      setChatHistory(session.chatHistory || []);
    },
    [sessions]
  );

  return (
    <>
      <div className={`main ${showChatView || showDashboard ? "hidden" : ""} ${mainTransitioning ? "fade-to-chat" : ""}`}>
        <div className={`hero ${isWriting ? "fade-out" : ""}`}>
          <h1>YOUR MIND IS LOUD</h1>
          <p>let it speak</p>
        </div>

        <div className={`writing-container ${isWriting ? "fullscreen" : ""}`}>
          <textarea
            ref={textareaRef}
            className="writing-area"
            placeholder="tell me who you are..."
            value={content}
            onChange={(e) => handleInput(e.target.value)}
            onKeyDown={handleKeyDown}
            onPaste={(e) => e.preventDefault()}
            autoFocus
          />
          <div
            className={`session-bar ${isWriting ? "visible" : ""} ${isAnky ? "anky-ready" : ""}`}
          >
            <span className={`stat-pill ${isAnky ? "purple" : ""}`}>
              {formatDuration(duration)}
            </span>
            <div
              className={`timer-bar-wrapper ${!timerVisible ? "timer-hidden" : ""}`}
            >
              <div
                className={`timer-bar ${timerProgress < 25 ? "danger" : ""} ${isAnky ? "anky-bar" : ""}`}
                style={{ width: `${timerProgress}%` }}
              />
            </div>
            <span className={`stat-pill ${isAnky ? "purple" : ""}`}>
              {wordCount} words
            </span>
            {isAnky && <span className="stat-pill purple">anky</span>}
            <div className="keystroke-stats">
              <span className="keystroke-stat" title="Backspace attempts">
                ⌫ {keystrokeStats.backspace}
              </span>
              <span className="keystroke-stat" title="Enter attempts">
                ↵ {keystrokeStats.enter}
              </span>
              <span className="keystroke-stat" title="Arrow key attempts">
                ← {keystrokeStats.arrows}
              </span>
            </div>
          </div>
        </div>

        {/* Version */}
        {!isWriting && <span className="version-tag">v0.8.5</span>}
      </div>

      {isMobile() && (
        <MobileChatView
          visible={showChatView}
          sessionData={currentSessionData}
          ankyData={currentAnkyData}
          chatHistory={chatHistory}
          backendSessionId={backendSessionId}
          backendUserId={backendUser?.id}
          onAnkyDataUpdate={handleAnkyDataUpdate}
          onChatHistoryUpdate={handleChatHistoryUpdate}
          onBack={handleWriteAgain}
          onMint={handleMint}
          isMinting={isMinting}
        />
      )}

      {!isMobile() && (
        <DesktopDashboard
          visible={showDashboard}
          sessions={sessions}
          activeSessionIndex={activeSessionIndex}
          sessionData={currentSessionData}
          ankyData={currentAnkyData}
          chatHistory={chatHistory}
          backendSessionId={backendSessionId}
          backendUserId={backendUser?.id}
          onSessionSelect={handleSessionSelect}
          onAnkyDataUpdate={handleAnkyDataUpdate}
          onChatHistoryUpdate={handleChatHistoryUpdate}
          onWriteAgain={handleWriteAgain}
          onMint={handleMint}
          isMinting={isMinting}
        />
      )}
    </>
  );
}
