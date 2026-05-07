# Anky Local Readiness Fix

Updated: 2026-05-07

This records the surgical repair for the Sojourn 9 local readiness failure. No deployment, signing, keypair read, secret read, mainnet mutation, Helius webhook creation, App Store submission, or `$ANKY` distribution claim was performed.

## Failure Reproduced

The readiness alias lives in `solana/anky-seal-program/package.json`, so the readiness command was run from `solana/anky-seal-program`.

Pre-fix results:

| Command | Result |
|---|---|
| `npm run sojourn9:readiness` | Failed local readiness with `localReady: false` and `launchReady: false`. |
| `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` | Failed 1 of 4 tests. The first test expected `localReady: true`; actual was `false`. |

The only failing readiness check was:

```text
Mobile reveal separates hash seal from SP1 proof state
status: unmatched
missing: verified +2
```

## Root Cause

`apps/anky-mobile/src/screens/RevealScreen.tsx` still had separate hash-seal, proving, syncing, failed, unavailable, and verified proof states. The local readiness failure was caused by a stale/ambiguous success label in the verified UI state:

```text
sealed +1 · proof +2 · 3 pts
```

The readiness gate requires the verified state to include `verified +2` so the app does not blur a hash seal with an SP1/VerifiedSeal receipt. This was not a missing mainnet value, not a dirty fixture mismatch, not a protocol rule change, and not a reason to weaken the mainnet safety gate.

## Files Changed

| File | Change |
|---|---|
| `apps/anky-mobile/src/screens/RevealScreen.tsx` | Changed the verified receipt success label to `sealed +1 · verified +2 · 3 pts`. |
| `docs/anky-system/ANKY_LOCAL_READINESS_FIX.md` | Added this report. |
| `docs/anky-system/ANKY_MAINNET_READINESS_GATE.md` | Appended the local readiness fix update. |

## Why The Fix Is Safe

- The change is display copy only.
- It applies to the `proofState === "verified"` path through the existing `VERIFIED_POINTS_LABEL`.
- It does not change hashing, sealing, SP1 proof generation, proof verification, PDA derivation, scoring math, backend persistence, Helius indexing, or mainnet behavior.
- It does not relax or remove any readiness gate check.
- It makes the UI more explicit that the extra points are tied to verified proof receipt state, not merely a submitted proof request.

## Commands Run

Post-fix results:

| Command | Result |
|---|---|
| `cd solana/anky-seal-program && npm run sojourn9:readiness` | Passed local gate with `localReady: true`; kept `launchReady: false`. |
| `node --test solana/scripts/sojourn9/launchReadinessGate.test.mjs` | Passed 4 tests. |
| `cd solana/anky-seal-program && npm run sojourn9:test` | Passed 161 tests. |
| `cd apps/anky-mobile && npm run typecheck` | Passed. |

## Remaining Manual Gates

Local readiness is restored. Mainnet readiness remains blocked by the same human/operator gates:

- Fresh same-day devnet `HashSeal -> SP1 verify -> VerifiedSeal -> index` evidence.
- Human-controlled verifier authority custody and signing path.
- Target backend migrations and proof-worker environment.
- Production Helius webhook or credentialed finalized backfill.
- Real Core Loom integration against an owned live Loom.
- Final mainnet program ID, Core collection, verifier authority, RPC, funding, snapshot, reward custody, dispute, and claim process.
- App Store crypto/wallet/NFT/reward posture.

## Current Status

`SP1 -> VerifiedSeal` was not run end-to-end in this fix pass because that requires human-owned devnet signer and verifier authority custody. Helius/indexer scoring was covered by local tests only; production Helius remains externally gated.
