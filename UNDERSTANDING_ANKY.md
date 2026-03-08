# Understanding Anky — A Complete Guide to the System You Built

*For JP. From the inside out.*

---

## Table of Contents

1. [The Big Picture](#the-big-picture)
2. [How a Computer Serves a Website](#how-a-computer-serves-a-website)
3. [Rust — The Language Anky Speaks](#rust--the-language-anky-speaks)
4. [The Server — Axum and How Requests Flow](#the-server--axum-and-how-requests-flow)
5. [The Database — SQLite and How Data Lives](#the-database--sqlite-and-how-data-lives)
6. [Authentication — How Users Are Recognized](#authentication--how-users-are-recognized)
7. [The Writing Pipeline — From Keystroke to Anky](#the-writing-pipeline--from-keystroke-to-anky)
8. [The AI Services — Claude, Ollama, Gemini](#the-ai-services--claude-ollama-gemini)
9. [The Guidance Pipeline — Meditation and Breathwork Generation](#the-guidance-pipeline--meditation-and-breathwork-generation)
10. [The Queue — Free vs Premium](#the-queue--free-vs-premium)
11. [The Facilitator Marketplace](#the-facilitator-marketplace)
12. [State Management — How the Server Remembers](#state-management--how-the-server-remembers)
13. [The File System — Where Everything Lives](#the-file-system--where-everything-lives)
14. [Deployment — How Code Becomes a Running Server](#deployment--how-code-becomes-a-running-server)
15. [The iOS App — Swift and How It Talks to Rust](#the-ios-app--swift-and-how-it-talks-to-rust)
16. [The Complete Journey of a User](#the-complete-journey-of-a-user)
17. [Glossary](#glossary)

---

## The Big Picture

Anky is a system with three layers:

```
┌──────────────────────────────────────────────┐
│                  THE USER                     │
│         (iPhone app or web browser)           │
└─────────────────────┬────────────────────────┘
                      │ HTTPS (encrypted internet)
                      ▼
┌──────────────────────────────────────────────┐
│              THE SERVER (Rust)                │
│         Running on your machine "poiesis"     │
│         Accessible via anky.app               │
│                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │
│  │ Routes  │  │ Services│  │  Pipelines   │  │
│  │ (doors) │  │ (brains)│  │ (assembly    │  │
│  │         │  │         │  │  lines)      │  │
│  └────┬────┘  └────┬────┘  └──────┬───────┘  │
│       │            │              │           │
│       ▼            ▼              ▼           │
│  ┌──────────────────────────────────────┐    │
│  │         DATABASE (SQLite)             │    │
│  │     data/anky.db — one single file    │    │
│  └──────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
                      │
                      │ API calls
                      ▼
┌──────────────────────────────────────────────┐
│            EXTERNAL AI SERVICES              │
│                                              │
│  ┌─────────┐  ┌─────────┐  ┌────────────┐   │
│  │ Claude  │  │ Gemini  │  │  Ollama    │   │
│  │ (cloud) │  │ (cloud) │  │  (local)   │   │
│  └─────────┘  └─────────┘  └────────────┘   │
└──────────────────────────────────────────────┘
```

**The user** interacts through a phone or browser. They never see the server directly.

**The server** is a program written in Rust, running on your physical machine called "poiesis" (your computer at home). It receives requests from users, processes them, talks to AI services, stores data, and sends responses back.

**The database** is a single file on your hard drive (`data/anky.db`). It stores everything: users, writings, meditations, sadhana commitments, facilitators. All of it.

**External AI services** are other computers in the cloud (Claude by Anthropic, Gemini by Google) or on your own machine (Ollama). Your server calls them when it needs intelligence — to generate a meditation script, create an image prompt, give writing feedback, or match a user with a facilitator.

**Cloudflare** sits in between the user and your machine. When someone visits `anky.app`, Cloudflare routes that request through a tunnel to your machine. This means your home computer serves the entire internet, but is protected behind Cloudflare's security. The tunnel is a program called `cloudflared` that maintains a persistent connection from your machine to Cloudflare's network.

---

## How a Computer Serves a Website

When you type `anky.app` in a browser, here's what happens step by step:

1. **DNS lookup**: Your browser asks "what's the IP address of anky.app?" Cloudflare answers.
2. **Connection**: Your browser connects to Cloudflare's server (not your machine directly).
3. **Tunnel**: Cloudflare forwards the request through the tunnel to your machine on port 8889.
4. **Routing**: Your Rust server receives the request and looks at the URL path to decide what to do.
5. **Processing**: The server runs the appropriate code — maybe querying the database, calling an AI, or generating HTML.
6. **Response**: The server sends back HTML (for web pages) or JSON (for API calls) through the same tunnel back to the user's browser.

This entire round trip takes between 50 milliseconds and a few seconds, depending on whether AI is involved.

**For the iOS app**, the flow is identical except:
- The app sends JSON instead of requesting HTML
- It sends an `Authorization` header with every request (the Bearer token)
- It uses the `/swift/v1/*` routes instead of the web routes

---

## Rust — The Language Anky Speaks

Rust is the programming language the server is written in. You don't need to write Rust to understand how Anky works, but here are the core concepts:

### Why Rust?

Rust programs are fast (as fast as C), safe (the compiler catches most bugs before the program runs), and use very little memory. Your entire server — handling hundreds of users, AI calls, database queries, image generation — runs in about 20MB of RAM. A Node.js server doing the same would use 200-500MB.

### Key Concepts

**Functions** are blocks of code that do one thing:
```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```
This says: "I take two integers, add them, and return an integer." The `->` means "returns". There's no `return` keyword needed if the last line is the result.

**Structs** are containers for related data:
```rust
struct User {
    id: String,
    username: Option<String>,
    is_premium: bool,
}
```
This defines what a "User" looks like. `Option<String>` means "this might be a string or it might be nothing" — like how a username is optional.

**`async` and `await`** handle operations that take time (network calls, database queries):
```rust
async fn get_user(id: &str) -> User {
    let result = database.query("SELECT * FROM users WHERE id = ?", id).await;
    // .await means "pause here until the database responds, but let other
    // requests be handled in the meantime"
    result
}
```

Without async, the server would freeze while waiting for the database. With async, it says "I'll come back to this later" and goes to handle other requests. When the database responds, it picks up where it left off.

**`Result` and error handling**: Rust forces you to handle errors. A function that might fail returns `Result<Success, Error>`:
```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}
```

The `?` operator is shorthand for "if this fails, return the error to whoever called me":
```rust
let result = divide(10.0, 0.0)?;  // If this fails, the whole function fails
```

This is why Rust programs rarely crash — the compiler won't let you ignore potential failures.

### The Files

Every `.rs` file in the `src/` directory is Rust source code. The compiler reads all of them and produces one binary: `target/release/anky`. That single file IS the server. You can copy it to any Linux machine and it will run.

---

## The Server — Axum and How Requests Flow

Axum is a web framework for Rust. Think of it as the skeleton that gives the server its shape. It handles:

- Listening for incoming HTTP requests
- Matching URLs to the right code (routing)
- Parsing request bodies (JSON, form data)
- Sending responses back

### Routes — The Doors Into the Server

A route is a combination of an HTTP method (GET, POST, DELETE) and a URL path. Each route is connected to a function that handles it.

```
POST /swift/v1/write  →  swift::submit_writing()
GET  /swift/v1/me     →  swift::get_me()
```

Think of routes as doors. Each door leads to a different room in the house. The method (GET/POST/DELETE) is like knocking in a specific way — GET means "show me something," POST means "I'm giving you something," DELETE means "remove something."

All the routes are defined in `src/routes/mod.rs`. This is the master map of every door in the building. When you look at this file, you can see every single thing the server can do.

### How a Request Becomes a Response

Let's trace what happens when the iOS app submits a writing session:

```
1. iPhone sends: POST https://anky.app/swift/v1/write
   Headers: Authorization: Bearer abc123
   Body: { "text": "I woke up thinking about...", "duration": 523.4, ... }

2. Cloudflare receives this, forwards through tunnel to localhost:8889

3. Axum matches the URL "/swift/v1/write" with method POST
   → calls swift::submit_writing()

4. The function:
   a. Reads the Authorization header → extracts "abc123"
   b. Looks up "abc123" in the auth_sessions table
   c. Finds that this token belongs to user "user-uuid-xyz"
   d. Reads the JSON body → gets the writing text, duration, keystrokes
   e. Calculates word count and flow score
   f. Saves to writing_sessions table in SQLite
   g. If 8+ minutes and 300+ words:
      - Creates an anky record
      - Spawns a background task to generate an image (doesn't wait for it)
      - Spawns a background task to generate a meditation + breathwork
   h. If under 8 minutes:
      - Calls Ollama for quick feedback (waits for response)
   i. Returns JSON: { "ok": true, "is_anky": true, ... }

5. Axum serializes the response to JSON bytes

6. Response flows back through Cloudflare tunnel → iPhone
```

The whole thing takes about 200ms for a non-anky session (Ollama feedback), or 50ms for an anky session (because the heavy work is spawned in the background).

### Background Tasks

Some operations are too slow to make the user wait. These are "spawned" — they run in the background while the user gets their response immediately.

```rust
tokio::spawn(async move {
    // This code runs independently in the background.
    // The user already got their response.
    // If this fails, the user doesn't see an error.
    generate_anky_image(&state, &anky_id, &writing).await;
});
```

`tokio::spawn` is like saying "start this task on a separate thread, I'm moving on." The spawned task can take seconds or minutes — it doesn't block anything.

Current background tasks in Anky:
- **Image generation** after an 8-min writing session
- **Meditation script generation** after a writing session (premium: Claude, free: queued)
- **Breathwork script generation** after a writing session
- **Ollama deep reflection** for anky sessions
- **Auto-retry** of failed image generations (every 5 minutes)
- **Queue worker** for free-tier guidance generation (every 60 seconds)
- **X filtered stream** for Twitter mention detection (always running)
- **Checkpoint recovery** for orphaned writing sessions (every 5 minutes)

---

## The Database — SQLite and How Data Lives

SQLite is the simplest possible database. It's a single file (`data/anky.db`) that contains all your data. No separate database server to install or manage. The Rust server opens this file on startup and reads/writes to it directly.

### Tables

A table is like a spreadsheet. Each row is a record, each column is a field.

**The core tables and what they store:**

```
users
├── id (UUID)
├── username ("jpfraneto")
├── email
├── wallet_address
├── privy_did (Privy identity)
├── is_premium (true/false)
└── created_at

writing_sessions
├── id (UUID)
├── user_id → points to users.id
├── content (the full writing text)
├── duration_seconds (523.4)
├── word_count (847)
├── is_anky (true/false)
├── response (Ollama feedback)
├── flow_score (0.73)
├── keystroke_deltas (JSON array)
└── created_at

ankys
├── id (UUID)
├── writing_session_id → points to writing_sessions.id
├── user_id → points to users.id
├── image_prompt (the prompt sent to Gemini/Flux)
├── reflection (Claude's reflection on the writing)
├── title ("The Mirror Speaks")
├── image_path ("/data/images/abc.webp")
├── status ("pending" → "complete" or "failed")
└── created_at

auth_sessions
├── token (UUID — the Bearer token)
├── user_id → points to users.id
├── expires_at (30-90 days from creation)
└── created_at

sadhana_commitments
├── id, user_id, title, description
├── frequency ("daily"), target_days (30)
├── start_date, is_active
└── created_at

sadhana_checkins
├── id, commitment_id, user_id
├── date ("2026-03-07"), completed (true/false)
├── notes
└── UNIQUE(commitment_id, date) — one check-in per day

personalized_meditations
├── id, user_id, writing_session_id
├── script_json (the full phase-by-phase meditation)
├── status ("pending" → "generating" → "ready" or "failed")
├── tier ("free" or "premium")
└── created_at

personalized_breathwork
├── id, user_id, writing_session_id
├── style ("wim_hof", "calming", etc.)
├── script_json, status, tier
└── created_at

facilitators
├── id, user_id, name, bio
├── specialties (JSON array: ["grief", "trauma", ...])
├── session_rate_usd, booking_url, contact_method
├── status ("pending" → "approved" or "suspended")
├── avg_rating, total_reviews, total_sessions
└── created_at

facilitator_reviews
├── id, facilitator_id, user_id
├── rating (1-5), review_text
└── UNIQUE(facilitator_id, user_id) — one review per user

facilitator_bookings
├── id, facilitator_id, user_id
├── payment_amount_usd, platform_fee_usd (8%)
├── payment_method ("usdc" or "stripe")
├── user_context_shared, shared_context_json
└── created_at

user_profiles
├── user_id
├── psychological_profile (AI-generated summary)
├── emotional_signature
├── core_tensions
├── growth_edges
├── total_sessions, total_anky_sessions
└── current_streak, longest_streak, best_flow_score
```

### Migrations

When you add a new table or column, you write a "migration" — code that modifies the database schema. In Anky, all migrations are in `src/db/migrations.rs`. They run every time the server starts. Each migration checks "does this table/column already exist?" before creating it, so it's safe to run repeatedly.

```rust
// Example: adding a new column
let has_premium: bool = conn.prepare("SELECT is_premium FROM users LIMIT 0").is_ok();
if !has_premium {
    conn.execute_batch("ALTER TABLE users ADD COLUMN is_premium BOOLEAN NOT NULL DEFAULT 0;");
}
```

This says: "try to read `is_premium` from the users table. If it fails (column doesn't exist), add it."

### Queries

All database operations are in `src/db/queries.rs`. Each function does one thing:

```rust
pub fn is_user_premium(conn: &Connection, user_id: &str) -> Result<bool> {
    // Runs: SELECT is_premium FROM users WHERE id = ?
    // Returns: true or false
}
```

The `?1`, `?2` etc. in SQL queries are placeholders. The actual values are passed separately — this prevents SQL injection (a common security vulnerability where an attacker puts malicious code in a text field).

### The Mutex — One Writer at a Time

SQLite allows only one write operation at a time. To prevent conflicts, the database connection is wrapped in a `Mutex`:

```rust
let db = state.db.lock().await;
// Now I have exclusive access to the database.
// No other request can write until I'm done.
queries::insert_something(&db, ...);
// The lock is automatically released when `db` goes out of scope.
```

This is like a key to a room. Only one person can hold the key at a time. Everyone else waits in line. The `.await` means "wait for the key without blocking other tasks."

---

## Authentication — How Users Are Recognized

### The Problem

HTTP is stateless — each request is independent. The server doesn't inherently know that two requests came from the same person. We need a way to say "this request is from user X."

### The Solution: Session Tokens

When a user logs in (via Privy), the server creates a random UUID (like `f47ac10b-58cc-4372-a567-0e02b2c3d479`) and stores it in the `auth_sessions` table alongside the user's ID and an expiry date (90 days out).

This UUID is the **session token**. The iPhone stores it in the Keychain (a secure, encrypted storage on the device). On every subsequent request, the app sends it as a header:

```
Authorization: Bearer f47ac10b-58cc-4372-a567-0e02b2c3d479
```

The server receives this, looks it up in the `auth_sessions` table, finds the associated `user_id`, and now knows who's making the request. If the token doesn't exist or is expired, the server returns a 401 (unauthorized) error.

### Privy — The Login Provider

Privy is a third-party service that handles the messy parts of authentication: email verification, wallet connection, social logins (Apple, Google), etc. The flow:

```
User → Privy SDK (on iPhone) → Privy's servers → JWT (JSON Web Token)
                                                       │
                                                       ▼
                                    Your server verifies the JWT
                                    Creates/finds the user
                                    Creates a session token
                                    Returns it to the iPhone
```

A JWT is a signed token. "Signed" means Privy used a secret key to create it, and your server can verify it's authentic using Privy's public key. This prevents someone from forging a login.

### Web vs Mobile Auth

On the web, tokens are stored in cookies (small pieces of data the browser automatically sends with every request). On mobile, there are no cookies — the app manually includes the token in the `Authorization` header. But the underlying `auth_sessions` table is the same. A user who logs in on the web and on the app gets the same user account — their writings, meditations, and everything are shared across both.

---

## The Writing Pipeline — From Keystroke to Anky

This is the heart of the system. Let's trace every step.

### On the iPhone

1. User taps "Begin" → a timer starts, the keyboard appears
2. Every keystroke is recorded with its timestamp delta (time since last keystroke, in milliseconds)
3. If the user stops typing for 8 seconds → session ends
4. The app collects: full text, total duration, array of keystroke deltas
5. Sends `POST /swift/v1/write`

### On the Server

```
submit_writing() receives the request
│
├── Step 1: Authenticate
│   Read Authorization header → look up session → get user_id
│
├── Step 2: Validate
│   Word count < 10? → reject ("write more")
│
├── Step 3: Calculate Flow Score
│   The keystroke_deltas array reveals HOW the person wrote:
│   - Consistent rhythm = high flow (0.8-1.0)
│   - Lots of pauses and bursts = lower flow (0.3-0.5)
│   - The algorithm measures: rhythm consistency, typing velocity,
│     sustained attention, and long-pause frequency
│
├── Step 4: Determine if Anky
│   is_anky = duration >= 480 seconds AND word_count >= 300
│   (8 minutes of writing with enough substance)
│
├── Step 5: Save to Database (FIRST — writing is never lost)
│   INSERT INTO writing_sessions (id, user_id, content, duration, ...)
│   UPDATE user flow stats (streak, best score, etc.)
│
├── Step 6a: If IS anky (8+ minutes)
│   │
│   ├── Spawn background: Generate Anky image
│   │   └── Ollama creates image prompt → Gemini/Flux generates image
│   │       → save to data/images/ → update ankys table
│   │
│   ├── Spawn background: Generate personalized meditation
│   │   └── Claude/Ollama reads writing → creates 10-min guided meditation JSON
│   │       → save to personalized_meditations table
│   │
│   ├── Spawn background: Generate personalized breathwork
│   │   └── Detect mood → pick style → Claude/Ollama generates 8-min session
│   │       → save to personalized_breathwork table
│   │
│   └── Return immediately: { is_anky: true, anky_id: "..." }
│       (user doesn't wait for any of the background work)
│
└── Step 6b: If NOT anky (under 8 minutes)
    │
    ├── Call Ollama synchronously for quick feedback
    │   "You wrote for 3 minutes about loneliness. The words were
    │    circling something you're not ready to name yet. Come back
    │    tomorrow and try for 8 minutes."
    │
    ├── Save feedback to database
    │
    └── Return: { is_anky: false, response: "Ollama's feedback..." }
```

### The Flow Score

The flow score is a number between 0 and 1 that measures how "in flow" the writer was. It uses the keystroke_deltas — the time gap between each keystroke:

```
Keystroke deltas (milliseconds):
[120, 85, 110, 95, 130, 88, 105, 92, ...]  ← consistent rhythm = high flow
[50, 30, 2000, 45, 60, 5000, 40, 35, ...]  ← bursts and pauses = lower flow
```

The algorithm looks at:
- **Rhythm consistency**: how similar are the gaps? (standard deviation)
- **Sustained attention**: are there long pauses (> 2 seconds)?
- **Velocity**: average typing speed
- **Warm-up**: the first 30 seconds are excluded (everyone is slow at first)

A score of 0.7+ usually means the person was genuinely in flow — the ego stopped editing and consciousness started flowing directly through the fingers.

### The Anky Image Pipeline

When a writing session qualifies as an anky:

```
Writing text
    │
    ▼
Ollama (local) — generate an image prompt
    "A figure standing at the edge of a vast ocean at twilight,
     their silhouette dissolving into light..."
    │
    ▼
Gemini (Google Cloud) — generate an image from the prompt
    Returns: raw image bytes (1024x1024)
    │
    ▼ (if Gemini fails)
Flux/ComfyUI (local GPU) — fallback image generation
    │
    ▼
Save to data/images/{anky_id}.webp
Convert to thumbnail
Update ankys table: image_path, status = "complete"
```

---

## The AI Services — Claude, Ollama, Gemini

### Claude (Anthropic) — The Deep Thinker

Claude is the most capable model. Used for:
- Personalized meditation scripts (premium)
- Personalized breathwork scripts (premium)
- Facilitator matching (analyzing user profiles against facilitator specialties)
- Writing reflections (the deep reflection on an 8-min writing session)

Claude is a cloud API — your server sends a request to Anthropic's servers and gets a response. Costs money per token (a token is roughly 4 characters).

**Cost breakdown:**
- Claude Haiku (fast, cheap): ~$0.001 per meditation script
- Claude Sonnet (deeper): ~$0.005 per reflection

All calls go through `src/services/claude.rs`:
```rust
pub async fn call_claude_public(
    api_key: &str,     // your Anthropic API key
    model: &str,       // "claude-haiku-4-5-20251001"
    system: &str,      // instructions for Claude's persona
    user_message: &str, // the actual prompt
    max_tokens: u32,   // maximum response length
) -> Result<ClaudeResult>
```

### Ollama (Local) — The Workhorse

Ollama runs AI models on your own machine. No cloud, no API costs, no internet needed. Currently running Qwen 3.5 (35B parameter model).

Used for:
- Quick writing feedback (under 8 minutes)
- Free-tier meditation/breathwork generation (queued)
- Image prompt generation (from writing to visual description)

The tradeoff: slower than Claude (your GPU vs a data center), and less nuanced. But free and private.

All calls go through `src/services/ollama.rs`:
```rust
pub async fn call_ollama(
    base_url: &str,  // "http://localhost:11434"
    model: &str,     // "qwen3.5:35b"
    prompt: &str,    // the full prompt
) -> Result<String>
```

### Gemini (Google) — The Artist

Gemini is Google's AI, used specifically for image generation. When someone writes an anky, Ollama creates an image description, and Gemini turns that description into an actual image.

Gemini's image generation is high quality and fast. If it fails (rate limits, outages), the system falls back to Flux running on your local GPU via ComfyUI.

---

## The Guidance Pipeline — Meditation and Breathwork Generation

This is the most novel part of the system. After every writing session, two AI-generated guidance sessions are created: a personalized meditation and a mood-matched breathwork session.

### Mood Detection

Before generating breathwork, the system analyzes the emotional tone of the writing:

```
Writing mentions "grief", "loss", "died"    → calming (extended exhale)
Writing mentions "anxious", "panic", "fear" → 4-7-8 (parasympathetic activation)
Writing mentions "angry", "rage", "hate"    → wim_hof (channel it outward)
Writing mentions "tired", "numb", "empty"   → energizing (bellows breath)
Writing mentions "excited", "alive", "fire" → wim_hof (ride the wave)
Writing mentions "soul", "spirit", "sacred" → pranayama (yogic balance)
Nothing detected                            → box (neutral, grounding)
```

This is done with simple keyword matching in `src/pipeline/guidance_gen.rs`. It doesn't need to be fancy — it just needs to pick the right general direction. The AI then does the nuanced work.

### The Generation Prompt

For meditation, the prompt sent to Claude/Ollama includes:
1. The full writing text (truncated to 2000 characters)
2. Any existing memory context (psychological profile, recurring patterns)
3. Instructions to generate a structured JSON script with phases

The prompt tells the AI to:
- Reference specific things from the writing (not generic)
- Not solve problems — hold space
- Use narration, breathing, body scan, visualization, and rest phases
- Total 600 seconds (10 minutes) across all phases
- Speak as Anky — warm, present, slightly mystical

### The Output Format

Both meditation and breathwork return the same JSON structure:

```json
{
  "title": "The Weight You Carry",
  "description": "A meditation born from what you wrote about your father.",
  "duration_seconds": 600,
  "background_beat_bpm": 40,
  "phases": [
    {
      "name": "Arriving",
      "phase_type": "narration",
      "duration_seconds": 45,
      "narration": "You wrote about a conversation that never happened...",
      "inhale_seconds": null,
      "exhale_seconds": null,
      "hold_seconds": null,
      "reps": null
    },
    {
      "name": "Grounding Breath",
      "phase_type": "breathing",
      "duration_seconds": 60,
      "narration": "Let your body find its own rhythm.",
      "inhale_seconds": 4.0,
      "exhale_seconds": 6.0,
      "hold_seconds": null,
      "reps": 6
    }
  ]
}
```

The iPhone app reads this JSON, loops through the phases, and uses the built-in iOS text-to-speech engine to read the narration aloud while showing breathing animations. Zero audio files. Zero storage. Zero cost for voice generation. Just text tokens and a free Apple API.

---

## The Queue — Free vs Premium

### The Two Tracks

```
PREMIUM USER writes → Claude generates meditation + breathwork IMMEDIATELY
                      (background task, takes ~5 seconds)
                      → Ready before user finishes looking at the result

FREE USER writes    → Records inserted as "pending" in the database
                    → Queue worker picks them up (checks every 60 seconds)
                    → Ollama generates one at a time
                    → Ready in 1-20 minutes depending on queue length
```

### How the Queue Worker Runs

In `src/main.rs`, there's a background loop:

```rust
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        // Check: is there a pending free-tier job?
        match process_free_queue(&state).await {
            Ok(true) => {
                // Processed one job. Check again quickly for backlog.
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Ok(false) => {
                // Nothing pending. Sleep for 60s and check again.
            }
            Err(e) => {
                // Log the error, keep going.
            }
        }
    }
});
```

The `process_free_queue` function:
1. Queries for the oldest `pending` meditation job
2. If found: sets status to `generating`, calls Ollama, sets status to `ready` or `failed`
3. If no meditation jobs: checks for breathwork jobs, same flow
4. Returns `true` if it processed something, `false` if the queue was empty

This means free users' jobs are processed in order, one at a time, so Ollama (your local GPU) is never overloaded.

### What the iPhone Sees

The app polls `GET /swift/v1/meditation/ready`:

```json
// Still generating:
{ "status": "generating" }

// Done:
{ "status": "ready", "session": { "title": "...", "phases": [...] } }
```

The app shows "Anky is preparing your meditation..." and polls every 10 seconds. When it gets `"ready"`, it transitions to "Your meditation is ready" — and the user can begin.

---

## The Facilitator Marketplace

### The Flow

```
FACILITATOR                              USER
     │                                     │
     ├── Applies: POST /facilitators/apply  │
     │   (name, bio, specialties, rate)    │
     │                                     │
     ▼                                     │
  [PENDING]                                │
     │                                     │
  Admin approves                           │
     │                                     │
     ▼                                     │
  [APPROVED] ──────────────────────────────┤
     │                                     │
     │              GET /facilitators/recommended
     │                        │
     │                        ▼
     │              Claude reads user's writing profile:
     │              "core tensions: relationship with father,
     │               fear of vulnerability, recurring loneliness"
     │                        │
     │              Claude matches against facilitator specialties:
     │              "Maria: grief, relationships, somatic work"
     │                        │
     │                        ▼
     │              Returns ranked list with match_reason:
     │              "Maria works with grief and relational patterns,
     │               which aligns with what surfaces in your writing"
     │                        │
     │                        ▼
     │              User taps "Book" → Payment (USDC or Stripe)
     │                        │
     │                        ▼
     │              8% platform fee deducted
     │              Booking record created
     │              User optionally shares Anky profile with facilitator
     │                        │
     │                        ▼
     │              User contacts facilitator via booking_url/contact_method
     │              (session happens OFF platform — Zoom, in-person, etc.)
     │                        │
     │                        ▼
     │              After session: user leaves review (1-5 stars)
     │              Facilitator's avg_rating recalculated
     │
```

### The AI Matching

This is what makes the marketplace unique. The `user_profiles` table stores AI-generated summaries of each user's writing patterns:

- **psychological_profile**: "Tends toward introspection, avoids direct confrontation, processes emotions through metaphor"
- **core_tensions**: "Unresolved grief around father's absence, fear of being seen, conflict between ambition and stillness"
- **growth_edges**: "Beginning to name anger directly, emerging capacity for self-compassion"
- **emotional_signature**: "Melancholic with bursts of fierce clarity"

These are built over time from the user's writing sessions by the memory pipeline (`src/pipeline/memory_pipeline.rs`). The more someone writes, the richer their profile becomes.

When the user asks for recommendations, Claude reads this profile and the list of facilitators (with their specialties), and generates a ranked list with specific reasons why each facilitator would be a good fit.

---

## State Management — How the Server Remembers

### AppState

The server has a single `AppState` struct that holds everything it needs:

```rust
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,     // SQLite database connection
    pub tera: Arc<Tera>,                // HTML template engine (web only)
    pub config: Arc<Config>,            // Environment variables (API keys, etc.)
    pub gpu_status: Arc<RwLock<GpuStatus>>,  // Is the GPU busy?
    pub log_tx: broadcast::Sender<LogEntry>, // Live log broadcasting
    pub write_limiter: RateLimiter,     // Prevents write spam
    pub image_limiter: RateLimiter,     // Prevents image gen spam
    pub memory_cache: Arc<Mutex<HashMap<String, String>>>, // User memory cache
    // ... more fields
}
```

This state is passed to every route handler. When a request comes in, the handler receives a reference to this shared state, uses it to access the database, check config, etc.

### Key concepts:

**`Arc`** (Atomic Reference Count): Allows multiple parts of the program to share the same data safely. Without Arc, Rust wouldn't let you share data between threads.

**`Mutex`** (Mutual Exclusion): Ensures only one thread accesses the data at a time. Like a lock on a door — you must acquire the lock before entering.

**`RwLock`** (Read-Write Lock): Like a Mutex, but allows multiple simultaneous readers. Only blocks when someone needs to write.

**`broadcast::Sender`**: A channel for sending data to multiple listeners simultaneously. Used for live log streaming and webhook events.

### Config

All configuration comes from environment variables, loaded once at startup. The `.env` file contains:

```
PORT=8889
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=qwen3.5:35b
ANTHROPIC_API_KEY=sk-ant-...
GEMINI_API_KEY=AI...
PRIVY_APP_ID=...
PRIVY_APP_SECRET=...
PRIVY_VERIFICATION_KEY=...
TWITTER_CLIENT_ID=...
# ... etc
```

These are secrets. Never commit the `.env` file to git. The `.env.example` file shows what keys are needed without the actual values.

---

## The File System — Where Everything Lives

```
/home/kithkui/anky/
│
├── src/                          # All Rust source code
│   ├── main.rs                   # Entry point — starts the server
│   ├── config.rs                 # Reads environment variables
│   ├── state.rs                  # AppState definition
│   ├── error.rs                  # Error types
│   │
│   ├── routes/                   # Request handlers (the "doors")
│   │   ├── mod.rs                # Master route map
│   │   ├── swift.rs              # Mobile API (/swift/v1/*)
│   │   ├── api.rs                # Web API routes
│   │   ├── auth.rs               # Login/logout (web)
│   │   ├── writing.rs            # Writing submission (web)
│   │   ├── meditation.rs         # Meditation routes (web)
│   │   ├── pages.rs              # HTML page rendering
│   │   └── ...
│   │
│   ├── db/                       # Database layer
│   │   ├── mod.rs                # Opens database connection
│   │   ├── migrations.rs         # Creates/modifies tables
│   │   └── queries.rs            # All SQL operations
│   │
│   ├── services/                 # External service integrations
│   │   ├── claude.rs             # Anthropic Claude API
│   │   ├── ollama.rs             # Local Ollama API
│   │   ├── gemini.rs             # Google Gemini API
│   │   ├── comfyui.rs            # Local image generation
│   │   ├── twitter.rs            # X/Twitter OAuth
│   │   ├── x_bot.rs              # X bot (mention detection, replies)
│   │   └── ...
│   │
│   ├── pipeline/                 # Multi-step processes
│   │   ├── image_gen.rs          # Writing → image prompt → image
│   │   ├── guidance_gen.rs       # Writing → meditation/breathwork scripts
│   │   ├── memory_pipeline.rs    # Writing → user profile extraction
│   │   ├── video_gen.rs          # Anky → video generation
│   │   └── ...
│   │
│   ├── models/                   # Shared data structures
│   │   └── mod.rs                # WriteRequest, WriteResponse, etc.
│   │
│   └── middleware/               # Request processing layers
│       ├── api_auth.rs           # API key validation
│       ├── security_headers.rs   # Security HTTP headers
│       └── honeypot.rs           # Attack detection
│
├── data/                         # Runtime data (not in git)
│   ├── anky.db                   # SQLite database
│   ├── images/                   # Generated anky images
│   └── generations/              # Batch generation outputs
│
├── templates/                    # HTML templates (web only)
│   ├── base.html                 # Layout template
│   ├── home.html                 # Landing page
│   ├── changelog.html            # Changelog page
│   └── ...
│
├── static/                       # Static files served directly
│   ├── style.css                 # CSS
│   ├── changelog/                # Changelog prompt files
│   └── ...
│
├── Cargo.toml                    # Rust dependencies (like package.json)
├── Cargo.lock                    # Locked dependency versions
├── .env                          # Secrets (NOT in git)
├── .env.example                  # Template for secrets
├── CLAUDE.md                     # Instructions for Claude Code
├── SWIFT_AGENT_BRIEF.md          # Instructions for Swift agent
└── target/
    └── release/
        └── anky                  # The compiled binary — THIS is the server
```

---

## Deployment — How Code Becomes a Running Server

### The Build Process

Rust is a compiled language. Unlike Python or JavaScript (which are interpreted — read and executed line by line), Rust code is transformed into machine code BEFORE it runs. This happens in two steps:

1. **Compile**: `cargo build --release`
   - Reads all `.rs` files
   - Checks for errors (Rust's compiler is famously strict)
   - Optimizes the code
   - Produces a single binary: `target/release/anky`
   - Takes ~15 seconds

2. **The binary is the server**. It's a standalone executable. No runtime, no interpreter, no dependencies. Copy it to any Linux machine and it runs.

### systemd — Keeping the Server Alive

`systemd` is the Linux service manager. It ensures the Anky server starts on boot, restarts if it crashes, and can be controlled with simple commands.

The service file (`~/.config/systemd/user/anky.service`) tells systemd:
- What program to run (`target/release/anky`)
- What directory to run it in (`/home/kithkui/anky`)
- To restart it if it crashes
- To start it on system boot

**Commands you need:**
```bash
# Build new code
cargo build --release

# Restart the server (loads the new binary)
systemctl --user restart anky.service

# Check if it's running
systemctl --user status anky.service

# View live logs
journalctl --user -u anky.service -f

# If you edit the service file itself
systemctl --user daemon-reload
```

### The Full Deployment Flow

```
1. Make code changes (or have Claude Code make them)
2. cargo build --release        ← compile
3. systemctl --user restart anky.service  ← restart with new binary
4. Server is live with new code in ~2 seconds
```

There is no staging environment. No CI/CD pipeline. No Docker. No Kubernetes. Just a binary on your machine, restarted in place. This is intentional — simplicity is a feature.

### Cloudflare Tunnel

The `cloudflared` service maintains a persistent encrypted connection from your machine to Cloudflare's network. When someone visits `anky.app`:

```
User → Cloudflare CDN → Cloudflare Tunnel → your machine:8889 → Axum → Response
```

The tunnel means:
- Your machine doesn't need a public IP address
- No ports need to be opened on your router
- All traffic is encrypted
- DDoS protection is handled by Cloudflare
- If your machine goes offline, Cloudflare shows an error page

---

## The iOS App — Swift and How It Talks to Rust

### Swift Basics

Swift is Apple's programming language for iOS, macOS, etc. If Rust is the server's language, Swift is the client's language.

Key Swift concepts:

**Views** are what the user sees. In SwiftUI (Apple's modern UI framework), views are declared with code:
```swift
struct MeditationView: View {
    var body: some View {
        VStack {
            Text("Your meditation is ready")
            Button("Begin") {
                startMeditation()
            }
        }
    }
}
```

**`async/await`** works just like in Rust — pause while waiting for a network response without freezing the UI:
```swift
let response: MeditationReady = try await AnkyAPI.shared.get("/meditation/ready")
```

**`Codable`** is Swift's way of converting between JSON and Swift objects:
```swift
struct User: Codable {
    let userId: String
    let username: String?
    let isPremium: Bool
}
// Automatically converts to/from:
// { "user_id": "...", "username": "...", "is_premium": true }
```

### How the App Talks to the Server

Every API call follows this pattern:

```
iPhone App                                  Server
    │                                         │
    ├── Build URLRequest                      │
    │   URL: https://anky.app/swift/v1/...   │
    │   Method: POST                          │
    │   Headers: Authorization: Bearer xxx    │
    │   Body: { JSON }                        │
    │                                         │
    ├── URLSession.data(for: request) ────────┤
    │                                         │
    │   ···· internet ····                    │
    │                                         │
    │                              Axum receives request
    │                              Authenticates user
    │                              Processes logic
    │                              Returns JSON
    │                                         │
    ├── Receives (Data, HTTPURLResponse) ◄────┤
    │                                         │
    ├── Decode JSON → Swift struct            │
    │                                         │
    └── Update UI                             │
```

### AVSpeechSynthesizer — The Voice of Anky

This is the iOS text-to-speech engine. It's built into every iPhone, works offline, and costs nothing. When the app receives a meditation or breathwork session (a JSON object with phases and narration text), it loops through the phases and speaks each narration:

```swift
let synthesizer = AVSpeechSynthesizer()

let utterance = AVSpeechUtterance(string: "You wrote about loneliness today...")
utterance.voice = AVSpeechSynthesisVoice(language: "en-US")
utterance.rate = 0.42        // slow, meditative
utterance.pitchMultiplier = 0.9  // slightly deeper

synthesizer.speak(utterance)
```

The synthesizer has delegate callbacks that tell you when speaking finishes, so you can chain phases together: speak → wait for finish → start breathing timer → speak next narration → etc.

This is why the whole system works on text tokens alone. The AI generates text. The phone speaks text. No audio files, no ElevenLabs, no storage, no bandwidth for streaming audio. Just structured text and a free Apple API.

---

## The Complete Journey of a User

Let's follow a person from their first interaction to a facilitator booking. This ties everything together.

### Day 1: First Write

```
Maria downloads the Anky app.
Opens it → Privy login screen → signs in with email.

   iPhone → POST /swift/v1/auth/privy { auth_token: "..." }
   Server → verifies JWT, creates user, creates session
   Server → returns { session_token: "abc", user_id: "xyz" }
   iPhone → stores "abc" in Keychain

She sees the Write tab. A prompt: "What are you afraid to say out loud?"
She taps Begin.

She writes for 4 minutes and 23 seconds. 312 words. Then she pauses
for 8 seconds. The session ends.

   iPhone → POST /swift/v1/write {
     text: "I've been pretending that everything is fine...",
     duration: 263.0,
     keystroke_deltas: [0.12, 0.09, ...],
     session_id: "sess-123"
   }

   Server → 263 seconds, 312 words → NOT an anky (under 480s)
   Server → saves to writing_sessions
   Server → calls Ollama for feedback:
     "You wrote for 4 minutes about pretending. That word appeared
      three times. What are you pretending? Come back tomorrow
      and write for 8 minutes. Let the pretense fall away."
   Server → queues personalized meditation + breathwork (free tier)
   Server → returns { is_anky: false, response: "..." }

Maria reads the feedback. Something in it lands.
```

### Day 7: First Anky

```
Maria has written every day. Today she writes for 11 minutes.
867 words. Flow score: 0.78. The words poured out about her mother.

   Server → 660 seconds, 867 words → IS an anky
   Server → saves writing
   Server → spawns background: image generation
   Server → spawns background: meditation (free → queued)
   Server → spawns background: breathwork (free → queued, style: "calming"
            because writing contained "grief", "miss", "mom")
   Server → returns { is_anky: true, anky_id: "anky-456" }

iPhone shows: "An anky was born. Your meditation is being prepared."

The queue worker picks up her meditation job 2 minutes later.
Ollama reads her writing about her mother and generates:

{
  "title": "What She Couldn't Say",
  "phases": [
    {
      "phase_type": "narration",
      "narration": "You wrote about your mother today. About the
                    things she held back. I want you to notice
                    where in your body you feel that holding..."
    },
    ...
  ]
}

Maria opens the Sit tab. "Your meditation is ready."
She taps Begin. The iPhone speaks, slowly:

"You wrote about your mother today..."

She sits for 10 minutes with a meditation that knows her.
Tears come. Something shifts.
```

### Day 30: Premium

```
Maria upgrades to premium.

   POST /swift/v1/admin/premium { user_id: "xyz", is_premium: true }

Now when she writes, her meditation and breathwork are generated
instantly by Claude (not queued via Ollama). Claude's output is
more nuanced — it catches subtleties in her writing that the local
model missed.

Her user_profiles entry now contains:
  psychological_profile: "Processes emotions through narrative.
    Tends to intellectualize grief. Emerging capacity for direct
    emotional expression. Strong self-awareness."
  core_tensions: "Unresolved grief around mother. Fear of repeating
    inherited emotional patterns. Desire for authentic connection
    vs. protective withdrawal."
  growth_edges: "Beginning to express anger without guilt.
    More present tense in recent writings."
```

### Day 60: Facilitator Recommendation

```
Maria opens her profile and sees:
"Anky recommends a facilitator for you"

   GET /swift/v1/facilitators/recommended

   Claude reads Maria's profile and the facilitator list:

   "Maria's writing consistently explores grief, inherited
    emotional patterns, and the gap between what she feels
    and what she expresses..."

   "Facilitator match: Elena Rodriguez
    Specialties: grief, somatic experiencing, family systems
    Match reason: Elena works specifically with inherited grief
    and the body's role in holding what the mind can't process.
    Your writing suggests you know things in your body before
    your mind catches up — Elena can help you trust that."

Maria reads Elena's profile. 4.9 stars, 28 reviews.
She taps Book → pays $90 (USDC on Base) → 8% ($7.20) to platform.
She checks "Share my Anky profile" — Elena receives a summary
of her writing patterns before the first session.

They meet on Zoom. Elena doesn't start with "tell me about yourself."
She starts with "I've read your Anky profile. You've been writing
about your mother's silence. Let's go there."

Maria arrives already cracked open. Already knowing what she carries.
The work begins immediately.
```

---

## Glossary

**API** (Application Programming Interface): A set of rules for how two programs talk to each other. When the iPhone sends a request to the server, it's using the API.

**async/await**: A pattern for handling operations that take time (network calls, database queries) without blocking other work.

**Axum**: The Rust web framework that handles HTTP requests and responses. The skeleton of the server.

**Bearer token**: An authentication credential sent in HTTP headers. "Bearer" means "whoever carries this token is authenticated."

**Binary**: A compiled program. The file `target/release/anky` is a binary.

**CORS** (Cross-Origin Resource Sharing): Browser security rules that control which websites can call your API. Not relevant for native mobile apps.

**CRUD**: Create, Read, Update, Delete — the four basic database operations.

**Endpoint**: A specific URL + method combination that does something (e.g., `POST /swift/v1/write`).

**Flow score**: A measure (0-1) of how "in flow" someone was while writing, calculated from the rhythm of their keystrokes.

**HTTP**: The protocol web browsers and apps use to communicate with servers. Every request has a method (GET/POST/DELETE), a URL, headers, and optionally a body.

**JSON** (JavaScript Object Notation): A text format for structured data. `{ "name": "Anky", "age": 1 }`. Used for all API communication.

**JWT** (JSON Web Token): A signed token that proves identity. Privy creates these; your server verifies them.

**Keychain**: iOS's encrypted storage for secrets (passwords, tokens). More secure than UserDefaults.

**Mutex**: A lock that ensures only one thread accesses a resource at a time. Prevents data corruption.

**OAuth**: A standard for "log in with X." The user authenticates with Twitter/Google/etc., and your app receives proof of their identity without ever seeing their password.

**Ollama**: A program that runs AI models locally on your machine. No internet needed, no API costs.

**Pipeline**: A multi-step process where the output of one step feeds the next. Writing → mood detection → breathwork generation is a pipeline.

**Privy**: A third-party service that handles user authentication (login/signup) across email, wallets, and social accounts.

**Route**: A mapping from a URL pattern to a function. `POST /write → submit_writing()`.

**SQLite**: A database engine that stores everything in a single file. Simple, fast, reliable.

**Spawn**: Starting a background task that runs independently of the current request.

**Struct**: A data structure that groups related values together. Like a form with labeled fields.

**systemd**: The Linux service manager that starts, stops, and monitors programs like the Anky server.

**TTS** (Text-to-Speech): Converting text to spoken audio. iOS has this built in via `AVSpeechSynthesizer`.

**UUID** (Universally Unique Identifier): A random 128-bit ID that's practically guaranteed to be unique. Used for user IDs, session tokens, anky IDs, etc. Looks like: `f47ac10b-58cc-4372-a567-0e02b2c3d479`.

**WebP**: An image format that's smaller than JPEG/PNG. Used for anky images to save bandwidth and storage.

---

*This document describes the system as of March 7, 2026. The architecture will evolve, but the principles remain: technology in service of human unfolding, invisible by design, honest in its limitations.*
