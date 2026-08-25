# Authentication

Crabbase ships a complete JWT auth system that works per collection — no code required. Any collection marked as type `auth` becomes login-capable; the built-in `_superusers` collection is used for admin access.

## Auth collections

Create an `auth` collection — in the dashboard's Create Collection drawer, pick type **Auth**; or via API:

```sh
curl -X POST http://localhost:8989/api/collections \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{
    "name": "users",
    "collection_type": "auth",
    "columns": [
      { "name": "name", "data_type": "PlainText" }
    ]
  }'
```

Auth collections expect these system-managed columns (created automatically for the seeded `_superusers`; when designing your own, keep them in mind):

- `email` — unique identifier for login
- `password` — automatically **bcrypt-hashed** on record create
- `token_key` — random per-user value mixed into JWT secrets (auto-generated)
- `verified` — email verification flag

Each auth collection also gets its own random signing secret (`options.authToken.secret`) generated at creation time.

## Registering users

There is no dedicated registration endpoint yet. The easiest way to create users is **through the admin dashboard**: open the auth collection in the sidebar and use its create-record drawer — enter an email, password, and any custom fields. Passwords are hashed automatically on save.

Via API, create users by POSTing records to the auth collection:

```sh
curl -X POST http://localhost:8989/api/collections/users/records \
  -H 'Content-Type: application/json' \
  -d '{"data": {"email": "jane@example.com", "password": "secret123", "name": "Jane"}}'
```

The plaintext `password` you send is hashed before storage; never returned afterwards. (`AppSettings.allowPublicUserRegistration` exists as a setting but is not enforced yet.)

## Endpoints

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/auth/{collection}/login` | Obtain access + refresh tokens |
| `POST` | `/api/auth/{collection}/auth-refresh` | Rotate tokens (requires bearer token + body) |
| `POST` | `/api/auth/{collection}/logout` | Revoke the refresh-token family |
| `GET` | `/api/auth/profile` | Current user from bearer token |
| `POST` | `/api/auth/{collection}/forget-password` | Email a 6-digit OTP |
| `POST` | `/api/auth/{collection}/reset-password` | Reset password with OTP |

### Login

```sh
curl -X POST http://localhost:8989/api/auth/users/login \
  -H 'Content-Type: application/json' \
  -d '{"email": "jane@example.com", "password": "secret123"}'
```

Response:

```json
{
  "tokens": {
    "accessToken": "eyJ...",
    "refreshToken": "eyJ..."
  }
}
```

Access tokens default to ~1 hour (or the duration configured on the collection; seeded collections use 5 days). Refresh tokens last ~7 days.

### Calling authenticated endpoints

Send the access token as a bearer token:

```sh
curl http://localhost:8989/api/auth/profile \
  -H "Authorization: Bearer <accessToken>"
```

```json
{ "id": "5f0c...", "email": "jane@example.com" }
```

The middleware resolves the collection's secret plus your per-user `token_key`, validates the JWT, and loads your full record so API rules can reference it via `@request.auth.*`. Superuser-only endpoints (collections management, settings) reject any token whose claims are not from `_superusers`.

### Refreshing tokens

Refresh uses **rotation with reuse detection**: every refresh invalidates the old token and issues a new pair; reusing an already-rotated token revokes the whole session family. A short grace window tolerates concurrent refreshes.

```sh
curl -X POST http://localhost:8989/api/auth/users/auth-refresh \
  -H "Authorization: Bearer <accessToken>" \
  -H 'Content-Type: application/json' \
  -d '{"refreshToken": "<refreshToken>"}'
```

Response: a fresh `{ "tokens": { "accessToken", "refreshToken" } }`.

### Logout

Revokes the entire refresh-token family for that session:

```sh
curl -X POST http://localhost:8989/api/auth/users/logout \
  -H 'Content-Type: application/json' \
  -d '{"email": "jane@example.com", "refreshToken": "<refreshToken>"}'
```

## Password reset (email OTP)

Requires SMTP to be configured first — see [Settings & email](settings-email.md).

1. Request a reset — sends a templated email containing a 6-digit one-time code (valid 5 minutes):

   ```sh
   curl -X POST http://localhost:8989/api/auth/users/forget-password \
     -H 'Content-Type: application/json' \
     -d '{"email": "jane@example.com"}'
   ```

2. Reset with the OTP:

   ```sh
   curl -X POST http://localhost:8989/api/auth/users/reset-password \
     -H 'Content-Type: application/json' \
     -d '{"email": "jane@example.com", "otp": 123456, "new_pwd": "new-secret"}'
   ```

OTPs are stored HMAC-hashed server-side. Templates for these emails are editable under `/api/settings/email-templates` with variables like `{{otp}}`, `{{link}}`, `{{name}}`, `{{app_name}}` (see [Settings & email](settings-email.md)).

## Superusers

- The `_superusers` collection is seeded by migrations and repaired at boot.
- Its credentials come from `ADMIN_USERNAME` / `ADMIN_PASSWORD` env vars.
- Log in against it like any auth collection (this is exactly what the dashboard's login page does):

  ```sh
  curl -X POST http://localhost:8989/api/auth/_superusers/login \
    -H 'Content-Type: application/json' \
    -d '{"email": "admin@crabbase.local", "password": "..."}'
  ```

- Only `_superusers` tokens pass the admin guard on `/api/collections*` and `/api/settings*`.
- The admin dashboard logs in against this same endpoint.

## Security notes

- Passwords: bcrypt.
- Tokens: HS256 JWTs; the effective secret is `{collection secret}-{user token_key}`, so changing a user's `token_key` invalidates their existing tokens.
- Refresh tokens are persisted hashed in `_refresh_tokens` with family tracking and expiry cleanup indexes.
- While evolving, note: the seeded `_superusers` secret in migration `00001` is a hardcoded placeholder worth rotating in production.
