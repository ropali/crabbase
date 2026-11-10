-- ============================================================
-- Cron schedules
-- ============================================================

CREATE TABLE IF NOT EXISTS _crons (
    id          UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    name        TEXT UNIQUE NOT NULL,              -- stable key, e.g. 'db.backup'
    kind        TEXT NOT NULL,                     -- handler key, e.g. 'backup.create'
    schedule    TEXT NOT NULL,                     -- 5-field cron, UTC
    payload     JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    next_run    TIMESTAMPTZ,                       -- NULL until first computed
    last_run    TIMESTAMPTZ,                       -- when the last run finished
    last_status TEXT,                              -- 'queued' | 'running' | 'succeeded' | 'failed'
    last_error  TEXT,
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_crons_due
    ON _crons (next_run)
    WHERE enabled;

-- ============================================================
-- Durable job queue
-- ============================================================

CREATE TABLE IF NOT EXISTS _jobs (
    id            UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    kind          TEXT NOT NULL,
    payload       JSONB NOT NULL DEFAULT '{}'::jsonb,
    status        TEXT NOT NULL DEFAULT 'queued'
                  CHECK (status IN ('queued','running','succeeded','failed','cancelled')),
    priority      SMALLINT NOT NULL DEFAULT 100,   -- lower = sooner
    run_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempts      INTEGER NOT NULL DEFAULT 0,
    max_attempts  INTEGER NOT NULL DEFAULT 3,
    locked_by     TEXT,                            -- worker id; also the fencing token
    locked_until  TIMESTAMPTZ,                     -- lease expiry
    last_error    TEXT,
    result        JSONB,
    dedupe_key    TEXT UNIQUE,                     -- NULL for manual jobs
    cron_id       UUID REFERENCES _crons(id) ON DELETE SET NULL,
    created       TIMESTAMPTZ NOT NULL DEFAULT now(),
    started       TIMESTAMPTZ,
    finished      TIMESTAMPTZ,

    CONSTRAINT _jobs_attempts_check CHECK (attempts >= 0),
    CONSTRAINT _jobs_lease_check CHECK (
        (status = 'running' AND locked_by IS NOT NULL AND locked_until IS NOT NULL)
        OR status <> 'running'
    )
);

-- The claim query's supporting index.
CREATE INDEX IF NOT EXISTS idx_jobs_claim
    ON _jobs (priority, run_at)
    WHERE status = 'queued';

-- Reaper: find expired leases.
CREATE INDEX IF NOT EXISTS idx_jobs_lease
    ON _jobs (locked_until)
    WHERE status = 'running';

-- Admin history.
CREATE INDEX IF NOT EXISTS idx_jobs_kind_created
    ON _jobs (kind, created DESC);

-- Overlap policy for scheduled jobs: at most one queued/running job per cron.
-- With ON CONFLICT DO NOTHING this makes duplicate dispatch and double-click
-- "run now" safe. Drop this index only if you later support overlap_policy='queue'.
CREATE UNIQUE INDEX IF NOT EXISTS uq_jobs_active_cron
    ON _jobs (cron_id)
    WHERE cron_id IS NOT NULL AND status IN ('queued','running');

-- ============================================================
-- Backup catalog
-- ============================================================

CREATE TABLE IF NOT EXISTS _backups (
    id              UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    job_id          UUID REFERENCES _jobs(id) ON DELETE SET NULL,
    filename        TEXT UNIQUE NOT NULL,          -- generated server-side, never user input
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','running','completed','failed')),
    triggered_by    TEXT NOT NULL DEFAULT 'manual'
                    CHECK (triggered_by IN ('manual','scheduled','cli')),
    storage_backend TEXT NOT NULL DEFAULT 'local'
                    CHECK (storage_backend IN ('local','s3')),
    size_bytes      BIGINT,
    checksum        TEXT,                          -- sha256 hex
    pg_dump_version TEXT,
    error           TEXT,
    started         TIMESTAMPTZ,
    finished        TIMESTAMPTZ,
    created         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_backups_created ON _backups (created DESC);
CREATE INDEX IF NOT EXISTS idx_backups_status  ON _backups (status);

-- ============================================================
-- Built-in schedules
-- ============================================================
-- next_run stays NULL until the scheduler computes it, so a row does not run
-- immediately on first boot. db.backup is opt-in: backups can surprise a
-- self-hoster with disk usage, so the UI toggle enables it.

INSERT INTO _crons (name, kind, schedule, payload, enabled) VALUES
    ('db.backup',     'backup.create', '0 2 * * *',  '{}',                         FALSE),
    ('backups.prune', 'backups.prune', '0 4 * * *',  '{"keepDays": 14}',           TRUE),
    ('logs.cleanup',  'logs.cleanup',  '0 3 * * *',  '{"olderThanDays": 30}',      TRUE),
    ('otp.cleanup',   'otp.cleanup',   '*/15 * * *', '{}',                         TRUE),
    ('db.vacuum',     'db.vacuum',     '0 5 * * 0',  '{}',                         FALSE)
ON CONFLICT (name) DO NOTHING;


-- ============================================================
-- Insert settings table entries
-- ============================================================
INSERT INTO _settings (name, value)
VALUES (
    'backups',
    '{
  "enabled": true,
  "dir": "./backups",
  "use_s3": false,
  "retention_days": 14,
  "schema": "crabbase",
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();


INSERT INTO _settings (name, value)
VALUES (
    'jobs',
    '{
  "enabled": true,
  "tick_seconds": 30,
  "use_s3": false,
  "lease_seconds": 900: ,
  "concurrency": 1,
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();
