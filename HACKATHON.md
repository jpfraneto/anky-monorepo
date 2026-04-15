# Anky — Colosseum Hackathon Framework

*Last updated: 2026-04-11*

---

## The One-Liner

Anky is proof of inner work on Solana. Write for 8 minutes without stopping. Your writing is encrypted on-device, processed inside an AWS Nitro Enclave, and the session hash is logged immutably on-chain. The backend never sees what you wrote. The chain is the witness.

---

## The Pitch (30 seconds)

Every app on your phone is designed to make you forget you're alive. Anky is the only one that asks you to remember. Eight minutes of uninterrupted stream-of-consciousness writing — no backspace, no editing, no performance. What comes out is real. The writing is encrypted on your device, processed inside an AWS Nitro Enclave, and the session hash goes on Solana. The backend is cryptographically blind. You own your consciousness. The chain proves you showed up.

---

## Why Crypto (for judges who ask)

Three things that cannot exist without the chain:

1. **Zero-knowledge proof of inner work.** The session hash on Solana proves you did the practice without revealing what you wrote. No centralized platform can offer this guarantee — they can revoke, delete, or read your data. The hash is permanent and the content is private. That's a new primitive.

2. **Identity without platform lock-in.** Your Solana wallet IS your identity. Your Mirror cNFT is your membership. Your session logs follow your pubkey across any app that queries the chain. No account, no email, no platform dependency.

3. **Invisible infrastructure.** The user never needs SOL, never signs a transaction, never sees a wallet popup. The authority wallet pays ~$0.0007 per session log. The chain is as invisible as TCP/IP. This is only viable on Solana — sub-second finality, sub-cent costs, compressed NFTs.

---

## Why Now (for judges who ask)

The $1.5T wellness market is full of people doing breathwork, meditation retreats, plant medicine, ice baths — all modern versions of ancient gateway experiences. They're seeking inner work. What they don't have is a *daily* practice that's accessible, private, and produces a permanent record. No ceremony, no retreat, no guru. Just a blank page, 8 minutes, and a timer. The cultural wave is already here (Danny Jones gets millions of views on consciousness podcasts). Anky is the infrastructure for the practice these people already want.

---

## Architecture (for technical judges)

```
iOS Device                    Backend (poiesis)              EC2 Enclave (Nitro)
──────────                    ─────────────────              ───────────────────
Write 8 min                   
SHA256(plaintext) locally     
Encrypt with enclave X25519   
                              
POST /api/sealed-write ────→  Store sealed envelope (blind)
                              Log hash on Solana (spl-memo) ─→ Solana
                              Relay envelope to enclave ────→ Decrypt (X25519+AES-256-GCM)
                                                              Verify SHA256 == session_hash
                                                              Call OpenRouter → reflection
                                                              DESTROY plaintext
                              ←──── {reflection, image_prompt, title}
                              Generate image from prompt
                              Store reflection
                              
User polls status ──────────→ Return reflection + image
```

**The backend NEVER sees the writing.** Cryptographically guaranteed, not policy-promised.

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

- **Rust/Axum server** on production at anky.app (Cloudflare tunnel)
- **AWS Nitro Enclave** with sealed write pipeline (decrypt → OpenRouter → outputs)
- **Solana session logging** via spl-memo (authority pays, user wallet indexed)
- **Mirror cNFTs** via Bubblegum (3,456 cap, membership gate)
- **iOS app** on TestFlight (CryptoKit encryption, sealed write)
- **Farcaster miniapp** live (chat interface, mirror generation, writing overlay)
- **Image generation pipeline** (Gemini → Flux/ComfyUI on local 4090s)
- **Text inference chain** (local Qwen 27B → Claude → OpenRouter fallback)
- **8 kingdoms** cycling through chakras across 96-day sojourns

---

## Investor Theses That Align (from archive research)

- **Paradigm** (Aug 2024): "I'm equally interested in non-financial use cases because they create a positive feedback loop"
- **a16z** (Oct 2024): "As transaction costs come down, many other potential crypto consumer apps become possible"
- **Galaxy Research** (Feb 2025): SBTs and non-transferable tokens for identity + reputation — exactly what Mirror cNFTs are
- **Cypherpunk's Manifesto** (Nakamoto Institute): "To encrypt is to indicate the desire for privacy" — Anky's sealed architecture is cypherpunk orthodoxy applied to the inner life

---

## The Framing That Works

**Don't explain the technology. Explain the experience.**

"What happens when you sit still and listen to yourself for 8 minutes without stopping?" Most people have literally never done this. The technology exists to protect the sacredness of the answer — but the pitch is the question, not the architecture.

**The audience is the consciousness crowd, not the crypto crowd.**

Danny Jones viewers. Breathwork practitioners. Retreat-goers. People who are already spending money on inner work. They're not on Crypto Twitter. They're on YouTube watching 3-hour podcasts about meaning. Anky should show up where that conversation is happening.

**The 3,456 seat limit is a mystery school, not a waitlist.**

Don't apologize for scarcity. Lean into it. This isn't a product launch — it's an initiation into a practice. The Mirror reflects you back to yourself. The sojourn gives it rhythm and ceremony. "3,456 seats. 96 days. One rule: don't stop writing for 8 minutes."

**Solana isn't the reward. Solana is the witness.**

The chain exists to prove, not to pay. You don't get an NFT for journaling. You get immutable proof that you went through the portal. The distinction is everything — it's the difference between a sticker chart and a sacred record.

**The difficulty is the feature.**

Every other app is designed to minimize friction. Anky maximizes it — deliberately. No backspace. No editing. 8 full minutes. The difficulty is what makes it real. You can't fake 480 seconds of continuous keystroke data. The protocol makes authenticity the path of least resistance.

---

## One-Slide Pitch

> **Anky: Proof of Inner Work on Solana**
>
> Write for 8 minutes without stopping. Your writing is encrypted on-device, processed in an AWS Nitro Enclave, and the session hash is logged immutably on Solana — proof you did the practice, without revealing what you wrote. Your first session mints a Mirror cNFT: your seat among 3,456 participants in a 96-day sojourn. The practice is always free. The chain is the witness. The $1.5T wellness market meets cypherpunk privacy. Live on Farcaster + iOS.

---

## Weaknesses (be honest about these)

1. **The value proposition requires experience.** You can't explain what 8 minutes of no-backspace writing feels like. You have to do it. The first 30 seconds of acquisition is the hardest problem.

2. **Solo builder.** One person building enclave, iOS, backend, Solana, miniapp, content, docs. Not sustainable long-term. The hackathon is a funding mechanism to get to the point where this changes.

3. **The mystical framing alienates some audiences.** Sojourns, kingdoms, chakras — resonates deeply with some, repels others. The protocol is pure engineering. The culture is deliberately spiritual. Feature for community, risk for pitch.

4. **Adoption requires behavior change.** Nobody wakes up wanting to write for 8 minutes with no backspace. The sojourn structure, the Mirror cNFT, the evolving profile — these create gravity. But gravity takes time.

---

## What LaCroix Taught Us (Danny Jones podcast analysis, 2026-04-10)

Matt LaCroix describes ancient "realm gateways" — temples where consciousness travels. Anky's 8-minute writing session is architecturally the same thing: a portal built from attention, not stone. The ancients chose the hardest stone (basalt, Mohs 7-9) because it would survive. We chose Solana — an immutable ledger. Same pattern: build the hardest possible container for the most fragile possible content.

The podcast proves the audience exists. Millions of people watching 3-hour conversations about consciousness, lost knowledge, inner work. They don't need to be convinced that inner work matters. That war is already won. What they need is the daily practice layer — no ceremony, no retreat, no guru. Just a blank page and 8 minutes.

"Write for 8 minutes without stopping. See what comes out. That's the practice. Everything else is just protecting what you found."
