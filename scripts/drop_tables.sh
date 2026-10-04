#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Drop all tables and reset the Crabbase database schema.

Usage:
  scripts/drop_tables.sh [OPTIONS]

Options:
  --db-url URL           PostgreSQL connection URL (overrides crabbase.toml and DATABASE_URL)
  --config FILE          Path to config file (default: crabbase.toml)
  --schema NAME          Specific schema to drop/reset (default: public and schema from config)
  --keep-test-schemas    Do not drop orphaned test schemas (rec_api_*, rules_*, etc.)
  -f, --force            Skip confirmation prompt
  --dry-run              Show what would be dropped without executing
  -h, --help             Show this help

Examples:
  scripts/drop_tables.sh
  scripts/drop_tables.sh --force
  scripts/drop_tables.sh --db-url "postgres://postgres:postgres@localhost:5432/crabbase"
  make db-drop
USAGE
}

CONFIG_FILE="crabbase.toml"
EXPLICIT_DB_URL=""
EXPLICIT_SCHEMA=""
CLEAN_TEST_SCHEMAS=1
FORCE=0
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db-url)
      EXPLICIT_DB_URL="${2:-}"
      shift 2
      ;;
    --config)
      CONFIG_FILE="${2:-}"
      shift 2
      ;;
    --schema)
      EXPLICIT_SCHEMA="${2:-}"
      shift 2
      ;;
    --keep-test-schemas)
      CLEAN_TEST_SCHEMAS=0
      shift
      ;;
    -f|--force)
      FORCE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
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

DB_URL=""
CONFIG_SCHEMA="public"

# 1. Resolve DB URL and schema
if [[ -n "$EXPLICIT_DB_URL" ]]; then
  DB_URL="$EXPLICIT_DB_URL"
elif [[ -n "${DATABASE_URL:-}" ]]; then
  DB_URL="$DATABASE_URL"
fi

if [[ -z "$DB_URL" ]] && [[ -f "$CONFIG_FILE" ]]; then
  parsed=$(python3 -c '
import os, sys, urllib.parse
try:
    import tomllib
    with open(sys.argv[1], "rb") as f:
        cfg = tomllib.load(f)
    db = cfg.get("database", {})
    host = db.get("host", "localhost")
    port = db.get("port", 5432)
    user = db.get("user", "postgres")
    pwd = str(db.get("password", "postgres"))
    if pwd.startswith("$"):
        pwd = os.environ.get(pwd[1:], "")
    dbname = db.get("database", "crabbase")
    schema = db.get("schema", "public")
    user_enc = urllib.parse.quote_plus(str(user))
    pwd_enc = urllib.parse.quote_plus(str(pwd))
    print(f"postgres://{user_enc}:{pwd_enc}@{host}:{port}/{dbname}|{schema}")
except Exception:
    sys.exit(1)
' "$CONFIG_FILE" 2>/dev/null || true)

  if [[ -n "$parsed" ]]; then
    DB_URL="${parsed%|*}"
    CONFIG_SCHEMA="${parsed#*|}"
  fi
fi

if [[ -z "$DB_URL" ]]; then
  DB_URL="postgres://postgres:postgres@localhost:5432/crabbase"
fi

TARGET_SCHEMA="${EXPLICIT_SCHEMA:-$CONFIG_SCHEMA}"

# Mask password for display
MASKED_URL=$(echo "$DB_URL" | sed -E 's|(:[^:/@]+)@|:****@|')

# Helper to run psql command
run_sql() {
  local sql="$1"
  if command -v psql >/dev/null 2>&1; then
    psql "$DB_URL" -t -A -c "$sql"
  elif command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' | grep -q '^crabbase-postgres$'; then
    docker exec -i crabbase-postgres psql -U postgres -d crabbase -t -A -c "$sql"
  else
    echo "Error: Neither 'psql' nor running docker container 'crabbase-postgres' was found." >&2
    exit 1
  fi
}

echo "🦀 Crabbase Database Reset"
echo "Target Database: $MASKED_URL"
echo "Target Schema(s): public${TARGET_SCHEMA:+", $TARGET_SCHEMA"}"

# Verify connection
if ! run_sql "SELECT 1;" >/dev/null 2>&1; then
  echo "Error: Failed to connect to database at $MASKED_URL" >&2
  exit 1
fi

# Count existing tables
TABLES_BEFORE=$(run_sql "
  SELECT count(*) FROM information_schema.tables
  WHERE table_schema IN ('public', '$TARGET_SCHEMA')
    AND table_type = 'BASE TABLE';
")

TEST_SCHEMAS_BEFORE=0
if [[ "$CLEAN_TEST_SCHEMAS" -eq 1 ]]; then
  TEST_SCHEMAS_BEFORE=$(run_sql "
    SELECT count(*) FROM information_schema.schemata
    WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'public', '$TARGET_SCHEMA')
      AND (schema_name LIKE 'rec_api_%' OR schema_name LIKE 'rules_%' OR schema_name LIKE 'settings_%' OR schema_name LIKE 'validation_%' OR schema_name LIKE 'test_%' OR schema_name LIKE 'sanitize_%');
  ")
fi

echo "Found $TABLES_BEFORE table(s) in schema."
if [[ "$CLEAN_TEST_SCHEMAS" -eq 1 ]] && [[ "$TEST_SCHEMAS_BEFORE" -gt 0 ]]; then
  echo "Found $TEST_SCHEMAS_BEFORE leftover test schema(s)."
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "[DRY RUN] Would drop all tables and recreate schema 'public'${TARGET_SCHEMA:+" and '$TARGET_SCHEMA'"}"
  if [[ "$CLEAN_TEST_SCHEMAS" -eq 1 ]] && [[ "$TEST_SCHEMAS_BEFORE" -gt 0 ]]; then
    echo "[DRY RUN] Would drop $TEST_SCHEMAS_BEFORE leftover test schemas."
  fi
  exit 0
fi

# Confirmation prompt if interactive and not forced
if [[ "$FORCE" -ne 1 ]] && [[ -t 0 ]]; then
  read -r -p "Are you sure you want to drop all tables? [y/N] " response
  case "$response" in
    [yY][eE][sS]|[yY])
      ;;
    *)
      echo "Aborted."
      exit 0
      ;;
  esac
fi

# 1. Reset public schema
run_sql "DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public; GRANT ALL ON SCHEMA public TO CURRENT_USER; GRANT ALL ON SCHEMA public TO public;" >/dev/null

# 2. Reset target schema if different from public
if [[ -n "$TARGET_SCHEMA" ]] && [[ "$TARGET_SCHEMA" != "public" ]]; then
  run_sql "DROP SCHEMA IF EXISTS \"$TARGET_SCHEMA\" CASCADE; CREATE SCHEMA \"$TARGET_SCHEMA\"; GRANT ALL ON SCHEMA \"$TARGET_SCHEMA\" TO CURRENT_USER;" >/dev/null
fi

# 3. Clean up leftover test schemas if requested (run without wrapping in a transaction)
if [[ "$CLEAN_TEST_SCHEMAS" -eq 1 ]] && [[ "$TEST_SCHEMAS_BEFORE" -gt 0 ]]; then
  # Fetch test schema names and drop each individually to avoid exceeding max_locks_per_transaction
  test_schemas=$(run_sql "
    SELECT schema_name FROM information_schema.schemata
    WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'public', '$TARGET_SCHEMA')
      AND (schema_name LIKE 'rec_api_%' OR schema_name LIKE 'rules_%' OR schema_name LIKE 'settings_%' OR schema_name LIKE 'validation_%' OR schema_name LIKE 'test_%' OR schema_name LIKE 'sanitize_%');
  ")

  if [[ -n "$test_schemas" ]]; then
    while IFS= read -r sname; do
      [[ -z "$sname" ]] && continue
      run_sql "DROP SCHEMA IF EXISTS \"$sname\" CASCADE;" >/dev/null 2>&1 || true
    done <<< "$test_schemas"
  fi
fi

TABLES_AFTER=$(run_sql "
  SELECT count(*) FROM information_schema.tables
  WHERE table_schema IN ('public', '$TARGET_SCHEMA')
    AND table_type = 'BASE TABLE';
")

echo "✓ Successfully dropped all tables in schema: public${TARGET_SCHEMA:+", $TARGET_SCHEMA"}"
if [[ "$CLEAN_TEST_SCHEMAS" -eq 1 ]] && [[ "$TEST_SCHEMAS_BEFORE" -gt 0 ]]; then
  echo "✓ Successfully cleaned up $TEST_SCHEMAS_BEFORE leftover test schema(s)"
fi
echo "✓ Remaining tables: $TABLES_AFTER"
echo "✨ Database is clean and ready for fresh migrations."
