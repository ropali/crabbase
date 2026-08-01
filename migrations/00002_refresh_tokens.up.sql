-- System table for tracking refresh token rotation and session family state
CREATE TABLE IF NOT EXISTS _refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id       UUID NOT NULL,                      -- Session family UUID
    collection_ref  TEXT NOT NULL,                      -- Auth collection name ('_superusers', 'users', etc.)
    record_ref      TEXT NOT NULL,                      -- User ID
    token_hash      TEXT UNIQUE NOT NULL,               -- SHA-256 hash of token JTI (automatically creates unique index)
    parent_id       UUID REFERENCES _refresh_tokens(id) ON DELETE CASCADE, -- Predecessor token
    used            BOOLEAN NOT NULL DEFAULT FALSE,     -- Consumption flag
    used_at         TIMESTAMPTZ,                        -- Exact consumption timestamp
    revoked         BOOLEAN NOT NULL DEFAULT FALSE,    -- Security revocation flag
    expires_at      TIMESTAMPTZ NOT NULL,               -- Expiry matching JWT exp
    created         TIMESTAMPTZ NOT NULL DEFAULT now(), -- Standard Crabbase creation timestamp
    updated         TIMESTAMPTZ NOT NULL DEFAULT now()  -- Standard Crabbase update timestamp
);

-- Partial Index for active family lookups (small, fast, RAM-efficient)
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_family
    ON _refresh_tokens (family_id)
    WHERE revoked = FALSE;

-- Composite Index for user session lookups
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_record
    ON _refresh_tokens (collection_ref, record_ref);

-- Index for background expiry cleanup
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires
    ON _refresh_tokens (expires_at);
