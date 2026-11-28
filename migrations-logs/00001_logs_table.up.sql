-- ============================================================
-- Audit logging schema
-- ============================================================
--
-- Lives in its own migration directory (migrations-logs/) and therefore gets
-- its own `_sqlx_migrations` table inside the logging schema. It can be
-- applied against the application database (different schema) or against a
-- dedicated log database without changing anything here.

-- _logs: Application audit trail for the admin UI.
--
-- This is the end-state of the former "stub + ALTER" design, folded into
-- a single CREATE: the columns the admin UI renders (activity_logs.rs
-- LogItem) plus the ones an audit trail requires, with `data` already
-- JSONB and only the indexes that earn their write cost.
--
-- Index budget (four, and that is a budget, not a starting point):
--   - created    BRIN: time is the access pattern ("last hour", "recent
--     activity"); cheap on an append-mostly table.
--   - actor_id   (actor_id, created DESC): the admin UI's primary filter,
--     also covers "most recent for this actor".
--   - target     (target_collection, target_id, created DESC): second
--     primary filter.
--
--   Deliberately NOT indexed:
--   - level/severity -> low cardinality; BRIN on created narrows the scan first.
--   - message        -> btree on free-text; nobody queries message = 'x'.
--   - action         -> low cardinality, same reasoning as severity.
--   - request_id/trace_id/span_id -> NULL until a tracing backend is
--     installed; add a btree on request_id only if O(1) lookup is needed.
--   - data (GIN)     -> only pays when querying inside the JSON; add then.

CREATE TABLE IF NOT EXISTS _logs (
    id                TEXT PRIMARY KEY NOT NULL,
    level             INTEGER     NOT NULL DEFAULT 0,
    message           TEXT        NOT NULL DEFAULT '',
    data              JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created           TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- UI-facing severity plus the audit-trail fields.
    severity          TEXT        NOT NULL DEFAULT 'INFO',
    action            TEXT,
    actor_id          TEXT,
    actor_collection  TEXT,
    target_collection TEXT,
    target_id         TEXT,
    method            TEXT,
    path              TEXT,
    status_code       INTEGER,
    duration_ms       DOUBLE PRECISION,
    ip_address        TEXT,
    user_agent        TEXT,
    outcome           TEXT        NOT NULL DEFAULT 'success'
    -- Step 2 seam: populated when a tracing backend with trace context
    -- is installed.
    -- request_id        TEXT,
    -- trace_id          TEXT,
    -- span_id           TEXT
);

CREATE INDEX IF NOT EXISTS idx_logs_level   ON _logs (level);
CREATE INDEX IF NOT EXISTS idx_logs_created ON _logs USING BRIN (created)
    WITH (pages_per_range = 32);
CREATE INDEX IF NOT EXISTS idx_logs_actor   ON _logs (actor_id, created DESC);
CREATE INDEX IF NOT EXISTS idx_logs_target  ON _logs (target_collection, target_id, created DESC);
