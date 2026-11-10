-- Backup and job runtime settings live in `_settings` as JSON, not in
-- crabbase.toml. This migration normalizes the rows seeded by 00006/00007 to
-- the full schema the application reads.

INSERT INTO _settings (name, value)
VALUES (
    'backups',
    '{
  "enabled": true,
  "dir": "./backups",
  "pgDumpPath": "pg_dump",
  "retentionDays": 14,
  "maxConcurrent": 1,
  "dumpSchema": false,
  "schema": "crabbase"
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();

INSERT INTO _settings (name, value)
VALUES (
    'jobs',
    '{
  "enabled": true,
  "worker": true,
  "scheduler": true,
  "tickSeconds": 30,
  "leaseSeconds": 900,
  "concurrency": 1
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();
