# AGENTS.md — Anky Sojourn 9 Solana Launch Constitution

This file is the canonical operating context for Codex working on the Anky monorepo during the Solana Frontier / Colosseum sprint.

The mission is to make Anky Sojourn 9 launch-ready: a React Native mobile writing ritual connected to Solana through Metaplex Core Looms, a custom Anchor hash-seal program, a runnable SP1 proof path, and Helius-indexed proof-of-practice scoring.

The active sojourn begins with the Colosseum submission. After that point, core infrastructure should freeze. The work before launch must therefore prioritize the real seal/proof/indexing path over speculative future architecture.

---

## 0. Highest-Level Mission

Anky is a mobile-first proof-of-practice protocol.

The user writes privately for 8 minutes in a canonical `.anky` file. The content stays local by default. The app computes SHA-256 over the exact UTF-8 `.anky` bytes and seals only that hash on Solana.

The protocol statement is:

> Wallet W privately completed one valid `.anky` rite for UTC day D, producing hash H, without revealing the writing.

For Sojourn 9, the infrastructure must support:

1. React Native mobile write/seal UX.
2. Metaplex Core Loom ownership as the season access artifact.
3. Custom Anchor Anky Seal Program for `DailySeal`, `HashSeal`, `LoomState`, and `VerifiedSeal`.
4. SP1 proof generation and local verification for private `.anky` validity.
5. Authority-gated `record_verified_anky` submission after off-chain SP1 verification.
6. Helius-backed indexing of sealed and verified days.
7. Deterministic practice-based scoring for an 8% token supply distribution to up to 3,456 Sojourn 9 participants.

Winning Colosseum means showing a working consumer product with Solana-native infrastructure, not only a protocol document. Optimize for a demo a judge can understand in minutes:

> write on phone → hash exact `.anky` bytes → seal on Solana → prove privately with SP1 → attach verified receipt → index score → show fair reward logic.

---

## 1. Non-Negotiable Truths

### Privacy

- Never require `.anky` plaintext for scoring.
- Never store writing plaintext as part of the canonical proof/scoring system.
- Plaintext may only enter a backend/prover path as an explicit opt-in operation and must be treated as transient process memory.
- If plaintext is accepted by any endpoint or script, the implementation must document that it is transient, must not persist it, and must return only derived receipt/proof metadata.
- Do not log `.anky` plaintext.
- Do not print `.anky` plaintext in errors.
- Do not put `.anky` plaintext in database rows, queue payloads, analytics events, or webhook payloads.

### Hashing

- The sealed commitment is SHA-256 over exact canonical `.anky` UTF-8 bytes.
- Do not normalize line endings before hashing.
- Do not trim trailing whitespace before hashing.
- Do not hash reconstructed prose.
- Do not hash parsed JSON.
- Do not hash a transformed representation unless explicitly versioned as a new protocol.

### ZK / SP1 honesty

Current truth:

- SP1 proof generation exists in `solana/anky-zk-proof`.
- The SP1 guest proves private `.anky` validity and commits public receipt values.
- SP1 Core proof has been generated and locally verified from the current code path.
- The Solana program currently records verifier-authority-attested proof receipts.
- Direct on-chain SP1/Groth16 verification is not implemented today.

Safe wording:

- “ZK-enabled proof-of-practice.”
- “SP1 proves private `.anky` validity off-chain.”
- “The current on-chain verified badge is verifier-authority-attested after off-chain SP1 verification.”
- “Future hardening is direct on-chain SP1/Groth16 verification.”

Forbidden wording unless implemented and tested:

- “Fully trustless ZK on Solana.”
- “Solana verifies the SP1 proof directly today.”
- “The hash encrypts the writing.”
- “Anonymous writing.”
- “The chain proves the user wrote for 8 minutes” without explaining the timing model.

### Mainnet honesty

- Do not claim mainnet deployment happened unless it actually happened.
- Do not invent mainnet program IDs, collection addresses, token mint addresses, webhook IDs, deployment signatures, or audit status.
- Stop before any mainnet deployment or mainnet transaction unless explicitly instructed by the human.
- If mainnet values are missing, use placeholders and call them out.

### Secrets

Never read, print, copy, commit, or summarize secret values:

- `.env`, `.env.*`
- keypair JSON files
- wallet files
- deployer files
- private keys
- API keys
- paid service tokens
- Helius API key values
- Apple/Google/Stripe/Privy credentials

Env var names are okay. Values are not.

---

## 2. Repo Reality From Reconnaissance

The monorepo root observed on poiesis:

```text
/home/kithkui/anky
```

Top-level structure includes:

```text
apps/
  anky-mobile/
  anky-content-os/
  anky-loom-engine/
solana/
  anky-seal-program/
  anky-zk-proof/
  scripts/
  setup/
  worker/
src/
migrations/
docs/
contracts/
scripts/
sojourn9/
handoff/
static/
tools/
```

Important status:

- Git branch observed: `main`.
- Worktree has many modified/untracked files.
- `solana/anky-zk-proof/` is untracked but is critical launch work.
- Active mobile app lives in `apps/anky-mobile`.
- Active Anchor seal program lives in `solana/anky-seal-program`.
- Active SP1 proof path lives in `solana/anky-zk-proof`.
- Active backend route file is `src/routes/mobile_sojourn.rs`.
- Helius-backed Anky seal/verified scoring is not implemented yet.

Prefer running Codex inside a git worktree on poiesis, not in an empty detached folder.

Recommended human setup:

```bash
cd /home/kithkui/anky
git status
git worktree add ../anky-solana-master -b sojourn9-mainnet-zk-freeze
cd ../anky-solana-master
# place this AGENTS.md at repo root
codex
```

Codex may edit this worktree. Codex must not deploy, use private keys, or spend funds unless explicitly instructed by the human.

---

## 3. Canonical Active Paths

### Mobile

```text
apps/anky-mobile/src/lib/ankyProtocol.ts
apps/anky-mobile/src/lib/ankyProtocol.test.ts
apps/anky-mobile/src/lib/ankyStorage.ts
apps/anky-mobile/src/lib/ankyState.ts
apps/anky-mobile/src/screens/WriteScreen.tsx
apps/anky-mobile/src/screens/RevealScreen.tsx
apps/anky-mobile/src/screens/LoomScreen.tsx
apps/anky-mobile/src/components/seal/SwipeToSealAction.tsx
apps/anky-mobile/src/lib/solana/sealAnky.ts
apps/anky-mobile/src/lib/solana/ankySolanaConfig.ts
apps/anky-mobile/src/lib/solana/mobileSolanaConfig.ts
apps/anky-mobile/src/lib/solana/mintLoom.ts
apps/anky-mobile/src/lib/solana/mobileLoomMint.ts
apps/anky-mobile/src/lib/solana/loomStorage.ts
apps/anky-mobile/src/lib/privy/useAnkyPrivyWallet.ts
apps/anky-mobile/src/lib/privy/ExternalSolanaWalletProvider.tsx
apps/anky-mobile/src/lib/privy/PrivyProvider.tsx
```

### Solana / Anchor

```text
solana/anky-seal-program/Anchor.toml
solana/anky-seal-program/Cargo.toml
solana/anky-seal-program/programs/anky-seal-program/Cargo.toml
solana/anky-seal-program/programs/anky-seal-program/src/lib.rs
solana/anky-seal-program/tests/anky-seal-program.ts
solana/scripts/admin/
```

### SP1 / ZK

```text
solana/anky-zk-proof/Cargo.toml
solana/anky-zk-proof/src/lib.rs
solana/anky-zk-proof/src/main.rs
solana/anky-zk-proof/fixtures/full.anky
solana/anky-zk-proof/sp1/program/src/main.rs
solana/anky-zk-proof/sp1/script/src/bin/main.rs
solana/anky-zk-proof/sp1/script/src/bin/vkey.rs
```

Generated proof artifacts may exist:

```text
solana/anky-zk-proof/sp1/script/receipt.json
solana/anky-zk-proof/sp1/script/proof-with-public-values.bin
```

Do not treat generated artifacts as source of truth unless regenerated or explicitly inspected as expected outputs.

### Backend

```text
src/routes/mobile_sojourn.rs
migrations/
```

Relevant existing routes include:

```text
GET  /api/v1/config
GET  /api/v1/credits/balance
POST /api/v1/credits/checkout
POST /api/v1/processing/tickets
POST /api/v1/processing/run
GET  /api/v1/seals
GET  /api/mobile/solana/config
POST /api/mobile/looms/mint-authorizations
POST /api/mobile/looms/prepare-mint
POST /api/mobile/looms/record
GET  /api/mobile/looms
POST /api/mobile/reflections
GET  /api/mobile/reflections/{job_id}
GET  /api/mobile/seals
POST /api/mobile/seals/record
```

Prover endpoint not found at reconnaissance time.

### Existing DB tables / migrations to inspect

```text
mobile_credit_accounts
mobile_credit_events
mobile_mint_authorizations
mobile_loom_mints
mobile_seal_receipts
mobile_reflection_jobs
mobile_credit_purchases
sealed_sessions
```

Known migration references:

```text
migrations/017_mobile_solana_integration.sql
migrations/018_mobile_native_credit_purchases.sql
```

---

## 4. Current Anchor Program Facts

Program source:

```text
solana/anky-seal-program/programs/anky-seal-program/src/lib.rs
```

Declared program ID observed:

```text
4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX
```

Important caveat:

- This ID appears configured for devnet, localnet, and mainnet in `Anchor.toml`.
- Provider cluster was observed as devnet.
- Mainnet deployment status must be confirmed before any public claim.

Instructions:

```rust
pub fn seal_anky(
    ctx: Context<SealAnky>,
    session_hash: [u8; 32],
    utc_day: i64,
) -> Result<()>

pub fn record_verified_anky(
    ctx: Context<RecordVerifiedAnky>,
    session_hash: [u8; 32],
    utc_day: i64,
    proof_hash: [u8; 32],
    protocol_version: u16,
) -> Result<()>
```

Accounts:

```text
LoomState:
  loom_asset
  total_seals
  latest_session_hash
  rolling_root
  created_at
  updated_at

DailySeal:
  writer
  loom_asset
  session_hash
  utc_day
  timestamp

HashSeal:
  writer
  loom_asset
  session_hash
  utc_day
  timestamp

VerifiedSeal:
  writer
  session_hash
  utc_day
  proof_hash
  verifier
  protocol_version
  timestamp
```

PDA seeds:

```text
LoomState:    [b"loom_state", loom_asset]
DailySeal:    [b"daily_seal", writer, utc_day_le_bytes]
HashSeal:     [b"hash_seal", writer, session_hash]
VerifiedSeal: [b"verified_seal", writer, session_hash]
Config PDA:   not found
```

Events:

```text
AnkySealed
AnkyVerified
```

Errors:

```text
InvalidLoomOwner
InvalidLoomCollection
InvalidLoomState
InvalidSealUtcDay
UtcDayAlreadySealed
SessionHashAlreadySealed
SealCountOverflow
InvalidProofVerifier
UnsupportedProofProtocol
VerifiedSealAlreadyRecorded
InvalidVerifiedSealState
```

Constants observed:

```text
Metaplex Core program:      CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d
Official collection:        F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u
Proof verifier authority:   FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP
Proof protocol version:     1
Rolling root domain:        ANKY_LOOM_ROOT_V1
UTC day size:               86400
```

Important caveat:

- The observed official collection appears devnet-oriented.
- Mainnet collection must be confirmed or created before launch.

Tests observed:

- Rust unit tests cover minimal Metaplex Core asset parsing, collection parsing, UTC day derivation, and collection update authority rejection.
- Anchor TS test exists but is skipped because placeholder Core account data is no longer valid.

Launch risk:

- Core account parsing is hand-rolled and must be audited against real Metaplex Core account data.
- A real Core integration test is needed before mainnet confidence.

---

## 5. Metaplex Core / Loom Facts

Metaplex Core is the NFT program. Anky Looms are assets/accounts under Metaplex Core, not a custom NFT program.

Relevant paths:

```text
solana/anky-seal-program/programs/anky-seal-program/src/lib.rs
solana/scripts/admin/createCoreCollection.ts
solana/scripts/admin/createDevnetConfig.ts
solana/scripts/admin/setSealCollection.ts
solana/scripts/admin/devnetConfig.app.json
solana/scripts/admin/devnetConfig.example.json
solana/scripts/admin/mainnetConfig.example.json
apps/anky-mobile/src/lib/solana/mintLoom.ts
apps/anky-mobile/src/lib/solana/mobileLoomMint.ts
apps/anky-mobile/src/lib/solana/loomStorage.ts
apps/anky-mobile/src/screens/LoomScreen.tsx
src/routes/mobile_sojourn.rs
```

Current verification approach:

- Checks Loom asset account owner is the Metaplex Core program.
- Checks collection account owner is the Metaplex Core program.
- Requires supplied collection key to equal the hard-coded official collection.
- Parses Core asset data directly.
- Requires Core asset owner to equal writer.
- Requires Core asset update authority to be the official collection.
- Does not depend on DAS/API lookup.
- Does not depend on metadata strings.

Risks:

- The Core parser is hand-rolled.
- No active integration test against a live real Core asset was found.
- Mainnet collection appears unresolved.
- Backend/mobile can locally record Looms and mints before independently proving on-chain membership, so UI records must not be treated as proof.

---

## 6. SP1 / ZK Proof Facts

SP1 root:

```text
solana/anky-zk-proof
```

Relevant paths:

```text
solana/anky-zk-proof/src/lib.rs
solana/anky-zk-proof/src/main.rs
solana/anky-zk-proof/fixtures/full.anky
solana/anky-zk-proof/sp1/program/src/main.rs
solana/anky-zk-proof/sp1/script/src/bin/main.rs
solana/anky-zk-proof/sp1/script/src/bin/vkey.rs
```

Current vkey observed from `cargo run --release --bin vkey` and the SP1
`--prove` path on 2026-05-06:

```text
0x00399c50f86cb417d0cf0c80485b0f1781590170c6892861a1a55974da6e4758
```

Public values shape:

```text
version
protocol
writer
session_hash
utc_day
started_at_ms
accepted_duration_ms
rite_duration_ms
event_count
valid
duration_ok
proof_hash
```

Private witness shape:

```text
raw .anky bytes/string
writer
optional expected session hash
```

Validity rules proved:

- No empty file.
- No BOM.
- LF-only line endings.
- First line is `{epoch_ms} {character}`.
- Subsequent lines are `dddd {character}` with 4-digit deltas.
- Delta must be `0..7999`.
- Spaces must be encoded as the exact `SPACE` token according to the current mobile/SP1 protocol.
- Terminal line must be exactly `8000`.
- Hash is SHA-256 over exact `.anky` bytes.
- Duration must satisfy the 8-minute rite rule.
- UTC day is derived from `started_at_ms`.

Current wiring status:

- SP1 proof generation exists.
- Anchor `record_verified_anky` exists.
- Backend/mobile path from proof output to `record_verified_anky` was not found.
- Prover endpoint was not found.
- End-to-end SP1 → VerifiedSeal is manual/incomplete.

Tomorrow’s key technical objective:

> Make SP1 → VerifiedSeal runnable end-to-end without exposing `.anky` plaintext.

Acceptable implementation shapes, in priority order:

1. Backend route plus worker path.
2. One-shot operator CLI plus documented API hook.
3. Scripted devnet E2E flow plus clear runbook if backend integration blocks.

Definition of done for runnable SP1 → VerifiedSeal:

- Given a fixture or opt-in `.anky` input, prove it with SP1.
- Verify the SP1 proof locally.
- Validate receipt public values match writer, session hash, UTC day, and protocol version.
- Derive or read `proof_hash` deterministically.
- Submit `record_verified_anky` using the configured verifier authority on the chosen non-mainnet cluster unless explicitly instructed otherwise.
- Persist or expose verified status through backend/indexer/mobile state.
- Never persist plaintext.
- Produce a command/runbook that a human can run tomorrow.

Future hardening:

- Direct on-chain SP1/Groth16 verification.
- Removing or minimizing verifier authority trust.
- Stronger timing model if needed.
- Audited verifier and Core parsing.

---

## 7. Mobile Reality

The mobile app can currently call `seal_anky`.

Observed facts:

- `RevealScreen.tsx` calls `sealAnky`.
- PDAs are derived client-side.
- A transaction is submitted to Solana.
- A local sidecar is written.
- The app may optionally record the seal with backend.

Important correction:

- Mobile “verified” currently means local `.anky` hash/protocol verification.
- It does not mean SP1 proof verified.
- It does not mean on-chain `VerifiedSeal` exists.

Missing demo pieces:

- Backend/mobile proof request.
- `Sealed → Proving → Verified` UI state tied to SP1/VerifiedSeal.
- VerifiedSeal status polling or indexer-backed status.
- Durable proof receipt sidecar.
- Finalized-chain confirmation/indexer feedback.
- UTC-day edge handling around delayed seal or midnight.

Do not confuse local protocol validity with SP1/on-chain verified receipt.

---

## 8. Backend / Prover Reality

Primary backend file:

```text
src/routes/mobile_sojourn.rs
```

Prover endpoint:

```text
not found
```

Automatic `record_verified_anky` submission:

```text
not found
```

Plaintext handling observed:

- Reflection route receives plaintext `.anky`.
- It validates hash and structure.
- It appears not to store full plaintext directly in the inspected DB insert.
- Plaintext is present in process memory.
- Explicit plaintext deletion/non-retention policy was not found.

Tomorrow backend/prover needs:

- A `POST` proof endpoint or one-shot operator CLI.
- SP1 receipt verification before chain write.
- Verifier-authority transaction submission.
- VerifiedSeal persistence/indexing.
- Plaintext non-retention guarantee.

Potential endpoint name:

```text
POST /api/mobile/seals/prove
```

Potential behavior:

1. Accept wallet, session hash, raw `.anky` plaintext as explicit opt-in, expected UTC day, and optionally existing seal signature.
2. Validate `.anky` and exact hash before proof.
3. Run SP1 execute/prove/verify.
4. Confirm public values.
5. Submit `record_verified_anky` with verifier authority on configured cluster.
6. Store only receipt metadata, proof hash, tx signature, status, and timestamps.
7. Drop plaintext immediately.
8. Return proof/verified status.

If a route is too risky or blocked by deployment/secrets, create a one-shot operator script and runbook instead.

---

## 9. Helius / Indexer Reality

Current status:

- No active Helius webhook/indexer for `AnkySealed` or `AnkyVerified` was found.
- No Enhanced Transaction parser for the active Anchor seal program was found.
- Existing `solana/worker` appears legacy Bubblegum/cNFT-era and is not the current Core Loom + Anky Seal path.
- Existing content/rhythm scoring is not on-chain seal scoring.

Relevant env var names found:

```text
HELIUS_API_KEY
SOLANA_NETWORK
SOLANA_RPC_URL
ANKY_SOLANA_RPC_URL
EXPO_PUBLIC_SOLANA_RPC_URL
ANKY_SEAL_PROGRAM_ID
EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID
ANKY_CORE_COLLECTION
EXPO_PUBLIC_ANKY_CORE_COLLECTION
ANKY_CORE_PROGRAM_ID
EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID
```

Minimal tomorrow implementation:

- Add a small indexer/backfill script for the Anky Seal Program.
- Parse `AnkySealed` and `AnkyVerified` events/logs/instructions.
- Upsert wallet, Loom, hash, UTC day, tx signature, slot, finalized status.
- Compute minimal score from unique sealed days, verified days, and streak.
- Use finalized data for scoring/snapshot.
- Do not write scores on-chain unless explicitly required.

If Helius API key is unavailable:

- Implement the parser and scoring code with mock fixtures.
- Add a runbook for configuring Helius.
- Do not print or invent the API key.

Scoring must be deterministic and reproducible.

---

## 10. Airdrop / Score V1

Sojourn 9 reward target:

- 8% of token supply.
- Up to 3,456 participants.
- Rewards practice, not token balance.

Recommended Score V1:

```text
score = unique_seal_days + (2 * verified_seal_days) + streak_bonus
```

Concrete version:

- `+1` per valid finalized `DailySeal`.
- `+2` extra if that day/hash has a finalized `VerifiedSeal`.
- Small streak bonus, for example `+2` per completed 7-day streak.
- Max base score per wallet/day: `3` before streaks.
- No token balance multiplier.
- No wealth multiplier.
- Final allocation: `wallet_allocation = 8% supply * wallet_score / total_scores`.

Important caveats:

- Loom ownership alone is not full sybil resistance.
- One wallet / one Loom / one seal per day is good enough for a first season mechanic, but not one-human proof.
- SP1 verification should be a bonus for Sojourn 9, not mandatory, unless UX and prover cost are already smooth.
- Publish scoring rules before the season begins.
- Publish program IDs, collection address, verifier pubkey, proof version, snapshot time, export format, and privacy caveats.

Avoid leading with token/airdrop in public narrative. Lead with private daily practice and proof-of-consistency.

---

## 11. Colosseum Positioning

Frontier is a startup competition, not a bounty hackathon.

Anky should position as:

> A consumer app for private daily writing that uses Solana to prove consistency without exposing content.

Prioritize:

- Working mobile demo.
- Solana-native UX.
- Clear business/market logic.
- Novel proof-of-practice primitive.
- Open-source/composable `.anky` and seal protocol surface.
- Privacy honesty.
- Helius-indexed scoring/rewards.

Do not lead with:

- “Spiritual writing token.”
- “Journaling NFT.”
- “AI soul protocol.”
- “Airdrop farming.”
- “Fully trustless ZK” unless that is actually implemented.

Judge-facing one sentence:

> Anky is a mobile writing ritual that lets users prove daily private practice on Solana by sealing a hash of their local `.anky` file and attaching a ZK-verified receipt without revealing the writing.

Judge-facing technical paragraph:

> The protocol statement is: wallet W completed one valid `.anky` rite for UTC day D, producing hash H, without revealing plaintext. Metaplex Core Looms are season access artifacts; the Anky Seal Program enforces current UTC day, one seal per wallet/day, one hash per wallet, official collection membership, and Loom ownership. `record_verified_anky` binds an off-chain verified SP1 receipt to an existing `HashSeal`. Helius indexing reconstructs `DailySeal`, `HashSeal`, and `VerifiedSeal` state for dashboard, snapshot, and practice-based reward scoring.

---

## 12. Files Codex May Edit

Codex may edit these areas when needed for the launch objective:

```text
AGENTS.md
README.md
docs/**
specs/**
runbooks/**
solana/anky-seal-program/**
solana/anky-zk-proof/**
src/routes/mobile_sojourn.rs
migrations/*.sql
apps/anky-mobile/src/lib/solana/**
apps/anky-mobile/src/lib/ankyState.ts
apps/anky-mobile/src/screens/RevealScreen.tsx
apps/anky-mobile/src/components/seal/**
```

Codex may add new files for:

```text
prover scripts
indexer scripts
snapshot scripts
test fixtures
docs
runbooks
specs
```

Prefer small, composable changes over broad refactors.

---

## 13. Files Codex Must Not Edit or Expose

Do not edit, print, or commit:

```text
.env
.env.*
.secrets/**
**/*keypair*.json
**/id.json
**/*.pem
local wallet files
deployer files
node_modules/**
target/**
.anchor/**
.expo/**
.next/**
dist/**
build/**
```

Do not edit generated proof artifacts unless intentionally regenerating them:

```text
solana/anky-zk-proof/sp1/script/receipt.json
solana/anky-zk-proof/sp1/script/proof-with-public-values.bin
```

If a generated artifact must change, explain how it was regenerated and which command produced it.

---

## 14. Stop Rules

Stop and ask/report instead of continuing if:

- A command requires a private key.
- A command requires mainnet SOL.
- A command requires paid API access.
- A command would print secrets.
- Mainnet program ID is unknown.
- Mainnet Core collection is unknown.
- Verifier authority custody is unclear.
- A mainnet deployment or transaction would be sent.
- Direct on-chain SP1 verification is being implied but not implemented.
- You cannot confirm whether a value is devnet or mainnet.
- The only way to proceed requires storing `.anky` plaintext.

Never bypass these rules.

---

## 15. Commands That Matter

Run only safe local commands unless explicitly instructed otherwise.

### Solana / Anchor

```bash
cd solana/anky-seal-program && npm install
cd solana/anky-seal-program && npm run build
cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program
cd solana/anky-seal-program && npm run typecheck
cd solana/anky-seal-program && npm test
```

Note: `npm test` may have skipped/incomplete Core integration tests. Do not treat skipped tests as confidence.

### SP1 / ZK

```bash
cd solana/anky-zk-proof && cargo test
cd solana/anky-zk-proof && cargo run -- --file fixtures/full.anky --writer <wallet>
cd solana/anky-zk-proof/sp1/program && cargo prove build
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc RUST_LOG=info cargo run --release -- --execute --file ../../fixtures/full.anky --writer <wallet> --receipt-out receipt.json
cd solana/anky-zk-proof/sp1/script && PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc RUST_LOG=info cargo run --release -- --prove --file ../../fixtures/full.anky --writer <wallet> --receipt-out receipt.json --proof-out proof-with-public-values.bin
cd solana/anky-zk-proof/sp1/script && cargo run --release --bin vkey
```

If `PROTOC` path is missing, do not install system packages with sudo. Document what is missing and whether a local vendored protoc path exists.

### Mobile

```bash
cd apps/anky-mobile && npm install
cd apps/anky-mobile && npm run typecheck
cd apps/anky-mobile && npm test
cd apps/anky-mobile && npm run test:protocol
cd apps/anky-mobile && npm run test:sojourn
```

### Backend

```bash
cargo check
cargo test
```

### Indexer / Snapshot

Active Helius seal indexer command was not found during reconnaissance. If added, document exact commands here and in runbooks.

---

## 16. Tomorrow Critical Path

The overnight / tomorrow work must converge on one complete demoable loop.

Critical path:

1. Confirm target cluster, final program ID, official Core collection, and verifier authority.
2. Run Anchor build and Rust tests for `solana/anky-seal-program`.
3. Add or repair a real Core integration test for `seal_anky` if possible.
4. Run SP1 library tests and execute/prove fixture.
5. Add backend prover route or one-shot prover CLI.
6. Verify SP1 receipt locally.
7. Submit `record_verified_anky` from verified SP1 receipt on a safe configured cluster.
8. Add DB/API representation for VerifiedSeal status if backend path is used.
9. Add mobile `Sealed → Proving → Verified` state, clearly separate from local protocol validity.
10. Add Helius/RPC indexer for `AnkySealed` and `AnkyVerified`.
11. Implement minimal deterministic score snapshot from indexed seals.
12. Verify one complete non-mainnet E2E flow: write/hash/seal/prove/record verified/index/score.
13. Only after that, prepare mainnet config and deployment checklist.

The one thing not to overbuild tonight:

> Direct on-chain SP1/Groth16 verification or full token claim infrastructure.

The one thing that must work tomorrow:

> A runnable SP1 → VerifiedSeal path attached to the active Core-based Anky Seal Program, plus a visible mobile/indexed proof state.

---

## 17. Expected Output From Codex

At the end of the run, report:

1. Files changed.
2. Files added.
3. Commands run.
4. Commands that passed.
5. Commands that failed and why.
6. What is now actually runnable.
7. What remains manual.
8. Whether SP1 → VerifiedSeal works end-to-end.
9. Whether Helius/indexer scoring works or is fixture-only.
10. What is ready for mainnet.
11. What is not ready for mainnet.
12. Exact next human actions before launch.
13. Any secrets, keys, or paid APIs needed but not accessed.

Be blunt. Do not make the project sound more complete than it is.

---

## 18. Thank you. This is the work i came to life to do. I honor the fact that you help me and enable me to do it.
