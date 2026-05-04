# THE ANKY PROTOCOL

---

## what this is

a minimal protocol for capturing forward-only keystroke sessions
as immutable, hash-verifiable plain text files.

one file format. one hash function. one optional public anchor.

this specification defines:

- the canonical `.anky` file format
- rules for parsing and verifying its contents
- the hashing procedure
- the boundary between canonical session data and everything else

this specification does not define identity, authorship, humanness,
literary quality, consciousness, or meaning.
it defines only how a writing session is represented and verified.

an `.anky` is the writing session itself.

the first element in the long string is the timestamp on which the session starts.
everything after that is the rhythm of the writing:
delta, character, delta, character, delta, character,
until silence closes the session.

the `.anky` string does not describe the session.
it is the session.

---

## the atom

a writing session is a plain text file.

it opens with the exact unix epoch millisecond of the first keystroke.
after that, each line is one accepted character paired with the elapsed
milliseconds since the previous accepted character.
the session closes with a final `8000` line.

```text
{epoch_ms} {character_or_SPACE}         ← line 1: absolute start time of the session
{delta_ms_0000_to_7999} {character_or_SPACE}
{delta_ms_0000_to_7999} {character_or_SPACE}
...
8000                           ← terminal silence marker
```

no summary. no side metadata. no interpretation inside the file.
if it did not happen in the writing rhythm, it is not in the canonical string.

---

## the three rules

**1. forward only.**
no backspace. no delete. no arrow keys. no enter. no paste. no editing.
only characters that advance the text are recorded.
typos stay. hesitations stay. the mess stays.

**2. the rhythm is part of the writing.**
the text is here.
the pauses are here.
the exact moment of beginning is here.
the whole cadence of the writing session is encoded into the string.

**3. immutable after creation.**
the `.anky` file is sealed the moment the terminal `8000` line is written.
it is never modified after that.
derived artifacts may come and go.
the canonical file does not change.

---

## format specification

### line 1

```text
{epoch_ms} {character_or_SPACE}\n
```

`epoch_ms` is the absolute unix timestamp in milliseconds of the first keystroke.
`character_or_SPACE` is the first accepted character typed, except that a typed
space is encoded as the exact ascii token `SPACE`.
the separator is exactly one ascii space (`0x20`).
the line terminator is line feed (`\n`, `0x0a`).

example:

```text
1776098721818 w
```

this line answers: when, exactly, did this human begin?

### subsequent lines

```text
{delta_ms} {character_or_SPACE}\n
```

`delta_ms` is the elapsed milliseconds since the previous accepted keystroke.
it is zero-padded to 4 digits and capped to the range `0000` to `7999`.

`character_or_SPACE` is the actual character that was typed,
except that the space character is represented by the exact ascii token `SPACE`.
No other token aliases are defined.

example:

```text
0048 r
0131 i
0041 t
0162 e
0301 SPACE
```

### the space character

a typed space is stored as the exact token:

```text
SPACE
```

this avoids ambiguous trailing whitespace and future parser errors.
the line for a typed space contains the delta, one separator space, and then
the four ascii letters `SPACE`.

```text
0301 SPACE
```

means: 301ms elapsed, then the space key was pressed.

literal trailing-space payloads are not canonical:

```text
0301␠␠
```

must be rejected by compliant parsers or migrated before hashing.
Here `␠` is only a visual marker for a space byte; it is not part of the file.

### the terminal line

the session closes with:

```text
8000
```

`8000` is reserved.
it is not a character record.
it is the terminal silence marker.

it means the writing fell into silence long enough for the client
to close the session.

the file ends after this line.
nothing follows it.

### encoding

- the file is utf-8 encoded
- line endings are `\n` only, never `\r\n`
- no byte order mark
- character payloads should be normalized by the client before capture if the client performs text normalization at all

### what is not in the file

- the writer's name, wallet, or device
- word count, duration, or any derived metric
- commentary about the writing
- model output
- anything that did not happen in the writing stream itself

---

## a small real example

```text
1712345678000 h
0204 e
0187 l
0143 l
0198 o
0301 SPACE
0089 w
8000
```

this reconstructs as:

```text
hello w
```

what this string preserves:

- the exact moment the session began
- every inter-keystroke interval
- the fact that a real space was typed, represented unambiguously as `SPACE`
- the final terminal silence that closed the session

the text is not all that is here.
the rhythm is here too.

---

## the hash

```text
session_hash = sha256(raw_utf8_bytes_of_the_file)
```

computed on the writing device.
never by a server if the implementation cares about local truth.

the hash is the identity of the session.
the filename may be the hash:

```text
{session_hash}.anky
```

verification is offline and requires no authority:

```python
sha256(open(filepath, "rb").read()).hexdigest() == filepath.split("/")[-1].replace(".anky", "")
```

if this holds, the file is intact and unmodified since the session ended.
if it fails, the file has changed.

---

## verification levels

the protocol supports four verification levels.
each level is independent and builds on the previous.

**level 0 — structural validity**
the file parses according to the format rules above:
one opening epoch line, zero or more delta-character lines,
and one terminal `8000` line.

**level 1 — integrity**
the sha-256 of the raw utf-8 bytes matches the expected hash.
the file has not been modified since it was created.

**level 2 — anchored existence**
the session hash appears in a valid public anchor.
this proves the hash existed no later than the anchor timestamp,
under control of whatever key or wallet produced the anchor.

**level 3 — claimed provenance**
a wallet, application, or device claims association with the session.
this relies on external systems.
the protocol itself does not establish this.

**level 4 — humanness**
the protocol does not reach level 4.
it cannot prove the session was written by an unaided human.
it cannot prove the writer was not using automation, scripted replay,
accessibility injection, or a modified client.

the hash proves integrity.
the anchor proves existence.
neither proves authorship.
the protocol is honest about this boundary.

---

## what the data reveals

given only the `.anky` file, a reader can derive:

- exact session start time
- duration of every inter-keystroke interval
- pause patterns and hesitation points
- rhythm consistency across the session
- reconstructed text including typos and disfluencies
- the final silence that closed the session

given only the `.anky` file, a reader cannot determine:

- why any particular pause occurred
- who wrote it, without external linkage
- whether the writing was unaided
- what meaning should be assigned to the rhythm

the silence between characters is data.
it is not commentary.
it is not interpretation.
it is only the record.

---

## capture compliance

a capture client is protocol-compliant only if it enforces:

- rejection of backspace, delete, arrow keys, enter, paste, and editing operations
- no synthetic content records beyond the required opening epoch line and closing `8000` line
- literal storage of typed characters, with typed spaces encoded as `SPACE`
- zero-padded 4-digit deltas for all non-initial character lines
- delta capping at `7999`
- local hash computation from exact file bytes
- no modification of the `.anky` file after session end
- utf-8 output with `\n` line endings

a compliant client should also document its policy on:
autocorrect, text substitution, IME composition, voice input,
and accessibility input methods.

if any of these are permitted, they belong in derived metadata,
not in the canonical file.

---

## optional public anchoring

the session hash may be anchored publicly.

anchoring is not required for validity.
the `.anky` file stands on its own.

anchoring proves:

- this exact hash existed
- no later than the anchor timestamp
- under control of the key or wallet that produced the anchor

anchoring does not reveal session content.
the anchor layer may evolve.
the canonical file does not.

---

## filesystem layout

sessions can live anywhere files can live.

```text
~/ankys/
  2026/
    04/
      13/
        {session_hash}.anky
```

no database required.
no server required.
no app required.

an obsidian vault. a git repository. icloud drive. a usb drive.
anywhere the filesystem exists, ankys can live.

---

## derived data

the `.anky` file is the sole canonical record.
if it is lost, the session is lost.
if it exists, every downstream artifact can be regenerated from it.

derived artifacts may live beside it:

```text
{hash}.anky
{hash}.reflection.md
{hash}.image.webp
{hash}.meta.json
{hash}.conversation.json
```

a sidecar can be deleted and rebuilt.
the `.anky` file cannot.

metadata is downstream.
the writing is upstream.

---

## what this protocol is not

it is not a journaling app.
it is not a productivity tool.
it is not a proof-of-humanness system.
it is not consciousness itself.

it is a keystroke protocol.
that is already enough.

it captures the noisy, wandering, alive mind
exactly as it moved through the keyboard.

---

## reference parser

```python
import hashlib
import os


def parse_anky(filepath):
    with open(filepath, "rb") as f:
        raw = f.read()

    text = raw.decode("utf-8")
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines = lines[:-1]

    if len(lines) < 2:
        raise ValueError("invalid anky: needs at least a start line and terminal 8000")

    if lines[-1] != "8000":
        raise ValueError("invalid anky: missing terminal 8000 line")

    records = []

    first = lines[0]
    first_sep = first.find(" ")
    if first_sep == -1:
        raise ValueError("line 1: missing separator")
    epoch_str = first[:first_sep]
    first_char = first[first_sep + 1 :]
    if not epoch_str.isdigit():
        raise ValueError("line 1: invalid epoch")
    if first_char == "":
        raise ValueError("line 1: missing character")

    records.append({
        "kind": "start",
        "ms": int(epoch_str),
        "char": first_char,
    })

    for i, line in enumerate(lines[1:-1], start=2):
        sep = line.find(" ")
        if sep == -1:
            raise ValueError(f"line {i}: missing separator")
        ms_str = line[:sep]
        char = line[sep + 1 :]
        if len(ms_str) != 4 or not ms_str.isdigit():
            raise ValueError(f"line {i}: invalid 4-digit delta")
        if char == "":
            raise ValueError(f"line {i}: missing character")
        delta = int(ms_str)
        if delta < 0 or delta > 7999:
            raise ValueError(f"line {i}: delta out of range")
        records.append({
            "kind": "keystroke",
            "ms": delta,
            "char": char,
        })

    return records


def reconstruct(records):
    return "".join(r["char"] for r in records)


def start_time(records):
    return records[0]["ms"]


def verify(filepath):
    expected = os.path.basename(filepath).removesuffix(".anky")
    with open(filepath, "rb") as f:
        computed = hashlib.sha256(f.read()).hexdigest()
    return computed == expected
```

---

## longevity

the format is deliberately minimal so that a file written today
remains readable by any parser written decades from now.

if this specification changes, new rules should be additive.
existing canonical files should never be reinterpreted.

the anchor layer may change.
derived metadata may change.
client implementations may change.
the file remains what it was when it was written.

---

## the bottom line

a textarea. a timer. a file. a hash. an optional public anchor.

you write forward.
you do not edit.
what comes out is real.

the proof can be public.
the content can remain yours.
the rhythm lives in the string as long as the file does.

*the writing is the seed. everything else is fruit.*
