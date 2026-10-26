-- ============================================================
-- File Storage Metadata
-- ============================================================
-- Records where each uploaded object's bytes physically live, so a file keeps
-- resolving to the backend that received it even after the active storage
-- config changes (e.g. local -> S3). `key` stays backend-agnostic and opaque:
-- it is the only value stored in a record's file field, and it never encodes a
-- bucket, path, or URL scheme.

CREATE TABLE IF NOT EXISTS _files (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key             TEXT UNIQUE NOT NULL,             -- Opaque object key, e.g. uploads/<uuid>/report.pdf
    backend         TEXT NOT NULL DEFAULT 'local',   -- Engine holding the bytes: 'local' | 's3'
    container       TEXT NOT NULL,                    -- Local base path, or S3 bucket name
    original_name   TEXT NOT NULL,                    -- Client-supplied file name
    content_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
    size_bytes      BIGINT NOT NULL DEFAULT 0,
    checksum        TEXT,                             -- Hex-encoded digest, when known
    collection_name TEXT NOT NULL,                     -- Target collection name
    created         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated         TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT _files_backend_check CHECK (backend IN ('local', 's3')),
    CONSTRAINT _files_size_check CHECK (size_bytes >= 0)
);

-- Retention sweeps and paginated admin listings
CREATE INDEX IF NOT EXISTS idx_files_created ON _files (created);

-- Index for retrieving all files belonging to a collection
CREATE INDEX IF NOT EXISTS idx_files_collection_name ON _files (collection_name);

-- Drives batched backend migrations (e.g. draining local -> S3): the migration
-- worker pages through rows for one (backend, container) pair and flips each
-- row's pointer only after the copy is verified.
CREATE INDEX IF NOT EXISTS idx_files_backend_container ON _files (backend, container);
