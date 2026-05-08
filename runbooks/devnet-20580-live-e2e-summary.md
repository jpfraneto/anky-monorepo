# Devnet 20580 Live E2E Summary

Date: 2026-05-07 UTC day 20580

Status for sponsored-payer launch evidence: STALE.

This evidence predates the sponsored-payer Anchor account model where `seal_anky`
has separate `writer` and `payer` signer accounts. It remains useful historical
evidence for the old HashSeal -> SP1 -> VerifiedSeal -> index loop, but it must
not be used as validation of the new sponsored-payer program model. Use
`runbooks/devnet-20581-sponsored-payer-validation-evidence.json` for the
devnet sponsored-payer seal/index validation.

This run proves the current worktree can execute:

demo witness -> SP1 proof -> HashSeal -> VerifiedSeal -> known-signature Helius indexing -> score snapshot -> evidence audit.

## Public values

Writer: 5xf7VcURsgiy3SvkBUirAYSPu3SYhto9qX6AFrLTvN1Q
Loom asset: 6oEyFPQPksvKyCtdjsSEzL6JMxAPPwBPkMBBAMvUnNLJ
Core collection: F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u
Program ID: 4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX
Verifier: FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP

Session hash: d301bf83d402d197042c9418028579ef0d1e2f2eb9d09cb1d1b3ee5ec288d1e5
Proof hash: cd61237a0c7e00f20e47deb940613fff90e3923c1245f582c478e3e0604bcf90
SP1 vkey: 0x00418281239458876cbe43d5431998057856f03e0b47d85695fce2a45b200da4

HashSeal signature: 4r1LuKhGHpDiYKjF6HQ9HvvzThxNUHzgknywciRVepXvDG143ehsZvvWuC1yo8hCC9ZnF3tHSJAhQUd2VGSqcR4e
VerifiedSeal signature: 56anm3Va83spZ33ffFRtvMjuuTDZT6XRwkcRLUxehsTHfWYSQFXTV4LzLbGXVRxB4pVfnbdmmZEq3qEGcnabznZE

Score snapshot: runbooks/devnet-20580-score-snapshot.json
Evidence artifact: runbooks/devnet-20580-live-e2e-evidence.json

## Result

Score snapshot:
- indexed events: 2
- sealed events: 1
- verified events: 1
- score rows: 1
- total score: 3

Evidence audit:
- ok: true
- issues: []

## Caveat

This run used known-signature Helius getTransaction indexing, not a confirmed live webhook delivery.

This run does not prove mainnet readiness.
This run does not prove $ANKY distribution readiness.
