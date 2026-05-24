#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Seed SQLite table with generated test data.

Usage:
  scripts/sqlite_seed.sh --table TABLE [--rows N] [--db PATH] [--truncate]

Options:
  --table TABLE    Target table name (required)
  --rows N         Number of rows to insert (default: 10)
  --db PATH        SQLite database path (default: app.db)
  --truncate       Delete all existing rows in table before seeding
  --help           Show this help

Examples:
  scripts/sqlite_seed.sh --table users --rows 25
  scripts/sqlite_seed.sh --db app.db --table _logs --rows 100 --truncate
USAGE
}

DB_PATH="app.db"
TABLE=""
ROWS=10
TRUNCATE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db)
      DB_PATH="${2:-}"
      shift 2
      ;;
    --table)
      TABLE="${2:-}"
      shift 2
      ;;
    --rows)
      ROWS="${2:-}"
      shift 2
      ;;
    --truncate)
      TRUNCATE=1
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

if [[ -z "$TABLE" ]]; then
  echo "Error: --table is required" >&2
  usage
  exit 1
fi

if ! [[ "$ROWS" =~ ^[0-9]+$ ]] || [[ "$ROWS" -lt 1 ]]; then
  echo "Error: --rows must be a positive integer" >&2
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

TABLE_EXISTS="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$TABLE';")"
if [[ "$TABLE_EXISTS" != "1" ]]; then
  echo "Error: table '$TABLE' does not exist in $DB_PATH" >&2
  exit 1
fi

schema_rows="$(sqlite3 -separator '|' "$DB_PATH" "PRAGMA table_info(\"$TABLE\");")"
if [[ -z "$schema_rows" ]]; then
  echo "Error: failed to read schema for table '$TABLE'" >&2
  exit 1
fi

columns=()
exprs=()

while IFS='|' read -r cid col_name col_type col_notnull col_default col_pk; do
  # Guard against malformed schema rows or wrapped output lines.
  if ! [[ "$cid" =~ ^[0-9]+$ ]]; then
    continue
  fi

  if ! [[ "$col_name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    continue
  fi

  # Skip primary key columns only when they are integer autoincrement style.
  if [[ "$col_pk" == "1" ]] && [[ "${col_type^^}" == *"INT"* ]]; then
    continue
  fi

  utype="${col_type^^}"

  expr=""
  case "$col_name" in
    id)
      expr="'r' || lower(hex(randomblob(7)))"
      ;;
    created|updated|applied_at)
      expr="strftime('%Y-%m-%d %H:%M:%fZ')"
      ;;
    email)
      expr="'user' || x || '_' || abs(random()) || '@example.test'"
      ;;
    token_key)
      expr="lower(hex(randomblob(16)))"
      ;;
    password_hash)
      expr="'test_hash_' || lower(hex(randomblob(12)))"
      ;;
    verified|email_visible|system)
      expr="abs(random()) % 2"
      ;;
    level)
      expr="abs(random()) % 6"
      ;;
    data|fields|indexes|options|value)
      expr="'{}'"
      ;;
    *)
      if [[ "$utype" == *"INT"* ]]; then
        expr="abs(random()) % 1000"
      elif [[ "$utype" == *"REAL"* ]] || [[ "$utype" == *"FLOA"* ]] || [[ "$utype" == *"DOUB"* ]]; then
        expr="(abs(random()) / 9223372036854775808.0) * 1000"
      elif [[ "$utype" == *"BLOB"* ]]; then
        expr="randomblob(12)"
      else
        expr="'$col_name' || '_' || x || '_' || lower(hex(randomblob(3)))"
      fi
      ;;
  esac

  columns+=("\"$col_name\"")
  exprs+=("$expr")
done <<< "$schema_rows"

if [[ "${#columns[@]}" -eq 0 ]]; then
  echo "Error: no writable columns resolved for table '$TABLE'" >&2
  exit 1
fi

cols_csv="$(IFS=,; echo "${columns[*]}")"
exprs_csv="$(IFS=,; echo "${exprs[*]}")"

if [[ "$TRUNCATE" -eq 1 ]]; then
  sqlite3 "$DB_PATH" "DELETE FROM \"$TABLE\";"
  before_count=0
else
  before_count="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM \"$TABLE\";")"
fi

sqlite3 "$DB_PATH" <<SQL
WITH RECURSIVE seq(x) AS (
  SELECT 1
  UNION ALL
  SELECT x + 1 FROM seq WHERE x < $ROWS
)
INSERT INTO "$TABLE" ($cols_csv)
SELECT $exprs_csv FROM seq;
SQL

total="$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM \"$TABLE\";")"
inserted=$((total - before_count))

echo "Inserted rows: $inserted"
echo "Total rows in $TABLE: $total"
