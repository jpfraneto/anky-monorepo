import { useEffect } from "react";
import type { GeneratedImageData } from "../api";
import { formatDate, formatTime } from "../utils/helpers";

interface ImageModalProps {
  image: GeneratedImageData | null;
  onClose: () => void;
  onNavigate?: (direction: "prev" | "next") => void;
  hasNavigation?: boolean;
}

export function ImageModal({
  image,
  onClose,
  onNavigate,
  hasNavigation = false,
}: ImageModalProps) {
  // Close on escape key, navigate with arrows
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      } else if (hasNavigation && onNavigate) {
        if (e.key === "ArrowLeft") {
          onNavigate("prev");
        } else if (e.key === "ArrowRight") {
          onNavigate("next");
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, onNavigate, hasNavigation]);

  if (!image) return null;

  return (
    <div className="image-modal-overlay" onClick={onClose}>
      <div className="image-modal" onClick={(e) => e.stopPropagation()}>
        <button className="image-modal-close" onClick={onClose}>
          &times;
        </button>

        {/* Navigation arrows */}
        {hasNavigation && onNavigate && (
          <>
            <button
              className="image-modal-nav image-modal-nav-prev"
              onClick={() => onNavigate("prev")}
            >
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="15 18 9 12 15 6"/>
              </svg>
            </button>
            <button
              className="image-modal-nav image-modal-nav-next"
              onClick={() => onNavigate("next")}
            >
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="9 18 15 12 9 6"/>
              </svg>
            </button>
          </>
        )}

        <div className="image-modal-content">
          <div className="image-modal-image-container">
            <img
              src={image.imageUrl}
              alt={image.prompt}
              className="image-modal-image"
            />
          </div>
          <div className="image-modal-details">
            <h3 className="image-modal-title">Prompt</h3>
            <p className="image-modal-prompt">{image.prompt}</p>

            <div className="image-modal-meta">
              <div className="image-modal-meta-item">
                <span className="image-modal-label">Model</span>
                <span className="image-modal-value">{image.model}</span>
              </div>
              {image.generationTimeMs > 0 && (
                <div className="image-modal-meta-item">
                  <span className="image-modal-label">Generation Time</span>
                  <span className="image-modal-value">
                    {(image.generationTimeMs / 1000).toFixed(2)}s
                  </span>
                </div>
              )}
              <div className="image-modal-meta-item">
                <span className="image-modal-label">Created</span>
                <span className="image-modal-value">
                  {formatDate(image.createdAt)} at {formatTime(image.createdAt)}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
