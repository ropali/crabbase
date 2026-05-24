#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Create N test collections (SQLite tables) and register them in metadata tables.

Usage:
  scripts/create_test_collections.sh --count N [--prefix NAME] [--rows N] [--db PATH] [--reset]

Options:
  --count N       Number of collections to create (required)
  --prefix NAME   Table/collection prefix (default: test_collection)
  --rows N        Seed rows per collection table (default: 10)
  --db PATH       SQLite database path (default: app.db)
  --reset         Drop existing matching tables and recreate metadata
  --help          Show help

Examples:
  scripts/create_test_collections.sh --count 20
  scripts/create_test_collections.sh --count 5 --prefix perf --rows 100 --reset
USAGE
}

DB_PATH="app.db"
COUNT=""
PREFIX="test_collection"
ROWS=10
RESET=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --count)
      COUNT="${2:-}"
      shift 2
      ;;
    --prefix)
      PREFIX="${2:-}"
      shift 2
      ;;
    --rows)
      ROWS="${2:-}"
      shift 2
      ;;
    --db)
      DB_PATH="${2:-}"
      shift 2
      ;;
    --reset)
      RESET=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$COUNT" ]]; then
  echo "Error: --count is required" >&2
  usage
  exit 1
fi

if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 ]]; then
  echo "Error: --count must be a positive integer" >&2
  exit 1
fi

if ! [[ "$ROWS" =~ ^[0-9]+$ ]] || [[ "$ROWS" -lt 0 ]]; then
  echo "Error: --rows must be an integer >= 0" >&2
  exit 1
fi

if ! [[ "$PREFIX" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
  echo "Error: --prefix must match [A-Za-z_][A-Za-z0-9_]*" >&2
  exit 1
fi

if [[ ! -f "$DB_PATH" ]]; then
  echo "Error: database file not found: $DB_PATH" >&2
  exit 1
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "Error: sqlite3 command not found" >&2
  exit 1
fi

need_table() {
  local t="$1"
  local exists
  exists="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$t';")"
  if [[ "$exists" != "1" ]]; then
    echo "Error: required table '$t' not found in $DB_PATH" >&2
    exit 1
  fi
}

need_table "_collections"
collections_exists="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='collections';")"

created=0
seeded=0
skipped=0

for ((i = 1; i <= COUNT; i++)); do
  table_name="${PREFIX}_${i}"

  table_exists="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$table_name';")"

  if [[ "$table_exists" == "1" && "$RESET" -eq 0 ]]; then
    skipped=$((skipped + 1))
    continue
  fi

  if [[ "$RESET" -eq 1 ]]; then
    sqlite3 "$DB_PATH" <<SQL
DROP TABLE IF EXISTS "$table_name";
DELETE FROM _collections WHERE name = '$table_name';
SQL
    if [[ "$collections_exists" == "1" ]]; then
      sqlite3 "$DB_PATH" "DELETE FROM collections WHERE name = '$table_name';"
    fi
  fi

  sqlite3 "$DB_PATH" <<SQL
CREATE TABLE IF NOT EXISTS "$table_name" (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT,
  status TEXT,
  score INTEGER,
  is_active INTEGER,
  created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
  updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
);
CREATE INDEX IF NOT EXISTS idx_${table_name}_name ON "$table_name" (name);
CREATE INDEX IF NOT EXISTS idx_${table_name}_status ON "$table_name" (status);
SQL

  # Keep JSON in the same compact shape as serde_json::to_string output.
  fields_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true},{"name":"score","data_type":"Number","index":false},{"name":"is_active","data_type":"Bool","index":false}]'
  indexes_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true}]'

  if [[ "$(sqlite3 "$DB_PATH" "SELECT json_valid('$fields_json');")" != "1" ]]; then
    echo "Error: generated fields_json is not valid JSON" >&2
    exit 1
  fi

  sqlite3 "$DB_PATH" <<SQL
INSERT OR REPLACE INTO _collections (id, system, type, name, fields, indexes, options)
VALUES (
  lower(hex(randomblob(16))),
  0,
  'base',
  '$table_name',
  '$fields_json',
  '$indexes_json',
  '{}'
);
SQL

  if [[ "$collections_exists" == "1" ]]; then
    sqlite3 "$DB_PATH" <<SQL
INSERT OR REPLACE INTO collections (id, name, description)
VALUES (
  lower(hex(randomblob(16))),
  '$table_name',
  'Generated test collection $table_name'
);
SQL
  fi

  if [[ "$ROWS" -gt 0 ]]; then
    sqlite3 "$DB_PATH" <<SQL
WITH RECURSIVE seq(x) AS (
  SELECT 1
  UNION ALL
  SELECT x + 1 FROM seq WHERE x < $ROWS
)
INSERT INTO "$table_name" (name, status, score, is_active)
SELECT
  'name_' || x || '_' || lower(hex(randomblob(2))),
  CASE abs(random()) % 3 WHEN 0 THEN 'new' WHEN 1 THEN 'active' ELSE 'archived' END,
  abs(random()) % 1000,
  abs(random()) % 2
FROM seq;
SQL
    seeded=$((seeded + ROWS))
  fi

  created=$((created + 1))
done

echo "Created collections: $created"
echo "Seeded records: $seeded"
echo "Skipped existing tables: $skipped"
