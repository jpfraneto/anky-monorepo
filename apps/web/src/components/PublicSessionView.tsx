import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { getSessionByShareId, getAnkyBySessionId, type Session, type Anky } from "../api";
import { formatDuration, ANKY_THRESHOLD } from "../utils/helpers";

export function PublicSessionView() {
  const { shareId } = useParams<{ shareId: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<Session | null>(null);
  const [anky, setAnky] = useState<Anky | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!shareId) {
      setError("Invalid session link");
      setLoading(false);
      return;
    }

    async function fetchSession() {
      try {
        const sessionData = await getSessionByShareId(shareId!);
        setSession(sessionData);

        // If it's an anky session, try to fetch the anky data
        if (sessionData.isAnky) {
          const ankyData = await getAnkyBySessionId(sessionData.id);
          setAnky(ankyData);
        }
      } catch (e) {
        console.error("Failed to fetch session:", e);
        setError("Session not found or is private");
      } finally {
        setLoading(false);
      }
    }

    fetchSession();
  }, [shareId]);

  if (loading) {
    return (
      <div className="public-session-view">
        <div className="public-session-loading">
          <div className="status-spinner large" />
          <span>Loading session...</span>
        </div>
      </div>
    );
  }

  if (error || !session) {
    return (
      <div className="public-session-view">
        <div className="public-session-error">
          <h2>{error || "Session not found"}</h2>
          <button className="btn btn-secondary" onClick={() => navigate("/")}>
            Go Home
          </button>
        </div>
      </div>
    );
  }

  const isAnky = session.durationSeconds >= ANKY_THRESHOLD && session.content.length >= 100;
  const date = new Date(session.createdAt);
  const dateStr = date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div className="public-session-view">
      <div className="public-session-container">
        {/* Anky Image */}
        {anky?.imageUrl && (
          <div className="public-session-image-container">
            <img
              src={anky.imageUrl}
              alt={anky.title || "Anky"}
              className="public-session-image"
            />
          </div>
        )}

        {/* Title and timestamp */}
        <div className="public-session-header">
          <h1 className="public-session-title">
            {anky?.title || (isAnky ? "Anky Session" : "Writing Session")}
          </h1>
          <div className="public-session-meta">
            <span className="public-session-date">{dateStr}</span>
            <span className="public-session-stats">
              {formatDuration(session.durationSeconds)} | {session.wordCount} words
            </span>
          </div>
        </div>

        {/* Writing content */}
        <div className="public-session-content">
          {session.content}
        </div>

        {/* Back home button */}
        <div className="public-session-actions">
          <button className="btn btn-primary" onClick={() => navigate("/")}>
            Start Writing
          </button>
        </div>
      </div>
    </div>
  );
}
