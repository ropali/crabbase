#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Create N test collections (PostgreSQL tables) and register them in metadata tables.

Usage:
  scripts/create_test_collections.sh --count N [--prefix NAME] [--rows N] [--db-url URL] [--reset]

Options:
  --count N       Number of collections to create (required)
  --prefix NAME   Table/collection prefix (default: test_collection)
  --rows N        Seed rows per collection table (default: 10)
  --db-url URL    PostgreSQL database URL (default: env DATABASE_URL or postgres://postgres:postgres@localhost:5432/crabbase)
  --reset         Drop existing matching tables and recreate metadata
  --help          Show help

Examples:
  scripts/create_test_collections.sh --count 20
  scripts/create_test_collections.sh --count 5 --prefix perf --rows 100 --reset
USAGE
}

DB_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/crabbase}"
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
    --db-url)
      DB_URL="${2:-}"
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

if ! command -v psql >/dev/null 2>&1; then
  echo "Error: psql command not found. Please install postgresql-client." >&2
  exit 1
fi

need_table() {
  local t="$1"
  local exists
  exists=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$t' AND table_schema = current_schema();")
  if [[ "$exists" != "1" ]]; then
    echo "Error: required table '$t' not found in database" >&2
    exit 1
  fi
}

need_table "_collections"

created=0
seeded=0
skipped=0

for ((i = 1; i <= COUNT; i++)); do
  table_name="${PREFIX}_${i}"

  table_exists=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$table_name' AND table_schema = current_schema();")

  if [[ "$table_exists" == "1" && "$RESET" -eq 0 ]]; then
    skipped=$((skipped + 1))
    continue
  fi

  if [[ "$RESET" -eq 1 ]]; then
    psql "$DB_URL" -c "
      DROP TABLE IF EXISTS \"$table_name\" CASCADE;
      DELETE FROM _collections WHERE name = '$table_name';
    " >/dev/null
  fi

  psql "$DB_URL" -c "
  CREATE TABLE IF NOT EXISTS \"$table_name\" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT,
    status TEXT,
    score BIGINT,
    is_active BOOLEAN,
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now()
  );
  CREATE INDEX IF NOT EXISTS idx_${table_name}_name ON \"$table_name\" (name);
  CREATE INDEX IF NOT EXISTS idx_${table_name}_status ON \"$table_name\" (status);
  " >/dev/null

  # Keep JSON in the same shape as schema structures.
  fields_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true},{"name":"score","data_type":"Number","index":false},{"name":"is_active","data_type":"Bool","index":false}]'
  indexes_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true}]'

  psql "$DB_URL" -c "
  INSERT INTO _collections (id, system, type, name, fields, indexes, options)
  VALUES (
    'r' || substring(md5(random()::text) from 1 for 14),
    0,
    'base',
    '$table_name',
    '$fields_json'::jsonb,
    '$indexes_json'::jsonb,
    '{}'::jsonb
  ) ON CONFLICT (name) DO UPDATE SET fields = EXCLUDED.fields, indexes = EXCLUDED.indexes;
  " >/dev/null

  if [[ "$ROWS" -gt 0 ]]; then
    psql "$DB_URL" -c "
    INSERT INTO \"$table_name\" (name, status, score, is_active)
    SELECT
      'name_' || x || '_' || substring(md5(random()::text) from 1 for 4),
      (ARRAY['new', 'active', 'archived'])[floor(random() * 3 + 1)],
      floor(random() * 1000)::bigint,
      random() > 0.5
    FROM generate_series(1, $ROWS) as x;
    " >/dev/null
    seeded=$((seeded + ROWS))
  fi

  created=$((created + 1))
done

echo "Created collections: $created"
echo "Seeded records: $seeded"
echo "Skipped existing tables: $skipped"
