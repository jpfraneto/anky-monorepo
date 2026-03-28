# iOS Implementation: Post-Writing Flow + Local Notification Reminders

## Context

The backend already handles everything. After a writing session is submitted (`POST /swift/v2/write`), the server spawns background generation and returns immediately. The iOS app polls a status endpoint to watch artifacts materialize. Your job is to build the post-writing UX that makes the user feel like Anky is *listening* to what they wrote, and to schedule a local notification for their next session using the prompt Anky generates.

## What the backend gives you

### 1. Submit writing: `POST /swift/v2/write`

Returns immediately with:

```json
{
  "ok": true,
  "sessionId": "uuid",
  "outcome": "anky" | "short_session" | "checkpoint",
  "wordCount": 847,
  "durationSeconds": 512.3,
  "flowScore": 0.82,
  "persisted": true,
  "spawned": {
    "ankyId": "uuid-or-null",
    "feedback": false,
    "cuentacuentos": true
  },
  "walletAddress": "0x...",
  "statusUrl": "/swift/v2/writing/{sessionId}/status",
  "ankyResponse": null,
  "nextPrompt": null,
  "mood": null
}
```

Key fields:
- `outcome` tells you what kind of session this was. `"anky"` means 8+ minutes, 300+ words. `"short_session"` means they stopped early.
- `ankyResponse`, `nextPrompt`, and `mood` are **always null** in this initial response — they're being generated async.
- `statusUrl` is where you poll for the generated artifacts.

### 2. Poll status: `GET /swift/v2/writing/{sessionId}/status`

Returns the evolving state:

```json
{
  "sessionId": "uuid",
  "isAnky": true,
  "durationSeconds": 512.3,
  "wordCount": 847,
  "anky": {
    "id": "uuid",
    "status": "generating" | "ready",
    "imageUrl": "/static/anky-images/uuid.png",
    "title": "the thread you keep pulling",
    "reflection": "..."
  },
  "cuentacuentos": {
    "id": "uuid",
    "status": "generating" | "ready",
    "chakra": 4,
    "kingdom": "...",
    "city": "...",
    "title": "...",
    "translationsDone": ["es"],
    "imagesTotal": 5,
    "imagesDone": 3
  },
  "ankyResponse": "that thing you said about your mother's garden...\nyou've been circling that image for weeks.\nmaybe it's not the garden you're mourning.",
  "nextPrompt": "what lives underneath the anger?",
  "mood": "reflective"
}
```

Key fields:
- `ankyResponse` — Anky's personalized reply to the writing. Appears as a string with `\n` line breaks. This is null while generating, then populated once ready. **This is the main thing to display.**
- `nextPrompt` — A short question (max 10 words) for the user's next writing session. **This is what you schedule as the local notification body.**
- `mood` — One of: `reflective`, `celebratory`, `gentle`, `curious`, `deep`. Use this to set the visual/tonal feel of the post-writing screen.
- `anky.imageUrl` — The generated Anky image. Appears later than the text response.
- `anky.status` — `"generating"` while pipelines are running, `"ready"` when the image is done.

## What to build

### Post-Writing Screen (the "Anky is listening" flow)

This screen appears immediately after the user taps "Done" / the writing session ends and the `POST /swift/v2/write` response arrives. The experience should feel like sending a message and waiting for someone thoughtful to reply — not like a loading spinner.

**Phase 1: Anky is absorbing (0-15 seconds typically)**

The user just poured themselves out for 8 minutes. Honor that. The screen should feel quiet, held, like the space between exhale and inhale.

- Show a minimal, breathing state. Not a spinner. Something alive but calm — a slow pulse, a gentle glow, subtle particle drift. The mood here is: *someone is reading what you wrote*.
- No text like "Processing..." or "Generating response...". If you must show words, something like: *"..."* or just nothing. Silence is fine. The user just wrote for 8 minutes; they can sit in stillness for 15 seconds.
- Start polling `GET /swift/v2/writing/{sessionId}/status` every 2-3 seconds.
- **This applies regardless of outcome** — whether `outcome` is `"anky"` or `"short_session"`. Anky always listens, always responds. The difference is in what Anky says, not whether Anky shows up.

**Phase 2: Anky responds (when `ankyResponse` becomes non-null)**

- Transition the breathing/ambient state into Anky's reply appearing.
- Display `ankyResponse` with a typewriter/reveal effect — character by character or line by line. Not all at once. Anky is *speaking*, not dumping text.
- Respect the `\n` line breaks in the response. Each line should feel like a separate thought, with a pause between them.
- Use `mood` to color the experience:
  - `reflective` — muted, still, maybe cooler tones
  - `celebratory` — warmer, brighter, but never loud
  - `gentle` — soft, intimate
  - `curious` — slightly playful, lighter
  - `deep` — darker, more weight, slower reveal
- The text styling should be consistent: lowercase, intimate, like a message from someone who knows you. Think iMessage from your wisest friend, not a notification card.

**Phase 3: The image arrives (when `anky.imageUrl` becomes non-null)**

- If the session was a full anky (`isAnky: true`), an image is being generated. It typically arrives after the text response.
- Fade/reveal the image behind or alongside the text. Don't replace the text with the image — they coexist.
- If the image isn't ready yet when the text appears, that's fine. Let it appear when it's ready. Don't block the text waiting for it.

**Phase 4: Next prompt + dismiss**

- After Anky's response has been fully revealed, show `nextPrompt` as a quieter element — a seed planted for tomorrow. Something like: *"tomorrow: what lives underneath the anger?"* or just the prompt text itself in a distinct but understated style.
- This is also the moment to schedule the local notification (see below).
- Give the user a way to dismiss / return to the main screen. No rush.

### Local Notification for Daily Reminder

**Settings:**
- Add a "Reminder Time" picker to the app's settings screen. Simple hour/minute picker. Store in `UserDefaults`.
- Default: no reminder (opt-in). Or pick a sensible default like 8:00 AM local time.
- Request notification permission (`UNUserNotificationCenter.requestAuthorization`) when the user enables reminders or on first session completion — not on app launch.

**Scheduling logic:**
- When the status poll returns a non-null `nextPrompt`, schedule (or replace) a local notification:
  - Identifier: use a fixed identifier like `"anky-daily-reminder"` so each new one replaces the previous.
  - Trigger: `UNCalendarNotificationTrigger` at the user's chosen reminder time, next occurrence.
  - Title: `"anky"`
  - Body: the `nextPrompt` value (e.g., `"what lives underneath the anger?"`)
  - Sound: default
- If the user hasn't written in a while and there's no fresh `nextPrompt`, fall back to the last one stored locally, or a hardcoded gentle nudge: `"the page is waiting."`
- When the user taps the notification, open the app directly into the writing screen with that prompt pre-loaded as the session prompt.
- Only schedule one notification at a time. Each completed session replaces the previous scheduled notification with the new prompt.

**Deep link on tap:**
- Tapping the notification should open the app into the writing screen.
- Pass the prompt text so it displays as the session's opening question.
- If you use `userInfo` on the notification content, include `["prompt": nextPrompt]` and read it in the notification handling delegate.

### Edge cases

- **User quits app before `ankyResponse` arrives:** Next app open, check if there's an unfinished post-writing state (session submitted but response never displayed). If so, fetch status and show the response then. Don't lose Anky's reply.
- **`nextPrompt` is null:** Sometimes the AI doesn't generate one. Don't schedule a notification with empty text. Keep the previous one or skip.
- **Short sessions (`outcome: "short_session"`):** Same flow. Anky still responds. The response will be different in tone (the backend handles this — it tells the AI the session "ended early"). Still show the listening → response flow. Still schedule the next prompt notification if one comes back.
- **No network on poll:** Retry with exponential backoff. If still no response after ~60 seconds, show a gentle fallback: *"anky is still thinking. check back soon."* and let them dismiss. Fetch the response on next app open.
- **Multiple sessions in a day:** Each one generates a new `nextPrompt`. The latest one wins for the notification. Replace, don't stack.

### What NOT to build

- Don't build server-side push scheduling. The server already has APNs infrastructure but the daily reminder is better as a local notification since the prompt comes from the session response.
- Don't add a reminder time setting to the backend / `PATCH /swift/v2/settings`. This is purely local (`UserDefaults`).
- Don't show the `cuentacuentos` data in this flow unless you're building that screen separately. It's in the status response for completeness but is a separate feature.
- Don't persist notification state to the server. Local notification scheduling is ephemeral and device-local by design.

## API Reference (quick)

All endpoints require `Authorization: Bearer <session_token>` header.

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/swift/v2/write` | Submit writing session |
| `GET` | `/swift/v2/writing/{sessionId}/status` | Poll for generated artifacts |
| `GET` | `/swift/v2/prompt` | Get next writing prompt (if you need it outside the status flow) |

Base URL: `https://anky.app`
