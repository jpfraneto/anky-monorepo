// anky-protocol.js — the complete protocol for browsers
//
// encrypt .anky session → upload to Irys → anchor to Solana
// no dependencies except @solana/web3.js (loaded via CDN)

const PROGRAM_ID = "2Q3xXCd4f9nMbb2kMyg7opEncU9J638BYUU1XhM8UukH";
const ENCLAVE_PUBLIC_KEY = Uint8Array.from(atob("mbuydCxEAulK+qRSrs23V87hbzemvI7MNPo6JwBRHls="), c => c.charCodeAt(0));
const FEE_PAYER_PUBKEY = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const RELAYER_URL = "/api/v1/relay";

// ── .anky session builder ──────────────────────────────────────────────

function createSessionBuilder() {
  let lines = [];
  let lastTime = 0;
  let started = false;

  return {
    keystroke(char) {
      const now = Date.now();
      if (!started) {
        lines.push(now + " " + char);
        lastTime = now;
        started = true;
      } else {
        const delta = Math.min(now - lastTime, 7999);
        lines.push(String(delta).padStart(4, "0") + " " + char);
        lastTime = now;
      }
    },
    end() {
      lines.push("8000");
      return lines.join("\n");
    },
    elapsed() {
      if (!started) return 0;
      return Date.now() - parseInt(lines[0].split(" ")[0]);
    },
    lineCount() {
      return lines.length;
    }
  };
}

// ── sha256 ─────────────────────────────────────────────────────────────

async function sha256(str) {
  const data = new TextEncoder().encode(str);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  return new Uint8Array(hashBuffer);
}

function toHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join("");
}

// ── encryption (ECIES: X25519 + AES-256-GCM) ──────────────────────────
// Uses Web Crypto API — works in all modern browsers, no libraries needed.
// X25519 key agreement requires importing tweetnacl for the scalar mult,
// since Web Crypto doesn't expose raw X25519 in all browsers yet.
// For now we use nacl.scalarMult from the global (loaded via CDN).

async function encryptSession(sessionString) {
  const plaintext = new TextEncoder().encode(sessionString);

  // ephemeral X25519 keypair
  const ephemeral = nacl.box.keyPair();

  // derive shared secret
  const sharedSecret = nacl.scalarMult(ephemeral.secretKey, ENCLAVE_PUBLIC_KEY);

  // derive AES key from shared secret via SHA-256
  const aesKeyRaw = await crypto.subtle.digest("SHA-256", sharedSecret);
  const aesKey = await crypto.subtle.importKey("raw", aesKeyRaw, "AES-GCM", false, ["encrypt"]);

  // encrypt with AES-256-GCM
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const ciphertextWithTag = await crypto.subtle.encrypt({ name: "AES-GCM", iv: nonce }, aesKey, plaintext);

  // Web Crypto appends the 16-byte tag to the ciphertext
  const ciphertext = new Uint8Array(ciphertextWithTag.slice(0, -16));
  const tag = new Uint8Array(ciphertextWithTag.slice(-16));

  const sessionHash = toHex(await sha256(sessionString));

  return {
    ephemeralPublicKey: btoa(String.fromCharCode(...ephemeral.publicKey)),
    nonce: btoa(String.fromCharCode(...nonce)),
    tag: btoa(String.fromCharCode(...tag)),
    ciphertext: btoa(String.fromCharCode(...ciphertext)),
    sessionHash,
  };
}

// ── upload to relayer (which handles Irys + Solana) ────────────────────

async function anchorSession(sessionString, writerPublicKey) {
  const encrypted = await encryptSession(sessionString);
  const hash = encrypted.sessionHash;

  const resp = await fetch(RELAYER_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      encrypted,
      writer_pubkey: writerPublicKey,
    }),
  });

  if (!resp.ok) throw new Error("relay failed: " + await resp.text());
  return await resp.json();
  // returns: { hash, arweave_tx, solana_tx, explorer_url, arweave_url }
}
