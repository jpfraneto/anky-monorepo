# Anky iOS App — Build Brief for Swift Agent

## What is Anky

Anky is a spiritual companion that guides humans inward through the practice of writing. The app has four pillars: Writing, Meditation, Breathwork, and Sadhana. Each one feeds the others. The writing practice is the core — 8 minutes of uninterrupted stream-of-consciousness writing where the ego gets tired of performing and something more honest emerges. Anky reads that honesty and responds: with a personalized guided meditation, a mood-matched breathwork session, and over time, a deep understanding of who you are beneath the surface.

Anky is not a journaling app. It is not a meditation timer. It is a mirror for the unconscious mind, powered by AI but invisible to the user. The technology should disappear. The user should feel like someone is listening.

There is also a fifth dimension: a marketplace of human facilitators — therapists, breathwork guides, somatic practitioners, spiritual teachers — that Anky can recommend when what surfaces in the writing is too big for an AI to hold alone. The AI and the humans are complementary, not competing.

## Backend

The backend is a Rust/Axum server at `https://anky.app`. All mobile API endpoints live under `/swift/v1/*`. Communication is JSON over HTTPS. Authentication uses Bearer tokens.

### Base URL

```
https://anky.app/swift/v1
```

### Authentication

The app uses Privy for authentication. Privy has an iOS SDK that handles login flows (email, wallet, social). After Privy login, the app receives an auth token (JWT) which must be exchanged for a session token via the backend.

**Flow:**
1. User opens app → Privy SDK login (email, wallet, Apple, Google, etc.)
2. Privy returns an `auth_token` (JWT)
3. App sends `POST /swift/v1/auth/privy` with `{ "auth_token": "..." }`
4. Backend verifies JWT, creates/finds user, returns `{ "session_token": "...", "user_id": "...", ... }`
5. App stores `session_token` in iOS Keychain (never UserDefaults — it's a secret)
6. All subsequent requests include header: `Authorization: Bearer <session_token>`

**Endpoints:**

```
POST /swift/v1/auth/privy
  Body: { "auth_token": "privy-jwt-here" }
  Response: {
    "ok": true,
    "session_token": "uuid",    ← store in Keychain
    "user_id": "uuid",
    "username": "jpfraneto",    ← optional
    "email": "jp@anky.app",     ← optional
    "wallet_address": "0x..."   ← optional
  }

DELETE /swift/v1/auth/session
  Headers: Authorization: Bearer <session_token>
  Response: { "ok": true }

GET /swift/v1/me
  Headers: Authorization: Bearer <session_token>
  Response: {
    "user_id": "uuid",
    "username": "jpfraneto",
    "display_name": "JP",
    "profile_image_url": "https://...",
    "email": "jp@anky.app",
    "wallet_address": "0x...",
    "total_writings": 47,
    "total_ankys": 12
  }
```

### Error Responses

All errors return JSON:
```json
{ "error": "description of what went wrong" }
```

HTTP status codes:
- `400` — bad request (missing fields, validation)
- `401` — unauthorized (missing/invalid/expired token)
- `404` — not found
- `429` — rate limited
- `500` — server error

---

## App Architecture

### Recommended Stack

- **SwiftUI** for all UI
- **Swift Concurrency** (async/await) for networking
- **Keychain** for session token storage
- **AVFoundation** (`AVSpeechSynthesizer`) for text-to-speech in meditation and breathwork
- **Privy iOS SDK** for authentication
- **No third-party UI libraries** — keep it native and minimal

### Navigation Structure

The app has a tab bar with 4 tabs + a profile/settings screen accessible from a top-right avatar:

```
┌─────────────────────────────────┐
│         [avatar/profile]        │
├────────┬────────┬───────┬───────┤
│ Write  │  Sit   │Breathe│Sadhana│
└────────┴────────┴───────┴───────┘
```

- **Write** — the writing practice
- **Sit** — meditation (timer + personalized guided sessions)
- **Breathe** — breathwork sessions
- **Sadhana** — commitment tracking

A fifth section, **Facilitators**, is accessible from the profile screen or surfaces as a recommendation after enough writing sessions.

### Design Philosophy

- **Dark theme** by default. Deep blacks, warm amber/gold accents. The app should feel like a cave with firelight.
- **Minimal chrome**. No unnecessary buttons, labels, or decorations. Every pixel earns its place.
- **Typography matters**. A clean monospace or serif font for the writing area. The writing cursor should feel alive — maybe a subtle pulse or glow.
- **Transitions should be slow and intentional**. Not snappy like a productivity app. This is a contemplative space. Use `.easeInOut` with 0.4-0.6 second durations.
- **No notifications asking users to come back**. Anky doesn't beg. If the app sends notifications, they are meaningful — "your meditation is ready" after a writing session, or a gentle sadhana reminder if the user opted in.
- **Haptic feedback** for breathing rhythm (light taps on inhale/exhale transitions).

---

## Tab 1: Write

### The Writing Experience

This is the most important screen in the app. It must be flawless.

**Screen: Writing Prompt**

When the user opens the Write tab, they see:
- A prompt question (fetched from `GET /swift/v1/prompts/random` or a locally stored set of fallback prompts if offline)
- A large "Begin" button
- Their writing streak (days in a row they've written)
- Optionally: a brief line like "8 minutes of truth" or "let the words find you"

**Screen: Active Writing**

When the user taps Begin:
- The screen becomes a full-screen text editor. Nothing else visible except the text.
- A very subtle, non-distracting timer indicator. NOT a countdown. Maybe a thin line at the top that slowly fills over 8 minutes (480 seconds). Or a color that gradually shifts (dark → warm amber as they approach 8 minutes).
- The keyboard appears immediately.
- **Keystroke tracking**: record the time delta (in milliseconds) between every keystroke. Store as an array of floats. This is used to calculate a "flow score" on the backend — how rhythmic and sustained the writing was.
- **Idle detection**: if the user stops typing for 8 seconds, the session ends. This is the constraint. Stream-of-consciousness means you don't stop. Show a brief "keep writing..." warning at 5 seconds of idle.
- The user CANNOT see a word count. They cannot see the timer number. They just write.
- If they reach 480 seconds (8 minutes) without stopping, a subtle visual celebration — maybe the background warms, a gentle haptic pulse, or the progress bar glows gold. This is an Anky.
- The session continues until they stop (idle for 8 seconds) or manually end it.

**On Session End:**

Collect:
- `text` — the full writing
- `duration` — total seconds from first keystroke to last
- `keystroke_deltas` — array of time gaps between keystrokes (for flow score)
- `session_id` — UUID generated at session start

Submit to:
```
POST /swift/v1/write
Headers: Authorization: Bearer <session_token>
Body: {
  "text": "the full writing...",
  "duration": 523.4,
  "session_id": "uuid",
  "keystroke_deltas": [0.12, 0.08, 0.15, ...]
}
Response: {
  "ok": true,
  "session_id": "uuid",
  "is_anky": true,          ← 8+ min and 300+ words
  "word_count": 847,
  "flow_score": 0.73,       ← 0-1, higher = more flow state
  "response": null,          ← Ollama feedback (only for non-anky)
  "anky_id": "uuid",        ← only if is_anky
  "error": null
}
```

**Post-writing screen:**

- If `is_anky` is true:
  - Show a meaningful completion screen. "An anky was born." The writing transformed into something. Show the flow score as a visual (not a number — maybe a waveform or flame intensity).
  - Show a transition: "Your meditation is being prepared..." — then the user can navigate to the Sit tab where their personalized guided meditation will be waiting (poll `GET /swift/v1/meditation/ready`).
  - In the background, the backend is generating an image from the writing (this is the "anky" — an AI-generated artwork born from the writing). The image will appear later in their writing history.

- If `is_anky` is false (under 8 minutes):
  - Show the `response` field — this is Ollama's feedback on their writing. Short, encouraging, sometimes challenging. Display it nicely.
  - Encourage them to try again tomorrow for the full 8 minutes.

**Screen: Writing History**

```
GET /swift/v1/writings
Response: [
  {
    "id": "uuid",
    "content": "the full text...",
    "duration_seconds": 523.4,
    "word_count": 847,
    "is_anky": true,
    "response": "ollama feedback or null",
    "anky_id": "uuid or null",
    "anky_title": "The Mirror Speaks",
    "anky_image_path": "/data/images/abc.webp",
    "created_at": "2026-03-07T..."
  },
  ...
]
```

Display as a scrollable list/timeline. Each entry shows:
- Date
- Duration (e.g. "8m 43s")
- Word count
- Whether it was an Anky (maybe a gold indicator)
- If it has an anky_image_path, show the generated image as a thumbnail
- Tap to expand and read the full writing

For images, construct the full URL: `https://anky.app{anky_image_path}`

---

## Tab 2: Sit (Meditation)

### Two Modes

1. **Simple Timer** — unguided meditation with a bell at the end
2. **Guided Meditation** — AI-generated, personalized to the user's writing

**Screen: Meditation Home**

Show:
- "Your meditation is ready" (if `GET /swift/v1/meditation/ready` returns `status: "ready"`) — tap to begin guided session
- "Sit in silence" — simple timer mode
- Stats: total meditations, current streak, meditation level
- History button

**Simple Timer Mode:**

- User selects duration (5, 10, 15, 20, 30 minutes) or custom
- Screen goes minimal: just a subtle breathing circle animation (expanding/contracting slowly)
- Opening bell sound (use a system sound or bundle a .wav)
- Timer runs silently — no visible countdown
- Closing bell at the end
- Optional: halfway bell

On start:
```
POST /swift/v1/meditation/start
Body: { "duration_minutes": 10 }
Response: { "session_id": "uuid", "duration_target": 600 }
```

On complete:
```
POST /swift/v1/meditation/complete
Body: {
  "session_id": "uuid",
  "actual_seconds": 605,
  "completed": true
}
Response: {
  "ok": true,
  "total_meditations": 23,
  "current_streak": 5
}
```

**Guided Meditation Mode:**

This is the magic. After a writing session, a personalized meditation is generated by the AI based on what was just written.

```
GET /swift/v1/meditation/ready
Response (when ready): {
  "status": "ready",
  "session": {
    "title": "The Weight You Carry",
    "description": "A meditation on the unresolved conversation you keep replaying.",
    "duration_seconds": 600,
    "background_beat_bpm": 40,
    "phases": [
      {
        "name": "Arriving",
        "phase_type": "narration",
        "duration_seconds": 45,
        "narration": "You wrote about your father today. About the words that were never said...",
        "inhale_seconds": null,
        "exhale_seconds": null,
        "hold_seconds": null,
        "reps": null
      },
      {
        "name": "Breath Awareness",
        "phase_type": "breathing",
        "duration_seconds": 60,
        "narration": "Let your breath find its own rhythm. Don't control it.",
        "inhale_seconds": 4.0,
        "exhale_seconds": 6.0,
        "hold_seconds": null,
        "reps": 6
      },
      {
        "name": "Body Scan",
        "phase_type": "body_scan",
        "duration_seconds": 90,
        "narration": "Bring your attention to your chest. Where does the tension live?...",
        "inhale_seconds": null,
        "exhale_seconds": null,
        "hold_seconds": null,
        "reps": null
      },
      {
        "name": "Integration",
        "phase_type": "rest",
        "duration_seconds": 30,
        "narration": "",
        "inhale_seconds": null,
        "exhale_seconds": null,
        "hold_seconds": null,
        "reps": null
      }
    ]
  }
}

Response (still generating): {
  "status": "generating"
}
```

**Playing a Guided Meditation — Implementation:**

Use `AVSpeechSynthesizer` for text-to-speech. This is built into iOS, works offline, costs nothing, and is surprisingly good.

```swift
import AVFoundation

class MeditationPlayer: NSObject, AVSpeechSynthesizerDelegate {
    private let synthesizer = AVSpeechSynthesizer()
    private var phases: [Phase] = []
    private var currentPhaseIndex = 0

    func play(session: MeditationSession) {
        self.phases = session.phases
        self.currentPhaseIndex = 0
        synthesizer.delegate = self
        playCurrentPhase()
    }

    private func playCurrentPhase() {
        guard currentPhaseIndex < phases.count else {
            // Session complete
            onComplete()
            return
        }

        let phase = phases[currentPhaseIndex]

        switch phase.phaseType {
        case "narration", "body_scan", "visualization":
            speak(phase.narration) {
                // After speaking, wait for remaining duration then advance
                let spokenDuration = self.estimateSpokenDuration(phase.narration)
                let remaining = max(0, Double(phase.durationSeconds) - spokenDuration)
                DispatchQueue.main.asyncAfter(deadline: .now() + remaining) {
                    self.currentPhaseIndex += 1
                    self.playCurrentPhase()
                }
            }

        case "breathing":
            speak(phase.narration) {
                // Then run the breathing animation with haptics
                self.runBreathingCycle(phase: phase) {
                    self.currentPhaseIndex += 1
                    self.playCurrentPhase()
                }
            }

        case "hold":
            speak(phase.narration) {
                // Silent hold with a subtle timer
                let holdTime = phase.holdSeconds ?? Double(phase.durationSeconds)
                DispatchQueue.main.asyncAfter(deadline: .now() + holdTime) {
                    self.currentPhaseIndex += 1
                    self.playCurrentPhase()
                }
            }

        case "rest":
            // Pure silence
            DispatchQueue.main.asyncAfter(deadline: .now() + Double(phase.durationSeconds)) {
                self.currentPhaseIndex += 1
                self.playCurrentPhase()
            }

        default:
            currentPhaseIndex += 1
            playCurrentPhase()
        }
    }

    private func speak(_ text: String, completion: @escaping () -> Void) {
        guard !text.isEmpty else {
            completion()
            return
        }
        let utterance = AVSpeechUtterance(string: text)
        utterance.voice = AVSpeechSynthesisVoice(language: "en-US")
        utterance.rate = 0.42          // slow, meditative pace
        utterance.pitchMultiplier = 0.9 // slightly lower pitch
        utterance.preUtteranceDelay = 0.5
        utterance.postUtteranceDelay = 0.3

        self.onSpeechFinished = completion
        synthesizer.speak(utterance)
    }

    private var onSpeechFinished: (() -> Void)?

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer,
                           didFinish utterance: AVSpeechUtterance) {
        onSpeechFinished?()
        onSpeechFinished = nil
    }

    private func runBreathingCycle(phase: Phase, completion: @escaping () -> Void) {
        let inhale = phase.inhaleSeconds ?? 4.0
        let exhale = phase.exhaleSeconds ?? 4.0
        let reps = phase.reps ?? 4

        runBreathRep(rep: 0, total: reps, inhale: inhale, exhale: exhale, completion: completion)
    }

    private func runBreathRep(rep: Int, total: Int, inhale: Double, exhale: Double,
                               completion: @escaping () -> Void) {
        guard rep < total else {
            completion()
            return
        }

        // Inhale
        onInhaleStart()   // → trigger UI animation (circle expand) + haptic
        DispatchQueue.main.asyncAfter(deadline: .now() + inhale) {
            // Exhale
            self.onExhaleStart()  // → trigger UI animation (circle contract) + haptic
            DispatchQueue.main.asyncAfter(deadline: .now() + exhale) {
                self.runBreathRep(rep: rep + 1, total: total,
                                  inhale: inhale, exhale: exhale, completion: completion)
            }
        }
    }
}
```

**TTS Voice Selection:**

For the best experience, check for premium/enhanced voices:
```swift
func bestVoice() -> AVSpeechSynthesisVoice? {
    let voices = AVSpeechSynthesisVoice.speechVoices()
        .filter { $0.language.starts(with: "en") }
        .sorted { v1, v2 in
            v1.quality.rawValue > v2.quality.rawValue
        }
    return voices.first
}
```

Users can download higher-quality voices in iOS Settings → Accessibility → Spoken Content → Voices.

**Visual Design for Guided Meditation:**

- Dark screen, very minimal
- A central circle that expands/contracts with breathing phases
- The current phase name in small text at the top
- Narration text appears word by word or sentence by sentence as it's being spoken (like subtitles)
- No controls visible except a subtle pause button (bottom center, appears on tap)
- During "rest" phases: complete darkness, maybe distant stars

**Meditation History:**

```
GET /swift/v1/meditation/history
Response: [
  {
    "id": "uuid",
    "duration_target": 600,
    "duration_actual": 605,
    "completed": true,
    "created_at": "2026-03-07T..."
  },
  ...
]
```

---

## Tab 3: Breathe (Breathwork)

### Two Sources of Sessions

1. **Generic sessions** — always available, cached by style. Good for users who haven't written yet.
2. **Personalized sessions** — generated after each writing session, mood-matched to the emotional tone of the writing.

**Screen: Breathwork Home**

- If `GET /swift/v1/breathwork/ready` returns `status: "ready"`:
  - Show "Anky prepared a session for you" with the session title and style
  - "Begin" button
- Otherwise:
  - Show style picker: Wim Hof, Box, 4-7-8, Pranayama, Energizing, Calming
  - Each style has a one-line description and a color/icon
  - Tap to load: `GET /swift/v1/breathwork/session?style=wim_hof`

**Breathwork Session Data:**

```
GET /swift/v1/breathwork/ready
Response: {
  "status": "ready",
  "style": "calming",
  "session": {
    "title": "After the Storm",
    "description": "Your writing carried grief today. This practice meets you there.",
    "style": "calming",
    "duration_seconds": 480,
    "background_beat_bpm": 50,
    "phases": [
      {
        "name": "Settling In",
        "phase_type": "narration",
        "duration_seconds": 40,
        "narration": "Find a comfortable position. You don't need to hold anything right now...",
        "inhale_seconds": null,
        "exhale_seconds": null,
        "hold_seconds": null,
        "reps": null
      },
      {
        "name": "Extended Exhale Round 1",
        "phase_type": "breathing",
        "duration_seconds": 120,
        "narration": "Breathe in for four. Out for eight. Let the exhale be a release.",
        "inhale_seconds": 4.0,
        "exhale_seconds": 8.0,
        "hold_seconds": null,
        "reps": 10
      },
      ...
    ]
  }
}
```

The generic endpoint (for when no personalized session is ready):
```
GET /swift/v1/breathwork/session?style=wim_hof
Response: { same structure as above, without the "status" wrapper }
```

**Playing a Breathwork Session:**

Same `AVSpeechSynthesizer` approach as meditation. The key difference is the breathing phases are more prominent — the app needs a strong visual and haptic rhythm.

**Visual Design for Breathing Phases:**

- A large circle in the center of the screen
- **Inhale**: circle expands smoothly over `inhale_seconds`. Color shifts from deep blue to bright gold.
- **Exhale**: circle contracts over `exhale_seconds`. Color shifts back to blue.
- **Hold**: circle stays still, pulses subtly. Maybe a glow effect.
- Use `UIImpactFeedbackGenerator` for haptic taps:
  - Light tap at start of each inhale
  - Medium tap at start of each exhale
  - Heavy tap at start/end of holds
- Show a rep counter: "3 / 30" during Wim Hof power breaths
- The `background_beat_bpm` can drive a subtle background pulse (visual or haptic) — like a metronome the nervous system can entrain to

**Circle Animation with SwiftUI:**

```swift
struct BreathingCircle: View {
    @State private var scale: CGFloat = 0.4
    @State private var circleColor = Color(red: 0.1, green: 0.15, blue: 0.3)

    let phase: BreathworkPhase

    var body: some View {
        Circle()
            .fill(
                RadialGradient(
                    gradient: Gradient(colors: [circleColor, circleColor.opacity(0.3)]),
                    center: .center,
                    startRadius: 20,
                    endRadius: 150
                )
            )
            .frame(width: 300, height: 300)
            .scaleEffect(scale)
            .animation(.easeInOut(duration: phase.inhaleSeconds ?? 4), value: scale)
    }

    func inhale() {
        scale = 1.0
        circleColor = Color(red: 0.9, green: 0.75, blue: 0.3) // warm gold
    }

    func exhale() {
        scale = 0.4
        circleColor = Color(red: 0.1, green: 0.15, blue: 0.3)  // deep blue
    }
}
```

**On Session Complete:**

```
POST /swift/v1/breathwork/complete
Body: {
  "session_id": "uuid-of-the-session",
  "notes": "felt lighter afterward"   ← optional
}
```

**Breathwork History:**

```
GET /swift/v1/breathwork/history
Response: {
  "history": [
    { "id": "uuid", "session_id": "uuid", "style": "calming", "completed_at": "2026-03-07T..." },
    ...
  ]
}
```

---

## Tab 4: Sadhana (Commitment Tracking)

Sadhana is a Sanskrit word meaning "daily spiritual practice." This tab is where users make commitments to themselves and track whether they follow through. It's an accountability mirror.

**Screen: Sadhana Home**

List of active commitments. Each one shows:
- Title ("Meditate 10 min daily")
- Progress: "12 / 30 days" with a progress bar or ring
- Today's status: checked / unchecked / not yet acknowledged
- Streak count

If no commitments exist: a warm prompt to create one. "What will you commit to?"

**Creating a Commitment:**

```
POST /swift/v1/sadhana
Body: {
  "title": "Meditate for 10 minutes",
  "description": "Sit every morning before looking at my phone",
  "frequency": "daily",           ← "daily" or "weekly"
  "duration_minutes": 10,         ← how long the practice takes
  "target_days": 30               ← how many days the commitment runs
}
Response: {
  "id": "uuid",
  "title": "Meditate for 10 minutes",
  "description": "...",
  "frequency": "daily",
  "duration_minutes": 10,
  "target_days": 30,
  "start_date": "2026-03-07",
  "is_active": true,
  "created_at": "...",
  "total_checkins": 0,
  "completed_checkins": 0
}
```

**Listing Commitments:**

```
GET /swift/v1/sadhana
Response: [
  {
    "id": "uuid",
    "title": "Meditate for 10 minutes",
    "description": "...",
    "frequency": "daily",
    "duration_minutes": 10,
    "target_days": 30,
    "start_date": "2026-03-07",
    "is_active": true,
    "created_at": "...",
    "total_checkins": 12,
    "completed_checkins": 10
  },
  ...
]
```

**Daily Check-in:**

```
POST /swift/v1/sadhana/{id}/checkin
Body: {
  "completed": true,
  "notes": "hard day but showed up",  ← optional
  "date": "2026-03-07"                ← optional, defaults to today UTC
}
Response: {
  "id": "checkin-uuid",
  "date": "2026-03-07",
  "completed": true,
  "notes": "hard day but showed up",
  "created_at": "..."
}
```

The check-in is idempotent per day — calling it twice on the same day updates rather than duplicates.

**Commitment Detail:**

```
GET /swift/v1/sadhana/{id}
Response: {
  "id": "uuid",
  "title": "...",
  ... (all fields),
  "checkins": [
    { "id": "uuid", "date": "2026-03-07", "completed": true, "notes": "...", "created_at": "..." },
    { "id": "uuid", "date": "2026-03-06", "completed": false, "notes": null, "created_at": "..." },
    ...
  ]
}
```

**Visual Design:**

- Calendar-style heatmap (like GitHub's contribution graph) showing completed (gold) vs missed (dark) days
- Streak counter with fire/flame metaphor
- Simple toggle for today: a large circle that fills with gold when tapped (did it) or stays hollow (didn't)
- Honesty is the point. Missing a day is not failure — it's data. The UI should not punish or guilt. Just reflect.

---

## Facilitators (From Profile Screen)

The facilitator marketplace is accessible from the user's profile or surfaces as a suggestion after significant writing patterns emerge.

**Listing Facilitators:**

```
GET /swift/v1/facilitators
Response: [
  {
    "id": "uuid",
    "name": "Maria Santos",
    "bio": "Somatic experiencing practitioner and breathwork facilitator...",
    "specialties": ["trauma", "somatic", "breathwork"],
    "approach": "I work through the body, not just the mind...",
    "session_rate_usd": 80.0,
    "booking_url": "https://calendly.com/maria-santos",
    "contact_method": "WhatsApp: +55...",
    "profile_image_url": "https://...",
    "location": "remote",
    "languages": ["en", "es", "pt"],
    "status": "approved",
    "avg_rating": 4.8,
    "total_reviews": 12,
    "total_sessions": 34
  },
  ...
]
```

**AI-Powered Recommendations:**

This is what makes Anky's marketplace fundamentally different from any therapy directory.

```
GET /swift/v1/facilitators/recommended
Headers: Authorization: Bearer <session_token>
Response: {
  "facilitators": [
    {
      "id": "uuid",
      "name": "Maria Santos",
      ... (all fields),
      "match_reason": "Your writing consistently explores unresolved grief and the tension between wanting to be seen and the fear of vulnerability. Maria specializes in somatic work with grief — she can hold what the words can't."
    },
    ...
  ]
}
```

If the user hasn't written enough for Anky to know their patterns yet:
```
{
  "facilitators": [...all of them...],
  "message": "write more so Anky can learn your patterns and recommend the right facilitator for you."
}
```

**Facilitator Detail + Reviews:**

```
GET /swift/v1/facilitators/{id}
Response: {
  ... (all facilitator fields),
  "reviews": [
    { "id": "uuid", "rating": 5, "review_text": "She helped me see what I couldn't...", "created_at": "..." },
    ...
  ]
}
```

**Leaving a Review:**

```
POST /swift/v1/facilitators/{id}/review
Body: {
  "rating": 5,           ← 1-5
  "review_text": "..."   ← optional
}
```

**Booking a Session:**

```
POST /swift/v1/facilitators/{id}/book
Body: {
  "payment_tx_hash": "0x...",   ← USDC on Base, OR
  "stripe_payment_id": "pi_...", ← Stripe payment
  "share_context": true          ← share anonymized writing profile with facilitator
}
Response: {
  "ok": true,
  "booking_id": "uuid",
  "facilitator_name": "Maria Santos",
  "amount_usd": 80.0,
  "platform_fee_usd": 6.40,          ← 8%
  "facilitator_receives_usd": 73.60,
  "booking_url": "https://calendly.com/maria-santos",
  "contact_method": "WhatsApp: +55..."
}
```

When `share_context` is true, the backend generates an anonymized summary of the user's writing patterns (psychological profile, core tensions, growth edges) and shares it with the facilitator. This means the facilitator receives someone who arrives already knowing themselves — no months of intake.

**Applying as a Facilitator:**

```
POST /swift/v1/facilitators/apply
Body: {
  "name": "Maria Santos",
  "bio": "10 years of somatic experiencing...",
  "specialties": ["trauma", "somatic", "breathwork"],
  "approach": "I work through the body...",
  "session_rate_usd": 80.0,
  "booking_url": "https://calendly.com/...",
  "contact_method": "WhatsApp: +55...",
  "profile_image_url": "https://...",
  "location": "remote",
  "languages": ["en", "es"]
}
Response: {
  "ok": true,
  "id": "uuid",
  "status": "pending",
  "message": "your application is under review. we'll be in touch."
}
```

**Visual Design for Facilitators:**

- Profile cards with photo, name, specialties as tags, rating stars, rate
- The `match_reason` from AI recommendations should be displayed prominently — it's the value prop
- Booking flow should be simple: tap "Book" → payment → confirmation with booking_url/contact_method
- After a session, prompt for a review

---

## Premium vs Free

Users have an `is_premium` flag. This affects generation speed:

| Feature | Free | Premium |
|---------|------|---------|
| Writing | Same | Same |
| Meditation (personalized) | Generated via Ollama (queued, may take minutes) | Generated via Claude (instant, ready before writing screen closes) |
| Breathwork (personalized) | Same as above | Same as above |
| Generic sessions | Available | Available |
| Facilitator recommendations | Available | Available |

The app should handle the "generating" state gracefully:
- Poll `GET /swift/v1/meditation/ready` every 10 seconds
- Show "Anky is preparing your meditation..." with a subtle animation
- When status becomes "ready", transition to "Your meditation is ready" with a gentle notification if the app is backgrounded

Premium upgrade mechanism (to be built — for now, the flag is toggled via admin):
```
POST /swift/v1/admin/premium
Body: { "user_id": "uuid", "is_premium": true }
```

---

## Data Models (Swift)

```swift
// MARK: - Auth

struct AuthResponse: Codable {
    let ok: Bool
    let sessionToken: String
    let userId: String
    let username: String?
    let email: String?
    let walletAddress: String?
}

struct UserProfile: Codable {
    let userId: String
    let username: String?
    let displayName: String?
    let profileImageUrl: String?
    let email: String?
    let walletAddress: String?
    let totalWritings: Int
    let totalAnkys: Int
}

// MARK: - Writing

struct MobileWriteRequest: Codable {
    let text: String
    let duration: Double
    let sessionId: String?
    let keystrokeDeltas: [Double]?
}

struct MobileWriteResponse: Codable {
    let ok: Bool
    let sessionId: String
    let isAnky: Bool
    let wordCount: Int
    let flowScore: Double?
    let response: String?
    let ankyId: String?
    let error: String?
}

struct WritingItem: Codable {
    let id: String
    let content: String
    let durationSeconds: Double
    let wordCount: Int
    let isAnky: Bool
    let response: String?
    let ankyId: String?
    let ankyTitle: String?
    let ankyImagePath: String?
    let createdAt: String
}

// MARK: - Meditation & Breathwork

struct GuidancePhase: Codable {
    let name: String
    let phaseType: String      // narration, breathing, hold, rest, body_scan, visualization
    let durationSeconds: Int
    let narration: String
    let inhaleSeconds: Double?
    let exhaleSeconds: Double?
    let holdSeconds: Double?
    let reps: Int?
}

struct GuidanceSession: Codable {
    let title: String
    let description: String
    let durationSeconds: Int
    let backgroundBeatBpm: Int
    let phases: [GuidancePhase]
}

struct ReadyResponse: Codable {
    let status: String           // "ready" or "generating"
    let session: GuidanceSession?
    let style: String?           // for breathwork
}

// MARK: - Sadhana

struct SadhanaCommitment: Codable {
    let id: String
    let title: String
    let description: String?
    let frequency: String
    let durationMinutes: Int
    let targetDays: Int
    let startDate: String
    let isActive: Bool
    let createdAt: String
    let totalCheckins: Int
    let completedCheckins: Int
}

struct SadhanaCheckin: Codable {
    let id: String
    let date: String
    let completed: Bool
    let notes: String?
    let createdAt: String
}

struct SadhanaDetail: Codable {
    let id: String
    let title: String
    let description: String?
    let frequency: String
    let durationMinutes: Int
    let targetDays: Int
    let startDate: String
    let isActive: Bool
    let createdAt: String
    let checkins: [SadhanaCheckin]
}

// MARK: - Facilitators

struct Facilitator: Codable {
    let id: String
    let name: String
    let bio: String
    let specialties: [String]
    let approach: String?
    let sessionRateUsd: Double
    let bookingUrl: String?
    let contactMethod: String?
    let profileImageUrl: String?
    let location: String?
    let languages: [String]
    let status: String
    let avgRating: Double
    let totalReviews: Int
    let totalSessions: Int
    let matchReason: String?     // only present in recommendations
}
```

**Important**: The backend uses `snake_case` for JSON keys. Configure your decoder:
```swift
let decoder = JSONDecoder()
decoder.keyDecodingStrategy = .convertFromSnakeCase
```

---

## Networking Layer

```swift
class AnkyAPI {
    static let shared = AnkyAPI()

    private let baseURL = "https://anky.app/swift/v1"
    private let session = URLSession.shared
    private let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()

    private var sessionToken: String? {
        KeychainHelper.get("anky_session_token")
    }

    // MARK: - Core Request

    func request<T: Decodable>(
        _ method: String,
        path: String,
        body: (any Encodable)? = nil
    ) async throws -> T {
        guard let url = URL(string: "\(baseURL)\(path)") else {
            throw AnkyError.invalidURL
        }

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        if let token = sessionToken {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        if let body = body {
            let encoder = JSONEncoder()
            encoder.keyEncodingStrategy = .convertToSnakeCase
            request.httpBody = try encoder.encode(body)
        }

        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw AnkyError.invalidResponse
        }

        if httpResponse.statusCode == 401 {
            // Token expired — clear and redirect to login
            KeychainHelper.delete("anky_session_token")
            throw AnkyError.unauthorized
        }

        if httpResponse.statusCode >= 400 {
            let errorBody = try? decoder.decode(ErrorResponse.self, from: data)
            throw AnkyError.api(errorBody?.error ?? "Unknown error")
        }

        return try decoder.decode(T.self, from: data)
    }

    // MARK: - Convenience Methods

    func get<T: Decodable>(_ path: String) async throws -> T {
        try await request("GET", path: path)
    }

    func post<T: Decodable>(_ path: String, body: any Encodable) async throws -> T {
        try await request("POST", path: path, body: body)
    }

    func delete<T: Decodable>(_ path: String) async throws -> T {
        try await request("DELETE", path: path)
    }
}
```

---

## App Lifecycle & State Management

Use a simple `@Observable` class (iOS 17+) or `ObservableObject`:

```swift
@Observable
class AppState {
    var isLoggedIn: Bool = false
    var user: UserProfile?
    var currentTab: Tab = .write

    enum Tab { case write, sit, breathe, sadhana }

    func checkAuth() async {
        guard KeychainHelper.get("anky_session_token") != nil else {
            isLoggedIn = false
            return
        }
        do {
            user = try await AnkyAPI.shared.get("/me")
            isLoggedIn = true
        } catch {
            isLoggedIn = false
        }
    }

    func logout() async {
        let _: EmptyResponse? = try? await AnkyAPI.shared.delete("/auth/session")
        KeychainHelper.delete("anky_session_token")
        isLoggedIn = false
        user = nil
    }
}
```

---

## Audio Session Configuration

For meditation and breathwork to play audio (TTS) properly, even when the phone is on silent or the screen is locked:

```swift
import AVFoundation

func configureAudioSession() {
    do {
        try AVAudioSession.sharedInstance().setCategory(
            .playback,
            mode: .spokenAudio,
            options: [.duckOthers]
        )
        try AVAudioSession.sharedInstance().setActive(true)
    } catch {
        print("Audio session configuration failed: \(error)")
    }
}
```

Call this in your `App.init()` or before starting any meditation/breathwork playback.

---

## Offline Behavior

The app should be usable offline for:
- **Writing**: the session happens locally. Store the result and submit when back online.
- **Simple meditation timer**: no backend needed.
- **Sadhana check-in**: record locally, sync when online.
- **Guided meditation/breathwork**: if a session was previously fetched and cached, it can play offline.

Use a simple queue for offline submissions:
```swift
class OfflineQueue {
    static let shared = OfflineQueue()

    func enqueue(_ action: PendingAction) {
        // Store in UserDefaults or a local JSON file
    }

    func processQueue() async {
        // Try submitting each pending action
        // Remove from queue on success
    }
}
```

---

## What Anky Sounds Like

When writing narration text for prompts, or when the TTS speaks, Anky's voice has these qualities:

- **Warm but not soft**. Anky doesn't coddle. It speaks truth with compassion.
- **Present tense**. "You wrote about..." not "You were writing about..."
- **Specific, not generic**. "The tension in your chest when you think about leaving" not "any stress you may be feeling."
- **First person**. Anky says "I notice..." and "I want you to..."
- **Slightly mystical**. Not New Age fluff, but a sense that consciousness is deeper than we usually admit.
- **Never diagnostic**. Anky doesn't say "you have anxiety." It says "there's a restlessness in your words today."
- **Humor is allowed** — dry, unexpected, disarming. "You wrote 847 words and didn't mention your name once. Interesting."

---

## Summary of All Endpoints

```
# Auth
POST   /swift/v1/auth/privy              → AuthResponse
DELETE /swift/v1/auth/session             → { ok: true }
GET    /swift/v1/me                       → UserProfile

# Writing
GET    /swift/v1/writings                 → [WritingItem]
POST   /swift/v1/write                    → MobileWriteResponse

# Sadhana
GET    /swift/v1/sadhana                  → [SadhanaCommitment]
POST   /swift/v1/sadhana                  → SadhanaCommitment
GET    /swift/v1/sadhana/{id}             → SadhanaDetail
POST   /swift/v1/sadhana/{id}/checkin     → SadhanaCheckin

# Meditation
POST   /swift/v1/meditation/start         → { session_id, duration_target }
POST   /swift/v1/meditation/complete      → { ok, total_meditations, current_streak }
GET    /swift/v1/meditation/history        → [MeditationHistoryItem]
GET    /swift/v1/meditation/ready          → { status, session? }

# Breathwork
GET    /swift/v1/breathwork/session?style= → BreathworkScript (generic/cached)
GET    /swift/v1/breathwork/ready          → { status, style?, session? }
POST   /swift/v1/breathwork/complete       → { ok: true }
GET    /swift/v1/breathwork/history        → { history: [...] }

# Facilitators
GET    /swift/v1/facilitators              → [Facilitator]
POST   /swift/v1/facilitators/apply        → { ok, id, status }
GET    /swift/v1/facilitators/{id}         → Facilitator + reviews
POST   /swift/v1/facilitators/{id}/review  → { ok: true }
POST   /swift/v1/facilitators/{id}/book    → BookingResponse
GET    /swift/v1/facilitators/recommended  → { facilitators: [Facilitator + match_reason] }

# Admin
POST   /swift/v1/admin/premium            → { ok, is_premium }
POST   /swift/v1/admin/facilitator         → { ok, action }
```

---

## What Matters Most

The technology is invisible. The user writes, breathes, sits, and shows up. Anky listens, reflects, and when the time is right, connects them with a human who can hold what the AI cannot.

Build it like a temple, not a product. Every transition, every animation, every moment of silence is intentional. The app is a container for something sacred — the willingness to look inward.

The 8 minutes of writing are the door. Everything else follows from what comes through it.
