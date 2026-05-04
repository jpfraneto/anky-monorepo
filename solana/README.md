# Anky Solana Devnet Skeleton

This folder contains the first devnet Solana integration for Anky:

- `anky-seal-program/`: Anchor program that seals `.anky` session hashes through already-minted Looms.
- `scripts/admin/`: admin scripts to create the official Metaplex Core collection and produce devnet config.
- `../apps/anky-mobile/src/lib/solana/`: React Native scaffolding for mobile Loom minting and `.anky` hash sealing.

The older `solana/setup` and `solana/worker` folders are pre-existing Bubblegum/cNFT infrastructure. This integration is separate and uses Metaplex Core for Loom assets.

## Product Architecture

The mobile app mints Looms. The Anky Seal Program does not mint Looms.

The architecture is intentionally split:

- Metaplex Core handles creating transferable Loom assets in the official Anky Sojourn 9 Looms collection.
- The Anky Seal Program only anchors a 32-byte `session_hash` and updates Loom lineage.
- A `.anky` file stays local as plain text.
- The canonical session hash is `sha256(raw_utf8_bytes_of_the_file)`.
- Solana receives only the 32-byte hash, never the writing text.

Users can write without a wallet. If they want public witness, they need an Anky Sojourn 9 Loom. Once they own a Loom, they can seal unlimited `.anky` hashes through it.

## Devnet Setup

From the monorepo root:

```bash
solana config set --url devnet
solana airdrop 5
```

Create the Core collection:

```bash
cd solana/scripts/admin
npm install
DEVNET_RPC_URL=https://api.devnet.solana.com \
KEYPAIR_PATH=~/.config/solana/id.json \
COLLECTION_URI=https://anky.app/devnet/metadata/sojourn-9-looms.json \
npm run create-core-collection
```

The script writes `solana/scripts/admin/devnetConfig.json` with:

- `coreCollection`
- `collectionName`
- `collectionUri`
- `sealProgramId: "REPLACE_AFTER_ANCHOR_DEPLOY"`
- creation signature and timestamp

Then update the Anchor program:

1. Replace `OFFICIAL_COLLECTION` in `solana/anky-seal-program/programs/anky-seal-program/src/lib.rs` with the created Core collection pubkey.
2. Keep the value in `pubkey!("...")` form with a real base58 Solana public key.
3. Build once to generate a local deploy keypair, sync the program ID, then build and deploy:

```bash
cd solana/anky-seal-program
npm install
anchor build
anchor keys sync
anchor build
anchor deploy --provider.cluster devnet
```

After deploy, update the mobile config:

```bash
cd ../scripts/admin
npm run create-devnet-config -- \
  --seal-program-id <DEPLOYED_PROGRAM_ID> \
  --collection <CORE_COLLECTION_PUBKEY> \
  --rpc-url https://api.devnet.solana.com
```

Copy the resulting values into Expo env vars or `apps/anky-mobile/src/lib/solana/ankySolanaConfig.ts`:

- `EXPO_PUBLIC_SOLANA_RPC_URL`
- `EXPO_PUBLIC_ANKY_CORE_COLLECTION`
- `EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID`

## Mobile Flow

1. User writes a `.anky` file locally.
2. The app computes `sha256(raw_utf8_bytes_of_the_file)`.
3. User connects a Solana wallet through Phantom/Mobile Wallet Adapter or Privy embedded wallet.
4. User mints one Loom in the app through Metaplex Core.
5. The Loom owner seals the `.anky` `session_hash` through the Anky Seal Program.

The mobile `mintAnkyLoom` file now includes two devnet Metaplex Core builder paths:

- `buildSelfFundedCoreLoomMintTransaction`: useful for authority-wallet testing. It builds a Core `create` transaction locally and requires the connected wallet to be the collection update authority.
- `createBackendPreparedCoreLoomMintTransactionBuilder`: the normal product path. The backend prepares a transaction that is already signed by Anky's collection authority and the new asset keypair; the mobile wallet signs and sends as payer/owner.

Important mint authority note: creating an asset inside the official Core collection requires the collection update authority or a valid delegate. A normal user cannot unilaterally mint into Anky's official collection just by paying SOL.

The mobile `sealAnky` file builds the Anchor `seal_anky([u8; 32])` instruction directly with `@solana/web3.js`, derives the `loom_state` PDA, asks the wallet to sign, sends the transaction, and returns a confirmed receipt shape.

## Backend Mobile API

The Rust server now exposes the integration spine used by the Expo app:

- `GET /api/mobile/solana/config`: devnet RPC, Core collection, Core program, seal program, and metadata URLs.
- `GET /api/mobile/credits?identityId=...`: server-backed mobile credit balance.
- `POST /api/mobile/credits/spend`: debit credits for a named backend action.
- `POST /api/mobile/looms/mint-authorizations`: self-funded or invite-code mint authorization shape.
- `POST /api/mobile/looms/prepare-mint`: prepare a partially signed Metaplex Core transaction for an authorized Loom mint.
- `POST /api/mobile/looms/record`: record a Loom mint receipt after the mobile wallet sends the Core transaction.
- `GET /api/mobile/looms?wallet=...`: list Loom mint receipts recorded for a wallet.
- `POST /api/mobile/seals/record`: record a confirmed seal transaction receipt.
- `GET /api/mobile/seals?wallet=...|loomId=...|sessionHash=...`: look up seal receipts.
- `POST /api/mobile/reflections`: spend one credit and create a dev reflection artifact from explicitly submitted `.anky` plaintext.

Seal and Loom receipt endpoints store only hashes, public keys, transaction signatures, and timing data. The reflection endpoint is the one route that receives plaintext, and only because the user explicitly asks the backend to process a mirror.

The prepare-mint endpoint needs the Core collection authority keypair configured on the server:

```bash
ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR_PATH=/home/kithkui/.config/solana/deployer.json
```

The keypair must match the Core collection update authority. On current devnet that authority is `FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP`.

## Seal Semantics

A seal means:

> Wallet W used Loom L to anchor `.anky` hash H at Solana time T.

On each seal, `LoomState` stores:

- `loom_asset`
- `total_seals`
- `latest_session_hash`
- `rolling_root`
- `created_at`
- `updated_at`

The rolling root domain is `ANKY_LOOM_ROOT_V1` and includes the previous root, writer, Loom asset, session hash, total seal count, and timestamp.

## Core Verification Status

Do not use this on mainnet yet.

The current `verify_core_loom` implementation is no longer the original placeholder. It performs minimal Metaplex Core base-account verification:

- the Loom asset account is owned by the Metaplex Core program
- the collection account is owned by the Metaplex Core program
- the supplied collection account equals `OFFICIAL_COLLECTION`
- the Core asset discriminator is `AssetV1`
- the Core Asset owner equals the writer wallet
- the Core asset update authority is the official collection
- the collection discriminator is `CollectionV1`

This is enough for devnet end-to-end testing, but it is still not mainnet-safe. Before mainnet, audit the hand-rolled Core account parser against the exact mpl-core account layout, add integration tests against real Core assets, and harden the mint authority policy for the official collection.

Metaplex Core program ID reference: https://www.metaplex.com/docs/smart-contracts/core/collections
