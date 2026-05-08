# Anky Sponsored Transactions Devnet Validation

Date: 2026-05-08
Cluster: devnet
UTC day: 20581

This document records the controlled devnet validation pass for the sponsored-payer program model. It validates the `seal_anky` payer split and local/mobile/backend guard coverage. It does not claim mainnet readiness.

## Public Inputs

- Program ID: `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`
- Core collection: `F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u`
- Metaplex Core program: `CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d`
- Sponsor/devnet payer: `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`
- Evidence bundle: `runbooks/devnet-20581-sponsored-payer-validation-evidence.json`
- Index snapshot: `runbooks/devnet-20581-sponsored-payer-score-snapshot.json`

No keypair JSON, `.env` values, private keys, Helius API keys, or paid service tokens were printed or copied into this document.

## Deployment

Devnet deploy was run after explicit human approval.

- Result: pass
- Program ID: `4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX`
- Signature: `2UEhpCCu2tGAxzY2c2gZEXkDShsVPkwTAujLSrSPpWaeygeacjLCPjDVdFWVXVUenfbZq7Rt8DSHQejM1BZfBFWA`

The previous accidental deployment signature `4EvACDAMXe83heHsW28V8A7T2BAtN2zoZJh8Lcn6aZZjvngzGKLNwvLLoMcxE5uZHnrsrU93v7ULjRmDn56rHm8G` remains historical only and is not used as launch evidence.

## Fresh Devnet Looms

Two fresh Metaplex Core Looms were minted for isolated same-day seal validation:

| Case | Loom asset | Owner | Mint payer | Mint signature |
| --- | --- | --- | --- | --- |
| Writer-pays seal | `DHVdX41WRKmHFW2q8MUoJDRYPCmowpC5VJEvdjyviU1g` | `6yS2xjgYeBn6HSeMm5zwyYCWQhwFGfw6Sf9fvb8f1NX` | `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP` | `5w31EFoRHqqVT91RfFLZP2VJ3ykhABwGQMpchVuEKuTbqwbNWpcWMVXJamdepnHqgLXUvN4C6w57pE9W2Nb8qBau` |
| Sponsor-pays seal | `L5woSyRnN2P1G4v13BH95AVAVZftWN513axR59d9VGy` | `HK52v7KLU7TxPM3RnTYHjEmeg5kgERk1bpzHUmmuBkmR` | `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP` | `4L86mkMw7Nbs1eQp7hqZaesJ2ym6awNoEHeLyNVY4wMJfHVrB8KkXJZ5dAq3PJrzkbeLMuEyJJhctduLfV9kUSjP` |

Both Looms passed read-only Core account checks: Core-owned account, AssetV1 discriminator, official collection, collection update authority, and expected owner.

## Seal Matrix

| Requirement | Result | Evidence |
| --- | --- | --- |
| Writer-pays seal still works | pass | Writer and payer both `6yS2xjgYeBn6HSeMm5zwyYCWQhwFGfw6Sf9fvb8f1NX`; signature `5XTh9SmXvkNcFD1ZXyDsLf6aFjqL9hCubWRKp3kBw2osyijg9U718ezutsCbY9EGdbbLw8L1kH4H1FXjXkEukRNE` |
| Sponsor-pays seal works | pass | Writer `HK52v7KLU7TxPM3RnTYHjEmeg5kgERk1bpzHUmmuBkmR`; payer `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`; signature `4jPvBKCt81SgQxA2vPBkgeVPCU4xsAkg1RN17W1WaHrYzgnDCDjWWC1aokai875hcCBqdqZVEWPNaCAzUUtRtSJJ` |
| Sponsor cannot seal without writer authority | pass | A sponsor-only transaction failed strict signature verification with missing writer signature. A bypass-preflight raw signature `QZTkMuDNzdjXCPiEzWntJxmVgdNvbbwmqiGRYfpndkTVLWaiULHKzFwX1x6oWGZm2ZQN1b76i3vG2dQBeKbLxfH` remained not found on devnet. |
| Wrong Loom owner fails | pass | Writer `948dM9J4LrJshvRbNHMaDZjqiTHGkg5ez5jy8Ko84wn1` attempted to use Loom `L5woSyRnN2P1G4v13BH95AVAVZftWN513axR59d9VGy`; helper rejected `Core Loom asset owner does not match writer`. |
| Payer may equal writer | pass | Writer-pays seal landed. |
| Payer may differ from writer | pass | Sponsor-pays seal landed. |

## Indexer Result

Known-signature finalized indexing parsed both fresh `AnkySealed` events after the program/IDL payer split.

- Indexed events: `2`
- Sealed events: `2`
- Verified events: `0`
- Score rows: `2`
- Total score: `2`
- Snapshot: `runbooks/devnet-20581-sponsored-payer-score-snapshot.json`

This validates event/index parsing for the sponsored-payer seal model. It does not validate a fresh SP1 -> VerifiedSeal proof receipt.

## Local Gates

Passed:

- `git diff --check`
- `cargo test -q mobile_sponsorship`
- `cargo test -q sponsored_core`
- `cd solana/anky-seal-program && npm test`
- `cd solana/anky-seal-program && npm run build`
- `cd solana/anky-seal-program && cargo test --manifest-path Cargo.toml --package anky_seal_program`
- `cd apps/anky-mobile && npm run typecheck`
- `cd apps/anky-mobile && npm run test:protocol`
- `cd apps/anky-mobile && npm run test:sojourn`
- `cd apps/anky-mobile && npm run test -- src/lib/api/ankyApi.test.ts src/lib/solana/mintLoom.test.ts src/lib/solana/sealAnky.test.ts src/lib/solana/sponsoredSeal.test.ts`
- `node --test solana/scripts/indexer/ankySealIndexer.test.mjs`
- `cd solana/anky-seal-program && npm run sojourn9:readiness`

Readiness result:

- `localReady: true`
- `launchReady: false`

## Stale Evidence

The previous UTC day 20580 evidence is stale for the sponsored-payer program model:

- `runbooks/devnet-20580-live-e2e-summary.md`
- `runbooks/devnet-20580-live-e2e-evidence.json`

That evidence predates the new `seal_anky` account model with separate `writer` and `payer` signers. It remains historical evidence for the old HashSeal -> SP1 -> VerifiedSeal loop only.

## Not Validated In This Pass

- Full SP1 -> VerifiedSeal rerun after the sponsored-payer deploy.
- Backend target database migration application.
- Live backend `/api/mobile/seals/prepare` against a deployed backend.
- Live mobile phone flow.
- Helius webhook delivery. Indexing used known-signature finalized `getTransaction`.
- Mainnet deployment, mainnet config, App Store submission, paid API mutation, or `$ANKY` reward/distribution mechanics.

## Result

Sponsored-payer seal model: pass on devnet for writer-paid and sponsor-paid seals.

Mainnet ready: no.
