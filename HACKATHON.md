# Anky — Colosseum Hackathon Framework

*Last updated: 2026-05-06*

---

## The One-Liner

Anky is a private daily writing ritual with a Solana witness. Write for 8 minutes without stopping; your phone keeps the `.anky` file local, hashes the exact UTF-8 bytes, seals only that hash through an Anky Loom, and can attach an SP1-verified receipt without revealing the writing.

---

## The Pitch (30 seconds)

Every app on your phone is designed to make you forget you're alive. Anky is the one that asks you to remember. Eight minutes of uninterrupted stream-of-consciousness writing: no backspace, no editing, no performance. What comes out stays local by default. Solana sees the hash, the Loom, the wallet, and the UTC day; SP1 proves the private `.anky` file was structurally valid off-chain; the current on-chain verified badge is verifier-authority-attested after that SP1 verification.

---

## Why Crypto (for judges who ask)

Three things that cannot exist without the chain:

1. **Proof-of-practice without plaintext storage.** The session hash on Solana commits to the exact `.anky` bytes without revealing the writing. The SP1 path proves private `.anky` validity off-chain, and the Anky Seal Program records the public verified receipt after verifier-authority attestation. Future hardening is direct on-chain SP1/Groth16 verification.

2. **Identity without platform lock-in.** Your Solana wallet is the public identity for the season. Your Metaplex Core Loom is the access artifact. Your public seals and verified receipts follow your pubkey across any app that indexes the Anky Seal Program.

3. **Practice-based scoring.** Helius-backed indexing reconstructs finalized `DailySeal`, `HashSeal`, and `VerifiedSeal` state so rewards can be based on unique days, verified days, and streaks instead of token balance.

---

## Why Now (for judges who ask)

The $1.5T wellness market is full of people doing breathwork, meditation retreats, plant medicine, ice baths — all modern versions of ancient gateway experiences. They're seeking inner work. What they don't have is a *daily* practice that's accessible, private, and produces a permanent record. No ceremony, no retreat, no guru. Just a blank page, 8 minutes, and a timer. The cultural wave is already here (Danny Jones gets millions of views on consciousness podcasts). Anky is the infrastructure for the practice these people already want.

---

## Architecture (for technical judges)

```text
iOS Device                    Solana                         Backend / Indexer
──────────                    ──────                         ─────────────────
Write 8 min
Save exact .anky bytes locally
SHA-256(.anky bytes)

Own/mint Core Loom ─────────→ Metaplex Core Loom asset

seal_anky(hash, utc_day) ───→ Anky Seal Program
                              DailySeal + HashSeal

Opt-in proof file path ─────→ SP1 prover process
                              local SP1 proof verification

record_verified_anky ───────→ VerifiedSeal PDA
                              verifier-authority-attested

                              Helius finalized backfill ───→ score snapshot
Mobile polls backend ──────────────────────────────────────→ Sealed / Proving / Verified
```

The canonical proof/scoring path must not persist `.anky` plaintext. Plaintext may enter a prover or reflection path only as explicit opt-in transient process memory; it must not be logged, stored, or sent to chain.

---

## Competitive Landscape (from Colosseum Copilot research)

The wellness/journaling cluster (v1-c17) has the **lowest crowdedness score in the entire corpus: 107** (vs 323 for DEX infra, 325 for AI agents). The closest competitors:

- **Mind Diary** (Cypherpunk) — "AI journal with rewards." No prize, no accelerator, no encryption.
- **Wellnotes** (Radar) — "Gamified journaling with tokens." No prize, no accelerator, plaintext storage.
- **MoodChain** (Breakout) — "Mood diary with NFT achievements." No prize, no accelerator.

**Not a single wellness/journaling project has won a prize or entered the accelerator across 5 hackathons.**

Consumer apps that DID win: Banger (tweet marketplace, 1st Consumer Renaissance), TypeX (keyboard wallet, 3rd Consumer Breakout), Superfan (music labels, 2nd Consumer Cypherpunk). Pattern: clear crypto-native primitive that couldn't exist without the chain. Anky has this — the sealed write + on-chain proof is genuinely novel.

---

## What's Built and Deployed (not a demo)

- **React Native mobile app** with local `.anky` writing, hashing, Loom, seal, proof-state, and backend score surfaces
- **Metaplex Core Loom path** for Sojourn 9 access artifacts
- **Anky Seal Program** with `DailySeal`, `HashSeal`, `LoomState`, and verifier-attested `VerifiedSeal`
- **SP1 proof path** that proves private `.anky` validity off-chain and verifies proofs locally
- **VerifiedSeal operator/backend metadata path** for public receipt persistence after off-chain SP1 verification
- **Helius/RPC indexer** for finalized seal/verified event reconstruction and deterministic Score V1 snapshots
- **Farcaster miniapp** live (chat interface, mirror generation, writing overlay)
- **Image generation pipeline** (Gemini → Flux/ComfyUI on local 4090s)
- **Text inference chain** (local Qwen 27B → Claude → OpenRouter fallback)
- **Sojourn 9 structure**: 96 days as 12 regions of 8 days, with 8 kingdoms/chakras/colors as the inner symbolic cycle

Not claimed yet: mainnet deployment, direct on-chain SP1 verification, production Helius webhook creation, or a completed live devnet `seal -> prove -> record verified -> index -> score` run.

---

## Investor Theses That Align (from archive research)

- **Paradigm** (Aug 2024): "I'm equally interested in non-financial use cases because they create a positive feedback loop"
- **a16z** (Oct 2024): "As transaction costs come down, many other potential crypto consumer apps become possible"
- **Galaxy Research** (Feb 2025): SBTs and non-transferable tokens for identity + reputation — close to the role Sojourn 9 Looms play as season access artifacts
- **Cypherpunk's Manifesto** (Nakamoto Institute): "To encrypt is to indicate the desire for privacy" — Anky's local-first hash-seal architecture applies that privacy instinct to daily practice

---

## The Framing That Works

**Don't explain the technology. Explain the experience.**

"What happens when you sit still and listen to yourself for 8 minutes without stopping?" Most people have literally never done this. The technology exists to protect the sacredness of the answer — but the pitch is the question, not the architecture.

**The audience is the consciousness crowd, not the crypto crowd.**

Danny Jones viewers. Breathwork practitioners. Retreat-goers. People who are already spending money on inner work. They're not on Crypto Twitter. They're on YouTube watching 3-hour podcasts about meaning. Anky should show up where that conversation is happening.

**The 3,456 seat limit is a mystery school, not a waitlist.**

Don't apologize for scarcity. Lean into it. This isn't a product launch — it's an initiation into a practice. The Loom gives the season a public access artifact. The sojourn gives it rhythm and ceremony. "3,456 seats. 96 days. One rule: don't stop writing for 8 minutes."

**Solana isn't the reward. Solana is the witness.**

The chain exists to prove, not to pay. You don't get an NFT for journaling. You get immutable proof that you went through the portal. The distinction is everything — it's the difference between a sticker chart and a sacred record.

**The difficulty is the feature.**

Every other app is designed to minimize friction. Anky maximizes it — deliberately. No backspace. No editing. 8 full minutes. The difficulty is what makes it real. You can't fake 480 seconds of continuous keystroke data. The protocol makes authenticity the path of least resistance.

---

## One-Slide Pitch

> **Anky: Proof of Inner Work on Solana**
>
> Write for 8 minutes without stopping. Your phone keeps the `.anky` file local, hashes the exact bytes, and seals only that hash on Solana through a Metaplex Core Loom. SP1 proves private `.anky` validity off-chain, and the current on-chain verified receipt is verifier-authority-attested after that proof is checked. Helius indexing turns finalized seals and verified receipts into a deterministic practice score for up to 3,456 Sojourn 9 participants.

---

## Weaknesses (be honest about these)

1. **The value proposition requires experience.** You can't explain what 8 minutes of no-backspace writing feels like. You have to do it. The first 30 seconds of acquisition is the hardest problem.

2. **Solo builder.** One person building enclave, iOS, backend, Solana, miniapp, content, docs. Not sustainable long-term. The hackathon is a funding mechanism to get to the point where this changes.

3. **The mystical framing alienates some audiences.** Sojourns, kingdoms, chakras — resonates deeply with some, repels others. The protocol is pure engineering. The culture is deliberately spiritual. Feature for community, risk for pitch.

4. **Adoption requires behavior change.** Nobody wakes up wanting to write for 8 minutes with no backspace. The sojourn structure, the Loom, the evolving profile — these create gravity. But gravity takes time.

---

## What LaCroix Taught Us (Danny Jones podcast analysis, 2026-04-10)

Matt LaCroix describes ancient "realm gateways" — temples where consciousness travels. Anky's 8-minute writing session is architecturally the same thing: a portal built from attention, not stone. The ancients chose the hardest stone (basalt, Mohs 7-9) because it would survive. We chose Solana — an immutable ledger. Same pattern: build the hardest possible container for the most fragile possible content.

The podcast proves the audience exists. Millions of people watching 3-hour conversations about consciousness, lost knowledge, inner work. They don't need to be convinced that inner work matters. That war is already won. What they need is the daily practice layer — no ceremony, no retreat, no guru. Just a blank page and 8 minutes.

"Write for 8 minutes without stopping. See what comes out. That's the practice. Everything else is just protecting what you found."
