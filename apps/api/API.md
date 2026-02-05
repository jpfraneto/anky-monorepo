# Anky Backend API Documentation

**Base URL:** `http://localhost:3000/api`

## Overview

Anky is an AI-powered journaling platform where users write 8-minute stream-of-consciousness sessions. The backend generates:
- Reflections on the writing
- AI-generated images representing the emotional truth
- NFT metadata for minting

## Authentication

All protected endpoints require a `Authorization: Bearer <privy_token>` header. The token is obtained from Privy after user authentication on the frontend.

---

## Endpoints

### Health Check

#### `GET /`
Check if API is running.

**Response:**
```json
{ "status": "ok" }
```

#### `GET /db/status`
Check database availability.

**Response:**
```json
{ "available": true }
```

---

### AI Generation Endpoints

These endpoints power the core Anky experience.

#### `POST /prompt`
Generate an image prompt from a writing session.

**Request:**
```json
{
  "writingSession": "string (the user's raw writing)"
}
```

**Response:**
```json
{
  "prompt": "string (2-3 sentence image generation prompt)"
}
```

---

#### `POST /reflection`
Generate a mirror reflection on the user's writing.

**Request:**
```json
{
  "writingSession": "string",
  "locale": "string (optional, e.g., 'en', 'es')"
}
```

**Response:**
```json
{
  "reflection": "string (100-200 word reflection)"
}
```

---

#### `GET /images?limit=50&offset=0`
Get all generated Anky images (paginated).

**Query Parameters:**
- `limit` (optional): Number of images to return (default: 50)
- `offset` (optional): Number of images to skip (default: 0)

**Response:**
```json
{
  "images": [
    {
      "id": "uuid",
      "prompt": "string (the prompt used to generate)",
      "imageBase64": "string (base64 encoded image)",
      "imageUrl": "string (data URI)",
      "ankyId": "uuid | null (linked Anky if any)",
      "model": "string (e.g., gemini-2.5-flash-preview-05-20)",
      "generationTimeMs": "number (generation duration in ms)",
      "createdAt": "timestamp"
    }
  ]
}
```

---

#### `GET /images/:imageId`
Get a single generated image by ID.

**Response:**
```json
{
  "image": {
    "id": "uuid",
    "prompt": "string",
    "imageBase64": "string",
    "imageUrl": "string",
    "ankyId": "uuid | null",
    "model": "string",
    "generationTimeMs": "number",
    "createdAt": "timestamp"
  }
}
```

---

#### `POST /image`
Generate an Anky image from a prompt. The image is automatically saved to the database.

**Request:**
```json
{
  "prompt": "string (image generation prompt)"
}
```

**Response:**
```json
{
  "url": "string (data URI of the image)",
  "base64": "string (raw base64 encoded image)",
  "id": "uuid (ID of the saved image)"
}
```

---

#### `POST /title`
Generate a poetic title (1-3 words) for the session.

**Request:**
```json
{
  "writingSession": "string",
  "imagePrompt": "string",
  "reflection": "string"
}
```

**Response:**
```json
{
  "title": "string (max 3 words, lowercase)"
}
```

---

#### `POST /ipfs`
Upload writing, image, and metadata to IPFS via Pinata.

**Request:**
```json
{
  "writingSession": "string",
  "imageBase64": "string",
  "title": "string",
  "reflection": "string",
  "imagePrompt": "string"
}
```

**Response:**
```json
{
  "writingSessionIpfs": "string (IPFS hash)",
  "imageIpfs": "string (IPFS hash)",
  "tokenUri": "string (metadata IPFS hash for NFT)"
}
```

---

### Chat Endpoints

#### `POST /chat-short`
Chat with Anky for sessions under 8 minutes.

**Request:**
```json
{
  "writingSession": "string",
  "duration": "number (seconds)",
  "wordCount": "number",
  "history": [
    { "role": "user" | "assistant", "content": "string" }
  ]
}
```

**Response:**
```json
{
  "response": "string"
}
```

---

#### `POST /chat`
Chat with Anky for full 8+ minute sessions.

**Request:**
```json
{
  "writingSession": "string",
  "reflection": "string",
  "title": "string",
  "history": [
    { "role": "user" | "assistant", "content": "string" }
  ]
}
```

**Response:**
```json
{
  "response": "string"
}
```

---

### User Endpoints

#### `POST /users`
Get or create a user by wallet address.

**Request:**
```json
{
  "walletAddress": "string"
}
```

**Response:**
```json
{
  "user": {
    "id": "uuid",
    "walletAddress": "string",
    "dayBoundaryHour": 4,
    "timezone": "UTC",
    "createdAt": "timestamp",
    "updatedAt": "timestamp"
  }
}
```

---

#### `GET /users/:wallet`
Get user by wallet address.

**Response:**
```json
{
  "user": { ... }
}
```

---

#### `PATCH /users/:userId/settings`
Update user settings.

**Request:**
```json
{
  "dayBoundaryHour": "number (0-23, when 'day' resets)",
  "timezone": "string (e.g., 'America/New_York')"
}
```

**Response:**
```json
{
  "user": { ... }
}
```

---

#### `GET /users/:userId/streak`
Get user's streak statistics.

**Response:**
```json
{
  "streak": {
    "currentStreak": "number",
    "longestStreak": "number",
    "lastAnkyDate": "timestamp | null",
    "totalAnkys": "number",
    "totalWritingSessions": "number",
    "totalWordsWritten": "number",
    "totalTimeWrittenSeconds": "number",
    "hasWrittenToday": "boolean",
    "isActive": "boolean"
  }
}
```

---

#### `GET /users/:userId/ankys?limit=50`
Get user's Anky library.

**Response:**
```json
{
  "ankys": [
    {
      "id": "uuid",
      "title": "string",
      "reflection": "string",
      "imageUrl": "string",
      "isMinted": "boolean",
      "createdAt": "timestamp"
    }
  ]
}
```

---

#### `GET /users/:userId/sessions?limit=50`
Get user's writing sessions.

**Response:**
```json
{
  "sessions": [
    {
      "id": "uuid",
      "content": "string",
      "durationSeconds": "number",
      "wordCount": "number",
      "isAnky": "boolean",
      "shareId": "string",
      "isPublic": "boolean",
      "createdAt": "timestamp"
    }
  ]
}
```

---

#### `GET /users/:userId/conversations?limit=20`
Get user's chat conversations.

**Response:**
```json
{
  "conversations": [
    {
      "id": "uuid",
      "writingSessionId": "uuid",
      "messageCount": "number",
      "isActive": "boolean",
      "createdAt": "timestamp"
    }
  ]
}
```

---

### Session Endpoints

#### `POST /sessions`
Create a new writing session.

**Request:**
```json
{
  "userId": "uuid (optional if not logged in)",
  "content": "string (required)",
  "durationSeconds": "number (required)",
  "wordCount": "number (required)",
  "wordsPerMinute": "number (optional)",
  "isPublic": "boolean (default: true)",
  "dayBoundaryHour": "number (optional)",
  "timezone": "string (optional)"
}
```

**Response:**
```json
{
  "session": {
    "id": "uuid",
    "shareId": "string (8-char URL-safe ID)",
    "isAnky": "boolean (true if >= 480 seconds)",
    ...
  }
}
```

---

#### `GET /sessions/:sessionId`
Get session by ID.

**Response:**
```json
{
  "session": { ... }
}
```

---

#### `GET /s/:shareId`
Get public session by share ID (for sharing links).

**Response:**
```json
{
  "session": { ... }
}
```

**Error (404):** Session not found or private.

---

#### `PATCH /sessions/:sessionId/privacy`
Toggle session public/private.

**Request:**
```json
{
  "isPublic": "boolean"
}
```

**Response:**
```json
{
  "session": { ... }
}
```

---

### Anky Endpoints

#### `POST /ankys`
Create an Anky record for a session.

**Request:**
```json
{
  "writingSessionId": "uuid (required)",
  "userId": "uuid (optional)",
  "imagePrompt": "string",
  "reflection": "string",
  "title": "string",
  "imageBase64": "string",
  "imageUrl": "string"
}
```

**Response:**
```json
{
  "anky": { ... }
}
```

---

#### `PATCH /ankys/:ankyId`
Update an Anky.

**Request:** Any Anky fields to update.

**Response:**
```json
{
  "anky": { ... }
}
```

---

#### `GET /sessions/:sessionId/anky`
Get the Anky for a specific session.

**Response:**
```json
{
  "anky": { ... }
}
```

---

#### `POST /ankys/:ankyId/mint`
Record NFT mint transaction.

**Request:**
```json
{
  "txHash": "string (required)",
  "tokenId": "number (required)"
}
```

**Response:**
```json
{
  "anky": {
    "isMinted": true,
    "mintTxHash": "string",
    "tokenId": "number",
    "mintedAt": "timestamp"
  }
}
```

---

#### `GET /feed?limit=50&offset=0`
Get public Anky feed (paginated).

**Response:**
```json
{
  "ankys": [ ... ]
}
```

---

### Conversation Endpoints

#### `POST /conversations`
Get or create a conversation for a session.

**Request:**
```json
{
  "userId": "uuid (optional)",
  "writingSessionId": "uuid (optional)"
}
```

**Response:**
```json
{
  "conversation": {
    "id": "uuid",
    "messageCount": "number",
    "isActive": "boolean",
    "maxMessages": 50
  }
}
```

---

#### `POST /conversations/:conversationId/messages`
Add a message to a conversation.

**Request:**
```json
{
  "role": "user" | "assistant",
  "content": "string"
}
```

**Response (success):**
```json
{
  "message": { "id": "uuid", "role": "string", "content": "string", "createdAt": "timestamp" }
}
```

**Response (capped at 50 messages):**
```json
{
  "capped": true,
  "message": "You've talked enough here. Start a new conversation?"
}
```

---

#### `GET /conversations/:conversationId/messages?limit=100`
Get conversation message history.

**Response:**
```json
{
  "messages": [ ... ]
}
```

---

#### `POST /conversations/:conversationId/close`
Close a conversation.

**Response:**
```json
{
  "conversation": { "isActive": false, ... }
}
```

---

## Error Responses

All errors return JSON with an `error` field:

```json
{ "error": "Description of the error" }
```

**Common status codes:**
- `400` - Bad request (missing required fields)
- `404` - Resource not found
- `503` - Database not available

---

## Frontend Integration Example (React + Vite)

```typescript
const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

// Get auth token from Privy
import { usePrivy } from '@privy-io/react-auth';

const { getAccessToken } = usePrivy();

async function apiCall(endpoint: string, options: RequestInit = {}) {
  const token = await getAccessToken();

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`,
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || 'API Error');
  }

  return response.json();
}

// Example: Create user after Privy login
const { user: privyUser } = usePrivy();
const walletAddress = privyUser?.wallet?.address;

const { user } = await apiCall('/users', {
  method: 'POST',
  body: JSON.stringify({ walletAddress }),
});

// Example: Full 8-minute session flow
async function processAnkySession(content: string, durationSeconds: number) {
  // 1. Create session
  const { session } = await apiCall('/sessions', {
    method: 'POST',
    body: JSON.stringify({
      userId: user.id,
      content,
      durationSeconds,
      wordCount: content.split(/\s+/).length,
    }),
  });

  // 2. Generate all AI content in parallel
  const [promptRes, reflectionRes] = await Promise.all([
    apiCall('/prompt', { method: 'POST', body: JSON.stringify({ writingSession: content }) }),
    apiCall('/reflection', { method: 'POST', body: JSON.stringify({ writingSession: content }) }),
  ]);

  // 3. Generate image
  const { url: imageUrl, base64: imageBase64 } = await apiCall('/image', {
    method: 'POST',
    body: JSON.stringify({ prompt: promptRes.prompt }),
  });

  // 4. Generate title
  const { title } = await apiCall('/title', {
    method: 'POST',
    body: JSON.stringify({
      writingSession: content,
      imagePrompt: promptRes.prompt,
      reflection: reflectionRes.reflection,
    }),
  });

  // 5. Create Anky record
  const { anky } = await apiCall('/ankys', {
    method: 'POST',
    body: JSON.stringify({
      writingSessionId: session.id,
      userId: user.id,
      imagePrompt: promptRes.prompt,
      reflection: reflectionRes.reflection,
      title,
      imageBase64,
      imageUrl,
    }),
  });

  return { session, anky, title, reflection: reflectionRes.reflection, imageUrl };
}
```

---

## Environment Variables

```env
PORT=3000
DATABASE_URL=postgresql://user:password@localhost:5432/anky
ANTHROPIC_API_KEY=your_key
GEMINI_API_KEY=your_key
PINATA_JWT=your_jwt (optional, for NFT minting)
PRIVY_APP_ID=your_privy_app_id
PRIVY_APP_SECRET=your_privy_app_secret
```
