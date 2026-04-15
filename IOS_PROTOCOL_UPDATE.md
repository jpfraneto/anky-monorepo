# iOS Update: .anky Protocol v2 + Writing UI Simplification

## Context

The `.anky` session format has been simplified (spec v2.0.0). The writing UI has also been simplified. This document describes exactly what needs to change in the iOS app. Read the full spec at [anky.app/spec.md](https://anky.app/spec.md).

---

## 1. Session String Format — CHANGED

### Old (v1)
```
1712345678000 h     ← epoch on first line
0204 e              ← zero-padded 4-digit delta
0187 l
0143 l
0198 o
0301                ← bare delta = Enter key (newline)
0089 w
8000                ← end sentinel
```

### New (v2)
```
0 i                 ← delta 0, that's it
364 m               ← no padding
353                 ← space character (literal space after delta)
309 b
131 a
320 c
134 k
```

### What changed

| Aspect | v1 | v2 |
|--------|----|----|
| First line | 13-digit epoch + char | `0` + char (just like every other line) |
| Delta format | Zero-padded 4 digits (`0204`) | No padding (`204`) |
| Delta range | 0–7999 (capped) | 0+ (unbounded, no cap) |
| Space character | Was sometimes `SPACE` token | Literal space: `353 ` |
| Enter key | Bare delta line (`0301`) | **BANNED. Does not exist.** |
| End sentinel | `8000` on last line | **None. File ends when file ends.** |
| Metadata in stream | Epoch timestamp on line 1 | **None. Stream is pure keystrokes.** |

### Implementation changes in `WritingSessionStore`

**Building the session string:**

```swift
func buildSessionString() -> String {
    // keystrokes is [(character: String, deltaMs: Int)]
    // First keystroke delta is always 0
    var lines: [String] = []
    for (i, ks) in keystrokes.enumerated() {
        let delta = i == 0 ? 0 : ks.deltaMs
        lines.append("\(delta) \(ks.character)")
    }
    return lines.joined(separator: "\n")
}
```

That's it. No epoch. No padding. No sentinel. No special cases.

**Keystroke capture:**

```swift
// On each character typed:
let now = Date()
let deltaMs: Int
if keystrokes.isEmpty {
    deltaMs = 0
} else {
    deltaMs = Int((now.timeIntervalSince(lastKeystrokeTime)) * 1000)
}
lastKeystrokeTime = now
keystrokes.append((character: typed, deltaMs: deltaMs))
```

**Hashing:**

```swift
func sessionHash() -> String {
    let sessionString = buildSessionString()
    let data = sessionString.data(using: .utf8)!
    // SHA-256
    let hash = SHA256.hash(data: data)
    return hash.map { String(format: "%02x", $0) }.joined()
}
```

The hash is computed on the raw session string bytes. No normalization needed — the string is built deterministically. This hash is what goes on-chain.

---

## 2. Banned Keys — ENFORCED

The writing interface MUST disable:
- **Enter / Return** — no line breaks
- **Backspace / Delete** — no editing
- **Any key that modifies previous input**

Only forward motion. Only visible characters and spaces.

This was already partially implemented (backspace disabled). **Enter must also be blocked.** If the current `AnkyComposerTextView` allows Enter, it must be intercepted and discarded.

```swift
func textView(_ textView: UITextView, 
              shouldChangeTextIn range: NSRange, 
              replacementText text: String) -> Bool {
    // Block backspace (empty replacement in non-empty range)
    if text.isEmpty && range.length > 0 { return false }
    // Block Enter/Return
    if text == "\n" { return false }
    // Allow everything else
    return true
}
```

---

## 3. Writing UI — SIMPLIFIED

### What we have now (keep)
- Top: 3px idle bar (appears after 3s idle, orange to red over 8 seconds)
- Center: Visible textarea with placeholder prompt, forward-only, auto-focused on appear
- Bottom: Rainbow gradient progress bar (fills over 8 minutes) + monospace countdown timer

### What changed
- **1 life, not 2.** 8 seconds of silence = session ends. No pause/resume. No continue button.
- **No phase-colored backgrounds.** No warming/flow/transcendent color transitions.
- **No checkpoint glow.** No periodic save indicators.
- Post-session: word count + time header, raw writing text in scroll view, "write again" / "close" buttons. No swipe-to-seal.

### Idle detection (simplified)

```swift
// On each keystroke:
idleTimer?.invalidate()
hideIdleBar()
idleTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: false) { _ in
    self.showIdleBar()  // orange bar appears
    self.idleDrainStart = Date()
    self.drainTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { _ in
        let elapsed = Date().timeIntervalSince(self.idleDrainStart!)
        let progress = elapsed / 5.0  // 5 more seconds (3+5=8 total)
        self.updateIdleBar(progress: progress)  // orange → red
        if elapsed >= 5.0 {
            self.endSession()  // done. no second chance.
        }
    }
}
```

---

## 4. Submission — WHAT TO SEND

### `POST /swift/v2/write`

```json
{
    "text": "im back and I have feedback lots of feedback...",
    "duration": 486.2,
    "session_id": "client-uuid",
    "keystroke_deltas": [364, 353, 309, 131, 320, 134, 256, 259, 183]
}
```

The `keystroke_deltas` array is still sent as before — millisecond intervals between keystrokes. The backend uses these for flow score calculation.

**Additionally**, build the canonical `.anky` session string locally and compute its SHA-256 hash. This hash should be sent in a future version when we add on-chain anchoring from the device.

For now, store it locally:

```swift
struct CompletedSession {
    let sessionString: String   // the canonical .anky format string
    let sessionHash: String     // SHA-256 hex of sessionString
    let submittedAt: Date
    let ankyId: String?         // from server response
}
```

---

## 5. ECIES Encryption — NO CHANGE

The encryption flow for sealed sessions (`POST /api/sessions/seal`) is unchanged. The encrypted payload wraps the `.anky` session string (v2 format) instead of raw text. The enclave decrypts and processes.

```
plaintext = buildSessionString()   // v2 format
ciphertext = eciesEncrypt(plaintext, enclavePublicKey)
POST /api/sessions/seal { ciphertext, sessionHash }
```

---

## 6. What NOT to change

- Auth flow (challenge/verify) — unchanged
- Post-writing flow (Anky listening → response → image) — unchanged
- Polling `/swift/v2/writing/{sessionId}/status` — unchanged
- Local notification scheduling — unchanged
- Three-tab architecture — unchanged
- Profile / Stories tabs — unchanged

---

## 7. Migration checklist

- [ ] Update `buildSessionString()` — no epoch, no padding, no sentinel
- [ ] Block Enter key in `AnkyComposerTextView`
- [ ] Remove 2-life system, implement 1-life (8s silence = end)
- [ ] Remove pause/resume/continue UI
- [ ] Remove phase-colored backgrounds
- [ ] Remove checkpoint glow indicator
- [ ] Verify `keystroke_deltas` array still sent correctly in POST
- [ ] Store canonical session string + hash locally after each session
- [ ] Test round-trip: session string → SHA-256 → matches what backend computes

---

## Reference

- Full spec: [anky.app/spec.md](https://anky.app/spec.md)
- Protocol: [anky.app/protocol.md](https://anky.app/protocol.md)
- Encoder (visual): [anky.app/encoder](https://anky.app/encoder)
