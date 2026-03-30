# Architecture Changes

## What Changed

Phase 2 replaces the SQLite/r2d2 backend with Postgres via `sqlx::PgPool`, adds `ANKY_MODE` run modes (`full`, `web`, `worker`), introduces configurable Claude model tiering for reflections versus follow-up conversation, and adds deployment support files for Railway plus a dedicated Poiesis worker unit.

The database migration now lives in `migrations/001_init.sql` and mirrors the full live SQLite schema, including the extra production tables discovered from `sqlite3 data/anky.db ".tables"` and `sqlite3 data/anky.db ".schema"` such as `collections`, `video_projects`, `memory_*`, `x_*`, `facilitator_*`, `cuentacuentos_*`, `training_*`, and `social_*`.

## Postgres Migration

`AppState.db` is now a `sqlx::PgPool`. Startup uses `db::create_pool(&config.database_url).await?`, which:

1. creates a Postgres pool with `max_connections(10)` and `min_connections(2)`
2. runs the SQL migrations from `./migrations`

The app’s query layer was ported off `rusqlite`/`r2d2`. The old `state.db.get()` pattern is gone from `src/`. Existing query logic now goes through the Postgres-backed compatibility layer in `src/db/mod.rs`, so handlers and helpers still use the same read/write flow while executing against Postgres.

### Database Access Pattern

Use a Postgres-backed connection handle via:

```rust
let db = crate::db::conn(&state.db)?;
```

Then call the query helpers or direct SQL methods on that handle:

```rust
let anky = queries::get_anky_by_id(&db, anky_id)?;
db.execute(
    "UPDATE ankys SET status = ?2 WHERE id = ?1",
    crate::params![anky_id, "complete"],
)?;
```

That keeps the Phase 1 discipline intact: get a handle, do the synchronous DB work for that step, and do not carry DB work across unrelated async boundaries.

### Migration / Data Move

One-time data export/import is documented in `scripts/migrate_sqlite_to_postgres.sh`. It enumerates every table in the SQLite database, exports each one as CSV, and imports into Postgres with `psql \\copy`.

## Run Modes

`src/config.rs` now parses:

- `ANKY_MODE=full` (default): web server + GPU worker
- `ANKY_MODE=web`: Axum only
- `ANKY_MODE=worker`: Redis GPU worker only

Worker mode does not start Axum. It recovers Redis processing jobs, starts the GPU worker loop, and then stays alive until `ctrl-c` / systemd shutdown. This is the intended Poiesis topology once Railway owns the web tier and Postgres.

## LLM Tiering

`src/config.rs` now loads:

- `ANKY_REFLECTION_MODEL` (default `claude-opus-4-20250514`)
- `ANKY_CONVERSATION_MODEL` (default `claude-sonnet-4-20250514`)

The reflection stream and the fallback reflection generator now use `reflection_model` first and fall back to `conversation_model` before dropping to lower-tier/cloud fallbacks. The post-reflection chat route now uses `conversation_model` first and falls back to Haiku if needed.

## Deployment Topology

Added:

- `Dockerfile`
- `railway.toml`
- `deploy/anky-worker.service`
- `/api/health` route alias

The Docker image copies the runtime assets the app actually needs: `templates/`, `static/`, `prompts/`, `agent-skills/`, `flux/`, and `migrations/`. The worker service sets `ANKY_MODE=worker` so the Poiesis systemd unit only runs the Redis-backed GPU worker.

## Breaking Changes / Watch-Outs

- `DATABASE_URL` is now required for normal Postgres operation.
- `rusqlite`, `r2d2`, and `r2d2_sqlite` were removed from the dependency graph.
- Any external tooling that assumed a local SQLite file as the source of truth must now target Postgres.
- The query layer still preserves a string-heavy schema surface for compatibility; the storage engine changed, but the app-side semantics stayed intentionally close to the SQLite version to reduce behavioral drift.

## Verification

Run:

```bash
cargo build --release
cargo clippy 2>&1 | head -100
rg "rusqlite" src/
rg "r2d2" src/
rg "state\\.db\\.get\\(\\)" src/
```

Expected:

- `cargo build --release` succeeds
- `cargo clippy` may still report existing repo-wide dead-code warnings, but no migration regressions should remain from the touched areas
- all three `rg` commands return no matches
