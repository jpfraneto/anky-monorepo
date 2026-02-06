const API_URL = import.meta.env.VITE_API_URL || "";
console.log("[API] Initialized with API_URL:", API_URL || "(empty - using relative URLs)");

let getAuthToken: (() => Promise<string | null>) | null = null;

export function setAuthTokenGetter(getter: () => Promise<string | null>) {
  getAuthToken = getter;
}

export async function fetchAPI<T>(
  endpoint: string,
  body: Record<string, unknown>,
  method: "POST" | "GET" | "PATCH" = "POST"
): Promise<T> {
  const url = `${API_URL}${endpoint}`;
  console.log(`[API] ${method} ${url}`);
  console.log("[API] Request body:", body);

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (getAuthToken) {
    try {
      const token = await getAuthToken();
      console.log("[API] Auth token:", token ? `${token.substring(0, 20)}...` : "null");
      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }
    } catch (e) {
      console.error("[API] Failed to get auth token:", e);
    }
  } else {
    console.log("[API] No auth token getter configured");
  }

  const options: RequestInit = {
    method,
    headers,
  };

  if (method !== "GET") {
    options.body = JSON.stringify(body);
  }

  console.log("[API] Request options:", { method, headers: Object.keys(headers) });

  try {
    const response = await fetch(url, options);
    console.log("[API] Response status:", response.status, response.statusText);

    if (!response.ok) {
      const errorText = await response.text();
      console.error("[API] Error response:", errorText);
      throw new Error(errorText || `Request failed with status ${response.status}`);
    }

    const data = await response.json();
    console.log("[API] Response data:", data);
    return data;
  } catch (e) {
    console.error("[API] Fetch failed:", e);
    console.error("[API] URL was:", url);
    console.error("[API] API_URL env:", API_URL || "(empty)");
    throw e;
  }
}

export async function fetchGet<T>(endpoint: string): Promise<T> {
  const url = `${API_URL}${endpoint}`;
  console.log(`[API] GET ${url}`);

  const headers: Record<string, string> = {};

  if (getAuthToken) {
    try {
      const token = await getAuthToken();
      console.log("[API] Auth token:", token ? `${token.substring(0, 20)}...` : "null");
      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }
    } catch (e) {
      console.error("[API] Failed to get auth token:", e);
    }
  }

  try {
    const response = await fetch(url, {
      method: "GET",
      headers,
    });
    console.log("[API] Response status:", response.status, response.statusText);

    if (!response.ok) {
      const errorText = await response.text();
      console.error("[API] Error response:", errorText);
      throw new Error(errorText || `Request failed with status ${response.status}`);
    }

    const data = await response.json();
    console.log("[API] Response data:", data);
    return data;
  } catch (e) {
    console.error("[API] GET failed:", e);
    console.error("[API] URL was:", url);
    console.error("[API] API_URL env:", API_URL || "(empty)");
    throw e;
  }
}

export async function fetchFeedHtml(): Promise<string> {
  const headers: Record<string, string> = {};

  if (getAuthToken) {
    const token = await getAuthToken();
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }
  }

  const response = await fetch(`${API_URL}/api/feed-html`, {
    method: "GET",
    headers,
  });

  if (!response.ok) {
    return "";
  }

  return response.text();
}

// API response types
export interface PromptResponse {
  prompt: string;
}

export interface ReflectionResponse {
  reflection: string;
}

export interface ImageResponse {
  url: string;
  base64?: string;
  id?: string;
}

export interface TitleResponse {
  title: string;
}

export interface IpfsResponse {
  writingSessionIpfs: string;
  imageIpfs: string;
  imageUrl: string;
  tokenUri: string;
}

export interface ChatResponse {
  response: string;
}

export interface GenerateImageResponse {
  url: string;
  base64?: string;
}

// User types
export interface User {
  id: string;
  walletAddress: string;
  dayBoundaryHour: number;
  timezone: string;
  createdAt: string;
}

export interface UserStreak {
  current: number;
  longest: number;
  isActive: boolean;
  hasWrittenToday: boolean;
}

export interface UserStats {
  totalAnkys: number;
  totalSessions: number;
  totalWords: number;
  totalTimeSeconds: number;
  averageWpm: number;
  averageSessionSeconds: number;
}

export interface RecentSessionAnky {
  id: string;
  title: string | null;
  imageUrl: string | null;
  reflection: string | null;
  imagePrompt: string | null;
  writingIpfsHash: string | null;
  imageIpfsHash: string | null;
  metadataIpfsHash: string | null;
  isMinted: boolean;
  tokenId: number | null;
}

export interface RecentSession {
  id: string;
  shareId: string;
  content: string;
  fullContent: string;
  durationSeconds: number;
  wordCount: number;
  wpm: number;
  isAnky: boolean;
  createdAt: string;
  anky: RecentSessionAnky | null;
}

export interface MeResponse {
  user: User;
  streak: UserStreak;
  stats: UserStats;
  recentSessions: RecentSession[];
}

export interface UserResponse {
  user: User;
}

// Session types
export interface Session {
  id: string;
  userId?: string;
  content: string;
  durationSeconds: number;
  wordCount: number;
  wordsPerMinute?: number;
  isAnky: boolean;
  shareId: string;
  isPublic: boolean;
  createdAt: string;
}

export interface SessionResponse {
  session: Session;
}

// Anky types
export interface Anky {
  id: string;
  writingSessionId: string;
  userId?: string;
  imagePrompt?: string;
  reflection?: string;
  title?: string;
  imageUrl?: string;
  writingIpfsHash?: string;
  imageIpfsHash?: string;
  metadataIpfsHash?: string;
  isMinted: boolean;
  mintTxHash?: string;
  tokenId?: number;
  mintedAt?: string;
  createdAt: string;
}

export interface AnkyResponse {
  anky: Anky;
}

// Streak types
export interface Streak {
  currentStreak: number;
  longestStreak: number;
  lastAnkyDate: string | null;
  totalAnkys: number;
  totalWritingSessions: number;
  totalWordsWritten: number;
  totalTimeWrittenSeconds: number;
  hasWrittenToday: boolean;
  isActive: boolean;
}

export interface StreakResponse {
  streak: Streak;
}

// API functions for user management
export async function createOrGetUser(walletAddress: string): Promise<User> {
  const response = await fetchAPI<UserResponse>("/api/users", { walletAddress });
  return response.user;
}

export async function fetchMe(): Promise<MeResponse> {
  return fetchGet<MeResponse>("/api/me");
}

export async function getUser(walletAddress: string): Promise<User | null> {
  try {
    const response = await fetchGet<UserResponse>(`/api/users/${walletAddress}`);
    return response.user;
  } catch {
    return null;
  }
}

export async function getUserStreak(userId: string): Promise<Streak> {
  const response = await fetchGet<StreakResponse>(`/api/users/${userId}/streak`);
  return response.streak;
}

// API functions for sessions
export async function createSession(data: {
  userId?: string;
  content: string;
  durationSeconds: number;
  wordCount: number;
  wordsPerMinute?: number;
  isPublic?: boolean;
}): Promise<Session> {
  const response = await fetchAPI<SessionResponse>("/api/sessions", data);
  return response.session;
}

export async function getSessionByShareId(shareId: string): Promise<Session> {
  const response = await fetchGet<SessionResponse>(`/api/s/${shareId}`);
  return response.session;
}

export async function getAnkyBySessionId(sessionId: string): Promise<Anky | null> {
  try {
    const response = await fetchGet<AnkyResponse>(`/api/sessions/${sessionId}/anky`);
    return response.anky;
  } catch {
    return null;
  }
}

// API functions for ankys
export async function createAnky(data: {
  writingSessionId: string;
  userId?: string;
  imagePrompt?: string;
  reflection?: string;
  title?: string;
  imageUrl?: string;
  writingIpfsHash?: string;
  imageIpfsHash?: string;
  metadataIpfsHash?: string;
  generatedImageId?: string;
}): Promise<Anky> {
  const response = await fetchAPI<AnkyResponse>("/api/ankys", data);
  return response.anky;
}

export async function updateAnky(
  ankyId: string,
  data: Partial<Anky>
): Promise<Anky> {
  const response = await fetchAPI<AnkyResponse>(
    `/api/ankys/${ankyId}`,
    data,
    "PATCH"
  );
  return response.anky;
}

export async function recordAnkyMint(
  ankyId: string,
  txHash: string,
  tokenId: number
): Promise<Anky> {
  const response = await fetchAPI<AnkyResponse>(`/api/ankys/${ankyId}/mint`, {
    txHash,
    tokenId,
  });
  return response.anky;
}

// Generated images types and functions
export interface GeneratedImageData {
  id: string;
  prompt: string;
  imageBase64: string;
  imageUrl: string;
  model: string;
  generationTimeMs: number;
  createdAt: string;
}

export interface GeneratedImagesResponse {
  images: GeneratedImageData[];
}

export async function getGeneratedImages(
  limit = 50,
  offset = 0
): Promise<GeneratedImageData[]> {
  const response = await fetchGet<GeneratedImagesResponse>(
    `/api/images?limit=${limit}&offset=${offset}`
  );
  return response.images;
}

export async function getGeneratedImage(
  imageId: string
): Promise<GeneratedImageData> {
  return fetchGet<GeneratedImageData>(`/api/images/${imageId}`);
}

// Gallery ankys (from writing sessions)
export interface GalleryAnky {
  id: string;
  title: string | null;
  imageUrl: string;
  reflection: string | null;
  createdAt: string;
  writerType?: "human" | "agent";
  session: {
    shareId: string;
    wordCount: number;
    durationSeconds: number;
  };
}

export interface GalleryAnkysResponse {
  ankys: GalleryAnky[];
  total: number;
  hasMore: boolean;
}

export type WriterTypeFilter = "all" | "human" | "agent";

export async function getGalleryAnkys(
  limit = 50,
  offset = 0,
  writerType: WriterTypeFilter = "all"
): Promise<GalleryAnkysResponse> {
  const params = new URLSearchParams({
    limit: limit.toString(),
    offset: offset.toString(),
  });
  if (writerType !== "all") {
    params.set("writerType", writerType);
  }
  return fetchGet<GalleryAnkysResponse>(`/api/ankys?${params.toString()}`);
}
