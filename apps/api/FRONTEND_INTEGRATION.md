# Frontend Integration Guide (React + Vite + Privy)

This guide explains how to connect your React Vite frontend to the Anky backend with Privy authentication.

## 1. Install Privy in Your Frontend

```bash
cd frontend
bun add @privy-io/react-auth
```

## 2. Set Up Privy Provider

In your `main.tsx` or `App.tsx`:

```tsx
import { PrivyProvider } from '@privy-io/react-auth';

const privyConfig = {
  appId: import.meta.env.VITE_PRIVY_APP_ID,
  config: {
    // Customize login methods
    loginMethods: ['wallet', 'email'],
    // Appearance
    appearance: {
      theme: 'dark',
      accentColor: '#8B5CF6', // Purple to match Anky
    },
    // Embedded wallets (optional)
    embeddedWallets: {
      createOnLogin: 'users-without-wallets',
    },
  },
};

function App() {
  return (
    <PrivyProvider {...privyConfig}>
      <YourApp />
    </PrivyProvider>
  );
}
```

## 3. Create API Client with Auth

Create a file `src/lib/api.ts`:

```typescript
import { usePrivy } from '@privy-io/react-auth';

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

// Create a hook for authenticated API calls
export function useApi() {
  const { getAccessToken, authenticated } = usePrivy();

  async function apiCall<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers as Record<string, string>,
    };

    // Add auth token if authenticated
    if (authenticated) {
      try {
        const token = await getAccessToken();
        if (token) {
          headers['Authorization'] = `Bearer ${token}`;
        }
      } catch (e) {
        console.warn('Failed to get access token:', e);
      }
    }

    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(error.error || `API Error: ${response.status}`);
    }

    return response.json();
  }

  return { apiCall, authenticated };
}

// Non-hook version for use outside components
export async function apiCallWithToken<T>(
  endpoint: string,
  token: string | null,
  options: RequestInit = {}
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...options.headers as Record<string, string>,
  };

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || `API Error: ${response.status}`);
  }

  return response.json();
}
```

## 4. Auth Component

Create `src/components/AuthButton.tsx`:

```tsx
import { usePrivy } from '@privy-io/react-auth';
import { useApi } from '../lib/api';
import { useEffect, useState } from 'react';

interface User {
  id: string;
  walletAddress: string;
  dayBoundaryHour: number;
  timezone: string;
}

export function AuthButton() {
  const { login, logout, authenticated, user: privyUser } = usePrivy();
  const { apiCall } = useApi();
  const [user, setUser] = useState<User | null>(null);

  // Create/fetch user when authenticated
  useEffect(() => {
    if (authenticated && privyUser?.wallet?.address) {
      apiCall<{ user: User }>('/users', {
        method: 'POST',
        body: JSON.stringify({
          walletAddress: privyUser.wallet.address,
        }),
      })
        .then(({ user }) => setUser(user))
        .catch(console.error);
    } else {
      setUser(null);
    }
  }, [authenticated, privyUser?.wallet?.address]);

  if (!authenticated) {
    return (
      <button onClick={login} className="btn-primary">
        Connect Wallet
      </button>
    );
  }

  return (
    <div className="flex items-center gap-4">
      <span className="text-sm text-gray-400">
        {privyUser?.wallet?.address?.slice(0, 6)}...
        {privyUser?.wallet?.address?.slice(-4)}
      </span>
      <button onClick={logout} className="btn-secondary">
        Disconnect
      </button>
    </div>
  );
}
```

## 5. Complete Writing Flow Example

Create `src/hooks/useAnkySession.ts`:

```typescript
import { useState } from 'react';
import { useApi } from '../lib/api';

interface AnkySession {
  session: {
    id: string;
    shareId: string;
    isAnky: boolean;
  };
  anky?: {
    id: string;
    title: string;
    reflection: string;
    imageUrl: string;
  };
  title?: string;
  reflection?: string;
  imageUrl?: string;
}

export function useAnkySession(userId?: string) {
  const { apiCall } = useApi();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submitSession(
    content: string,
    durationSeconds: number
  ): Promise<AnkySession> {
    setLoading(true);
    setError(null);

    try {
      const wordCount = content.trim().split(/\s+/).length;

      // 1. Create the session
      const { session } = await apiCall<{ session: AnkySession['session'] }>(
        '/sessions',
        {
          method: 'POST',
          body: JSON.stringify({
            userId,
            content,
            durationSeconds,
            wordCount,
          }),
        }
      );

      // If not an 8-minute session, return early
      if (!session.isAnky) {
        return { session };
      }

      // 2. Generate AI content in parallel
      const [promptRes, reflectionRes] = await Promise.all([
        apiCall<{ prompt: string }>('/prompt', {
          method: 'POST',
          body: JSON.stringify({ writingSession: content }),
        }),
        apiCall<{ reflection: string }>('/reflection', {
          method: 'POST',
          body: JSON.stringify({ writingSession: content }),
        }),
      ]);

      // 3. Generate image
      const { url: imageUrl, base64: imageBase64 } = await apiCall<{
        url: string;
        base64: string;
      }>('/image', {
        method: 'POST',
        body: JSON.stringify({ prompt: promptRes.prompt }),
      });

      // 4. Generate title
      const { title } = await apiCall<{ title: string }>('/title', {
        method: 'POST',
        body: JSON.stringify({
          writingSession: content,
          imagePrompt: promptRes.prompt,
          reflection: reflectionRes.reflection,
        }),
      });

      // 5. Create Anky record
      const { anky } = await apiCall<{ anky: AnkySession['anky'] }>('/ankys', {
        method: 'POST',
        body: JSON.stringify({
          writingSessionId: session.id,
          userId,
          imagePrompt: promptRes.prompt,
          reflection: reflectionRes.reflection,
          title,
          imageBase64,
          imageUrl,
        }),
      });

      return {
        session,
        anky,
        title,
        reflection: reflectionRes.reflection,
        imageUrl,
      };
    } catch (e) {
      const message = e instanceof Error ? e.message : 'Unknown error';
      setError(message);
      throw e;
    } finally {
      setLoading(false);
    }
  }

  return { submitSession, loading, error };
}
```

## 6. Chat Hook

Create `src/hooks/useChat.ts`:

```typescript
import { useState } from 'react';
import { useApi } from '../lib/api';

interface Message {
  role: 'user' | 'assistant';
  content: string;
}

export function useChat() {
  const { apiCall } = useApi();
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(false);

  async function sendMessage(
    userMessage: string,
    context: {
      writingSession: string;
      duration?: number;
      wordCount?: number;
      reflection?: string;
      title?: string;
    }
  ) {
    setLoading(true);

    const newMessages: Message[] = [
      ...messages,
      { role: 'user', content: userMessage },
    ];
    setMessages(newMessages);

    try {
      // Use appropriate endpoint based on session type
      const isFullSession = context.reflection && context.title;
      const endpoint = isFullSession ? '/chat' : '/chat-short';

      const body = isFullSession
        ? {
            writingSession: context.writingSession,
            reflection: context.reflection,
            title: context.title,
            history: newMessages,
          }
        : {
            writingSession: context.writingSession,
            duration: context.duration,
            wordCount: context.wordCount,
            history: newMessages,
          };

      const { response } = await apiCall<{ response: string }>(endpoint, {
        method: 'POST',
        body: JSON.stringify(body),
      });

      setMessages([...newMessages, { role: 'assistant', content: response }]);
      return response;
    } catch (e) {
      // Remove the user message on error
      setMessages(messages);
      throw e;
    } finally {
      setLoading(false);
    }
  }

  function clearMessages() {
    setMessages([]);
  }

  return { messages, sendMessage, clearMessages, loading };
}
```

## 7. Environment Variables

Create `.env` in your frontend:

```env
VITE_API_URL=http://localhost:3000/api
VITE_PRIVY_APP_ID=your_privy_app_id
```

## 8. CORS Configuration (if needed)

If you get CORS errors, add CORS middleware to the server. In `server/src/server.ts`:

```typescript
import { cors } from 'hono/cors';

// Add before routes
app.use('/*', cors({
  origin: ['http://localhost:5173', 'http://localhost:3000'], // Your frontend URLs
  credentials: true,
}));
```

## API Endpoints Summary

### Public (no auth required)
- `GET /api/` - Health check
- `GET /api/db/status` - Database status
- `GET /api/users/:wallet` - Get user by wallet
- `GET /api/sessions/:sessionId` - Get session
- `GET /api/s/:shareId` - Get public session
- `GET /api/sessions/:sessionId/anky` - Get anky for session
- `GET /api/feed` - Public anky feed
- `GET /api/conversations/:id/messages` - Get messages

### AI Generation (no auth, but rate-limit recommended)
- `POST /api/prompt` - Generate image prompt
- `POST /api/reflection` - Generate reflection
- `POST /api/image` - Generate image
- `POST /api/title` - Generate title
- `POST /api/ipfs` - Upload to IPFS
- `POST /api/chat` - Chat (full session)
- `POST /api/chat-short` - Chat (short session)

### Protected (auth required)
- `POST /api/users` - Create/get user
- `PATCH /api/users/:userId/settings` - Update settings
- `GET /api/users/:userId/streak` - Get streak
- `GET /api/users/:userId/ankys` - Get user's ankys
- `GET /api/users/:userId/sessions` - Get user's sessions
- `GET /api/users/:userId/conversations` - Get user's conversations
- `PATCH /api/sessions/:sessionId/privacy` - Toggle privacy
- `PATCH /api/ankys/:ankyId` - Update anky
- `POST /api/ankys/:ankyId/mint` - Record mint
- `POST /api/conversations/:id/close` - Close conversation

### Optional Auth (works with or without)
- `POST /api/sessions` - Create session
- `POST /api/ankys` - Create anky
- `POST /api/conversations` - Create conversation
- `POST /api/conversations/:id/messages` - Add message

## Privy Dashboard Setup

1. Go to https://dashboard.privy.io/
2. Create a new app
3. Copy your App ID and App Secret
4. Add them to:
   - Frontend: `VITE_PRIVY_APP_ID`
   - Backend: `PRIVY_APP_ID` and `PRIVY_APP_SECRET`
5. Configure allowed origins in the Privy dashboard
