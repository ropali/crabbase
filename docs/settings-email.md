# Settings & email

Crabbase stores runtime configuration in a key/value settings table (`_settings`). You can manage all of it from the **admin dashboard** — no code, no curl, no restarts:

- **Settings → General** — app name, URL, contact email
- **Settings → Mail** — SMTP host/port/credentials and sender identity
- **Settings → Email Templates** — visual editors for the password-reset and registration emails (with preview)

The REST endpoints below are the exact same operations the dashboard performs — useful for automation or scripted provisioning.

All endpoints in this doc require a **superuser** bearer token.

| Method | Path | Description |
|---|---|---|
| `GET` / `POST` | `/api/settings/mail` | SMTP / sender settings |
| `GET` / `POST` | `/api/settings/app` | App name, URL, contact, registration flag |
| `GET` / `POST` | `/api/settings/email-templates` | Password-reset + registration email templates |

## Mail settings

```sh
curl http://localhost:8989/api/settings/mail -H "Authorization: Bearer $TOKEN"
```

```json
{
  "senderName": "My App",
  "senderAddress": "no-reply@example.com",
  "smtpHost": "smtp.example.com",
  "smtpPort": 587,
  "smtpUsername": "no-reply@example.com",
  "smtpPassword": "secret"
}
```

Save with `POST` using the same shape:

```sh
curl -X POST http://localhost:8989/api/settings/mail \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{ "senderName": "My App", "senderAddress": "no-reply@example.com", "smtpHost": "smtp.example.com", "smtpPort": 587, "smtpUsername": "...", "smtpPassword": "..." }'
```

TLS behavior: implicit TLS is used on port 465; otherwise STARTTLS. Once saved, password-reset emails work immediately — no restart needed.

## App settings

```json
{
  "appName": "Crabbase",
  "appUrl": "http://localhost:8080",
  "contactEmail": "admin@crabbase.local",
  "allowPublicUserRegistration": false
}
```

- `appUrl` feeds template variables such as links in emails.
- `allowPublicUserRegistration` exists as a setting but is **not enforced yet** (see README roadmap).

`GET /api/settings/app` returns `204 No Content` if nothing has been saved yet.

## Email templates

Two templates are seeded by migrations and fully editable:

- `passwordReset` — used by the forgot-password flow
- `userRegistration` — reserved for the upcoming verification/registration flow

```sh
curl http://localhost:8989/api/settings/email-templates -H "Authorization: Bearer $TOKEN"
```

Each template has HTML and plain-text variants. Simple `{{variable}}` placeholders are rendered at send time.

### Template variables

| Variable | Meaning |
|---|---|
| `{{name}}` | Recipient's name/email |
| `{{email}}` | Recipient address |
| `{{otp}}` | The one-time code (password reset) |
| `{{link}}` | Action link built from app URL |
| `{{app_name}}` | From app settings |
| `{{app_url}}` | From app settings |

Example snippet:

```html
<p>Hi {{name}},</p>
<p>Your reset code is <strong>{{otp}}</strong>. It expires in 5 minutes.</p>
<p><a href="{{link}}">Reset your password</a></p>
```

Save via `POST /api/settings/email-templates` with the full `EmailTemplates` payload you got from the `GET`.

The admin dashboard provides editors for all of these under **Settings → General / Mail / Email Templates**.
