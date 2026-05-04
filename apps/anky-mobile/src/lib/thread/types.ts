export type ThreadMode = "complete" | "fragment" | "reflection";

export type ThreadRole = "anky" | "user";

export type ThreadMessage = {
  id: string;
  role: ThreadRole;
  content: string;
  createdAt: string;
};

export type AnkyThread = {
  version: 1;
  sessionHash: string;
  mode: ThreadMode;
  createdAt: string;
  updatedAt: string;
  messages: ThreadMessage[];
  userMessageCount: number;
};
