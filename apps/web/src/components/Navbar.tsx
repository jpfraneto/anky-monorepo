import { useNavigate } from "react-router-dom";
import { usePrivy } from "@privy-io/react-auth";

interface NavbarProps {
  onOpenHistory: () => void;
  isWriting?: boolean;
  streak?: number;
}

export function Navbar({ onOpenHistory, isWriting = false, streak }: NavbarProps) {
  const { login, logout, authenticated, user } = usePrivy();
  const navigate = useNavigate();

  const walletAddress = user?.wallet?.address;
  const displayAddress = walletAddress
    ? `${walletAddress.slice(0, 4)}...${walletAddress.slice(-3)}`
    : null;

  return (
    <nav className="navbar">
      <div className={`navbar-left ${isWriting ? "hidden" : ""}`}>
        <button
          className="navbar-btn icon-btn"
          onClick={onOpenHistory}
        >
          <div className="hamburger-icon">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </button>
        <button
          className="navbar-btn generate-nav-btn"
          onClick={() => navigate("/gallery")}
        >
          gallery
        </button>
      </div>
      <div className="navbar-right">
        {authenticated && streak !== undefined && streak > 0 && (
          <span className="streak-badge">{streak}</span>
        )}
        <button
          className={`navbar-btn ${authenticated ? "connected" : ""}`}
          onClick={authenticated ? logout : login}
        >
          {authenticated ? displayAddress || "connected" : "connect"}
        </button>
      </div>
    </nav>
  );
}
