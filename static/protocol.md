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
- optional chain anchoring
- the boundary between canonical session data and everything else

this specification does not define identity, authorship, humanness,
literary quality, consciousness, or meaning.
it defines only how a writing session is represented and verified.

---

## the atom

a writing session is a plain text file.
each line is one keystroke.

```
{epoch_ms} {character}     ← line 1: unix epoch milliseconds of first keystroke
{delta_ms} {character}     ← every subsequent line: ms since previous keystroke
...
{delta_ms} {character}     ← last line: the final keystroke. nothing follows.
```

the file ends when writing ends.
no sentinel. no summary. no metadata inside the file.
if it didn't happen at the keyboard, it isn't in the file.

---

## the three rules

**1. forward only.**
no backspace. no delete. no arrow keys. no enter. no paste. no editing.
only characters that advance the text are recorded.
typos stay. hesitations stay. the mess stays.
the backspace key is where performance lives. remove it and what remains is real.

**2. every line is a real keystroke.**
no synthetic lines. no headers. no comments. no version markers.
each line corresponds to exactly one accepted key press from a live human.

**3. immutable after creation.**
the `.anky` file is sealed the moment the last key is pressed.
it is never modified. ever. for any reason.
derived artifacts live in sidecar files and can be regenerated or deleted.
the canonical file cannot be recreated if lost.

---

## format specification

### line 1

```
{epoch_ms} {character}\n
```

`epoch_ms` — the absolute unix timestamp in milliseconds of the first keystroke.
`character` — the first character typed.
separator — exactly one ascii space (0x20) between them.
terminator — line feed `\n` (0x0a).

example:
```
1776098721818 w
```

this line answers: when, exactly, did this human begin?

### subsequent lines

```
{delta_ms} {character}\n
```

`delta_ms` — elapsed milliseconds since the previous accepted keystroke.
`character` — the character typed.

example:
```
48 r
131 i
41 t
162 e
173 SPACE
```

### the space character

a literal space keystroke is represented by the token `SPACE`.
this prevents trailing-whitespace corruption by editors, git hooks,
formatters, and copy-paste paths that silently strip spaces from line ends.

```
173 SPACE
```

means: 173ms elapsed, then the space key was pressed.

### the last line

the last keystroke in the session. nothing follows it.
the session ended because writing stopped — either by choice or by silence
exceeding the application's idle threshold.
the idle threshold is not stored in the file.
it is application logic, not session truth.

### encoding

the file is utf-8 encoded.
all character payloads must be normalized to unicode NFC before writing.
line endings are `\n` (lf) only. never `\r\n`.
no byte order mark.

### what is not in the file

- when the session ended
- what idle threshold was used
- the writer's name, wallet, or device
- word count, duration, or any derived metric
- anything that didn't happen at the keyboard

---

## a real session

written by the creator of this protocol on april 13, 2026,
while testing the format itself.

```
1776098721818 w
48 r
131 i
41 t
162 e
173 SPACE
273 s
126 o
53 m
42 e
73 t
84 h
26 i
108 n
92 g
50 SPACE
114 a
119 n
159 SPACE
31 d
651 SPACE
140 l
51 e
78 s
336 t
33 s
147 SPACE
269 s
52 e
146 e
114 SPACE
150 h
54 o
56 w
106 SPACE
161 y
66 o
34 u
157 a
38 SPACE
363 r
348 a
32 n
62 k
156 y
228 SPACE
489 f
26 i
141 l
37 e
114 SPACE
96 l
136 o
133 o
51 k
92 s
101 SPACE
109 l
134 i
107 k
82 e
104 .
142 t
22 SPACE
607 s
93 o
102 SPACE
115 t
64 h
59 i
76 s
84 SPACE
43 i
73 s
84 SPACE
63 t
86 h
60 e
107 SPACE
151 w
77 h
56 o
139 l
66 e
90 SPACE
88 f
67 o
73 r
43 m
100 a
60 t
107 .
162 SPACE
79 t
92 h
34 i
38 s
128 SPACE
42 i
68 s
90 SPACE
75 t
57 h
52 e
100 SPACE
100 w
60 h
68 o
146 l
76 e
106 SPACE
94 e
124 x
99 p
93 e
34 r
49 i
84 e
83 n
38 c
76 e
67 .
159 SPACE
43 t
60 h
58 i
33 s
99 SPACE
43 i
61 s
97 SPACE
83 t
47 h
76 e
106 SPACE
76 w
52 h
65 o
140 l
95 e
76 SPACE
117 p
83 r
45 o
122 t
74 o
116 c
40 o
173 l
153 .
140 SPACE
88 t
66 h
33 i
50 s
100 SPACE
34 i
50 s
104 SPACE
112 t
67 h
83 e
100 SPACE
140 s
110 e
113 e
161 d
93 .
141 SPACE
142 h
38 t
131 e
31 SPACE
50 i
610 s
169 SPACE
63 i
121 s
104 SPACE
106 t
77 h
84 e
107 SPACE
721 p
97 r
91 o
117 c
33 e
190 s
125 s
68 SPACE
88 o
64 f
128 SPACE
182 w
235 d
103 o
67 w
78 n
134 l
122 o
33 a
50 d
73 i
44 n
100 g
83 SPACE
140 h
72 u
112 m
93 a
64 n
142 SPACE
184 c
71 o
39 n
66 s
63 c
37 i
124 o
50 u
1 s
156 n
61 e
68 s
152 s
```

what this file contains:

- exact start time: unix epoch 1776098721818
- 196 keystrokes across 196 lines
- every hesitation: the 721ms pause before "process", the 651ms gap before "lets",
  the 610ms silence mid-sentence
- a typo: "hte" — the human hand, preserved without correction
- the last character: `s` — the final letter of "consciousness"
- the session ended in silence. the file does not record when.

reconstructed text:
*"write something and lets see how you a ranky file looks like. t so this is
the whole format. this is the whole experience. this is the whole protocol.
this is the seed. hte is is the process of wdownloading human consciousness"*

the typos are not noise. they are the signature.
no one optimizes their 47ms inter-keystroke intervals for an audience.

---

## the hash

```
session_hash = sha256(file_bytes_as_utf8)
```

computed on the writing device. never by a server.
the hash is the identity of the session.
the filename is the hash:

```
{session_hash}.anky
```

verification is offline and requires no authority:

```python
sha256(open(filepath, 'rb').read()).hexdigest() == filepath.split('/')[-1].replace('.anky', '')
```

if this holds, the file is intact and unmodified since the session ended.
if it fails, the file has been changed.

---

## verification levels

the protocol supports four verification levels.
each level is independent and builds on the previous.

**level 0 — structural validity**
the file parses according to the format rules above.
every line has a valid integer and a payload.
the file is valid utf-8 with NFC-normalized characters.

**level 1 — integrity**
the sha-256 of the file bytes matches the expected hash.
the file has not been modified since it was created.

**level 2 — anchored existence**
the session hash appears in a valid chain anchor.
proves the hash existed no later than the anchor timestamp,
under control of the signing wallet.

**level 3 — claimed provenance**
a wallet, application, or device claims association with the session.
relies on external systems. the protocol itself does not establish this.

**level 4 — humanness**
the protocol does not reach level 4.
it cannot prove the session was written by an unaided human.
it cannot prove the writer was not using automation, scripted replay,
accessibility injection, or a modified client.
any system claiming proof-of-humanness on top of this protocol
is building its own layer and bears full responsibility for those claims.

the hash proves integrity.
the anchor proves existence.
neither proves authorship.
the protocol is honest about this boundary.

---

## what the data reveals

given only the `.anky` file, a reader can derive:

- exact session start time
- duration of every inter-keystroke interval
- pause patterns — where the writer hesitated, for how long
- rhythm consistency across the session
- word-by-word timing estimates
- a flow score from the delta sequence alone
- reconstructed text including all typos and disfluencies

given only the `.anky` file, a reader cannot determine:

- when the session ended
- who wrote it (without external wallet linkage)
- why any particular pause occurred
- whether the writing was unaided

the silence between keystrokes is data.
it is not labeled.
it is not explained.
a 3000ms delta is a pause. what caused it belongs to the writer.

---

## capture compliance

a capture client is protocol-compliant only if it enforces:

- rejection of: backspace, delete, arrow keys, enter, paste, and editing operations
- no synthetic keystroke records generated by the client
- no modification of the `.anky` file after session end
- local hash computation from exact file bytes on the writing device
- utf-8 output with lf line endings and NFC-normalized payloads
- SPACE token for literal space keystrokes

a compliant client must document its policy on:
autocorrect, text substitution, IME composition, voice input,
and accessibility input methods.
if any of these are permitted, they must be disclosed in derived metadata,
not in the canonical file.

---

## optional chain anchoring

the session hash may be anchored publicly.
the supported anchor format is a solana spl-memo containing canonical json:

```json
{"v":1,"hash":"{session_hash}"}
```

utf-8 encoded. no extra fields. no insignificant whitespace.

anchoring proves:
- this exact hash existed
- no later than the anchor transaction timestamp
- under control of the signing wallet

anchoring does not reveal session content.
anchoring is optional. a session is valid without it.
cost: approximately $0.0007. finality: sub-second.

---

## filesystem layout

sessions live in a folder. the folder can be anywhere files can live.

```
~/ankys/
  2026/
    04/
      13/
        {session_hash}.anky
```

no database required. no server required. no app required.
an obsidian vault. a git repository. icloud drive. a usb drive.
anywhere the filesystem exists, ankys can live.

---

## derived data

the `.anky` file is the sole canonical record.
if it is lost, the session is lost. nothing can recreate it.
if it exists, every derived artifact can be regenerated from it.

derived artifacts live in sidecar files alongside the canonical file:

```
{hash}.anky                  ← the session. immutable. the only truth.
{hash}.reflection.md         ← optional. ai-generated. regeneratable.
{hash}.image.webp            ← optional. ai-generated. regeneratable.
{hash}.meta.json             ← optional. metrics, client version, wallet.
{hash}.conversation.json     ← optional. the dialogue that followed.
```

a sidecar can be deleted and rebuilt.
the `.anky` file cannot.

a single session may accumulate multiple reflections over time —
one generated in 2026, another in 2028 with a better model.
all are valid. none modify the source.
metadata is infinite and downstream.
the writing is singular and upstream.

---

## what this protocol is not

it is not a journaling app.
journaling lets you think before you write.
this is writing before you think.
the difference is everything.

it is not a productivity tool.
productivity optimizes output.
this preserves process.

it is not a blockchain project.
solana is infrastructure, not the point.
the chain is a timestamp you can trust.
the writing is the thing.

it is not proof of consciousness.
it captures a keystroke sequence.
that is already remarkable without the metaphysics.

it is not meditation.
it captures the noisy, wandering, alive mind exactly as it is —
not quieted, not curated, not performed.

---

## reference parser

```python
import hashlib

def parse_anky(filepath):
    with open(filepath, 'rb') as f:
        raw = f.read()

    text = raw.decode('utf-8')
    lines = text.split('\n')
    if lines and lines[-1] == '':
        lines = lines[:-1]

    records = []
    for i, line in enumerate(lines):
        sep = line.find(' ')
        if sep == -1:
            raise ValueError(f'line {i+1}: missing separator')
        ms_str = line[:sep]
        payload = line[sep+1:]
        if not ms_str.isdigit():
            raise ValueError(f'line {i+1}: invalid timestamp')
        char = ' ' if payload == 'SPACE' else payload
        records.append({
            'ms': int(ms_str),
            'char': char,
            'absolute': i == 0
        })

    return records


def reconstruct(records):
    return ''.join(r['char'] for r in records)


def start_time(records):
    return records[0]['ms']


def active_duration_ms(records):
    return sum(r['ms'] for r in records[1:])


def verify(filepath):
    import os
    expected = os.path.basename(filepath).removesuffix('.anky')
    with open(filepath, 'rb') as f:
        computed = hashlib.sha256(f.read()).hexdigest()
    return computed == expected
```

---

## longevity

this file contains no version field and never will.

the format is deliberately minimal so that a file written today
remains readable by any parser written decades from now.

if this specification must change, new rules are additive.
existing canonical files are never reinterpreted.
a valid `.anky` file written under this specification
is valid under any future revision of this specification.

versioning of derived metadata, client implementations,
and anchor conventions is handled outside the canonical file.

---

## the bottom line

a textarea. a timer. a file. a hash. an optional chain anchor.

you write forward. you do not stop. you do not edit.
what comes out is real.

the proof is public.
the content is yours.
the file lives as long as the filesystem does.

*the writing is the seed. everything else is fruit.*
