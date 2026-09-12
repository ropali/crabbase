# Crabbase

**A backend-as-a-service in a single binary. PocketBase's simplicity, PostgreSQL's power.**

Crabbase follows one philosophy: **the whole backend in one static binary — but bring your own PostgreSQL.** You get the same "unzip, run, done" experience as [PocketBase](https://pocketbase.io), with none of its scaling ceiling: instead of an embedded SQLite file, Crabbase runs entirely on a Postgres server *you* choose and operate.

What that means in practice:

- **Single binary, zero code.** Define collections through the admin dashboard or REST API, and instant CRUD endpoints, JWT auth, password-reset emails, and settings appear — no custom routes, no redeployments.
- **Bring your own PostgreSQL.** Set the `[database]` section in `crabbase.toml` to point at any Postgres you already run — local, managed (RDS, Cloud SQL, Supabase, Neon), or self-hosted. Your data lives in ordinary Postgres tables you can inspect, back up, and query with `psql` at any time.
- **Scales like your database.** Because state lives in Postgres, scaling is the solved problem of scaling Postgres: vertical sizing, read replicas, connection poolers (PgBouncer), managed backups and HA — not bespoke tooling. Run multiple Crabbase instances against the same database; they share everything by construction.
- **No lock-in.** Every collection is a real table, every record a real row. Leave whenever you want — your schema and data are already portable standard SQL.

You create "collections" (real Postgres tables) through the admin dashboard or the collections API, and Crabbase immediately exposes production-style CRUD endpoints, JWT authentication per collection, password-reset flows with OTP over email, and a settings system — so your frontend can talk to a complete backend without you writing a single custom route.

> **Status: evolving / pre-1.0.** Core features (collections, records, auth, settings, email) work; file storage, realtime, and hooks are not implemented yet (see [Roadmap & limitations](#roadmap--limitations)). Expect breaking changes between versions.

---

## Features

- **Single static binary** — API server, admin dashboard, migrations, and email templating all compiled into one executable. Deploy = copy a file.
- **Bring your own PostgreSQL** — the only infrastructure dependency. Works with any Postgres 14+, local or managed. All state (data, auth sessions, settings, logs) lives in your database.
- **Dynamic collections** — create Postgres tables at runtime via admin UI or API. Schema changes (add/drop/rename/retype columns, indexes) are applied live.
- **Instant CRUD REST API** — every collection automatically gets list/create/read/update/delete endpoints at `/api/collections/{name}/records`.
- **Dynamic record filtering (`?filter`)** — PocketBase-compatible client filtering syntax (`status='active' && price < 100`, `title ~ 'pocket'`, etc.) compiled to parameterized SQL.
- **Relation expansion (`?expand`)** — batch-expand foreign-key relations inline across list and single-record endpoints (e.g. `?expand=author,category`) in 1 query per relation without N+1 queries.
- **Rich field types** — text, rich text, number, bool, email, URL, datetime, auto-datetime, JSON, select, file (path), geo point, and relations between collections.
- **Per-collection auth** — mark a collection as an `auth` collection and its records can log in. Passwords are bcrypt-hashed and JWT secrets are generated automatically.
- **JWT sessions with rotation** — access + refresh tokens, refresh-token rotation with reuse detection and family revocation.
- **Password reset via email OTP** — templated emails rendered from configurable templates (`{{otp}}`, `{{link}}`, `{{app_name}}`, ...).
- **API rules** — PocketBase-style filter rules (`owner = @request.auth.id && status = 'active'`) compiled into safe parameterized SQL.
- **Admin dashboard** — embedded Yew/WASM SPA for managing collections, records (including search/filtering and user creation), schemas, API rules, settings, email templates, and logs. Runs bundled with the API or as its own process.
- **OpenAPI + Swagger UI** — machine-readable spec at `/api/openapi.json`, browsable docs at `/api/docs`.

## Architecture

```
crabbase/            Main binary (CLI: serve | admin)
crates/
  core/              Config, models, filter-rule parser/compiler, logging
  db/                Repositories: dynamic collections, records, auth, settings
  api/               Axum REST router (all HTTP endpoints)
  auth/              JWT service, refresh-token rotation, OTP/password reset
  files/             (placeholder) File storage
  realtime/          (placeholder) WebSockets / sync
  hooks/             (placeholder) Background hooks
  admin-ui/          Yew admin dashboard, embedded into the binary
migrations/          SQL migrations (embedded in the binary, run at boot)
```

Everything is compiled into one statically-linked binary — the only external dependency is a PostgreSQL server. The `admin` command serves both the dashboard and the full API on one port.

```
┌───────────────────────┐        ┌──────────────────────────┐
│  crabbase (1 binary)  │        │   Your PostgreSQL         │
│  ─ API + admin UI ─   │ ─────► │   (yours to operate:     │
│  embedded migrations, │  TCP   │    backups, replicas,    │
│  dashboard, templates │        │    pooling, HA, ...)      │
└───────────────────────┘        └──────────────────────────┘
```

Because all state lives in Postgres (data, sessions, settings, logs), scaling Crabbase means scaling Postgres — and you can run several Crabbase instances against the same database with no extra coordination.

## Quickstart

### Prerequisites

- Rust (stable) — <https://rustup.rs> (only for building; a release binary needs nothing)
- **Any PostgreSQL 14+ server** — local, Docker, or a managed instance. This is the whole infrastructure requirement.
- `psql` CLI available on PATH

For the prebuilt admin dashboard flow you only need `cargo`; rebuilding the dashboard additionally needs [Trunk](https://trunkrs.dev) and the WASM target.

### 1. Configure with `crabbase.toml`

Crabbase is configured entirely through a **TOML file** (default: `crabbase.toml` in the project root). Copy the example and edit it:

```sh
cp exampl.crabbase.toml crabbase.toml
```

The file has four sections:

```toml
[database]
host = "localhost"
port = 5432
user = "postgres"
password = "postgres"   # prefix with $ to load from an env var, e.g. $DB_PASSWORD
database = "crabbase"
schema = "crabbase"
max_connections = 10

[server]
api_host = "0.0.0.0"
api_port = 8989
admin_host = "0.0.0.0"
admin_port = 9898

[admin]
host = "0.0.0.0"
port = 9898

# Superusers to create/ensure on first boot (array of tables)
[[initial_users]]
email = "admin@crabbase.local"
name = "admin"
password = "change-me"   # or $ADMIN_PASSWORD to pull from env
```

**Secret values** — any string value that starts with `$` is treated as an environment variable reference. For example:

```toml
password = "$DB_PASSWORD"   # reads the DB_PASSWORD env var at startup
```

This keeps credentials out of the config file while still letting you commit a sanitised version to version control.

**Custom config path** — pass `--config <path>` to either subcommand if you want to keep the file somewhere else:

```sh
cargo run -- serve --config /etc/crabbase/prod.toml
cargo run -- admin --config /etc/crabbase/prod.toml
```

### 2. Run

```sh
make serve        # API server  →  http://0.0.0.0:8989
make admin        # Admin dashboard + API  →  http://0.0.0.0:9898
```

Or directly (the `--config` flag defaults to `crabbase.toml` in the current directory):

```sh
cargo run -- serve --config crabbase.toml
cargo run -- admin --config crabbase.toml
```

On startup Crabbase reads the TOML file, connects to Postgres, runs all pending migrations, ensures the `initial_users` superusers exist, and prints the banner.

### 3. Open the dashboard

Visit `http://localhost:8181` and log in with the superuser credentials (`admin@crabbase.local` / your `ADMIN_PASSWORD`). **Almost everything can be done here without touching curl or code:**

- **Create collections** — pick a name, type (`base`/`auth`), add fields of any type, toggle indexes
- **Manage records** — browse, create, edit, delete data in any collection; this includes **creating users** in auth collections
- **Edit schemas live** — add/remove/retype fields, set API rules per collection
- **Configure the app** — general settings, SMTP mail settings, email templates
- **View logs** — activity logs

The dashboard is a client of the same REST API your apps use — anything you click in the UI is also doable over HTTP.

> The admin command runs the dashboard *and* the full API on one port, so it can be your only process. But since both are stateless against Postgres, you can also run `admin` separately — e.g. keep the dashboard on an internal port/host while public traffic hits `serve`.

### 4. Or use pure curl

```sh
# Login as superuser
TOKEN=$(curl -s -X POST http://localhost:8989/api/auth/_superusers/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"admin@crabbase.local","password":"change-me"}' \
  | jq -r .tokens.accessToken)

# Create a collection -> a real table + instant REST endpoints
curl -X POST http://localhost:8989/api/collections \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{
    "name": "products",
    "columns": [
      { "name": "title",  "data_type": "PlainText", "index": true },
      { "name": "price",  "data_type": "Number" },
      { "name": "active", "data_type": "Bool" }
    ]
  }'

# Create a record — no deployment, no code
curl -X POST http://localhost:8989/api/collections/products/records \
  -H 'Content-Type: application/json' \
  -d '{"data":{"title":"Widget","price":999,"active":true}}'

# List records (with pagination & filtering)
curl 'http://localhost:8989/api/collections/products/records?page=1&per_page=20'
curl -G 'http://localhost:8989/api/collections/products/records' \
  --data-urlencode "filter=price < 1000 && active = true"

# List records with expanded relations (no N+1 queries)
curl 'http://localhost:8989/api/collections/books/records?expand=author'
```

## Documentation

| Doc | Contents |
|---|---|
| [Getting started](docs/getting-started.md) | Installation, configuration, env vars, running, Makefile targets |
| [Collections & records](docs/collections-and-records.md) | Field types, creating/updating schemas, CRUD endpoints, dynamic `?filter`, relation `?expand`, API rules |
| [Authentication](docs/authentication.md) | Auth collections, login/refresh/logout, password reset, superusers |
| [Settings & email](docs/settings-email.md) | App/mail settings, SMTP setup, email templates |
| **Interactive API docs** | Run the server → `GET /api/docs` (Swagger UI), spec at `/api/openapi.json` |

## Using Crabbase as your backend

The intended workflow requires **no custom REST code** — and for most of it, no curl either. The admin dashboard is the primary tool:

1. **Model your data as collections** — in the dashboard's Create Collection drawer (or `POST /api/collections`). Relations are first-class.
2. **Create users and seed data** — open an auth collection in the dashboard and add user records directly; passwords are hashed for you. Same for any other collection's initial data.
3. **Consume the generated endpoints** from your frontend/mobile app:
   - `GET/POST /api/collections/{name}/records`, `GET/PATCH/DELETE .../records/{id}` (with `?filter=...` and `?expand=...` support)
   - For auth collections: `POST /api/auth/{collection}/login`, `/auth-refresh`, `/logout`
4. **Protect data with API rules** — set `list_rule`, etc. in the collection's edit drawer, e.g. `owner = @request.auth.id` so users only see their own rows.
5. **Configure email** once in Settings → Mail, and password resets just work out of the box.

Every step has a REST equivalent documented in [docs/](docs/) if you prefer automation or IaC.

## Deploying & scaling

*Coming Soon*

## Development

```sh
make build                  # debug build
make test                   # cargo test --workspace
make test crate=db          # tests for a single crate
make watch                  # bacon watch + auto-reload (serve)
make watch-fe               # trunk dev server for the admin UI
make release                # optimized release build (builds admin UI first)
```

Utility scripts for load testing:

```sh
scripts/create_test_collections.sh --count 20 --rows 100   # generate test collections + data
scripts/pg_seed.sh --table _logs --rows 500                # seed any table
```

## Roadmap & limitations

Implemented and working:

- Collections (create/update/delete/truncate, live schema migration, indexes)
- Records CRUD with pagination, client `?filter` dynamic query evaluation, batch relation expansion (`?expand`), and `list_rule` / `view_rule` enforcement
- Auth collections, JWT login/refresh/logout, password reset with emailed OTP
- Superusers, settings APIs, email templates, admin dashboard (with real-time records filter search)
- Swagger UI / OpenAPI

Not yet (placeholders exist in the codebase):

- **File uploads/storage** — the `File` column type stores a path string only; there is no upload endpoint.
- **Realtime** — no websockets/SSE subscriptions.
- **Hooks** — no background triggers.
- **Public registration / email verification endpoints** — create users by POSTing records to an auth collection; the `allowPublicUserRegistration` setting exists but is not enforced yet.
- `create_rule`, `update_rule`, and `delete_rule` are stored but not evaluated yet (`list_rule` and `view_rule` are enforced).

Security notes while the project evolves:

- Rotate the seeded `_superusers` token secret (migration `00001` seeds a hardcoded value).
- CORS is fully permissive in the current API server — tighten before exposing publicly.
- Change `ADMIN_PASSWORD` immediately after first boot.
