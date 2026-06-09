-- ============================================================
-- Extensions
-- ============================================================

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- System Tables
-- ============================================================

-- _migrations: Migration history tracking
CREATE TABLE IF NOT EXISTS _migrations (
    id         TEXT PRIMARY KEY,
    file       TEXT UNIQUE NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- _collections: Collection metadata & schema definitions
-- Stores the structure of dynamic collections
CREATE TABLE IF NOT EXISTS _collections (
    id          TEXT PRIMARY KEY NOT NULL,
    system      INTEGER NOT NULL DEFAULT 0,  -- 1 for system collections, 0 for user-created
    type        TEXT NOT NULL DEFAULT 'base', -- 'base', 'auth', or 'view'
    name        TEXT UNIQUE NOT NULL,
    fields      JSONB NOT NULL DEFAULT '[]',   -- JSON array of field definitions
    indexes     JSONB NOT NULL DEFAULT '[]',   -- JSON array of index definitions
    list_rule   TEXT DEFAULT NULL,            -- Access rule for LIST operations
    view_rule   TEXT DEFAULT NULL,            -- Access rule for VIEW operations
    create_rule TEXT DEFAULT NULL,            -- Access rule for CREATE operations
    update_rule TEXT DEFAULT NULL,            -- Access rule for UPDATE operations
    delete_rule TEXT DEFAULT NULL,            -- Access rule for DELETE operations
    options     JSONB NOT NULL DEFAULT '{}',   -- JSON object for type-specific options
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx__collections_type ON _collections (type);

-- _params: Application settings/parameters
CREATE TABLE IF NOT EXISTS _params (
    id      TEXT PRIMARY KEY NOT NULL,
    value   TEXT DEFAULT NULL,  -- JSON value
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- _logs: Application audit logs
CREATE TABLE IF NOT EXISTS _logs (
    id      TEXT PRIMARY KEY NOT NULL,
    level   INTEGER NOT NULL DEFAULT 0,
    message TEXT NOT NULL DEFAULT '',
    data    TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_logs_level ON _logs (level);
CREATE INDEX IF NOT EXISTS idx_logs_message ON _logs (message);
CREATE INDEX IF NOT EXISTS idx_logs_created_hour ON _logs (date_trunc('hour', created AT TIME ZONE 'UTC'));

-- ============================================================
-- Auth Tables
-- ============================================================

-- _superusers: Administrator accounts
CREATE TABLE IF NOT EXISTS _superusers (
    id              TEXT PRIMARY KEY NOT NULL,
    email           TEXT UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,
    token_key       TEXT NOT NULL,
    email_visible   BOOLEAN NOT NULL DEFAULT FALSE,
    verified        BOOLEAN NOT NULL DEFAULT FALSE,
    created         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx__superusers_email ON _superusers (email);
CREATE INDEX IF NOT EXISTS idx__superusers_token_key ON _superusers (token_key);

-- _mfas: Multi-Factor Authentication records
CREATE TABLE IF NOT EXISTS _mfas (
    id             TEXT PRIMARY KEY NOT NULL,
    collection_ref TEXT NOT NULL,
    record_ref     TEXT NOT NULL,
    method         TEXT NOT NULL,  -- 'password', 'oauth2', 'otp'
    created        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_mfas_collection_ref_record_ref ON _mfas (collection_ref, record_ref);

-- _otps: One-Time Password records
CREATE TABLE IF NOT EXISTS _otps (
    id             TEXT PRIMARY KEY NOT NULL,
    collection_ref TEXT NOT NULL,
    record_ref     TEXT NOT NULL,
    password_hash  TEXT NOT NULL,  -- Hashed OTP
    sent_to        TEXT,           -- Email sent to
    created        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_otps_collection_ref_record_ref ON _otps (collection_ref, record_ref);

-- _external_auths: OAuth2 / external auth integrations
CREATE TABLE IF NOT EXISTS _external_auths (
    id             TEXT PRIMARY KEY NOT NULL,
    collection_ref TEXT NOT NULL,
    record_ref     TEXT NOT NULL,
    provider       TEXT NOT NULL,  -- 'google', 'github', etc.
    provider_id    TEXT NOT NULL,
    created        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_external_auths_record_provider
    ON _external_auths (collection_ref, record_ref, provider);
CREATE UNIQUE INDEX IF NOT EXISTS idx_external_auths_collection_provider
    ON _external_auths (collection_ref, provider, provider_id);

-- _auth_origins: Device/browser fingerprints for auth
CREATE TABLE IF NOT EXISTS _auth_origins (
    id            TEXT PRIMARY KEY NOT NULL,
    collection_ref TEXT NOT NULL,
    record_ref     TEXT NOT NULL,
    fingerprint   TEXT NOT NULL,
    created       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_origins_unique_pairs
    ON _auth_origins (collection_ref, record_ref, fingerprint);


-- ============================================================
-- Seed Data
-- ============================================================

-- Insert default _superusers collection entry in _collections
INSERT INTO _collections (id, system, type, name, fields, options)
VALUES (
    'r' || substring(md5(random()::text) from 1 for 14),
    1,
    'auth',
    '_superusers',
    '[{"name":"email","type":"Email","related_to": null, "index": true}]',
    '{"authToken": {"secret": "my-secret-key"}}'
)
ON CONFLICT DO NOTHING;
