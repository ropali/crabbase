-- ============================================================
-- App Settings Table
-- ============================================================

CREATE TABLE IF NOT EXISTS _settings (
    id      TEXT PRIMARY KEY NOT NULL,
    name    VARCHAR(60) NOT NULL,
    value   TEXT DEFAULT NULL,
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- System Collection Registration in _collections
-- ============================================================

INSERT INTO _collections (id, system, type, name, fields, options)
VALUES (
    'r' || substring(md5(random()::text) from 1 for 14),
    1,
    'base',
    '_settings',
    '[
        {"name": "id", "type": "text", "required": true},
        {"name": "name", "type": "text", "required": true},
        {"name": "value", "type": "json", "required": false},
        {"name": "created", "type": "autodate"},
        {"name": "updated", "type": "autodate"}
    ]'::jsonb,
    '{}'::jsonb
)
ON CONFLICT (name) DO NOTHING;
