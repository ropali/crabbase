# Database logging gaps

Audit of what the Postgres audit trail (`_logs`) does and does not capture.
**Status: partially implemented.** The items under “Implemented” are in the
tree; the rest are still open.

Scope: `[logging.database] enabled = true`. The request middleware already
emits one row per `/api` call, so a “gap” here means an action has no dedicated
event, no useful detail, or is not stored at the default `min_level`.

## Implemented

### Security / authentication

- **Failed logins** — `auth.login_failed` (`WARN`) from
  `crates/auth/src/service.rs`, with `reason` = `unknown_collection`,
  `unknown_user`, `invalid_password` or `unverified`, plus collection, email,
  target user id and `outcome = denied`.
- **Password-reset lifecycle** — `auth.password_reset_requested` (`INFO`, for
  known and unknown users), `auth.password_reset_failed` (`WARN`, `unknown_user`
  or `invalid_otp`) and `auth.password_reset` (`INFO`, success with actor).
  Never logs the OTP or the new password.
- **Logout** — `auth.logout` (`INFO`, actor + token family) and
  `auth.logout_failed` (`WARN`, invalid refresh token).
- **Refresh-token reuse** — existing warning now has
  `action = auth.refresh_reuse`, actor, collection, family id and
  `outcome = denied`.

### Privileged / destructive actions

- **Settings changes** — the request row now carries
  `data.setting` and `data.changedKeys` (key names only, never values). Actions
  are specific: `setting.mail.update`, `setting.app.update`,
  `setting.email-templates.update`, `setting.storage.update`,
  `setting.backups.update`, `setting.jobs.update` (and `.view` for GETs).
  Handlers attach the data through `RequestLog::add_data`; the middleware
  merges it into `_logs.data`.
- **Collection delete / truncate** — the request row carries `data.rows`, the
  row count captured immediately before the destructive operation.
- **File delete detail** — the request row carries `data.key`,
  `data.filename`, `data.sizeBytes` and `data.collection` when the metadata can
  be read.
- **Initial superuser provisioning** — `auth.superuser_created` (`INFO`) with
  the new superuser id and email (never the password or token key).
- **Backup lifecycle** — `backup.created`, `backup.failed` (with error) and
  `backup.pruned` (removed count + keep window) from the job handlers.
- **Job failures** — `job.failed` (`WARN`, job id/kind/error) and
  `job.cancelled` (`INFO`) from the worker.
- **Retention/deletion jobs** — `logs.cleanup`, `otp.cleanup` and `db.vacuum`
  now emit an event with the deleted count (where applicable).

### Audit quality

- **Specific action names** — `backup.download`, `cron.run`, `file.token`, and
  per-group settings actions instead of the generic verbs.
- **Denial reasons** — 401/403 rows carry `data.denyReason` (`missing_token`,
  `malformed_header`, `invalid_claims`, `unknown_collection`, `unknown_user`,
  `invalid_token`, `invalid_session`, `not_superuser`).
- **Email delivery failures** — `auth.email_failed` (`ERROR`) instead of an
  unfilterable message.
- **Logs API self-logging** — `/api/logs…` requests are skipped by the request
  middleware, so reading the Logs page no longer adds `logs.list` rows.

## Remaining gaps

- **Successful logins at the default level.** Successful requests (including
  logins) are `INFO`; with `min_level = "warn"` they are not stored. Use
  `min_level = "info"` for a full access log; a duplicate dedicated login event
  was intentionally not added.
- **Token rejection events.** The reason is now on the request row
  (`data.denyReason`), but there is no separate event for alerting; adding one
  would need rate limiting to avoid log flooding.
- **Password/role/rule changes through generic writes.** Updating a user record
  or a collection’s auth/view rules is still only `record.update` /
  `collection.update`; there is no indication that a protected field changed.
- **Record delete detail.** Record deletes identify the target but do not add a
  count (file deletes now add key/name/size).
- **Denial reasons for view-rule denials.** Authentication/admin denials now
  say why; a denied view rule still appears as a plain 403.
- **File downloads at `warn`.** `file.view` is `INFO`; no access trail unless
  `min_level = "info"` (or a dedicated event is added).
- **Refresh-token purge is unscheduled.** `purge_expired_refresh_tokens`
  (`crates/auth/src/repositories/auth.rs`) has no caller; if wired to a cron,
  log the deleted count.
- **Writer health / audit-DB outage.** Dropped/written/error counters are only
  emitted at shutdown at `debug` level; there is no runtime warning/metric when
  the audit trail is incomplete.

## Design notes

- Security/admin events are emitted as normal `tracing` events. The `PgLayer`
  maps known field names to columns and puts everything else in `_logs.data`;
  never insert into `_logs` directly.
- Handlers can enrich the request row without a second event:
  `RequestLog::add_data(json!({...}))` (see the collection and settings
  handlers). The middleware serializes it into the `data` field and `PgLayer`
  merges it. `data` is redacted by key, so never put secrets in it.
- Use `WARN` for failures/denials that must survive the default
  `min_level = "warn"`; use `INFO` for successful actions, which require
  `min_level = "info"` to be persisted.
- Always set an explicit `action`; the fallback is the module target, which is
  hard to filter or alert on.
