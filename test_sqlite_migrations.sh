#!/usr/bin/env bash
set -euo pipefail

DB_FILE="test.db"
DSN_NAME="test_sqlite"
DUMP_FILE="schema_dump.sql"

echo "[*] Resetting database: $DB_FILE"
rm -f "$DB_FILE"
sqlite3 "$DB_FILE" "VACUUM;" # create empty file

echo "[*] Verifying DSN exists: $DSN_NAME"
grep -q "^\[$DSN_NAME\]" ~/.odbc.ini || {
  echo "[!] DSN '$DSN_NAME' not found in ~/.odbc.ini"
  exit 1
}

echo "[*] Running migration apply"
cargo run --quiet -- apply --conn "DSN=$DSN_NAME;"

echo "[*] Verifying migration status"
STATUS_OUT=$(cargo run --quiet -- status --conn "DSN=$DSN_NAME;")

echo "$STATUS_OUT"

# ✅ Assert: All 3 migrations applied
if ! grep -q "Applied: 3" <<<"$STATUS_OUT"; then
  echo "[❌] Expected 3 applied migrations"
  exit 1
fi

echo "[*] Dumping schema"
{
  echo "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';"
  echo "SELECT sql FROM sqlite_master WHERE type='index' AND sql NOT NULL;"
} | isql -b "$DSN_NAME" >"$DUMP_FILE"

echo "[*] Schema written to: $DUMP_FILE"

# ✅ Assert: Users table present
if ! grep -q "CREATE TABLE users" "$DUMP_FILE"; then
  echo "[❌] 'users' table not found in schema dump"
  exit 1
fi

# ✅ Assert: Indexes created
for idx in idx_users_email idx_users_name idx_users_created_at; do
  if ! grep -q "$idx" "$DUMP_FILE"; then
    echo "[❌] Missing index: $idx"
    exit 1
  fi
done

echo "[✅] All assertions passed"
