-- `otp.cleanup` was seeded with a 4-field expression (`*/15 * * *`), which is
-- not a valid cron and made the scheduler disable the cron on first boot.
-- Correct it to every 15 minutes and let the scheduler recompute next_run.
UPDATE _crons
SET schedule = '*/15 * * * *',
    enabled = TRUE,
    next_run = NULL,
    last_error = NULL,
    updated = now()
WHERE name = 'otp.cleanup';

-- Maintenance vacuum runs weekly on Sunday at 05:00 UTC.
UPDATE _crons
SET enabled = TRUE,
    next_run = NULL,
    last_error = NULL,
    updated = now()
WHERE name = 'db.vacuum' AND enabled = FALSE;
