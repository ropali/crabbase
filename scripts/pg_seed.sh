#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Seed PostgreSQL table with generated test data based on column types.

Usage:
  scripts/pg_seed.sh --table TABLE [--rows N] [--db-url URL] [--truncate]

Options:
  --table TABLE    Target table name (required)
  --rows N         Number of rows to insert (default: 10)
  --db-url URL     PostgreSQL database URL (default: env DATABASE_URL or postgres://postgres:postgres@localhost:5432/crabbase)
  --truncate       Delete all existing rows in table before seeding
  --help           Show this help

Examples:
  scripts/pg_seed.sh --table test_collection_1 --rows 25
  scripts/pg_seed.sh --table _logs --rows 100 --truncate
USAGE
}

DB_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/crabbase}"
TABLE=""
ROWS=10
TRUNCATE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db-url)
      DB_URL="${2:-}"
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

if ! command -v psql >/dev/null 2>&1; then
  echo "Error: psql command not found. Please install postgresql-client." >&2
  exit 1
fi

# Verify connection and table existence
TABLE_EXISTS=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$TABLE' AND table_schema = current_schema();")
if [[ "$TABLE_EXISTS" != "1" ]]; then
  echo "Error: table '$TABLE' does not exist in the database." >&2
  exit 1
fi

# Fetch columns and data types
schema_rows=$(psql "$DB_URL" -t -A -F "|" -c "
  SELECT column_name, data_type
  FROM information_schema.columns
  WHERE table_name = '$TABLE' AND table_schema = current_schema();
")

if [[ -z "$schema_rows" ]]; then
  echo "Error: failed to read schema for table '$TABLE'" >&2
  exit 1
fi

columns=()
exprs=()

while IFS='|' read -r col_name col_type; do
  if ! [[ "$col_name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    continue
  fi

  # Skip primary keys that are automatically generated (UUID or serial/integer)
  if [[ "$col_name" == "id" ]]; then
    if [[ "$col_type" == "uuid" ]] || [[ "$col_type" == *"int"* ]] || [[ "$col_type" == "bigint" ]]; then
      continue
    fi
  fi

  expr=""
  case "$col_name" in
    id)
      expr="'r' || substring(md5(random()::text) from 1 for 14)"
      ;;
    created|updated|applied_at)
      expr="now()"
      ;;
    email)
      expr="'user_' || x || '_' || floor(random() * 100000) || '@example.test'"
      ;;
    token_key)
      expr="md5(random()::text)"
      ;;
    password_hash)
      expr="'test_hash_' || substring(md5(random()::text) from 1 for 12)"
      ;;
    verified|email_visible|system|is_active)
      expr="random() > 0.5"
      ;;
    level)
      expr="floor(random() * 6)::integer"
      ;;
    data|fields|indexes|options|value)
      expr="'{}'::jsonb"
      ;;
    *)
      if [[ "$col_type" == "uuid" ]]; then
        expr="gen_random_uuid()"
      elif [[ "$col_type" == *"int"* ]] || [[ "$col_type" == "bigint" ]]; then
        expr="floor(random() * 1000)::bigint"
      elif [[ "$col_type" == "numeric" ]] || [[ "$col_type" == "double precision" ]] || [[ "$col_type" == "real" ]]; then
        expr="random() * 1000"
      elif [[ "$col_type" == "boolean" ]]; then
        expr="random() > 0.5"
      elif [[ "$col_type" == "jsonb" ]] || [[ "$col_type" == "json" ]]; then
        expr="'{}'::jsonb"
      elif [[ "$col_type" == *"timestamp"* ]] || [[ "$col_type" == "date" ]] || [[ "$col_type" == *"time"* ]]; then
        expr="now()"
      else
        expr="'$col_name' || '_' || x || '_' || substring(md5(random()::text) from 1 for 6)"
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

cols_csv=$(IFS=,; echo "${columns[*]}")
exprs_csv=$(IFS=,; echo "${exprs[*]}")

if [[ "$TRUNCATE" -eq 1 ]]; then
  psql "$DB_URL" -c "TRUNCATE TABLE \"$TABLE\" CASCADE;" >/dev/null
  before_count=0
else
  before_count=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM \"$TABLE\";")
fi

psql "$DB_URL" -c "
INSERT INTO \"$TABLE\" ($cols_csv)
SELECT $exprs_csv FROM generate_series(1, $ROWS) as x;
" >/dev/null

total=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM \"$TABLE\";")
inserted=$((total - before_count))

echo "Inserted rows: $inserted"
echo "Total rows in $TABLE: $total"
