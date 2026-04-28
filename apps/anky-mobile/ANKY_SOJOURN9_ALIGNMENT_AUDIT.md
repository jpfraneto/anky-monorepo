# Anky Mobile Sojourn 9 Alignment Audit

## 1. Current files/screens related to writing capture

- `src/screens/WriteScreen.tsx`: hidden `TextInput` capture, forward-only character handling, 8-second silence close, 8-minute rite timer, active draft persistence.
- `src/components/anky/WordFocus.tsx`: visual emphasis for the latest accepted character.
- `src/lib/ankyProtocol.ts`: `.anky` line creation, parsing, reconstruction, terminal validation, local SHA-256 hash computation.
- `src/lib/ankyStorage.ts`: active draft, pending reveal, canonical hash-named `.anky` files, local seal sidecars, local artifact sidecars.
- `src/screens/RevealScreen.tsx`: reconstructed text, hash verification, copy, release, mock loom seal, mirror entry points.
- `src/screens/PastScreen.tsx`: offline local archive list.
- `src/screens/EntryScreen.tsx`: local `.anky` read, verification, seal details, derived artifact status.
- `src/screens/LoomScreen.tsx`: home/loom view, recoverable draft detection, local loom lineage summary.

## 2. Current `.anky` protocol compliance status

Implemented:

- UTF-8 hashing is computed locally from the raw `.anky` string encoded as UTF-8 bytes.
- First accepted character writes `{epoch_ms} {character}`.
- Subsequent accepted characters write zero-padded 4-digit deltas capped at `7999`.
- Literal typed spaces are preserved as separator space plus typed space.
- Terminal marker is now exactly the final line `8000` with no trailing newline or text.
- Parser rejects CRLF, missing terminal marker, extra text after terminal marker, invalid deltas, invalid characters, and BOM.
- Reconstruction returns only typed characters.

Input policy:

- Backspace/delete/enter/arrow keys are rejected.
- Paste and replacement text are rejected when more than one committed character arrives.
- Autocorrect and spellcheck are disabled where React Native exposes controls.
- IME composition is accepted only when the platform commits exactly one Unicode character.
- Voice input and text substitution are not intentionally supported.
- Accessibility input is not blocked, but it must still commit one accepted character at a time.

Remaining risk:

- React Native platform keyboards can vary. The code rejects multi-character commits, but some OS-level input methods may behave differently and need device testing.

## 3. Current storage behavior

- Active writing is persisted as `active.anky.draft`.
- On close, the app writes the immutable canonical file as `{session_hash}.anky`.
- `pending.anky` is only a local reveal/recovery copy.
- Offline archive listing reads local `.anky` files and verifies hashes without backend dependency.
- Seal events are local sidecars named `{hash}.seals.json`.
- Processing artifacts are sidecars such as `{hash}.reflection.md`, `{hash}.title.txt`, `{hash}.image.webp`, and `{hash}.meta.json`.
- Local state resolves to `drafting`, `closed`, `verified`, `sealed`, or `processed`.

## 4. Current seal/Solana behavior

- Real Solana sealing is not implemented.
- A clean adapter boundary exists:
  - `src/lib/solana/types.ts`
  - `src/lib/solana/loomClient.ts`
  - `src/lib/solana/loomClient.mock.ts`
- Screens call `sealAnky({ sessionHash, loomId })` and do not know whether the adapter is mocked or real.
- The mock client enforces a valid 32-byte lowercase hex session hash and ownership of a mock `Anky Sojourn 9 Loom`.
- The mock supports infinite seals conceptually; no daily or wallet seal limit exists.
- The seal flow sends only the hash and loom id, never raw `.anky` text.

Remaining gap:

- Replace the mock adapter with a devnet/mainnet Solana implementation before claiming public on-chain anchoring.

## 5. Current backend/API behavior

- Raw `fetch` calls are isolated in `src/lib/api/ankyApi.ts`.
- API contract types live in `src/lib/api/types.ts`.
- Implemented wrappers:
  - `GET /api/v1/config`
  - `GET /api/v1/credits/balance`
  - `POST /api/v1/credits/checkout`
  - `POST /api/v1/processing/tickets`
  - `POST /api/v1/processing/run`
  - `GET /api/v1/seals?...`
- Credit receipts are validated client-side before processing calls proceed.

Remaining gap:

- `CreditsScreen` reads `EXPO_PUBLIC_ANKY_API_URL`; without that value it shows that backend processing is unavailable and does not spend credits.

## 6. Current AI processing behavior

- Credit products and canonical costs are typed.
- The app can build an `AnkyCarpet` locally from one or many verified `.anky` strings.
- Ticket requests contain only processing type, count, and hashes.
- Raw `.anky` text enters a carpet only after the user explicitly starts processing.
- Plaintext carpet upload is only available when backend config returns `devPlaintextProcessingAllowed: true`.
- Encrypted `x25519_v1` carpet upload is not implemented and throws instead of silently sending plaintext.
- Returned artifacts can be stored locally as sidecars beside `.anky` files.

Remaining gap:

- Implement encryption before production processing can accept carpets safely.

## 7. Gaps against this spec

- Real Solana transaction construction/signing is mocked.
- Real loom ownership discovery is mocked.
- Encrypted carpet processing is not implemented.
- Backend base URL and production processing service are not configured in this repo.
- App background/crash recovery is improved by active draft and pending reveal files, but there is no separate transaction journal or fsync-level guarantee.
- Device-level testing is still needed for platform keyboard edge cases: autocorrect, smart punctuation, IME composition, voice input, and accessibility input.

## 8. Proposed minimal implementation plan

Completed in this alignment pass:

- Tighten `.anky` terminal/BOM validation and local hash verification.
- Start the 8-minute write timer on first accepted character.
- Persist canonical `{hash}.anky` files immediately when the thread closes.
- Stop passing raw `.anky` through navigation params.
- Add local `.anky` state resolution and expose states in archive/entry UI.
- Add clean loom types and a mocked `sealAnky` adapter.
- Store seal lineage locally as sidecars without modifying `.anky`.
- Add typed credit products, receipts, API client wrappers, carpet builder, and sidecar artifact storage.
- Add minimal credits/mirror UI.
- Add tests for protocol parsing/reconstruction/hash, literal spaces, terminal validation, carpets, credit costs/receipt validation, artifact sidecars, and mock loom sealing.

Next implementation steps:

- Wire a real wallet and Solana loom adapter behind `src/lib/solana/loomClient.ts`.
- Add production encryption for carpets and remove reliance on the dev plaintext path.
- Connect `EXPO_PUBLIC_ANKY_API_URL` to the deployed backend.
- Add device tests for keyboard/input policy edge cases.
