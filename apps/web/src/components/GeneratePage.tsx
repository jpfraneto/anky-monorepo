import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  fetchAPI,
  getGeneratedImages,
  getGalleryAnkys,
  fetchMe,
  type GenerateImageResponse,
  type GeneratedImageData,
  type RecentSession,
  type GalleryAnky,
  type WriterTypeFilter,
} from "../api";
import { ImageModal } from "./ImageModal";

type GalleryViewMode = "coverflow" | "grid";
type GallerySource = "community" | "mine" | "generated";

interface GalleryItem {
  id: string;
  imageUrl: string;
  prompt: string;
  title: string | null;
  shareId?: string;
  createdAt: string;
  wordCount?: number;
  durationSeconds?: number;
  writerType?: "human" | "agent";
}

export function GeneratePage() {
  const navigate = useNavigate();
  const [prompt, setPrompt] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [streak, setStreak] = useState<number | undefined>(undefined);

  // Loading states - show gallery as soon as current source is loaded
  const [communityLoading, setCommunityLoading] = useState(true);
  const [userLoading, setUserLoading] = useState(true);
  const [generatedLoading, setGeneratedLoading] = useState(true);

  // Gallery state
  const [galleryImages, setGalleryImages] = useState<GeneratedImageData[]>([]);
  const [myAnkys, setMyAnkys] = useState<GalleryItem[]>([]);
  const [communityAnkys, setCommunityAnkys] = useState<GalleryItem[]>([]);
  const [selectedImage, setSelectedImage] = useState<GeneratedImageData | null>(null);
  const [viewMode, setViewMode] = useState<GalleryViewMode>("coverflow");
  const [gallerySource, setGallerySource] = useState<GallerySource>("community");
  const [featuredIndex, setFeaturedIndex] = useState(0);
  const [writerTypeFilter, setWriterTypeFilter] = useState<WriterTypeFilter>("all");

  // Check if current gallery source is loading
  const isCurrentSourceLoading =
    (gallerySource === "community" && communityLoading) ||
    (gallerySource === "mine" && userLoading) ||
    (gallerySource === "generated" && generatedLoading);

  // Fetch community ankys
  useEffect(() => {
    setCommunityLoading(true);
    getGalleryAnkys(50, 0, writerTypeFilter)
      .then((data) => {
        const items: GalleryItem[] = data.ankys.map((anky: GalleryAnky) => ({
          id: anky.id,
          imageUrl: anky.imageUrl,
          prompt: anky.reflection || "",
          title: anky.title,
          shareId: anky.session.shareId,
          createdAt: anky.createdAt,
          wordCount: anky.session.wordCount,
          durationSeconds: anky.session.durationSeconds,
          writerType: anky.writerType || "human",
        }));
        setCommunityAnkys(items);
      })
      .catch((e) => {
        console.error("Failed to fetch community ankys:", e);
      })
      .finally(() => {
        setCommunityLoading(false);
      });
  }, [writerTypeFilter]);

  // Fetch user data and their ankys
  useEffect(() => {
    setUserLoading(true);
    fetchMe()
      .then((data) => {
        setStreak(data?.streak?.current);

        const userAnkys: GalleryItem[] = data?.recentSessions
          ?.filter((session: RecentSession) => session.anky?.imageUrl)
          .map((session: RecentSession) => ({
            id: session.anky!.id,
            imageUrl: session.anky!.imageUrl!,
            prompt: session.anky!.reflection || "",
            title: session.anky!.title,
            shareId: session.shareId,
            createdAt: session.createdAt,
            wordCount: session.wordCount,
            durationSeconds: session.durationSeconds,
          })) || [];

        setMyAnkys(userAnkys);
      })
      .catch((e) => {
        console.error("Failed to fetch user data:", e);
      })
      .finally(() => {
        setUserLoading(false);
      });
  }, []);

  // Fetch generated images
  useEffect(() => {
    setGeneratedLoading(true);
    getGeneratedImages(50, 0)
      .then((images) => {
        setGalleryImages(images);
      })
      .catch((e) => {
        console.error("Failed to fetch gallery:", e);
      })
      .finally(() => {
        setGeneratedLoading(false);
      });
  }, []);

  // Reset featured index when switching sources
  useEffect(() => {
    setFeaturedIndex(0);
  }, [gallerySource]);

  const handleGenerate = async () => {
    if (!prompt.trim()) return;

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetchAPI<GenerateImageResponse>("/api/image", {
        prompt: prompt.trim(),
      });

      const newImage: GeneratedImageData = {
        id: `temp-${Date.now()}`,
        prompt: prompt.trim(),
        imageUrl: response.url,
        imageBase64: response.base64 || "",
        model: "unknown",
        generationTimeMs: 0,
        createdAt: new Date().toISOString(),
      };

      setGalleryImages((prev) => [newImage, ...prev]);
      setGallerySource("generated");
      setFeaturedIndex(0);
      setPrompt("");
    } catch (e: unknown) {
      const err = e as { message?: string };
      setError(err.message || "Failed to generate image");
    } finally {
      setIsLoading(false);
    }
  };

  // Get current gallery items based on source
  const getCurrentGalleryItems = (): (GeneratedImageData & { _shareId?: string; _isAnky?: boolean; _writerType?: "human" | "agent" })[] => {
    if (gallerySource === "community") {
      return communityAnkys.map(item => ({
        id: item.id,
        imageUrl: item.imageUrl,
        prompt: item.title || item.prompt,
        imageBase64: "",
        model: "",
        generationTimeMs: 0,
        createdAt: item.createdAt,
        _shareId: item.shareId,
        _isAnky: true,
        _writerType: item.writerType,
      }));
    } else if (gallerySource === "mine") {
      return myAnkys.map(item => ({
        id: item.id,
        imageUrl: item.imageUrl,
        prompt: item.title || item.prompt,
        imageBase64: "",
        model: "",
        generationTimeMs: 0,
        createdAt: item.createdAt,
        _shareId: item.shareId,
        _isAnky: true,
        _writerType: item.writerType,
      }));
    } else {
      return galleryImages;
    }
  };

  const currentGalleryItems = getCurrentGalleryItems();

  const navigateGallery = (direction: "prev" | "next") => {
    if (currentGalleryItems.length === 0) return;

    setFeaturedIndex((prev) => {
      if (direction === "prev") {
        return prev === 0 ? currentGalleryItems.length - 1 : prev - 1;
      } else {
        return prev === currentGalleryItems.length - 1 ? 0 : prev + 1;
      }
    });
  };

  const handleImageClick = (image: GeneratedImageData & { _shareId?: string; _isAnky?: boolean; _writerType?: "human" | "agent" }, index: number) => {
    if (viewMode === "coverflow") {
      setFeaturedIndex(index);
    }

    if (image._isAnky && image._shareId) {
      navigate(`/session/${image._shareId}`);
    } else {
      setSelectedImage(image);
    }
  };

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLTextAreaElement) return;

      if (viewMode === "coverflow" && currentGalleryItems.length > 0) {
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          navigateGallery("prev");
        } else if (e.key === "ArrowRight") {
          e.preventDefault();
          navigateGallery("next");
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [viewMode, currentGalleryItems.length]);

  const featuredImage = currentGalleryItems[featuredIndex] as (GeneratedImageData & { _shareId?: string; _isAnky?: boolean; _writerType?: "human" | "agent" }) | undefined;

  const getEmptyMessage = () => {
    switch (gallerySource) {
      case "community":
        return "No community ankys yet. Be the first to create one!";
      case "mine":
        return "No ankys yet. Complete an 8-minute writing session to create one!";
      case "generated":
        return "No images generated yet. Create your first one!";
    }
  };

  // Loading screen - only show if current source is loading AND has no cached data
  const hasCurrentSourceData =
    (gallerySource === "community" && communityAnkys.length > 0) ||
    (gallerySource === "mine" && myAnkys.length > 0) ||
    (gallerySource === "generated" && galleryImages.length > 0);

  if (isCurrentSourceLoading && !hasCurrentSourceData) {
    return (
      <div className="relative z-[1] h-dvh flex items-center justify-center pt-[calc(var(--navbar-height)+1.5rem)] pb-[calc(1.5rem+var(--safe-bottom))]">
        <div className="flex flex-col items-center gap-6">
          <div className="w-12 h-12 border-3 border-purple-500/20 border-t-purple-500 rounded-full animate-spin" />
          <span className="text-zinc-500 text-base">Loading gallery...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="relative z-[1] h-dvh flex items-center justify-center p-6 pt-[calc(var(--navbar-height)+1.5rem)] pb-[calc(1.5rem+var(--safe-bottom))]">
      <div className="w-full max-w-[1400px] h-full grid grid-cols-[minmax(280px,380px)_1fr] gap-8 max-lg:grid-cols-1 max-lg:gap-6">

        {/* Left side - Generate form */}
        <div className="flex flex-col gap-4 w-full py-4 max-lg:order-1">
          {/* Streak display */}
          {streak !== undefined && streak > 0 && (
            <div className="flex items-center justify-center gap-2 px-4 py-2 bg-gradient-to-br from-amber-400/15 to-yellow-500/10 border border-amber-400/30 rounded-full mb-2">
              <span className="text-xl">🔥</span>
              <span className="text-2xl font-semibold text-amber-400">{streak}</span>
              <span className="text-sm text-zinc-500">day streak</span>
            </div>
          )}

          <h1 className="text-2xl font-light text-white text-center mb-1">Generate Anky</h1>
          <p className="text-sm text-zinc-500 text-center mb-3">
            Describe the anky you want to create
          </p>

          <textarea
            className="w-full min-h-[180px] p-4 font-sans text-base leading-relaxed text-white bg-white/[0.03] border border-white/[0.08] rounded-xl resize-y outline-none caret-purple-500 transition-all duration-300 placeholder:text-zinc-500 focus:border-purple-500/30 focus:bg-white/[0.05]"
            placeholder="A mystical creature with golden eyes and purple wings..."
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && e.metaKey) {
                handleGenerate();
              }
            }}
          />

          <button
            className="py-3.5 px-8 text-base font-medium text-zinc-900 bg-purple-500 border-none rounded-full cursor-pointer transition-all duration-300 hover:bg-purple-600 hover:-translate-y-0.5 disabled:opacity-50 disabled:cursor-not-allowed disabled:translate-y-0"
            onClick={handleGenerate}
            disabled={isLoading || !prompt.trim()}
          >
            {isLoading ? "generating..." : "generate"}
          </button>

          {isLoading && (
            <div className="flex items-center justify-center gap-2 text-purple-500 text-sm mt-2">
              <div className="w-5 h-5 border-2 border-purple-500/20 border-t-purple-500 rounded-full animate-spin" />
              <span>creating your anky...</span>
            </div>
          )}

          {error && (
            <div className="text-red-400 text-center text-sm mt-2">
              {error}
            </div>
          )}
        </div>

        {/* Right side - Gallery */}
        <div className="flex flex-col h-full max-h-[calc(100vh-var(--navbar-height)-3rem)] overflow-hidden max-lg:order-2 max-lg:max-h-[45vh]">
          {/* Controls */}
          <div className="flex items-center gap-3 mb-4 flex-wrap">
            {/* Source toggle */}
            <div className="flex bg-white/[0.05] rounded-lg p-1 gap-1">
              {(["community", "mine", "generated"] as const).map((source) => (
                <button
                  key={source}
                  className={`px-3 py-1.5 text-sm rounded-md transition-all duration-200 border-none cursor-pointer ${
                    gallerySource === source
                      ? "bg-purple-500/20 text-purple-500"
                      : "bg-transparent text-zinc-500 hover:text-white"
                  }`}
                  onClick={() => setGallerySource(source)}
                >
                  {source === "community" ? "Community" : source === "mine" ? "Mine" : "Generated"}
                </button>
              ))}
            </div>

            {/* Writer type filter (only show for community) */}
            {gallerySource === "community" && (
              <div className="flex bg-white/[0.05] rounded-lg p-1 gap-1">
                {(["all", "human", "agent"] as const).map((type) => (
                  <button
                    key={type}
                    className={`px-3 py-1.5 text-sm rounded-md transition-all duration-200 border-none cursor-pointer ${
                      writerTypeFilter === type
                        ? "bg-amber-500/20 text-amber-400"
                        : "bg-transparent text-zinc-500 hover:text-white"
                    }`}
                    onClick={() => setWriterTypeFilter(type)}
                  >
                    {type === "all" ? "All" : type === "human" ? "Human" : "Agent"}
                  </button>
                ))}
              </div>
            )}

            {/* View mode buttons */}
            <div className="flex gap-1">
              <button
                className={`w-9 h-9 flex items-center justify-center rounded-lg border transition-all duration-200 cursor-pointer ${
                  viewMode === "coverflow"
                    ? "bg-purple-500/20 border-purple-500 text-purple-500"
                    : "bg-white/[0.05] border-white/10 text-zinc-500 hover:text-white"
                }`}
                onClick={() => setViewMode("coverflow")}
                title="Coverflow view"
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="6" width="6" height="12" rx="1" opacity="0.4"/>
                  <rect x="9" y="4" width="6" height="16" rx="1"/>
                  <rect x="16" y="6" width="6" height="12" rx="1" opacity="0.4"/>
                </svg>
              </button>
              <button
                className={`w-9 h-9 flex items-center justify-center rounded-lg border transition-all duration-200 cursor-pointer ${
                  viewMode === "grid"
                    ? "bg-purple-500/20 border-purple-500 text-purple-500"
                    : "bg-white/[0.05] border-white/10 text-zinc-500 hover:text-white"
                }`}
                onClick={() => setViewMode("grid")}
                title="Grid view"
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="3" y="3" width="7" height="7"/>
                  <rect x="14" y="3" width="7" height="7"/>
                  <rect x="14" y="14" width="7" height="7"/>
                  <rect x="3" y="14" width="7" height="7"/>
                </svg>
              </button>
            </div>

            <span className="ml-auto text-sm text-zinc-500">{currentGalleryItems.length}</span>
          </div>

          {/* Gallery content */}
          {currentGalleryItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-4 text-zinc-500 h-[300px] text-center">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" opacity="0.3">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                <circle cx="8.5" cy="8.5" r="1.5"/>
                <polyline points="21 15 16 10 5 21"/>
              </svg>
              <span>{getEmptyMessage()}</span>
            </div>
          ) : viewMode === "coverflow" ? (
            /* Coverflow view */
            <div className="flex flex-col items-center h-full gap-4">
              {/* Featured image */}
              <div className="flex-1 flex flex-col items-center justify-center gap-4 min-h-0">
                {featuredImage && (
                  <>
                    <div className="relative max-w-full max-h-[calc(100%-3rem)]">
                      <img
                        src={featuredImage.imageUrl}
                        alt={featuredImage.prompt}
                        className="aspect-square object-contain rounded-2xl cursor-pointer transition-transform duration-300 hover:scale-[1.02] shadow-[0_0_30px_rgba(168,85,247,0.3)]"
                        onClick={() => handleImageClick(featuredImage, featuredIndex)}
                      />
                      {featuredImage._writerType === "agent" && (
                        <span className="absolute top-3 right-3 px-2 py-1 text-xs font-medium bg-purple-500/90 text-white rounded-full">
                          AI Agent
                        </span>
                      )}
                    </div>
                    <p className="max-w-[500px] text-center text-zinc-500 text-sm leading-relaxed m-0">
                      {featuredImage.prompt.length > 80
                        ? featuredImage.prompt.substring(0, 80) + "..."
                        : featuredImage.prompt}
                    </p>
                    {featuredImage._isAnky && (
                      <span className="text-xs text-purple-500 opacity-70">Click to view session</span>
                    )}
                  </>
                )}
              </div>

              {/* Navigation */}
              <div className="flex items-center gap-2 flex-shrink-0">
                <button
                  className="w-10 h-10 flex items-center justify-center bg-white/[0.05] border border-white/10 rounded-full text-white cursor-pointer transition-all duration-200 hover:bg-purple-500/20 hover:border-purple-500 hover:text-purple-500 disabled:opacity-30 disabled:cursor-not-allowed"
                  onClick={() => navigateGallery("prev")}
                  disabled={currentGalleryItems.length <= 1}
                >
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <polyline points="15 18 9 12 15 6"/>
                  </svg>
                </button>

                <div className="flex items-center gap-2 overflow-x-auto max-w-[400px] p-2 scrollbar-hide">
                  {currentGalleryItems.map((img, index) => {
                    const distance = Math.abs(index - featuredIndex);
                    const isVisible = distance <= 3 ||
                      (featuredIndex < 3 && index < 7) ||
                      (featuredIndex > currentGalleryItems.length - 4 && index > currentGalleryItems.length - 8);

                    if (!isVisible) return null;

                    const item = img as GeneratedImageData & { _shareId?: string; _isAnky?: boolean; _writerType?: "human" | "agent" };
                    return (
                      <div
                        key={item.id}
                        className={`flex-shrink-0 w-[50px] h-[50px] rounded-lg overflow-hidden cursor-pointer transition-all duration-300 border-2 ${
                          index === featuredIndex
                            ? "border-purple-500 shadow-[0_0_15px_rgba(168,85,247,0.4)]"
                            : "border-transparent hover:border-white/30"
                        }`}
                        onClick={() => handleImageClick(item, index)}
                        style={{
                          transform: `scale(${index === featuredIndex ? 1 : 0.8})`,
                          opacity: index === featuredIndex ? 1 : 0.6,
                        }}
                      >
                        <img src={item.imageUrl} alt={item.prompt} className="w-full h-full object-cover" />
                      </div>
                    );
                  })}
                </div>

                <button
                  className="w-10 h-10 flex items-center justify-center bg-white/[0.05] border border-white/10 rounded-full text-white cursor-pointer transition-all duration-200 hover:bg-purple-500/20 hover:border-purple-500 hover:text-purple-500 disabled:opacity-30 disabled:cursor-not-allowed"
                  onClick={() => navigateGallery("next")}
                  disabled={currentGalleryItems.length <= 1}
                >
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <polyline points="9 18 15 12 9 6"/>
                  </svg>
                </button>
              </div>

              <div className="text-sm text-zinc-500 flex-shrink-0">
                {featuredIndex + 1} / {currentGalleryItems.length}
              </div>
            </div>
          ) : (
            /* Grid view */
            <div className="grid grid-cols-[repeat(auto-fill,minmax(130px,1fr))] gap-3 overflow-y-auto pr-2 flex-1 max-lg:grid-cols-[repeat(auto-fill,minmax(80px,1fr))]">
              {currentGalleryItems.map((img, index) => {
                const item = img as GeneratedImageData & { _shareId?: string; _isAnky?: boolean; _writerType?: "human" | "agent" };
                return (
                  <div
                    key={item.id}
                    className={`relative aspect-square rounded-xl overflow-hidden cursor-pointer transition-all duration-200 hover:scale-[1.03] hover:shadow-[0_0_20px_rgba(168,85,247,0.3)] ${
                      item._isAnky ? "border-2 border-transparent hover:border-amber-400 hover:shadow-[0_0_20px_rgba(251,191,36,0.3)]" : ""
                    }`}
                    onClick={() => handleImageClick(item, index)}
                  >
                    <img
                      src={item.imageUrl}
                      alt={item.prompt}
                      className="w-full h-full object-cover"
                    />
                    {item._writerType === "agent" && (
                      <span className="absolute top-2 right-2 px-1.5 py-0.5 text-[10px] font-medium bg-purple-500/80 text-white rounded-full">
                        AI
                      </span>
                    )}
                    <div className="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/85 to-transparent opacity-0 transition-opacity duration-200 hover:opacity-100">
                      <p className="text-xs text-white m-0 leading-tight">
                        {item.prompt.length > 60
                          ? item.prompt.substring(0, 60) + "..."
                          : item.prompt}
                      </p>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <ImageModal
        image={selectedImage}
        onClose={() => setSelectedImage(null)}
        onNavigate={(direction) => {
          if (!selectedImage || gallerySource !== "generated") return;
          const currentIndex = galleryImages.findIndex(img => img.id === selectedImage.id);
          if (currentIndex === -1) return;

          const newIndex = direction === "prev"
            ? (currentIndex === 0 ? galleryImages.length - 1 : currentIndex - 1)
            : (currentIndex === galleryImages.length - 1 ? 0 : currentIndex + 1);

          setSelectedImage(galleryImages[newIndex]);
          setFeaturedIndex(newIndex);
        }}
        hasNavigation={gallerySource === "generated" && galleryImages.length > 1}
      />
    </div>
  );
}
