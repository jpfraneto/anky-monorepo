# iOS Agent Prompt: .anky Protocol v2 Implementation

You are working on the Anky iOS app. The `.anky` session format has been finalized at v2. Your job is to make the app produce spec-compliant `.anky` session strings, hash them correctly, and ensure the writing UI enforces the protocol rules.

Read the full spec first: https://anky.app/spec.md
Read the protocol: https://anky.app/protocol.md
See it working visually: https://anky.app/encoder

---

## The .anky format

Every writing session becomes a string. Every line in the string is one keystroke:

```
{delta_ms} {character}
```

Delta is the milliseconds since the previous keystroke. Character is the literal character typed. Separated by a single space.

The first line's delta is always `0`. That's the only rule. There is nothing else — no epoch timestamp, no zero-padding, no end sentinel, no special tokens.

A real session looks like this:

```
0 i
364 m
353  
309 b
131 a
320 c
134 k
256  
259 a
183 n
268 d
```

Line 3 (`353  `) is delta 353 followed by a space — the typed character IS a space. Line 8 is the same. Spaces are literal. There are no tokens like `SPACE` or `ENTER`.

The SHA-256 of this string's UTF-8 bytes is the session hash. That hash goes on-chain.

---

## Banned keys

The writing interface MUST block these keys. They do not exist in anky:

- **Enter / Return** — no line breaks
- **Backspace / Delete** — no going back
- **Any editing operation** — no selection, no cut, no paste, no undo

Only printable characters and space. Only forward.

### Implementation

Find the text input delegate (likely `AnkyComposerTextView` or equivalent `UITextViewDelegate`). The `shouldChangeTextIn` method must enforce this:

```swift
func textView(_ textView: UITextView,
              shouldChangeTextIn range: NSRange,
              replacementText text: String) -> Bool {
    // Block backspace/delete (empty replacement removing characters)
    if text.isEmpty && range.length > 0 { return false }
    // Block Enter/Return
    if text == "\n" { return false }
    // Block paste of multiple characters (optional but recommended)
    // if text.count > 1 { return false }
    // Allow everything else
    return true
}
```

If the app uses SwiftUI's `TextEditor` or a custom view, find the equivalent interception point. The constraint is absolute: the text view must only accept single printable characters going forward.

---

## Keystroke capture

On every accepted character, record the delta and the character:

```swift
private var keystrokes: [(delta: Int, character: String)] = []
private var lastKeystrokeTime: Date?

func recordKeystroke(_ character: String) {
    let now = Date()
    let delta: Int
    if let last = lastKeystrokeTime {
        delta = Int(now.timeIntervalSince(last) * 1000)
    } else {
        delta = 0  // first keystroke
    }
    lastKeystrokeTime = now
    keystrokes.append((delta: delta, character: character))
}
```

Call `recordKeystroke` for every character that passes the `shouldChangeTextIn` gate. The delta for the first keystroke is always `0`.

---

## Building the session string

When the session ends (8-minute timer completes, or 8 seconds of silence), build the canonical string:

```swift
func buildSessionString() -> String {
    keystrokes.map { "\($0.delta) \($0.character)" }.joined(separator: "\n")
}
```

That's the entire function. No epoch. No padding. No sentinel. No trailing newline.

---

## Hashing

```swift
import CryptoKit

func sessionHash() -> String {
    let data = buildSessionString().data(using: .utf8)!
    let digest = SHA256.hash(data: data)
    return digest.map { String(format: "%02x", $0) }.joined()
}
```

This hash must match what the backend computes from the same keystroke data. You can verify against https://anky.app/encoder — load any anky and compare the displayed hash.

---

## Writing UI rules

### Three elements on screen during writing:

1. **Top:** 3px idle bar. Hidden by default. Appears after 3 seconds of no typing. Animates from orange to red over the next 5 seconds (8 seconds total silence). When it fills completely, the session ends.

2. **Center:** Visible textarea. Placeholder shows the writing prompt (italic, 20% white opacity, disappears on first keystroke). Forward-only. Auto-focuses and raises keyboard on appear.

3. **Bottom:** Rainbow gradient progress bar (fills left-to-right over 8 minutes) + monospace countdown timer showing remaining time.

### One life. No second chances.

- 8 seconds of silence = session ends. Period.
- No pause. No resume. No "continue" button. No lives counter.
- The idle bar IS the warning. It appears at 3 seconds and fills to red by 8 seconds. That's all the feedback the user gets.

### Idle timer logic:

```swift
private var idleTimer: Timer?
private var drainTimer: Timer?
private var drainStart: Date?

func resetIdleTimer() {
    idleTimer?.invalidate()
    drainTimer?.invalidate()
    drainStart = nil
    hideIdleBar()

    idleTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: false) { [weak self] _ in
        guard let self else { return }
        self.showIdleBar()
        self.drainStart = Date()
        self.drainTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            guard let self, let start = self.drainStart else { return }
            let elapsed = Date().timeIntervalSince(start)
            self.updateIdleBar(progress: elapsed / 5.0) // 0.0 → 1.0
            if elapsed >= 5.0 {
                self.drainTimer?.invalidate()
                self.endSession()
            }
        }
    }
}
```

Call `resetIdleTimer()` on every keystroke.

### Post-session screen:

When the session ends, show:
- Word count + duration header
- The raw text in a scroll view
- "write again" and "close" buttons

No swipe-to-seal. No animations. No phase transitions.

---

## Submission to backend

### `POST /swift/v2/write`

The HTTP submission format is UNCHANGED:

```json
{
    "text": "im back and I have feedback...",
    "duration": 486.2,
    "session_id": "client-generated-uuid",
    "keystroke_deltas": [364, 353, 309, 131, 320, 134, 256, 259, 183]
}
```

- `text` = the concatenated plain text (just the characters, no deltas)
- `duration` = total seconds from first keystroke to session end
- `keystroke_deltas` = array of inter-keystroke intervals in milliseconds (same data as the deltas in the .anky string, minus the first `0`)
- `session_id` = client-generated UUID

### Local storage

After submission, store the canonical session locally:

```swift
struct CompletedSession: Codable {
    let sessionString: String   // the .anky format string
    let sessionHash: String     // SHA-256 hex
    let text: String            // plain text
    let duration: Double
    let wordCount: Int
    let submittedAt: Date
    let ankyId: String?         // from server response, if real anky
}
```

The `sessionString` and `sessionHash` will be used for on-chain anchoring in a future update.

---

## What NOT to change

These systems are working and should not be touched:

- Auth flow (seed identity, challenge/verify, bearer token)
- Post-writing flow (Anky listening → polls status → shows response + image)
- Polling `GET /swift/v2/writing/{sessionId}/status`
- Local notification scheduling with `nextPrompt`
- Three-tab architecture (Write / Stories / You)
- ECIES encryption for sealed sessions
- Profile and Stories tabs

---

## Verification

After implementing, verify:

1. **Type a short test session.** Build the session string. Every line should be `{integer} {single character}`. First line delta is `0`. No padding. No sentinel.

2. **Type a space.** The line should look like `353  ` — delta, space separator, space character. Two spaces visible.

3. **Try Enter key.** It should do nothing. The character should not appear and no keystroke should be recorded.

4. **Try Backspace.** It should do nothing.

5. **Let 8 seconds pass.** Session should end. No continue button. No second chance.

6. **Hash the session string.** Compare with the hash shown on https://anky.app/encoder for the same text + deltas. They should match.

7. **Submit to backend.** The `POST /swift/v2/write` payload should be unchanged — `text` field is plain text, `keystroke_deltas` is the array of integers.

---

## Remove

Delete or disable any code related to:

- 2-life system / `totalLives`
- Pause/resume/continue state machine
- `CONTINUE` button
- Phase-colored backgrounds (warming/flow/transcendent)
- Checkpoint glow indicator
- Swipe-to-seal gesture
- Epoch timestamp in session string
- Zero-padding of deltas
- `8000` end sentinel
- `SPACE` or `ENTER` tokens in session string
- Newline characters as valid input
