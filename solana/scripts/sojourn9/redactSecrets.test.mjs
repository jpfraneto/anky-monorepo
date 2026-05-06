import assert from "node:assert/strict";
import test from "node:test";

import { redactSecretValues } from "./redactSecrets.mjs";

test("redacts credential-bearing URLs, bearer tokens, and keypair-looking paths", () => {
  const redacted = redactSecretValues(
    [
      "https://devnet.helius-rpc.com/?api-key=secret-api-key",
      "Authorization: Bearer actual-backend-secret",
      "postgres://user:password@localhost/anky",
      "/home/user/verifier-keypair.json",
    ].join("\n"),
  );

  assert.match(redacted, /api-key=<redacted>/);
  assert.match(redacted, /Bearer <redacted>/);
  assert.match(redacted, /postgres:\/\/<redacted>/);
  assert.match(redacted, /<redacted-path>/);
  assert.doesNotMatch(redacted, /secret-api-key|actual-backend-secret|password|verifier-keypair/);
});

test("leaves public placeholders readable", () => {
  assert.equal(
    redactSecretValues("HELIUS_API_KEY=<configured_in_shell> ANKY_INDEXER_WRITE_SECRET=<backend_write_secret>"),
    "HELIUS_API_KEY=<configured_in_shell> ANKY_INDEXER_WRITE_SECRET=<backend_write_secret>",
  );
});
