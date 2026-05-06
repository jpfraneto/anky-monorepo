import assert from "node:assert/strict";
import { execFile, execFileSync } from "node:child_process";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "smokeVerifiedSealMigration.mjs",
);
const REPO_ROOT = path.resolve(path.dirname(SCRIPT_PATH), "../../..");
const POSTGRES_BINS_AVAILABLE = ["initdb", "pg_ctl", "createdb", "psql"].every((binary) =>
  commandExists(binary),
);

test("prints verified receipt migration smoke usage", async () => {
  const result = await runNode([SCRIPT_PATH, "--help"]);

  assert.equal(result.code, 0, result.stderr);
  assert.match(result.stdout, /smokeVerifiedSealMigration\.mjs/);
  assert.match(result.stdout, /disposable local Postgres cluster/);
});

test("rejects unknown migration smoke options", async () => {
  const result = await runNode([SCRIPT_PATH, "--database-url", "postgres://example"]);

  assert.notEqual(result.code, 0);
  assert.match(result.stderr, /Unknown option: --database-url/);
  assert.equal(result.stdout, "");
});

test(
  "verifies clean and partial verified receipt migrations against disposable Postgres",
  { skip: !POSTGRES_BINS_AVAILABLE },
  async () => {
    const result = await runNode([SCRIPT_PATH]);

    assert.equal(result.code, 0, result.stderr);
    const summary = JSON.parse(result.stdout);
    assert.equal(summary.ok, true);
    assert.deepEqual(
      summary.scenarios.map((scenario) => scenario.mode).sort(),
      ["clean", "partial_existing_verified_table"],
    );
    for (const scenario of summary.scenarios) {
      assert.ok(
        scenario.verifiedConstraints.includes("mobile_verified_seal_receipts_matching_seal"),
      );
      assert.ok(
        scenario.verifiedIndexes.includes(
          "idx_mobile_verified_seal_receipts_network_wallet_hash_unique",
        ),
      );
      assert.ok(
        scenario.verifiedIndexes.includes("idx_mobile_verified_seal_receipts_signature_unique"),
      );
      assert.ok(
        scenario.heliusConstraints.includes("mobile_helius_webhook_events_payload_hash_hex"),
      );
      assert.ok(
        scenario.heliusConstraints.includes("mobile_helius_webhook_events_event_count_positive"),
      );
      assert.ok(
        scenario.heliusIndexes.includes(
          "idx_mobile_helius_webhook_events_network_payload_hash_unique",
        ),
      );
      assert.ok(
        scenario.heliusIndexes.includes(
          "idx_mobile_helius_webhook_events_network_signature_unique",
        ),
      );
      assert.equal(scenario.scoreContract.score, 12);
      assert.equal(scenario.scoreContract.streakBonus, 2);
      assert.deepEqual(scenario.scoreContract.verifiedDays, [21000, 21006]);
      assert.deepEqual(scenario.scoreContract.sealedDays, [
        21000, 21001, 21002, 21003, 21004, 21005, 21006, 21008,
      ]);
    }
  },
);

function commandExists(binary) {
  try {
    execFileSync("bash", ["-lc", `command -v ${binary}`], { stdio: "ignore" });
    return true;
  } catch (_error) {
    return false;
  }
}

function runNode(args) {
  return new Promise((resolve) => {
    execFile(
      process.execPath,
      args,
      {
        cwd: REPO_ROOT,
      },
      (error, stdout, stderr) => {
        resolve({
          code: error?.code ?? 0,
          stderr,
          stdout,
        });
      },
    );
  });
}
