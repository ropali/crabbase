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
# Only events at this level and above are persisted. "warn" keeps failures
# and denials; "info" also stores every successful request (full access log).
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

## Best practices

### Storage and access

- **Use a dedicated database** (`cb_logs`), or at minimum a dedicated schema and
  role. The log writer must never be able to touch application tables, and
  retention/backups should not compete with user traffic.
- Create the database out of band. Crabbase creates the `logs` schema and
  `_logs` at startup, and the log migrations run on every boot (they use
  `CREATE TABLE IF NOT EXISTS`, which Postgres still permission-checks). The
  connection role therefore needs `CREATE` on the schema, plus `CONNECT`,
  `USAGE`, and `SELECT`/`INSERT`/`DELETE` on `_logs` for reading, writing and
  `logs.cleanup`. If the schema is pre-created, `CREATE` on the database is not
  needed; schema-level `CREATE` still is. Keep the role confined to the log
  database/schema.
- Keep the log database in UTC. Timestamps are stored as `timestamptz` and the
  stats buckets are aggregated in UTC; the dashboard converts to local time for
  display.
- Store the password as `$LOG_DB_PASSWORD` (or similar) rather than in plain
  text; every credential field supports the `$VAR` prefix.
- Keep the log target on a local or private network. Logging is out of band,
  but every batch is still a round trip and each batch has a hard insert
  timeout.
- Treat `_logs` as sensitive: it stores actor ids, IP addresses, user agents,
  paths and route templates. Sensitive keys inside `data` are redacted
  (`password`, `token`, `secret`, `otp`, `authorization`, `api_key`), but the
  rest of the row is still personal data. Restrict direct database access and
  rely on the admin API (superuser token) for reading.
- Decide retention and backups explicitly. A dedicated log database is not
  covered by an application-database backup; either back it up separately or
  accept losing history on a rebuild.

### Verbosity

- `min_level = "warn"` is a good default: it keeps failed/denied requests and
  all warning/error events without storing routine reads. Note that successful
  requests, successful logins and successful writes are `INFO` and therefore
  *not* in the audit trail at this level.
- `min_level = "info"` gives the full access log (one row per API request plus
  informational job/auth events). This is the right setting when the Logs page
  is used as an audit trail, but size the storage and retention for it.
- Avoid `debug`/`trace` in production. They add internal events (for example
  file-storage repair messages) and can multiply volume.
- `RUST_LOG` and `RUST_LOG_FILE` do not affect the database. Raising them is
  safe when you want quieter console/file output without losing DB rows.
- The Logs page is itself an API client. With `min_level = "info"`, page
  refreshes (including the 5-second live poll) show up as `logs.list` rows, so
  expect a small constant background and account for it in retention.

### Durability and throughput

- `synchronous_commit = false` is the right trade-off for an audit log: rows
  are WAL-written, but a power/OS failure can lose the last flush. Set it to
  `true` only when the trail must survive an OS crash, and measure the write
  cost first.
- The defaults (batch 100, flush 250 ms, queue 8,192) suit normal traffic.
  Raise `queue_capacity` for bursty workloads; keep `batch_size` modest so a
  single insert stays fast.
- Keep `insert_timeout_ms` short (2 s). A slow log database must never stall
  the writer; the bounded queue absorbs short outages and drops beyond that.
- Keep `pool_max_connections` small (2). The writer owns a dedicated pool, so
  it cannot starve application queries.

### Operations

- Keep the `logs.cleanup` cron enabled (created by default as
  `0 3 * * *` with `{"olderThanDays": 30}`) and set the window to your
  retention policy. Deleting rows does not return disk to the OS immediately;
  watch table size and autovacuum.
- Watch the console/file logs for `audit log batch insert failed` and
  `audit log batch insert timed out`. When the log database is unreachable the
  application keeps serving and events are dropped by design.
- The writer's `written` / `dropped` / `errors` counters are emitted at shutdown
  at `debug` level; temporarily set `RUST_LOG=debug` during a rollout if you
  need to confirm that nothing was dropped.
- Shut down gracefully. The writer drains its queue and flushes on shutdown;
  give the process a termination window (for example Kubernetes
  `terminationGracePeriodSeconds`) or the tail of the audit trail is lost.
- Verify the target from the startup banner (`Log target: cb_logs/logs ✓`) and
  with `SELECT count(*)`, and test config changes in staging before pointing
  production logs at a new server.

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

## What gets recorded

Every request to the `/api` surface emits exactly one event after the handler
returns, but it is only persisted if it passes `min_level`:

| Response | Event level | Stored at default `warn`? |
|---|---|---|
| `2xx` / `3xx` | `INFO` | no |
| `4xx` | `WARN` | yes |
| `5xx` | `ERROR` | yes |
| `OPTIONS` | — | never emitted (CORS preflight) |

So with the default `min_level = "warn"` the table holds failed and denied
requests only. Set `min_level = "info"` when successful requests must be
audited.

Each stored request row carries:

| Column | Source |
|---|---|
| `severity`, `level` | response status: `5xx → ERROR`, `4xx → WARN`, else `INFO` |
| `action` | `<resource>.<verb>` derived from the matched route template, e.g. `record.create`, `collection.truncate`, `auth.login` |
| `method`, `path` | the original URL as the client sent it |
| `status_code`, `duration_ms` | response status and wall-clock handler time |
| `actor_id`, `actor_collection` | filled by the auth middleware only after the token is verified; the login handler attributes its own row |
| `target_collection`, `target_id` | the `{name}` / `{id}` path parameters of the route, when present |
| `ip_address` | `X-Forwarded-For`, then `X-Real-IP`, then the socket address |
| `outcome` | `success`, `denied` (401/403) or `failure` |
| `data` | the route template (`route`) and any extra event fields, redacted |

Events that do not come from a request (jobs, workers, bootstrap) are recorded
too, at or above `min_level`; they have no actor, target or status and their
`action` falls back to the emitting module's target.

Handlers can add fields to their own request row with
`RequestLog::add_data(json!({...}))` (for example `rows` on a collection
truncate, `changedKeys` on a settings update, `denyReason` on a 401/403, or
file metadata on delete). The middleware merges those into `data`, where the
normal redaction rules apply.

Empty values are stored as `NULL`, not `''`, so `actor_id IS NULL` reliably
means "unauthenticated".

`min_level` is independent of `RUST_LOG`: the database layer declares its own
max-level hint, so raising `RUST_LOG` to `warn` for quieter console output does
not silently stop `INFO` rows from being persisted.

## Reading the logs

The dashboard's **Logs** page reads `GET /api/logs` (admin token required):

```
GET /api/logs?page=1&per_page=50&range=24h&level=Error&search=record&superusers=true
GET /api/logs/stats?range=24h
```

`range` accepts `5m`, `15m`, `30m`, `1h`, `2h`, `6h`, `24h`, `7d`, `30d`
(and `all`), matching the buttons on the Logs page.

`stats` returns the counters (`total`, `errors`, `warnings`, `avgDurationMs`)
plus the chart series. The series is **zero-filled**: one bucket per minute for
`5m`–`6h`, per hour for `24h`/`7d`, and per day for `30d`/`all`, covering the
whole window. A quiet interval is a bucket with `count: 0`, not a missing
bucket, so bar slots line up with real time. `all` is capped to the last 30 days
so the chart cannot scan the entire table.

Both are served from the *log* pool, so they keep working when `_logs` lives in
another database. When `[logging.database]` is disabled they answer
`{ "enabled": false }` and the page shows a configuration hint instead of an
error. Every stored field is visible: expand a row to see the full record,
including the raw `data` payload.

## Verifying

```sh
psql "$LOG_DATABASE_URL" -c 'SET search_path TO logs; SELECT count(*) FROM _logs;'
```

On startup the banner prints the resolved target, e.g. `Log target: cb_logs/logs ✓`.
