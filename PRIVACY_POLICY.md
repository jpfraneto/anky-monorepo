# Anky — Privacy Policy

_Last updated: 2026-04-15_

Anky is built around the premise that what you write in private should stay private. This policy describes what we collect, what we don't, and why.

## 1. What we collect

**Anonymous web users:** nothing is persisted. Your writing is ephemeral and disappears when you close the tab.

**Authenticated users (iOS / Farcaster miniapp):**
- Your Solana wallet address (used as your identity).
- For Farcaster users: your fid, username, and public profile info via Neynar.
- Session metadata: duration, word count, timestamp, and a SHA-256 hash of your writing.
- The sealed (encrypted) envelope of your writing.
- Derived outputs: reflection text, image prompt, generated image.

## 2. What we do NOT see

For authenticated users, your writing is encrypted on-device using the enclave's X25519 public key before it leaves your device. The backend stores the sealed envelope blind — we cannot decrypt it.

Plaintext is decrypted only inside the AWS Nitro Enclave on EC2, where it is processed by an LLM and discarded. The plaintext never touches our application server.

## 3. On-chain data

When you complete a writing session, the SHA-256 hash of your writing (not the writing itself) may be logged on Solana via spl-memo. This proves the writing existed without revealing its content.

cNFTs you mint (Mirror, Anky) are public and permanent on Solana devnet.

## 4. Third parties

We send data to:
- **OpenRouter / Anthropic** — for LLM reflections (called from inside the enclave on plaintext, or from the backend on derived prompts only).
- **Neynar** — to fetch your public Farcaster profile.
- **Helius** — Solana RPC.
- **Cloudflare R2** — image and asset storage.
- **Stripe** — if you make a payment on the altar.

## 5. Your rights

You can request deletion of your account and associated derived data by emailing jpfraneto@gmail.com. On-chain data (cNFTs, spl-memo logs) cannot be deleted.

## 6. Security

We use industry-standard encryption (X25519 + AES-256-GCM) for sealed writes. No system is perfectly secure; use Anky at your own risk.

## 7. Children

Anky is not intended for users under 13.

## 8. Changes

We may update this policy. Material changes will be announced in-app or on anky.app.

## 9. Contact

jpfraneto@gmail.com
