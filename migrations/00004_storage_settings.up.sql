-- ============================================================
-- Storage Settings
-- ============================================================

INSERT INTO _settings (name, value)
VALUES (
    'storage',
    '{
  "storageType": "local",
  "local": {
    "local_path": "$HOME/crabbase/files/"
  },
  "s3": {
    "endpoint": "",
    "bucket": "crabbase_files",
    "region": "us-east-1",
    "accessKey": "",
    "secret": ""
  }
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();
