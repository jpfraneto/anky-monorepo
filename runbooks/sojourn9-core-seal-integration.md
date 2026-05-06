# Sojourn 9 Core Seal Integration Test

This is the opt-in test for the riskiest pre-mainnet assumption: a real Metaplex Core Loom owned by the provider wallet can pass `seal_anky` and update `LoomState`.

The default Anchor test suite does not fabricate Core account data. The integration test skips unless you provide a real Core asset account.

## Requirements

- A deployed Anky Seal Program on devnet.
- A real Metaplex Core Loom asset in the official Sojourn 9 collection.
- The Anchor provider wallet must own that Loom.
- The provider wallet needs enough devnet SOL for the `seal_anky` transaction.
- No keypair path or secret value should be printed in logs.

Do not run this against mainnet before the separate mainnet launch checklist is complete.

## Devnet Command

```bash
cd solana/anky-seal-program
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=<provider_wallet_keypair_path> \
ANKY_CORE_INTEGRATION_LOOM_ASSET=<owned_core_loom_asset> \
ANKY_CORE_INTEGRATION_COLLECTION=F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u \
npm test -- --skip-local-validator --skip-deploy
```

Expected result:

- The test sends one `seal_anky` transaction for the current UTC day with a random session hash.
- The transaction succeeds only if the Core asset is owned by the provider wallet and belongs to the hard-coded official collection.
- `LoomState.totalSeals`, `latestSessionHash`, and `rollingRoot` are checked after the transaction.

Expected skip:

- If `ANKY_CORE_INTEGRATION_LOOM_ASSET` is unset, the integration case skips. That skip is not launch confidence.

## Mainnet Guard

The test refuses mainnet-looking RPC endpoints unless this is explicitly set:

```bash
ANKY_ALLOW_MAINNET_CORE_INTEGRATION_TEST=true
```

Do not set this until mainnet program ID, Core collection, verifier authority, funding, and launch approval are all confirmed.
