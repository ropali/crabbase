-- Repair the JSON seeds written by 00006. Early copies of that migration
-- contained invalid JSON (trailing commas / a stray colon), which made the
-- settings rows unreadable. Re-seed them with valid JSON.

INSERT INTO _settings (name, value)
VALUES (
    'backups',
    '{"enabled":true,"dir":"./backups","useS3":false,"retentionDays":14,"schema":"crabbase"}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();

INSERT INTO _settings (name, value)
VALUES (
    'jobs',
    '{"enabled":true,"tickSeconds":30,"leaseSeconds":900,"concurrency":1}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();
