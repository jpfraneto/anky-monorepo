#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SQLITE_DB="${SQLITE_DB:-$ROOT_DIR/data/anky.db}"
PG_URL="${DATABASE_URL:-postgres://postgres:postgres@127.0.0.1:5432/anky}"
TMP_DIR="${TMPDIR:-/tmp}/anky_sqlite_export"

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "ERROR: sqlite3 is required" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "ERROR: psql is required" >&2
  exit 1
fi

echo "=== SQLite → Postgres Migration ==="
echo "SQLite: $SQLITE_DB"
echo ""

# Get all real table names from SQLite (skip sqlite_sequence and other internal tables)
mapfile -t TABLES < <(sqlite3 "$SQLITE_DB" \
  "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;")

echo "Found ${#TABLES[@]} tables to migrate"
echo ""

# Tables with BLOB/binary columns that can't go through CSV
BINARY_TABLES="memory_embeddings user_memories"

# Export all tables to CSV
for table in "${TABLES[@]}"; do
  row_count=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM \"$table\";")
  if [ "$row_count" -eq 0 ]; then
    continue
  fi

  csv_path="$TMP_DIR/${table}.csv"

  # For tables with binary blob columns, skip the blob column in CSV export
  if echo "$BINARY_TABLES" | grep -qw "$table"; then
    # Get column names, replace embedding/blob columns with empty
    if [ "$table" = "memory_embeddings" ]; then
      sqlite3 "$SQLITE_DB" <<SQLEOF > "$csv_path"
.headers on
.mode csv
SELECT id, user_id, writing_session_id, source, content, '' as embedding, created_at FROM "$table";
SQLEOF
    elif [ "$table" = "user_memories" ]; then
      sqlite3 "$SQLITE_DB" <<SQLEOF > "$csv_path"
.headers on
.mode csv
SELECT id, user_id, writing_session_id, category, content, importance, occurrence_count, first_seen_at, last_seen_at, '' as embedding, created_at FROM "$table";
SQLEOF
    fi
  else
    sqlite3 "$SQLITE_DB" <<SQLEOF > "$csv_path"
.headers on
.mode csv
SELECT * FROM "$table";
SQLEOF
  fi
done

# Build a single psql script that:
# 1. Disables FK checks
# 2. Imports all tables
# 3. Re-enables FK checks
# 4. Updates sequences

PSQL_SCRIPT="$TMP_DIR/_import.sql"
cat > "$PSQL_SCRIPT" <<'HEADER'
SET session_replication_role = 'replica';
HEADER

SUCCEEDED=0
SKIPPED=0
FAILED_TABLES=""

for table in "${TABLES[@]}"; do
  row_count=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM \"$table\";")
  if [ "$row_count" -eq 0 ]; then
    echo "  SKIP  $table (empty)"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  csv_path="$TMP_DIR/${table}.csv"
  if [ ! -f "$csv_path" ]; then
    echo "  SKIP  $table (no CSV)"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  echo "\\echo 'Importing $table ($row_count rows)...'" >> "$PSQL_SCRIPT"
  echo "\\copy \"$table\" FROM '$csv_path' WITH (FORMAT csv, HEADER true, NULL '')" >> "$PSQL_SCRIPT"
  SUCCEEDED=$((SUCCEEDED + 1))
done

# Re-enable FK checks
echo "" >> "$PSQL_SCRIPT"
echo "SET session_replication_role = 'origin';" >> "$PSQL_SCRIPT"

# Update sequences for SERIAL/BIGSERIAL columns
for seq_table in agent_session_events cost_records interview_messages llm_training_runs notification_signups writing_checkpoints programming_classes; do
  echo "SELECT setval('${seq_table}_id_seq', COALESCE((SELECT MAX(id) FROM \"${seq_table}\"), 0) + 1, false);" >> "$PSQL_SCRIPT"
done

echo ""
echo "Running import ($SUCCEEDED tables)..."
echo ""

# Run the whole import in a single psql session so session_replication_role sticks
if psql "$PG_URL" -f "$PSQL_SCRIPT" 2>"$TMP_DIR/_errors.log"; then
  echo ""
  echo "Import completed successfully."
else
  echo ""
  echo "Some errors occurred. Check details below."
  cat "$TMP_DIR/_errors.log"
fi

# Verify row counts
echo ""
echo "=== Row Count Verification ==="
MISMATCH=0
for table in "${TABLES[@]}"; do
  sqlite_count=$(sqlite3 "$SQLITE_DB" "SELECT COUNT(*) FROM \"$table\";")
  pg_count=$(psql "$PG_URL" -t -c "SELECT COUNT(*) FROM \"$table\";" 2>/dev/null | tr -d ' ' || echo "0")
  if [ "$sqlite_count" != "$pg_count" ]; then
    echo "  MISMATCH  $table: SQLite=$sqlite_count Postgres=$pg_count"
    MISMATCH=$((MISMATCH + 1))
  fi
done

if [ "$MISMATCH" -eq 0 ]; then
  echo "  All row counts match!"
else
  echo "  $MISMATCH tables have mismatched counts"
fi

# Clean up
rm -rf "$TMP_DIR"

echo ""
echo "=== Summary ==="
echo "  Tables imported: $SUCCEEDED"
echo "  Tables skipped: $SKIPPED"
if [ "$MISMATCH" -gt 0 ]; then
  echo "  WARNING: $MISMATCH tables have count mismatches (binary blob columns are imported as empty)"
fi
echo ""
echo "Migration complete."
