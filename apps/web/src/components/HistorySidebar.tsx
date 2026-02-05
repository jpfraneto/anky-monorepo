import type { WritingSession } from "../types";
import { formatDuration, ANKY_THRESHOLD } from "../utils/helpers";

interface HistorySidebarProps {
  isOpen: boolean;
  sessions: WritingSession[];
  onClose: () => void;
  onSessionSelect?: (sessionIndex: number) => void;
}

export function HistorySidebar({
  isOpen,
  sessions,
  onClose,
  onSessionSelect,
}: HistorySidebarProps) {
  return (
    <>
      <div
        className={`history-overlay ${isOpen ? "visible" : ""}`}
        onClick={onClose}
      />
      <div className={`history-sidebar ${isOpen ? "visible" : ""}`}>
        <div className="history-header">
          <span className="history-title">Sessions</span>
          <button className="history-close" onClick={onClose}>
            x
          </button>
        </div>
        <div className="history-list">
          {sessions.length === 0 ? (
            <div className="history-empty">No sessions yet. Start writing!</div>
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
                    className={`history-item ${isAnky ? "is-anky" : ""}`}
                    onClick={() => {
                      onSessionSelect?.(realIdx);
                      onClose();
                    }}
                  >
                    <div className="history-item-header">
                      <div
                        className={
                          isAnky ? "history-anky-badge" : "history-no-anky"
                        }
                      />
                      <span className="history-item-title">{title}</span>
                    </div>
                    <div className="history-item-meta">
                      <span>{timeStr}</span>
                      <span>{formatDuration(session.duration)}</span>
                      <span>{session.stats?.wordCount || 0} words</span>
                    </div>
                    <div className="history-item-preview">
                      {session.content?.substring(0, 100) || ""}
                    </div>
                  </div>
                );
              })
          )}
        </div>
      </div>
    </>
  );
}
