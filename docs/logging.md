# Database logging

Crabbase can persist its `tracing` events to Postgres. Console and JSON-file
logging always run; the database layer is opt-in via `[logging.database]`.

Events are handed to a background writer (bounded queue, batched `INSERT …
UNNEST`, one dedicated pool) so a slow or unavailable log database never blocks
request handling. If the queue fills, events are dropped and counted rather than
stalling the process.

The table is `_logs`; the admin activity page's columns (`severity`, `action`,
`actor_id`, `target_collection`, …) are all populated from event fields.

## Where the logs live

Database logging connects independently of the application database, so the
`_logs` tables can live in the same database or in their own. Pick a layout:

| Layout | `database` | `schema` | Notes |
|---|---|---|---|
| **Dedicated log database** (recommended) | `cb_logs` | `logs` | Own backup/retention, own role, no lock contention with app data. Can be moved to another server later without code changes. |
| Same database, dedicated schema | app database | `logs` | Recommended minimum when you don't want a second database. |
| Same database, application schema | app database | e.g. `crabbase` | Allowed. Log migrations are then applied as idempotent DDL instead of getting their own `_sqlx_migrations` table. |

**Creating the database is your job.** Crabbase creates the schema (and the
`_logs` table) at startup, but it will not run `CREATE DATABASE`. If the log
database is missing, startup stops with:

```
Cannot connect to the log database `cb_logs` at localhost:5432.
Create it first, then restart:
    CREATE DATABASE cb_logs;
```

## Configuration

```toml
[logging.database]
enabled = true

host     = "localhost"
port     = 5432
user     = "postgres"
password = "postgres"
database = "cb_logs"   # create this database first
schema   = "logs"      # created automatically

batch_size           = 100
flush_interval_ms    = 250
queue_capacity       = 8192
pool_max_connections = 2
insert_timeout_ms    = 2000
synchronous_commit   = false
min_level            = "warn"
```

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | Turn the database layer on |
| `host` / `port` / `user` / `password` / `database` | — | Connection to the log target |
| `schema` | `logs` | Schema holding `_logs`; created if missing |
| `batch_size` | `100` | Rows per insert |
| `flush_interval_ms` | `250` | Max delay before a partial batch is written |
| `queue_capacity` | `8192` | In-memory buffer; overflow is dropped, not blocking |
| `pool_max_connections` | `2` | Dedicated pool size |
| `insert_timeout_ms` | `2000` | Per-batch timeout |
| `synchronous_commit` | `false` | Skips the fsync barrier for log writes |
| `min_level` | `"warn"` | Lowest severity persisted (`RUST_LOG` still governs console/file) |

`password` accepts a `$VAR` prefix to read from the environment, like the rest
of the config.

The `logs.cleanup` job uses this same connection (not the application pool), so
retention keeps working when the logs live in another schema or database.

## Migrations

Log DDL lives in `migrations-logs/`, separate from `migrations/`:

```
migrations/         app schema  → `_sqlx_migrations` in the app schema
migrations-logs/    logs schema → `_sqlx_migrations` in the log schema
```

Both run at startup, before any HTTP server starts. The log migrations run
*before* `init_logging`, so the table exists before the writer's first flush.

When the log schema *is* the application schema, two `_sqlx_migrations` tables
cannot coexist. In that layout the log migrations are executed directly as
idempotent DDL (every statement uses `IF NOT EXISTS`) and the app's migration
history stays untouched. Keep future log migrations idempotent if you use that
layout — or, better, move the logs to their own schema.

## Verifying

```sh
psql "$LOG_DATABASE_URL" -c 'SET search_path TO logs; SELECT count(*) FROM _logs;'
```

On startup the banner prints the resolved target, e.g. `Log target: cb_logs/logs ✓`.
