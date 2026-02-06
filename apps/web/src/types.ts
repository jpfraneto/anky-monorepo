export interface KeystrokeStats {
  backspace: number;
  enter: number;
  arrows: number;
}

export interface WritingSession {
  id?: string;
  shareId?: string;
  content: string;
  duration: number;
  stats: {
    wordCount: number;
    wpm: number;
    keystrokes?: KeystrokeStats;
  };
  timestamp: number;
  ankyData?: AnkyData;
  ankyTitle?: string;
  chatHistory?: ChatMessage[];
  minted?: boolean;
}

export interface AnkyData {
  prompt?: string;
  reflection?: string;
  url?: string;
  title?: string;
  writingSessionIpfs?: string;
  imageIpfs?: string;
  imagePrompt?: string;
  tokenUri?: string;
  writingSession?: string;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface SessionData {
  content: string;
  elapsed: number;
  wordCount: number;
  wpm: number;
  isFullSession: boolean;
  keystrokeStats?: KeystrokeStats;
}

export interface GeneratedImage {
  id: string;
  prompt: string;
  imageBase64: string;
  imageUrl: string;
  model: string;
  generationTimeMs: number;
  createdAt: string;
}
