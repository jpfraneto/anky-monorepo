# anky-loom-engine

Local TypeScript engine for generating a privacy-preserving SVG loom from `.anky` timing files.

## Input

Each `.anky` file is UTF-8 plain text:

```text
{epoch_ms} {character}
{delta_ms} {character}
{delta_ms} {character}
```

The character field is parsed only to preserve the format, including literal spaces. Characters are never stored in state and never written to the SVG.

## Generate

```bash
npm install
npm run generate -- ./input
npm run generate -- ./input --beats 16
npm run generate -- ./input --mode presence
```

## Realtime Chamber

Open `realtime-loom.html` directly in a browser to try the live keystroke loom. It does not show a text box or transcript: the current key appears at the center, and when the next key arrives the previous glyph travels out as a slower shuttle and settles into a non-readable thread. The live chamber uses a continuous thread head rather than pin-to-pin jumps, so the weave grows from the writing gesture instead of teleporting around the loom. Completed threads are painted onto a persistent canvas layer for performance instead of being redrawn every frame. The settled weave stays quiet while writing, brightens during pauses, and can be toggled with the reveal button.

Render modes:

- `weave`: default full loom
- `presence`: stronger origin pins with quieter threads
- `flow`: stronger flow-bucket contrast
- `time`: stronger day-order brightness, with newer sessions rendered brighter and on top

The CLI walks the input folder recursively, so `.anky` files inside day folders and branch folders are included. Files are sorted by relative path for deterministic output.

Day assignment is inferred in this order:

- filenames containing `day-N`, `day_N`, or `day N`
- ancestor folders named `day-N`, `day_N`, or `day N`
- date-style folders such as `03/10/...`, which assign day `10`
- the nearest numeric ancestor folder
- sorted order as a fallback

It writes:

- `output/loom.svg`
- `output/loom-state.json`

The rendered loom uses only timing-derived data, the SHA-256 hash of each raw `.anky` string, day assignment, and deterministic geometry. The state file does not store raw text, reconstructed text, characters, raw deltas, or rhythm samples. Per-session state includes the day, session hash, route pin indices, motif IDs, flow bucket, beat count, route diagnostics, render mode, and render version. Per-pin state marks each of the 96 pins as empty, visited, or origin, with visit counts and origin session hashes for activated writing days.
