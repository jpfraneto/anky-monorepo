# Anky Session Format Specification

**Version:** 2.0.0
**Status:** Draft
**Date:** 2026-04-09
**Author:** Jorge Pablo Franetovic, Anky, Inc.
**Media Type:** `application/vnd.anky`
**File Extension:** `.anky`

---

## 1. Introduction

The Anky Session Format (`.anky`) is a plain-text encoding of a timed keystroke session. One writing session, one file, one hash. Every character and the exact milliseconds between them — nothing else.

## 2. Design Principles

- **Uniform.** Every line has the same structure. No special cases.
- **Minimal.** Delta and character. That's it.
- **Deterministic.** Same keystrokes → same string → same hash.
- **Plain text.** UTF-8. No binary. No headers. No metadata.
- **Hashable.** SHA-256 of the raw bytes. That hash goes on-chain.

## 3. Format

An `.anky` file is a UTF-8–encoded text file. Each line is one keystroke:

```
<delta_ms> <char>
```

| Field | Type | Description |
|-------|------|-------------|
| `delta_ms` | Non-negative integer | Milliseconds since the previous keystroke. No padding. |
| `char` | Single Unicode character | The character typed. |

The two fields are separated by a single space (U+0020). That's the first space on the line — everything after it is the character. This means the character can itself be a space.

The first keystroke has a delta of `0`.

### 3.1 Banned Keys

Enter, Backspace, and Delete do not exist in anky. The writing interface disables them. An `.anky` file contains only forward motion — visible characters and spaces. No line breaks as input. No editing. No going back.

### 3.2 End of Session

The file ends when the file ends. EOF is the end marker. There is no sentinel value.

Timestamps, session duration, user identity, kingdom — all metadata. None of it belongs in the stream. The stream is pure: *what was typed and how fast.*

## 4. Complete Example

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
197  
352 I
243  
272 h
131 a
113 v
289 e
153  
276 f
198 e
188 e
249 d
```

This encodes: `im back and I have feed`. Delta `0` on the first line. Delta `353` followed by a space on line 3 — the space character is the input. Delta `256` followed by a space on line 8 — another space.

## 5. Character Encoding

- Files MUST be UTF-8.
- Any Unicode character is valid: emoji, CJK, accented, symbols.
- The separator is always the first U+0020 on the line. Everything after it is the character.
- Line endings are LF (U+000A). Normalize CRLF to LF before hashing.
- No trailing newline after the last line.

## 6. Parsing

One rule. Every line. No exceptions.

```
split(line, ' ', limit=2) → [delta_string, character]
```

Delta is `parseInt(delta_string)`. Character is everything after the first space.

## 7. Hashing

```
SHA-256( raw_utf8_bytes_of_file )
```

Normalize line endings to LF. No trailing newline. The resulting 256-bit hash is the session's unique identifier — what gets recorded on Solana.

## 8. MIME Type

```
application/vnd.anky
```

File extension: `.anky`

## 9. Security Considerations

- `.anky` files contain raw keystroke data and may capture sensitive content. Treat as private by default.
- The on-chain hash commits to the content without revealing it.
- Implementations MUST NOT execute any content within an `.anky` file.

## 10. ABNF Grammar

```abnf
session    = first-line *(LF event-line)
first-line = "0" SP char
event-line = delta SP char

delta      = 1*DIGIT
char       = %x01-09 / %x0B-0C / %x0E-10FFFF

SP         = %x20
LF         = %x0A
DIGIT      = %x30-39
```

---

**Anky, Inc.**
https://anky.app
