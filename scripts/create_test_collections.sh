#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Create N test collections (PostgreSQL tables) and register them in metadata tables.
Supports generating One-to-One (category_id) and One-to-Many (tag_ids) relation test data.

Usage:
  scripts/create_test_collections.sh --count N [--prefix NAME] [--rows N] [--db-url URL] [--no-relations] [--reset]

Options:
  --count N         Number of collections to create (required)
  --prefix NAME     Table/collection prefix (default: test_collection)
  --rows N          Seed rows per collection table (default: 10)
  --db-url URL      PostgreSQL database URL (default: env DATABASE_URL or postgres://postgres:postgres@localhost:5432/crabbase)
  --no-relations    Do not create relation fields (create plain tables only)
  --reset           Drop existing matching tables and recreate metadata
  --help            Show help

Examples:
  scripts/create_test_collections.sh --count 20
  scripts/create_test_collections.sh --count 5 --prefix perf --rows 100 --reset
  scripts/create_test_collections.sh --count 3 --no-relations --rows 50
USAGE
}

DB_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/crabbase}"
COUNT=""
PREFIX="test_collection"
ROWS=10
RESET=0
WITH_RELATIONS=1

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
    --no-relations)
      WITH_RELATIONS=0
      shift
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

cat_table="${PREFIX}_categories"
tags_table="${PREFIX}_tags"

# 1. Handle One-to-One and One-to-Many target collections
if [[ "$WITH_RELATIONS" -eq 1 ]]; then
  # Reset relation tables if requested
  if [[ "$RESET" -eq 1 ]]; then
    # Drop referencing tables first if reset
    for ((i = 1; i <= COUNT; i++)); do
      psql "$DB_URL" -c "
        DROP TABLE IF EXISTS \"${PREFIX}_${i}\" CASCADE;
        DELETE FROM _collections WHERE name = '${PREFIX}_${i}';
      " >/dev/null 2>&1 || true
    done
    psql "$DB_URL" -c "
      DROP TABLE IF EXISTS \"$cat_table\" CASCADE;
      DELETE FROM _collections WHERE name = '$cat_table';
      DROP TABLE IF EXISTS \"$tags_table\" CASCADE;
      DELETE FROM _collections WHERE name = '$tags_table';
    " >/dev/null 2>&1 || true
  fi

  # 1A. Create Categories collection (target for One-to-One)
  cat_exists=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$cat_table' AND table_schema = current_schema();")
  if [[ "$cat_exists" != "1" ]]; then
    psql "$DB_URL" -c "
    CREATE TABLE IF NOT EXISTS \"$cat_table\" (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      name TEXT NOT NULL,
      slug TEXT NOT NULL,
      description TEXT,
      created TIMESTAMPTZ NOT NULL DEFAULT now(),
      updated TIMESTAMPTZ NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS idx_${cat_table}_name ON \"$cat_table\" (name);
    CREATE INDEX IF NOT EXISTS idx_${cat_table}_slug ON \"$cat_table\" (slug);
    " >/dev/null

    cat_fields='[{"name":"name","data_type":"PlainText","index":true,"required":true},{"name":"slug","data_type":"PlainText","index":true,"required":true},{"name":"description","data_type":"PlainText","index":false}]'
    cat_indexes='[{"name":"name","data_type":"PlainText","index":true},{"name":"slug","data_type":"PlainText","index":true}]'

    psql "$DB_URL" -c "
    INSERT INTO _collections (id, system, type, name, fields, indexes, options, list_rule, view_rule)
    VALUES (
      'r' || substring(md5(random()::text) from 1 for 14),
      0,
      'base',
      '$cat_table',
      '$cat_fields'::jsonb,
      '$cat_indexes'::jsonb,
      '{}'::jsonb,
      '',
      ''
    ) ON CONFLICT (name) DO UPDATE SET fields = EXCLUDED.fields, indexes = EXCLUDED.indexes;

    INSERT INTO \"$cat_table\" (name, slug, description) VALUES
      ('Technology', 'tech', 'Software engineering, databases, and system architecture'),
      ('Design', 'design', 'UI/UX design, visual systems, and user interaction'),
      ('DevOps', 'devops', 'CI/CD pipelines, container orchestration, and cloud infrastructure'),
      ('Architecture', 'architecture', 'High-level software patterns and distributed design'),
      ('Tutorials', 'tutorials', 'Guides, walkthroughs, and educational programming material'),
      ('General', 'general', 'Community discussions, updates, and miscellaneous topics');
    " >/dev/null

    created=$((created + 1))
    seeded=$((seeded + 6))
  fi

  # 1B. Create Tags collection (target for One-to-Many)
  tags_exists=$(psql "$DB_URL" -t -A -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$tags_table' AND table_schema = current_schema();")
  if [[ "$tags_exists" != "1" ]]; then
    psql "$DB_URL" -c "
    CREATE TABLE IF NOT EXISTS \"$tags_table\" (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      name TEXT NOT NULL,
      color TEXT,
      created TIMESTAMPTZ NOT NULL DEFAULT now(),
      updated TIMESTAMPTZ NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS idx_${tags_table}_name ON \"$tags_table\" (name);
    " >/dev/null

    tags_fields='[{"name":"name","data_type":"PlainText","index":true,"required":true},{"name":"color","data_type":"PlainText","index":false}]'
    tags_indexes='[{"name":"name","data_type":"PlainText","index":true}]'

    psql "$DB_URL" -c "
    INSERT INTO _collections (id, system, type, name, fields, indexes, options, list_rule, view_rule)
    VALUES (
      'r' || substring(md5(random()::text) from 1 for 14),
      0,
      'base',
      '$tags_table',
      '$tags_fields'::jsonb,
      '$tags_indexes'::jsonb,
      '{}'::jsonb,
      '',
      ''
    ) ON CONFLICT (name) DO UPDATE SET fields = EXCLUDED.fields, indexes = EXCLUDED.indexes;

    INSERT INTO \"$tags_table\" (name, color) VALUES
      ('Rust', '#dea584'),
      ('WebAssembly', '#654ff0'),
      ('PostgreSQL', '#336791'),
      ('Docker', '#2496ed'),
      ('API', '#009688'),
      ('Security', '#e91e63'),
      ('Frontend', '#ff9800'),
      ('Backend', '#4caf50'),
      ('Performance', '#9c27b0'),
      ('Testing', '#607d8b');
    " >/dev/null

    created=$((created + 1))
    seeded=$((seeded + 10))
  fi
fi

# 2. Create the requested N collections
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

  if [[ "$WITH_RELATIONS" -eq 1 ]]; then
    psql "$DB_URL" -c "
    CREATE TABLE IF NOT EXISTS \"$table_name\" (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      name TEXT,
      status TEXT,
      score BIGINT,
      is_active BOOLEAN,
      category_id UUID REFERENCES \"$cat_table\"(\"id\") ON DELETE SET NULL,
      tag_ids JSONB DEFAULT '[]'::jsonb,
      created TIMESTAMPTZ NOT NULL DEFAULT now(),
      updated TIMESTAMPTZ NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS idx_${table_name}_name ON \"$table_name\" (name);
    CREATE INDEX IF NOT EXISTS idx_${table_name}_status ON \"$table_name\" (status);
    CREATE INDEX IF NOT EXISTS idx_${table_name}_category_id ON \"$table_name\" (category_id);
    " >/dev/null

    # Metadata schema including One-to-One (category_id) and One-to-Many (tag_ids) relations
    fields_json="[{\"name\":\"name\",\"data_type\":\"PlainText\",\"index\":true},{\"name\":\"status\",\"data_type\":\"PlainText\",\"index\":true},{\"name\":\"score\",\"data_type\":\"Number\",\"index\":false},{\"name\":\"is_active\",\"data_type\":\"Bool\",\"index\":false},{\"name\":\"category_id\",\"data_type\":\"Relation\",\"related_to\":\"$cat_table\",\"multiple\":false,\"required\":false,\"index\":true},{\"name\":\"tag_ids\",\"data_type\":\"Relation\",\"related_to\":\"$tags_table\",\"multiple\":true,\"required\":false,\"index\":false}]"
    indexes_json="[{\"name\":\"name\",\"data_type\":\"PlainText\",\"index\":true},{\"name\":\"status\",\"data_type\":\"PlainText\",\"index\":true},{\"name\":\"category_id\",\"data_type\":\"Relation\",\"index\":true}]"

    psql "$DB_URL" -c "
    INSERT INTO _collections (id, system, type, name, fields, indexes, options, list_rule, view_rule)
    VALUES (
      'r' || substring(md5(random()::text) from 1 for 14),
      0,
      'base',
      '$table_name',
      '$fields_json'::jsonb,
      '$indexes_json'::jsonb,
      '{}'::jsonb,
      '',
      ''
    ) ON CONFLICT (name) DO UPDATE SET fields = EXCLUDED.fields, indexes = EXCLUDED.indexes;
    " >/dev/null

    if [[ "$ROWS" -gt 0 ]]; then
      psql "$DB_URL" -c "
      INSERT INTO \"$table_name\" (name, status, score, is_active, category_id, tag_ids)
      SELECT
        'name_' || x || '_' || substring(md5(random()::text) from 1 for 4),
        (ARRAY['new', 'active', 'archived'])[floor(random() * 3 + 1)],
        floor(random() * 1000)::bigint,
        random() > 0.5,
        lat_cat.cat_id,
        lat_tags.tag_arr
      FROM generate_series(1, $ROWS) as x
      LEFT JOIN LATERAL (
        SELECT CASE WHEN random() < 0.15 THEN NULL ELSE id END as cat_id
        FROM \"$cat_table\"
        ORDER BY random() + (x * 0)
        LIMIT 1
      ) lat_cat ON true
      LEFT JOIN LATERAL (
        SELECT COALESCE(json_agg(id::text), '[]'::json)::jsonb as tag_arr
        FROM (
          SELECT id FROM \"$tags_table\"
          ORDER BY random() + (x * 0)
          LIMIT ((abs(hashtext(x::text || random()::text)) % 4))
        ) sub
      ) lat_tags ON true;
      " >/dev/null
      seeded=$((seeded + ROWS))
    fi

  else
    # Plain schema without relations
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

    fields_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true},{"name":"score","data_type":"Number","index":false},{"name":"is_active","data_type":"Bool","index":false}]'
    indexes_json='[{"name":"name","data_type":"PlainText","index":true},{"name":"status","data_type":"PlainText","index":true}]'

    psql "$DB_URL" -c "
    INSERT INTO _collections (id, system, type, name, fields, indexes, options, list_rule, view_rule)
    VALUES (
      'r' || substring(md5(random()::text) from 1 for 14),
      0,
      'base',
      '$table_name',
      '$fields_json'::jsonb,
      '$indexes_json'::jsonb,
      '{}'::jsonb,
      '',
      ''
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
  fi

  created=$((created + 1))
done

echo "Created collections: $created"
echo "Seeded records: $seeded"
echo "Skipped existing tables: $skipped"
if [[ "$WITH_RELATIONS" -eq 1 ]]; then
  echo "Relations configured:"
  echo "  - One-to-One:  category_id -> $cat_table"
  echo "  - One-to-Many: tag_ids     -> $tags_table"
fi
