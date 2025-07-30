#!/usr/bin/env bash
set -euo pipefail

DB_FILE="test.db"
DSN_NAME="test_sqlite"
DUMP_FILE="schema_dump.sql"
MIGRATIONS_DIR="migrations"

echo "[*] Resetting database: $DB_FILE"
rm -f "$DB_FILE"
sqlite3 "$DB_FILE" "VACUUM;" # create empty file

echo "[*] Verifying DSN exists: $DSN_NAME"
grep -q "^\[$DSN_NAME\]" ~/.odbc.ini || {
  echo "[❌] DSN '$DSN_NAME' not found in ~/.odbc.ini"
  exit 1
}

EXPECTED_COUNT=$(find "$MIGRATIONS_DIR" -type f -name '*.sql' | wc -l | tr -d '[:space:]')
echo "[*] Found $EXPECTED_COUNT migration files"

echo "[*] Running migration apply"
cargo run --quiet -- apply --conn "DSN=$DSN_NAME;"

echo "[*] Verifying migration status"
STATUS_OUT=$(cargo run --quiet -- status --conn "DSN=$DSN_NAME;")
echo "$STATUS_OUT"

ACTUAL_COUNT=$(grep -oP 'Applied:\s+\K\d+' <<<"$STATUS_OUT")

if [[ "$ACTUAL_COUNT" -ne "$EXPECTED_COUNT" ]]; then
  echo "[❌] Expected $EXPECTED_COUNT applied migrations, got $ACTUAL_COUNT"
  exit 1
fi

echo "[*] Dumping schema to: $DUMP_FILE"
{
  echo "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';"
  echo "SELECT sql FROM sqlite_master WHERE type='index' AND sql NOT NULL;"
} | isql -b "$DSN_NAME" >"$DUMP_FILE"

# ✅ Assert: All created (non-dropped) tables exist
echo "[*] Checking created tables"

# Detect dropped tables
DROPPED_TABLES=$(find "$MIGRATIONS_DIR" -type f -name '*.sql' -exec grep -hoP 'DROP TABLE\s+(IF\s+EXISTS\s+)?\K\w+' {} + | sort -u)

# Detect created tables
CREATED_TABLES=$(find "$MIGRATIONS_DIR" -type f -name '*.sql' \
  -exec grep -hoP 'CREATE TABLE\s+(IF\s+NOT\s+EXISTS\s+)?\K\w+' {} + | sort -u)

for tbl in $CREATED_TABLES; do
  if grep -q "^$tbl$" <<<"$DROPPED_TABLES"; then
    echo "[*] Skipping dropped table: $tbl"
    continue
  fi
  if ! grep -q -E "CREATE TABLE\s+(IF NOT EXISTS\s+)?$tbl" "$DUMP_FILE"; then
    echo "[❌] Missing table: $tbl"
    exit 1
  fi
done

# ✅ Assert: All created indexes found
echo "[*] Checking created indexes"
INDEXES=$(find "$MIGRATIONS_DIR" -type f -name '*.sql' -exec grep -hoP 'CREATE INDEX\s+(IF\s+NOT\s+EXISTS\s+)?\K\w+' {} + | sort -u)
for idx in $INDEXES; do
  if ! grep -q "$idx" "$DUMP_FILE"; then
    echo "[❌] Missing index: $idx"
    exit 1
  fi
done

echo "[✅] All assertions passed"
