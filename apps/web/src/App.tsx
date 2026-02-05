import { useState, useEffect, useMemo, useCallback } from "react";
import { Routes, Route, useLocation } from "react-router-dom";
import { usePrivy } from "@privy-io/react-auth";
import {
  fetchFeedHtml,
  setAuthTokenGetter,
  createOrGetUser,
  fetchMe,
  type MeResponse,
  type RecentSession,
} from "./api";
import { Navigate } from "react-router-dom";
import { Navbar, HistorySidebar, HomePage, GeneratePage, PublicSessionView, LandingPage } from "./components";
import type { WritingSession } from "./types";

// Convert backend RecentSession to frontend WritingSession format
function toWritingSession(session: RecentSession): WritingSession {
  return {
    id: session.id,
    shareId: session.shareId,
    content: session.fullContent || session.content,
    duration: session.durationSeconds,
    stats: {
      wordCount: session.wordCount,
      wpm: session.wpm,
    },
    timestamp: new Date(session.createdAt).getTime(),
    ankyData: session.anky
      ? {
          reflection: session.anky.reflection || undefined,
          url: session.anky.imageUrl || undefined,
          base64: session.anky.imageBase64 || undefined,
          title: session.anky.title || undefined,
          imagePrompt: session.anky.imagePrompt || undefined,
          writingSessionIpfs: session.anky.writingIpfsHash || undefined,
          imageIpfs: session.anky.imageIpfsHash || undefined,
          tokenUri: session.anky.metadataIpfsHash || undefined,
        }
      : undefined,
    ankyTitle: session.anky?.title || undefined,
    minted: session.anky?.isMinted || false,
  };
}

export default function App() {
  const { authenticated, user, getAccessToken } = usePrivy();
  const location = useLocation();

  // Set up auth token getter for API calls
  useEffect(() => {
    setAuthTokenGetter(getAccessToken);
  }, [getAccessToken]);

  const [historyOpen, setHistoryOpen] = useState(false);
  const [feedHtml, setFeedHtml] = useState("");
  const [meData, setMeData] = useState<MeResponse | null>(null);
  const [isWriting, setIsWriting] = useState(false);
  const [selectedSessionFromSidebar, setSelectedSessionFromSidebar] = useState<number | null>(null);

  // Convert backend sessions to frontend format
  const sessions = useMemo(() => {
    return (meData?.recentSessions || []).map(toWritingSession);
  }, [meData?.recentSessions]);

  // Refresh user data (called after new session created)
  const refreshUserData = useCallback(async () => {
    if (authenticated && user?.wallet?.address) {
      try {
        const data = await fetchMe();
        setMeData(data);
      } catch (e) {
        console.error("[App] Failed to refresh user data:", e);
      }
    }
  }, [authenticated, user?.wallet?.address]);

  // Load feed on mount
  useEffect(() => {
    fetchFeedHtml().then(setFeedHtml);
  }, []);

  // Fetch user data when authenticated
  useEffect(() => {
    if (authenticated && user?.wallet?.address) {
      console.log("[App] User authenticated, fetching /api/me...");

      // First ensure user exists in backend
      createOrGetUser(user.wallet.address)
        .then(() => {
          // Then fetch all user data
          return fetchMe();
        })
        .then((data) => {
          console.log("[App] /api/me response:", data);
          setMeData(data);
        })
        .catch((e) => {
          console.error("[App] Failed to fetch user data:", e);
        });
    } else {
      console.log("[App] User not authenticated, clearing data");
      setMeData(null);
    }
  }, [authenticated, user?.wallet?.address]);

  return (
    <>
      {/* Feed background (only on write page) */}
      <div
        className={`feed ${location.pathname !== "/write" ? "fade-out" : ""}`}
        dangerouslySetInnerHTML={{ __html: feedHtml }}
      />

      {/* Navbar */}
      <Navbar
        onOpenHistory={() => setHistoryOpen(true)}
        isWriting={isWriting}
        streak={meData?.streak.current}
      />

      {/* History Sidebar */}
      <HistorySidebar
        isOpen={historyOpen}
        sessions={sessions}
        onClose={() => setHistoryOpen(false)}
        onSessionSelect={(idx) => setSelectedSessionFromSidebar(idx)}
      />

      {/* Routes */}
      <Routes>
        <Route path="/" element={<LandingPage />} />
        <Route
          path="/write"
          element={
            <HomePage
              backendUser={meData?.user || null}
              userStats={meData?.stats}
              onWritingStateChange={setIsWriting}
              initialSessionIndex={selectedSessionFromSidebar}
              onSessionHandled={() => setSelectedSessionFromSidebar(null)}
              onSessionsChange={refreshUserData}
            />
          }
        />
        <Route path="/gallery" element={<GeneratePage />} />
        <Route path="/generate" element={<Navigate to="/gallery" replace />} />
        <Route path="/session/:shareId" element={<PublicSessionView />} />
      </Routes>
    </>
  );
}
