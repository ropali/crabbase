# Backups & cron jobs

Crabbase runs one durable Postgres job queue. Crons are producers that enqueue
jobs; a backup is one job handler. There is no Redis and no external cron.

## How it fits together

```
_crons ──dispatch──▶ _jobs ──claim──▶ worker (registry) ──▶ handler
  ▲                                                          │
  └──────────────── last_status / next_run ◀─────────────────┘
```

- Schedules live in `_crons` and are seeded by migration `00006`.
- Work lives in `_jobs`, claimed with `FOR UPDATE SKIP LOCKED` and a lease.
- Backup artifacts are catalogued in `_backups`.
- The scheduler and worker run inside the binary by default. `crabbase worker`
  runs only the runtime, no HTTP server.

Built-in handlers: `backup.create`, `backups.prune`, `logs.cleanup`,
`otp.cleanup`, `db.vacuum`.

## Configuration

Backup and job runtime settings live in the `_settings` table as JSON, not in
`crabbase.toml`. They can be read and updated from the admin API:

```sh
# Read
curl http://localhost:8989/api/settings/backups -H "Authorization: Bearer $TOKEN"
curl http://localhost:8989/api/settings/jobs    -H "Authorization: Bearer $TOKEN"

# Update (admin only)
curl -X POST http://localhost:8989/api/settings/backups \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{
    "enabled": true,
    "dir": "./backups",
    "pgDumpPath": "pg_dump",
    "retentionDays": 14,
    "maxConcurrent": 1,
    "dumpSchema": false,
    "schema": "crabbase"
  }'

curl -X POST http://localhost:8989/api/settings/jobs \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{
    "enabled": true,
    "worker": true,
    "scheduler": true,
    "tickSeconds": 30,
    "leaseSeconds": 900,
    "concurrency": 1
  }'
```

Missing keys fall back to the defaults shown above, so a partial JSON object is
valid. `pg_dump` is required for backups; install `postgresql-client` matching
the server major version. Startup preflight fails with an actionable message if
the binary is missing or too old.

`backups` settings are read at the start of every backup/prune job, so changes
take effect on the next run. `jobs` settings (enabled, worker, scheduler, tick,
lease, concurrency) are read at process start; restart the process to apply
changes to those.

On API-only replicas set `worker` and `scheduler` to `false` in the `jobs`
setting. On a dedicated worker set both to `true` and raise `concurrency` as
needed.

## Schedule semantics

- All times are UTC. `next_run` is computed from database time.
- Missed runs collapse: a process down at 02:00 starts with the next 02:00, not
  a burst of catch-up backups.
- Overlap is skipped. The partial unique index `uq_jobs_active_cron` allows one
  queued/running job per cron. A manual "run now" while a run is active returns
  `409 Conflict`.
- `db.backup` is seeded disabled. Enable it from **Settings → Backups** or
  **Settings → Crons**.

## API

All routes are admin-only (`_superusers`).

| Method | Path | Notes |
|---|---|---|
| `GET` | `/api/backups` | paginated catalog |
| `POST` | `/api/backups` | `202`, enqueues `backup.create` |
| `GET` | `/api/backups/{id}/download` | streamed; `409` until completed |
| `DELETE` | `/api/backups/{id}` | deletes row, then artifact |
| `GET` | `/api/crons` | schedules + `nextRun` / `lastStatus` |
| `PATCH` | `/api/crons/{id}` | validates expression, recomputes `nextRun` |
| `POST` | `/api/crons/{id}/run` | `409` if already queued/running |
| `GET` | `/api/jobs` | filters: `status`, `kind` |
| `GET` | `/api/jobs/{id}` | includes `lastError` |

## Retention policy

`backups.prune` runs daily at 04:00 UTC with `{"keepDays": 14}`. It deletes
completed backups older than the window **except the newest successful one**.
Rows are deleted before files. An orphan file is recoverable; a row pointing at
a missing file is not.

For a stricter policy (grandfather-father-son), implement it in SQL over
`_backups` using `created` buckets and always keep the newest completed backup.

## Restore runbook (manual, never in place)

Restoring over a live database from the dashboard is deliberately unsupported.

```bash
# 1. Inspect the archive.
pg_restore --list backups/crabbase-20261030T020000Z-<id>.dump

# 2. Restore into a scratch database.
createdb -h localhost -U postgres crabbase_restore
pg_restore -h localhost -U postgres -d crabbase_restore \
  --no-owner --no-privileges --exit-on-error \
  backups/crabbase-20261030T020000Z-<id>.dump

# 3. Validate.
psql -h localhost -U postgres -d crabbase_restore -c \
  "SELECT max(version) FROM _sqlx_migrations;"
psql -h localhost -U postgres -d crabbase_restore -c \
  "SELECT count(*) FROM _collections;"
psql -h localhost -U postgres -d crabbase_restore -c \
  "SELECT count(*) FROM _superusers;"

# 4. Stop Crabbase, point DATABASE_URL at crabbase_restore (or rename the
#    databases), then start Crabbase. Never pg_restore --clean over a live DB.
```

If the dump was schema-scoped (`dump_schema = true`), add `--schema=crabbase`
to `pg_restore`.

## Operations

Stuck job:

```sql
SELECT id, kind, status, attempts, locked_by, locked_until, last_error
FROM _jobs
WHERE status = 'running' AND locked_until < now();
```

The reaper will requeue it after the lease expires. To force it:

```sql
UPDATE _jobs
SET status = 'queued', locked_by = NULL, locked_until = NULL, run_at = now()
WHERE id = '<uuid>';
```

Queue health:

```sql
SELECT
    count(*) FILTER (WHERE status = 'queued')    AS queued,
    count(*) FILTER (WHERE status = 'running')   AS running,
    count(*) FILTER (WHERE status = 'failed')    AS failed,
    min(run_at) FILTER (WHERE status = 'queued') AS oldest_queued
FROM _jobs;
```

Alert when `oldest_queued` is older than 5 minutes, failed jobs are increasing,
or the last successful backup is older than twice the backup interval.

## Scaling notes

- Run multiple schedulers and workers safely: row locks, a unique `dedupe_key`,
  and the `locked_by` fencing token prevent duplicate execution.
- `pg_dump` is I/O heavy. Move the worker to its own process/deployment before
  raising concurrency.
- Backup downloads are local-disk only in the MVP. With multiple replicas, use a
  shared volume or implement S3 streaming uploads before relying on downloads
  from an arbitrary node.
