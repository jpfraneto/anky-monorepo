/**
 * Anky Solana Worker — Cloudflare Worker for Sojourn 9
 *
 * Endpoints:
 *   POST /mint         — mint a Mirror cNFT (membership, auth required)
 *   POST /mint-anky    — mint an Anky cNFT (milestone, auth required)
 *   POST /log-session  — log a writing session hash on-chain via spl-memo (auth required)
 *   GET  /supply       — current mirror mint count
 *
 * Secrets (set via wrangler secret put):
 *   MINT_SECRET, AUTHORITY_KEYPAIR, HELIUS_API_KEY, MERKLE_TREE, COLLECTION_MINT
 */

import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  mintV1,
  mplBubblegum,
} from "@metaplex-foundation/mpl-bubblegum";
import { mplTokenMetadata } from "@metaplex-foundation/mpl-token-metadata";
import {
  keypairIdentity,
  publicKey,
  transactionBuilder,
  type Umi,
} from "@metaplex-foundation/umi";
import bs58 from "bs58";

// SPL Memo Program v2 — deployed on all Solana clusters
const MEMO_PROGRAM_ID = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Env {
  MINT_SECRET: string;
  AUTHORITY_KEYPAIR: string;
  HELIUS_API_KEY: string;
  MERKLE_TREE: string;        // mirrors tree
  COLLECTION_MINT: string;    // mirrors collection
  ANKYS_MERKLE_TREE: string;  // ankys tree
  ANKYS_COLLECTION_MINT: string; // ankys collection
  SOLANA_NETWORK?: string;
  SOLANA_RPC_URL?: string;
}

interface MintRequest {
  mirror_id: string;
  recipient: string;
  name: string;
  uri: string;
  kingdom: number;
  symbol: string;
}

interface LogSessionRequest {
  session_hash: string;
  user_wallet: string;
  session_id: string;
  duration_seconds: number;
  word_count: number;
  kingdom_id: number;
  sojourn: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    },
  });
}

function errorResponse(message: string, status = 400): Response {
  return json({ error: message }, status);
}

function authenticate(request: Request, secret: string): boolean {
  const auth = request.headers.get("Authorization");
  return auth === `Bearer ${secret}`;
}

function getRpcUrl(env: Env): string {
  if (env.SOLANA_RPC_URL) return env.SOLANA_RPC_URL;
  if (env.HELIUS_API_KEY) {
    const network = env.SOLANA_NETWORK || "devnet";
    const rpcHost = network === "mainnet-beta" ? "mainnet" : "devnet";
    return `https://${rpcHost}.helius-rpc.com/?api-key=${env.HELIUS_API_KEY}`;
  }
  return "https://api.devnet.solana.com";
}

function buildUmi(env: Env): Umi {
  const rpcUrl = getRpcUrl(env);
  const umi = createUmi(rpcUrl).use(mplBubblegum()).use(mplTokenMetadata());

  const secretKey = bs58.decode(env.AUTHORITY_KEYPAIR);
  const keypair = umi.eddsa.createKeypairFromSecretKey(secretKey);
  umi.use(keypairIdentity(keypair));

  return umi;
}

// ---------------------------------------------------------------------------
// POST /mint
// ---------------------------------------------------------------------------

async function handleMint(request: Request, env: Env): Promise<Response> {
  if (!authenticate(request, env.MINT_SECRET)) {
    return errorResponse("unauthorized", 401);
  }

  let body: MintRequest;
  try {
    body = await request.json() as MintRequest;
  } catch {
    return errorResponse("invalid json");
  }

  const { mirror_id, recipient, name, uri, kingdom, symbol } = body;
  if (!mirror_id || !recipient || !name || !uri) {
    return errorResponse("missing required fields: mirror_id, recipient, name, uri");
  }

  const KINGDOMS = [
    "Primordia", "Emblazion", "Chryseos", "Eleutheria",
    "Voxlumis", "Insightia", "Claridium", "Poiesis",
  ];
  const kingdomName = KINGDOMS[kingdom] || "Unknown";

  try {
    const umi = buildUmi(env);
    const merkleTree = publicKey(env.MERKLE_TREE);
    const collectionMint = publicKey(env.COLLECTION_MINT);

    const builder = mintV1(umi, {
      leafOwner: publicKey(recipient),
      merkleTree,
      metadata: {
        name,
        symbol: symbol || "ANKY",
        uri,
        sellerFeeBasisPoints: 0,
        collection: { key: collectionMint, verified: false },
        creators: [
          {
            address: umi.identity.publicKey,
            verified: false,
            share: 100,
          },
        ],
      },
    });

    // Send without waiting for confirmation (devnet block height expiry workaround)
    const tx = await builder.send(umi);

    const signature = bs58.encode(tx);

    // Derive asset ID from the leaf — for now return the signature
    // The actual asset ID can be fetched via DAS after confirmation
    return json({
      success: true,
      signature,
      mirror_id,
      kingdom: kingdomName,
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("mint error:", message);
    return errorResponse(`mint failed: ${message}`, 500);
  }
}

// ---------------------------------------------------------------------------
// POST /mint-anky — mint an anky cNFT (separate collection from mirrors)
// ---------------------------------------------------------------------------

async function handleMintAnky(request: Request, env: Env): Promise<Response> {
  if (!authenticate(request, env.MINT_SECRET)) {
    return errorResponse("unauthorized", 401);
  }

  let body: MintRequest;
  try {
    body = await request.json() as MintRequest;
  } catch {
    return errorResponse("invalid json");
  }

  const { mirror_id, recipient, name, uri, kingdom, symbol } = body;
  if (!mirror_id || !recipient || !name || !uri) {
    return errorResponse("missing required fields: mirror_id, recipient, name, uri");
  }

  if (!env.ANKYS_MERKLE_TREE || !env.ANKYS_COLLECTION_MINT) {
    return errorResponse("ankys tree not configured", 500);
  }

  const KINGDOMS = [
    "Primordia", "Emblazion", "Chryseos", "Eleutheria",
    "Voxlumis", "Insightia", "Claridium", "Poiesis",
  ];
  const kingdomName = KINGDOMS[kingdom] || "Unknown";

  try {
    const umi = buildUmi(env);
    const merkleTree = publicKey(env.ANKYS_MERKLE_TREE);
    const collectionMint = publicKey(env.ANKYS_COLLECTION_MINT);

    const builder = mintV1(umi, {
      leafOwner: publicKey(recipient),
      merkleTree,
      metadata: {
        name,
        symbol: symbol || "ANKY",
        uri,
        sellerFeeBasisPoints: 0,
        collection: { key: collectionMint, verified: false },
        creators: [
          { address: umi.identity.publicKey, verified: false, share: 100 },
        ],
      },
    });

    const tx = await builder.send(umi);
    const signature = bs58.encode(tx);

    return json({ success: true, signature, mirror_id, kingdom: kingdomName });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("mint-anky error:", message);
    return errorResponse(`mint failed: ${message}`, 500);
  }
}

// ---------------------------------------------------------------------------
// POST /log-session — log a writing session hash on-chain via spl-memo
// ---------------------------------------------------------------------------

async function handleLogSession(request: Request, env: Env): Promise<Response> {
  if (!authenticate(request, env.MINT_SECRET)) {
    return errorResponse("unauthorized", 401);
  }

  let body: LogSessionRequest;
  try {
    body = await request.json() as LogSessionRequest;
  } catch {
    return errorResponse("invalid json");
  }

  const { session_hash, user_wallet, session_id, duration_seconds, word_count, kingdom_id, sojourn } = body;
  if (!session_hash || !session_id) {
    return errorResponse("missing required fields: session_hash, session_id");
  }

  // Compact memo: pipe-separated for on-chain parsability
  // Format: anky|<session_hash>|<session_id>|<wallet>|<duration>|<words>|<kingdom>|<sojourn>
  const memo = [
    "anky",
    session_hash,
    session_id,
    user_wallet || "",
    String(duration_seconds || 0),
    String(word_count || 0),
    String(kingdom_id || 0),
    String(sojourn || 9),
  ].join("|");

  try {
    const umi = buildUmi(env);

    const memoData = new TextEncoder().encode(memo);

    // Include user's wallet as a non-signer account reference on the memo instruction.
    // This makes the transaction indexed under BOTH the authority wallet AND the user's wallet,
    // so getSignaturesForAddress(userWallet) returns their session logs too.
    const keys: Array<{ pubkey: ReturnType<typeof publicKey>; isSigner: false; isWritable: false }> = [];
    if (user_wallet) {
      try {
        keys.push({
          pubkey: publicKey(user_wallet),
          isSigner: false,
          isWritable: false,
        });
      } catch {
        // Invalid wallet address — skip, log without account reference
      }
    }

    const builder = transactionBuilder().add({
      instruction: {
        programId: publicKey(MEMO_PROGRAM_ID),
        keys,
        data: memoData,
      },
      signers: [umi.identity],
      bytesCreatedOnChain: 0,
    });

    const tx = await builder.sendAndConfirm(umi, {
      confirm: { commitment: "confirmed" },
    });

    const signature = bs58.encode(tx.signature);

    return json({
      success: true,
      signature,
      session_id,
      session_hash,
      memo,
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("log-session error:", message);
    return errorResponse(`log failed: ${message}`, 500);
  }
}

// ---------------------------------------------------------------------------
// GET /supply
// ---------------------------------------------------------------------------

async function handleSupply(env: Env): Promise<Response> {
  const MAX_SUPPLY = 3456;

  try {
    const rpcUrl = getRpcUrl(env);

    const response = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: "supply-check",
        method: "getAssetsByGroup",
        params: {
          groupKey: "collection",
          groupValue: env.COLLECTION_MINT,
          page: 1,
          limit: 1,
        },
      }),
    });

    const data = await response.json() as {
      result?: { total?: number };
    };
    const minted = data?.result?.total ?? 0;

    return json({
      minted,
      max_supply: MAX_SUPPLY,
      remaining: Math.max(0, MAX_SUPPLY - minted),
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("supply check error:", message);
    // Fallback: return unknown rather than failing
    return json({
      minted: -1,
      max_supply: MAX_SUPPLY,
      remaining: -1,
      error: message,
    });
  }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // CORS preflight
    if (request.method === "OPTIONS") {
      return new Response(null, {
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Authorization",
        },
      });
    }

    if (url.pathname === "/mint" && request.method === "POST") {
      return handleMint(request, env);
    }

    if (url.pathname === "/mint-anky" && request.method === "POST") {
      return handleMintAnky(request, env);
    }

    if (url.pathname === "/log-session" && request.method === "POST") {
      return handleLogSession(request, env);
    }

    if (url.pathname === "/supply" && request.method === "GET") {
      return handleSupply(env);
    }

    return json({ status: "anky mint worker", endpoints: ["/mint", "/mint-anky", "/log-session", "/supply"] });
  },
};
