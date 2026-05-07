#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { redactSecretValues } from "./redactSecrets.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "../../..");
const MIGRATION_017 = path.join(REPO_ROOT, "migrations", "017_mobile_solana_integration.sql");
const MIGRATION_019_VERIFIED_SEAL = path.join(
  REPO_ROOT,
  "migrations",
  "019_mobile_verified_seal_receipts.sql",
);
const MIGRATION_020_HELIUS_WEBHOOK = path.join(
  REPO_ROOT,
  "migrations",
  "020_mobile_helius_webhook_events.sql",
);
const MIGRATION_021_HELIUS_SIGNATURE_DEDUPE = path.join(
  REPO_ROOT,
  "migrations",
  "021_mobile_helius_webhook_signature_dedupe.sql",
);
const MIGRATION_022_CREDIT_LEDGER = path.join(
  REPO_ROOT,
  "migrations",
  "022_credit_ledger_entries.sql",
);
const BOOLEAN_FLAGS = new Set(["--keep"]);
const VALUE_FLAGS = new Set([]);
const REQUIRED_BINS = ["initdb", "pg_ctl", "createdb", "psql"];

main().catch((error) => {
  console.error(redactSecretValues(error instanceof Error ? error.message : String(error)));
  process.exit(1);
});

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help === true) {
    printUsage();
    return;
  }

  requireBinaries(REQUIRED_BINS);
  requireFile(MIGRATION_017);
  requireFile(MIGRATION_019_VERIFIED_SEAL);
  requireFile(MIGRATION_020_HELIUS_WEBHOOK);
  requireFile(MIGRATION_021_HELIUS_SIGNATURE_DEDUPE);
  requireFile(MIGRATION_022_CREDIT_LEDGER);

  const workDir = fs.mkdtempSync(path.join(os.tmpdir(), "anky-verified-migration-"));
  const dataDir = path.join(workDir, "pgdata");
  const socketDir = path.join(workDir, "socket");
  const logPath = path.join(workDir, "postgres.log");
  const port = String(24_000 + crypto.randomInt(16_000));
  fs.mkdirSync(socketDir, { recursive: true });

  let started = false;
  try {
    await run("initdb", ["-D", dataDir, "-A", "trust", "-U", "postgres", "--no-instructions"], {
      cwd: workDir,
    });
    await run("pg_ctl", [
      "-D",
      dataDir,
      "-l",
      logPath,
      "-o",
      `-F -k ${socketDir} -p ${port}`,
      "start",
    ]);
    started = true;

    const clean = await runScenario({
      createPartialVerifiedTable: false,
      database: "clean_verified_smoke",
      port,
      socketDir,
    });
    const partial = await runScenario({
      createPartialVerifiedTable: true,
      database: "partial_verified_smoke",
      port,
      socketDir,
    });

    console.log(
      JSON.stringify(
        {
          ok: clean.ok && partial.ok,
          scenarios: [clean, partial],
          tempDir: args.keep === true ? workDir : undefined,
        },
        null,
        2,
      ),
    );
  } finally {
    if (started) {
      await run("pg_ctl", ["-D", dataDir, "stop", "-m", "fast"], { allowFailure: true });
    }
    if (args.keep !== true) {
      fs.rmSync(workDir, { force: true, recursive: true });
    }
  }
}

async function runScenario({ createPartialVerifiedTable, database, port, socketDir }) {
  await run("createdb", pgServerArgs({ port, socketDir }).concat(database));
  await psqlFile({ database, file: MIGRATION_017, port, socketDir });
  if (createPartialVerifiedTable) {
    await psqlExec({
      database,
      port,
      socketDir,
      sql: `
        CREATE TABLE mobile_verified_seal_receipts (
          id TEXT PRIMARY KEY,
          network TEXT NOT NULL DEFAULT 'devnet',
          wallet TEXT NOT NULL,
          session_hash TEXT NOT NULL,
          proof_hash TEXT NOT NULL,
          verifier TEXT NOT NULL,
          protocol_version INTEGER NOT NULL,
          signature TEXT NOT NULL,
          slot BIGINT,
          block_time BIGINT,
          status TEXT NOT NULL DEFAULT 'confirmed',
          created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
      `,
    });
  }

  await psqlFile({ database, file: MIGRATION_019_VERIFIED_SEAL, port, socketDir });
  await psqlFile({ database, file: MIGRATION_019_VERIFIED_SEAL, port, socketDir });
  await psqlFile({ database, file: MIGRATION_020_HELIUS_WEBHOOK, port, socketDir });
  await psqlFile({ database, file: MIGRATION_020_HELIUS_WEBHOOK, port, socketDir });
  await psqlFile({ database, file: MIGRATION_021_HELIUS_SIGNATURE_DEDUPE, port, socketDir });
  await psqlFile({ database, file: MIGRATION_021_HELIUS_SIGNATURE_DEDUPE, port, socketDir });
  await psqlFile({ database, file: MIGRATION_022_CREDIT_LEDGER, port, socketDir });
  await psqlFile({ database, file: MIGRATION_022_CREDIT_LEDGER, port, socketDir });
  const schema = await readSchema({ database, port, socketDir });
  assertSchema(schema);
  await assertInsertContract({ database, port, socketDir });
  const scoreContract = await assertScoreContract({ database, port, socketDir });

  return {
    database,
    mode: createPartialVerifiedTable ? "partial_existing_verified_table" : "clean",
    ok: true,
    scoreContract,
    heliusConstraints: schema.constraints
      .filter((constraint) => constraint.table === "mobile_helius_webhook_events")
      .map((constraint) => constraint.name)
      .sort(),
    heliusIndexes: schema.indexes
      .filter((index) => index.table === "mobile_helius_webhook_events")
      .map((index) => index.name)
      .sort(),
    verifiedConstraints: schema.constraints
      .filter((constraint) => constraint.table === "mobile_verified_seal_receipts")
      .map((constraint) => constraint.name)
      .sort(),
    verifiedIndexes: schema.indexes
      .filter((index) => index.table === "mobile_verified_seal_receipts")
      .map((index) => index.name)
      .sort(),
  };
}

async function readSchema({ database, port, socketDir }) {
  const output = await psqlCapture({
    database,
    port,
    socketDir,
    sql: `
      SELECT json_build_object(
        'columns', (
          SELECT json_agg(json_build_object(
            'table', table_name,
            'name', column_name,
            'type', data_type
          ) ORDER BY table_name, ordinal_position)
          FROM information_schema.columns
          WHERE table_name IN ('mobile_seal_receipts', 'mobile_verified_seal_receipts', 'mobile_helius_webhook_events')
        ),
        'constraints', (
          SELECT json_agg(json_build_object(
            'table', rel.relname,
            'name', con.conname,
            'type', con.contype
          ) ORDER BY rel.relname, con.conname)
          FROM pg_constraint con
          JOIN pg_class rel ON rel.oid = con.conrelid
          WHERE rel.relname IN ('mobile_seal_receipts', 'mobile_verified_seal_receipts', 'mobile_helius_webhook_events')
        ),
        'indexes', (
          SELECT json_agg(json_build_object(
            'table', tablename,
            'name', indexname,
            'definition', indexdef
          ) ORDER BY tablename, indexname)
          FROM pg_indexes
          WHERE tablename IN ('mobile_seal_receipts', 'mobile_verified_seal_receipts', 'mobile_helius_webhook_events')
        )
      )::text;
    `,
  });

  return JSON.parse(output);
}

function assertSchema(schema) {
  const columns = schema.columns ?? [];
  const constraints = schema.constraints ?? [];
  const indexes = schema.indexes ?? [];

  requireColumn(columns, "mobile_seal_receipts", "utc_day", "bigint");
  requireColumn(columns, "mobile_verified_seal_receipts", "utc_day", "bigint");
  requireColumn(columns, "mobile_helius_webhook_events", "payload_json", "text");

  for (const forbidden of ["anky", "plaintext", "witness", "private", "raw", "content"]) {
    const matchingColumn = columns.find(
      (column) =>
        column.table === "mobile_verified_seal_receipts" &&
        column.name.toLowerCase().includes(forbidden),
    );
    if (matchingColumn != null) {
      throw new Error(
        `mobile_verified_seal_receipts must not contain private/plaintext column ${matchingColumn.name}.`,
      );
    }
  }

  requireConstraint(constraints, "mobile_seal_receipts_utc_day_nonnegative", "c");
  requireConstraint(constraints, "mobile_verified_seal_receipts_matching_seal", "f");
  requireConstraint(constraints, "mobile_verified_seal_receipts_proof_hash_hex", "c");
  requireConstraint(constraints, "mobile_verified_seal_receipts_protocol_version", "c");
  requireConstraint(constraints, "mobile_verified_seal_receipts_session_hash_hex", "c");
  requireConstraint(constraints, "mobile_verified_seal_receipts_status", "c");
  requireConstraint(constraints, "mobile_verified_seal_receipts_utc_day_nonnegative", "c");
  requireConstraint(constraints, "mobile_helius_webhook_events_payload_hash_hex", "c");
  requireConstraint(constraints, "mobile_helius_webhook_events_event_count_positive", "c");
  requireConstraint(constraints, "mobile_helius_webhook_events_source", "c");

  requireIndex(indexes, "idx_mobile_seal_receipts_network_wallet_hash_unique", true);
  requireIndex(indexes, "idx_mobile_verified_seal_receipts_network_wallet_hash_unique", true);
  requireIndex(indexes, "idx_mobile_verified_seal_receipts_signature_unique", true);
  requireIndex(indexes, "idx_mobile_helius_webhook_events_network_payload_hash_unique", true);
  requireIndex(indexes, "idx_mobile_helius_webhook_events_network_signature_unique", true);
}

async function assertInsertContract({ database, port, socketDir }) {
  await psqlExec({
    database,
    port,
    socketDir,
    sql: `
      INSERT INTO mobile_seal_receipts
        (id, network, wallet, loom_asset, core_collection, session_hash, signature, utc_day, status)
      VALUES
        ('seal-ok', 'devnet', 'wallet-ok', 'loom-ok', 'collection-ok', '${"a".repeat(64)}', 'seal-sig-ok', 20579, 'finalized');

      INSERT INTO mobile_verified_seal_receipts
        (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
      VALUES
        ('verified-ok', 'devnet', 'wallet-ok', '${"a".repeat(64)}', '${"b".repeat(64)}', 'verifier-ok', 1, 20579, 'verified-sig-ok', 'finalized');

      DO $$
      DECLARE
        upsert_count INTEGER;
      BEGIN
        BEGIN
          INSERT INTO mobile_verified_seal_receipts
            (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
          VALUES
            ('verified-orphan', 'devnet', 'wallet-missing', '${"c".repeat(64)}', '${"d".repeat(64)}', 'verifier-ok', 1, 20579, 'verified-sig-orphan', 'finalized');
          RAISE EXCEPTION 'orphan verified receipt insert unexpectedly succeeded';
        EXCEPTION WHEN foreign_key_violation THEN
          NULL;
        END;

        BEGIN
          INSERT INTO mobile_verified_seal_receipts
            (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
          VALUES
            ('verified-pending', 'devnet', 'wallet-ok', '${"a".repeat(64)}', '${"c".repeat(64)}', 'verifier-ok', 1, 20579, 'verified-sig-pending', 'pending');
          RAISE EXCEPTION 'pending verified receipt insert unexpectedly succeeded';
        EXCEPTION WHEN check_violation THEN
          NULL;
        END;

        WITH upsert AS (
          INSERT INTO mobile_verified_seal_receipts
            (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
          VALUES
            ('verified-ok-idempotent', 'devnet', 'wallet-ok', '${"a".repeat(64)}', '${"b".repeat(64)}', 'verifier-ok', 1, 20579, 'verified-sig-ok', 'confirmed')
          ON CONFLICT (network, wallet, session_hash) DO UPDATE
          SET slot = COALESCE(EXCLUDED.slot, mobile_verified_seal_receipts.slot),
              block_time = COALESCE(EXCLUDED.block_time, mobile_verified_seal_receipts.block_time),
              status = EXCLUDED.status
          WHERE mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash
            AND mobile_verified_seal_receipts.verifier = EXCLUDED.verifier
            AND mobile_verified_seal_receipts.protocol_version = EXCLUDED.protocol_version
            AND mobile_verified_seal_receipts.utc_day IS NOT DISTINCT FROM EXCLUDED.utc_day
            AND mobile_verified_seal_receipts.signature = EXCLUDED.signature
          RETURNING 1
        )
        SELECT COUNT(*) INTO upsert_count FROM upsert;
        IF upsert_count != 1 THEN
          RAISE EXCEPTION 'idempotent verified receipt route upsert unexpectedly returned % rows', upsert_count;
        END IF;

        WITH upsert AS (
          INSERT INTO mobile_verified_seal_receipts
            (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
          VALUES
            ('verified-conflict', 'devnet', 'wallet-ok', '${"a".repeat(64)}', '${"c".repeat(64)}', 'verifier-ok', 1, 20579, 'verified-sig-conflict', 'confirmed')
          ON CONFLICT (network, wallet, session_hash) DO UPDATE
          SET slot = COALESCE(EXCLUDED.slot, mobile_verified_seal_receipts.slot),
              block_time = COALESCE(EXCLUDED.block_time, mobile_verified_seal_receipts.block_time),
              status = EXCLUDED.status
          WHERE mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash
            AND mobile_verified_seal_receipts.verifier = EXCLUDED.verifier
            AND mobile_verified_seal_receipts.protocol_version = EXCLUDED.protocol_version
            AND mobile_verified_seal_receipts.utc_day IS NOT DISTINCT FROM EXCLUDED.utc_day
            AND mobile_verified_seal_receipts.signature = EXCLUDED.signature
          RETURNING 1
        )
        SELECT COUNT(*) INTO upsert_count FROM upsert;
        IF upsert_count != 0 THEN
          RAISE EXCEPTION 'conflicting verified receipt route upsert unexpectedly returned % rows', upsert_count;
        END IF;

        BEGIN
          INSERT INTO mobile_seal_receipts
            (id, network, wallet, loom_asset, core_collection, session_hash, signature, utc_day, status)
          VALUES
            ('seal-bad-day', 'devnet', 'wallet-bad-day', 'loom-ok', 'collection-ok', '${"e".repeat(64)}', 'seal-sig-bad-day', -1, 'finalized');
          RAISE EXCEPTION 'negative seal utc_day insert unexpectedly succeeded';
        EXCEPTION WHEN check_violation THEN
          NULL;
        END;
      END
      $$;

      INSERT INTO mobile_helius_webhook_events
        (id, network, source, payload_hash, signature, event_count, payload_json)
      VALUES
        ('helius-ok', 'devnet', 'helius_enhanced_webhook', '${"f".repeat(64)}', 'helius-sig-ok', 1, '{"signature":"helius-sig-ok"}');

      DO $$
      DECLARE
        duplicate_count INTEGER;
        upsert_count INTEGER;
      BEGIN
        WITH signature_existing AS (
            UPDATE mobile_helius_webhook_events
            SET event_count = GREATEST(2, event_count),
                payload_json = '{"signature":"helius-sig-ok","retry":true}'
            WHERE network = 'devnet' AND signature = 'helius-sig-ok'
            RETURNING 1
        ),
        hash_existing AS (
            UPDATE mobile_helius_webhook_events
            SET signature = COALESCE(signature, 'helius-sig-ok'),
                event_count = GREATEST(2, event_count),
                payload_json = '{"signature":"helius-sig-ok","retry":true}'
            WHERE network = 'devnet'
              AND payload_hash = '${"2".repeat(64)}'
              AND NOT EXISTS (SELECT 1 FROM signature_existing)
            RETURNING 1
        ),
        inserted AS (
            INSERT INTO mobile_helius_webhook_events
              (id, network, source, payload_hash, signature, event_count, payload_json)
            SELECT
              'helius-ok-retry', 'devnet', 'helius_enhanced_webhook', '${"2".repeat(64)}', 'helius-sig-ok', 2, '{"signature":"helius-sig-ok","retry":true}'
            WHERE NOT EXISTS (SELECT 1 FROM signature_existing)
              AND NOT EXISTS (SELECT 1 FROM hash_existing)
            ON CONFLICT (network, signature) WHERE signature IS NOT NULL DO UPDATE
            SET event_count = GREATEST(EXCLUDED.event_count, mobile_helius_webhook_events.event_count),
                payload_json = EXCLUDED.payload_json
            RETURNING 1
        )
        SELECT COUNT(*) INTO upsert_count
        FROM (
          SELECT 1 FROM signature_existing
          UNION ALL
          SELECT 1 FROM hash_existing
          UNION ALL
          SELECT 1 FROM inserted
          LIMIT 1
        ) upsert;
        IF upsert_count != 1 THEN
          RAISE EXCEPTION 'idempotent Helius signature upsert unexpectedly returned % rows', upsert_count;
        END IF;

        SELECT COUNT(*) INTO duplicate_count
        FROM mobile_helius_webhook_events
        WHERE network = 'devnet' AND signature = 'helius-sig-ok';
        IF duplicate_count != 1 THEN
          RAISE EXCEPTION 'Helius signature dedupe unexpectedly kept % rows', duplicate_count;
        END IF;

        INSERT INTO mobile_helius_webhook_events
          (id, network, source, payload_hash, signature, event_count, payload_json)
        VALUES
          ('helius-payload-first', 'devnet', 'helius_enhanced_webhook', '${"3".repeat(64)}', NULL, 1, '{"fallback":true}');

        WITH signature_existing AS (
            UPDATE mobile_helius_webhook_events
            SET event_count = GREATEST(2, event_count),
                payload_json = '{"signature":"helius-sig-late"}'
            WHERE network = 'devnet' AND signature = 'helius-sig-late'
            RETURNING 1
        ),
        hash_existing AS (
            UPDATE mobile_helius_webhook_events
            SET signature = COALESCE(signature, 'helius-sig-late'),
                event_count = GREATEST(2, event_count),
                payload_json = '{"signature":"helius-sig-late"}'
            WHERE network = 'devnet'
              AND payload_hash = '${"3".repeat(64)}'
              AND NOT EXISTS (SELECT 1 FROM signature_existing)
            RETURNING 1
        ),
        inserted AS (
            INSERT INTO mobile_helius_webhook_events
              (id, network, source, payload_hash, signature, event_count, payload_json)
            SELECT
              'helius-payload-late-signature', 'devnet', 'helius_enhanced_webhook', '${"3".repeat(64)}', 'helius-sig-late', 2, '{"signature":"helius-sig-late"}'
            WHERE NOT EXISTS (SELECT 1 FROM signature_existing)
              AND NOT EXISTS (SELECT 1 FROM hash_existing)
            ON CONFLICT (network, signature) WHERE signature IS NOT NULL DO UPDATE
            SET event_count = GREATEST(EXCLUDED.event_count, mobile_helius_webhook_events.event_count),
                payload_json = EXCLUDED.payload_json
            RETURNING 1
        )
        SELECT COUNT(*) INTO upsert_count
        FROM (
          SELECT 1 FROM signature_existing
          UNION ALL
          SELECT 1 FROM hash_existing
          UNION ALL
          SELECT 1 FROM inserted
          LIMIT 1
        ) upsert;
        IF upsert_count != 1 THEN
          RAISE EXCEPTION 'payload-hash-to-signature Helius upsert unexpectedly returned % rows', upsert_count;
        END IF;

        SELECT COUNT(*) INTO duplicate_count
        FROM mobile_helius_webhook_events
        WHERE network = 'devnet' AND payload_hash = '${"3".repeat(64)}';
        IF duplicate_count != 1 THEN
          RAISE EXCEPTION 'Helius payload-hash fallback dedupe unexpectedly kept % rows', duplicate_count;
        END IF;

        BEGIN
          INSERT INTO mobile_helius_webhook_events
            (id, network, source, payload_hash, signature, event_count, payload_json)
          VALUES
            ('helius-bad-count', 'devnet', 'helius_enhanced_webhook', '${"1".repeat(64)}', 'helius-sig-bad-count', 0, '{}');
          RAISE EXCEPTION 'zero-count Helius webhook insert unexpectedly succeeded';
        EXCEPTION WHEN check_violation THEN
          NULL;
        END;
      END
      $$;
    `,
  });
}

async function assertScoreContract({ database, port, socketDir }) {
  await psqlExec({
    database,
    port,
    socketDir,
    sql: `
      INSERT INTO mobile_seal_receipts
        (id, network, wallet, loom_asset, core_collection, session_hash, signature, utc_day, status)
      VALUES
        ('score-seal-0', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"0".repeat(64)}', 'score-seal-sig-0', 21000, 'finalized'),
        ('score-seal-1', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"1".repeat(64)}', 'score-seal-sig-1', 21001, 'finalized'),
        ('score-seal-2', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"2".repeat(64)}', 'score-seal-sig-2', 21002, 'finalized'),
        ('score-seal-3', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"3".repeat(64)}', 'score-seal-sig-3', 21003, 'finalized'),
        ('score-seal-4', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"4".repeat(64)}', 'score-seal-sig-4', 21004, 'finalized'),
        ('score-seal-5', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"5".repeat(64)}', 'score-seal-sig-5', 21005, 'finalized'),
        ('score-seal-6', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"6".repeat(64)}', 'score-seal-sig-6', 21006, 'finalized'),
        ('score-seal-confirmed', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"7".repeat(64)}', 'score-seal-sig-confirmed', 21007, 'confirmed'),
        ('score-seal-no-day', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"8".repeat(64)}', 'score-seal-sig-no-day', NULL, 'finalized'),
        ('score-seal-mismatch-day', 'devnet', 'wallet-score', 'loom-score', 'collection-ok', '${"9".repeat(64)}', 'score-seal-sig-mismatch-day', 21008, 'finalized');

      INSERT INTO mobile_verified_seal_receipts
        (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, status)
      VALUES
        ('score-verified-0', 'devnet', 'wallet-score', '${"0".repeat(64)}', '${"a".repeat(64)}', 'verifier-ok', 1, 21000, 'score-verified-sig-0', 'finalized'),
        ('score-verified-6', 'devnet', 'wallet-score', '${"6".repeat(64)}', '${"b".repeat(64)}', 'verifier-ok', 1, 21006, 'score-verified-sig-6', 'finalized'),
        ('score-verified-wrong-verifier', 'devnet', 'wallet-score', '${"5".repeat(64)}', '${"c".repeat(64)}', 'wrong-verifier', 1, 21005, 'score-verified-sig-wrong-verifier', 'finalized'),
        ('score-verified-confirmed', 'devnet', 'wallet-score', '${"4".repeat(64)}', '${"d".repeat(64)}', 'verifier-ok', 1, 21004, 'score-verified-sig-confirmed', 'confirmed'),
        ('score-verified-mismatch-day', 'devnet', 'wallet-score', '${"9".repeat(64)}', '${"e".repeat(64)}', 'verifier-ok', 1, 21009, 'score-verified-sig-mismatch-day', 'finalized');
    `,
  });

  const score = JSON.parse(
    await psqlCapture({
      database,
      port,
      socketDir,
      sql: `
        WITH sealed AS (
          SELECT DISTINCT utc_day
          FROM mobile_seal_receipts
          WHERE network = 'devnet'
            AND wallet = 'wallet-score'
            AND utc_day IS NOT NULL
            AND status = 'finalized'
        ),
        verified AS (
          SELECT DISTINCT verified.utc_day
          FROM mobile_verified_seal_receipts verified
          JOIN mobile_seal_receipts seal
            ON seal.network = verified.network
           AND seal.wallet = verified.wallet
           AND seal.session_hash = verified.session_hash
           AND seal.utc_day = verified.utc_day
          WHERE verified.network = 'devnet'
            AND verified.wallet = 'wallet-score'
            AND verified.verifier = 'verifier-ok'
            AND verified.protocol_version = 1
            AND verified.utc_day IS NOT NULL
            AND verified.status = 'finalized'
            AND seal.status = 'finalized'
        )
        SELECT json_build_object(
          'sealedDays', COALESCE((SELECT json_agg(utc_day ORDER BY utc_day) FROM sealed), '[]'::json),
          'verifiedDays', COALESCE((SELECT json_agg(utc_day ORDER BY utc_day) FROM verified), '[]'::json)
        )::text;
      `,
    }),
  );

  const expectedSealedDays = [21000, 21001, 21002, 21003, 21004, 21005, 21006, 21008];
  const expectedVerifiedDays = [21000, 21006];
  if (JSON.stringify(score.sealedDays) !== JSON.stringify(expectedSealedDays)) {
    throw new Error(`Score SQL sealedDays mismatch: ${JSON.stringify(score.sealedDays)}`);
  }
  if (JSON.stringify(score.verifiedDays) !== JSON.stringify(expectedVerifiedDays)) {
    throw new Error(`Score SQL verifiedDays mismatch: ${JSON.stringify(score.verifiedDays)}`);
  }

  return {
    formula: "score = unique_seal_days + (2 * verified_seal_days) + streak_bonus",
    score: score.sealedDays.length + (2 * score.verifiedDays.length) + 2,
    sealedDays: score.sealedDays,
    streakBonus: 2,
    verifiedDays: score.verifiedDays,
  };
}

function requireColumn(columns, table, name, type) {
  const column = columns.find((candidate) => candidate.table === table && candidate.name === name);
  if (column == null) {
    throw new Error(`${table}.${name} column is missing.`);
  }
  if (column.type !== type) {
    throw new Error(`${table}.${name} must be ${type}, got ${column.type}.`);
  }
}

function requireConstraint(constraints, name, type) {
  const constraint = constraints.find((candidate) => candidate.name === name);
  if (constraint == null) {
    throw new Error(`${name} constraint is missing.`);
  }
  if (constraint.type !== type) {
    throw new Error(`${name} constraint type must be ${type}, got ${constraint.type}.`);
  }
}

function requireIndex(indexes, name, unique) {
  const index = indexes.find((candidate) => candidate.name === name);
  if (index == null) {
    throw new Error(`${name} index is missing.`);
  }
  if (unique && !/\bUNIQUE\b/i.test(index.definition)) {
    throw new Error(`${name} must be unique.`);
  }
}

function pgArgs({ database, port, socketDir }) {
  return pgServerArgs({ port, socketDir }).concat(database);
}

function pgServerArgs({ port, socketDir }) {
  return ["-h", socketDir, "-p", port, "-U", "postgres"];
}

function psqlFile({ database, file, port, socketDir }) {
  return run("psql", pgArgs({ database, port, socketDir }).concat(["-v", "ON_ERROR_STOP=1", "-f", file]));
}

function psqlExec({ database, port, socketDir, sql }) {
  return run("psql", pgArgs({ database, port, socketDir }).concat(["-v", "ON_ERROR_STOP=1", "-c", sql]));
}

async function psqlCapture({ database, port, socketDir, sql }) {
  return (await run(
    "psql",
    pgArgs({ database, port, socketDir }).concat(["-X", "-q", "-tA", "-v", "ON_ERROR_STOP=1", "-c", sql]),
    { capture: true },
  )).trim();
}

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (!BOOLEAN_FLAGS.has(arg) && !VALUE_FLAGS.has(arg)) {
      if (!arg.startsWith("--")) {
        throw new Error(`Unexpected argument: ${arg}`);
      }
      throw new Error(`Unknown option: ${arg}`);
    }
    if (arg === "--keep") {
      args.keep = true;
      continue;
    }

    const value = argv[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`${arg} requires a value.`);
    }
    args[arg.slice(2)] = value;
    index += 1;
  }

  return args;
}

function requireBinaries(binaries) {
  for (const binary of binaries) {
    const result = spawnSync("command", ["-v", binary], {
      shell: true,
      stdio: "ignore",
    });
    if (result.status !== 0) {
      throw new Error(`${binary} is required for the verified receipt migration smoke test.`);
    }
  }
}

function requireFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`${path.relative(REPO_ROOT, filePath)} is missing.`);
  }
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    let stdout = "";
    let stderr = "";
    const child = spawn(command, args, {
      cwd: options.cwd ?? REPO_ROOT,
      stdio: options.capture === true ? ["ignore", "pipe", "pipe"] : ["ignore", "ignore", "pipe"],
    });
    child.stdout?.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr?.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0 || options.allowFailure === true) {
        resolve(stdout);
        return;
      }

      reject(new Error(`${command} exited with ${signal ?? code}: ${stderr.trim()}`));
    });
  });
}

function printUsage() {
  console.log(`Usage:
  node solana/scripts/sojourn9/smokeVerifiedSealMigration.mjs

Options:
  --keep  Keep the temporary Postgres data directory for manual inspection.

This starts a disposable local Postgres cluster, applies migrations 017, 019_mobile_verified_seal_receipts,
020_mobile_helius_webhook_events, 021_mobile_helius_webhook_signature_dedupe, and
022_credit_ledger_entries,
checks clean and partial-existing-table scenarios, verifies public-only columns,
constraints, indexes, signature dedupe, insert guards, and finalized-only score SQL,
then removes the temp cluster.`);
}
