# Getting started

This guide gets Crabbase running locally so you can use it as a ready-made backend — no custom REST code required. The only required external service is a PostgreSQL server (yours): everything else — API, admin dashboard, migrations — ships inside the single binary.

> **Evolving project:** flag names, defaults, and response shapes may change between versions. The Swagger UI (`GET /api/docs`) and OpenAPI spec (`GET /api/openapi.json`) are available in debug builds only; release builds exclude them.

## Prerequisites

- **Rust** (stable toolchain) — <https://rustup.rs>
- **PostgreSQL 14+** running locally or reachable via network
- `psql` (optional — only the helper scripts use it)

The dashboard's compiled JS/WASM assets are build artifacts (only the HTML shell is committed), so a fresh checkout must build them once with [Trunk](https://trunkrs.dev) and the `wasm32-unknown-unknown` target:

```sh
cd crates/admin-ui && trunk build
```

`make release` does this automatically. The API-only `serve` command never needs the dashboard assets.

## Configuration

Crabbase is configured with a **TOML file** — default `crabbase.toml` in the working directory. Pass `--config <path>` to either subcommand to use a different file. Start from the example:

```sh
cp example.crabbase.toml crabbase.toml
```

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
host = "0.0.0.0"
port = 8989

[admin]
host = "0.0.0.0"
port = 9898

# Superusers to create if missing on first boot (array of tables)
[[initial_users]]
email = "admin@crabbase.local"
name = "admin"
password = "admin123"   # or $ADMIN_PASSWORD to pull from env
```

**Environment references:** the `password` fields (`[database]` and each `[[initial_users]]` entry) treat a value starting with `$` as an environment variable reference. For example, `password = "$DB_PASSWORD"` reads `DB_PASSWORD` at startup and fails with a clear error if it is unset.

**Ports and bind addresses:** change `[server].host` / `[server].port` for the API and `[admin].host` / `[admin].port` for the dashboard. There are no `--host`/`--port` flags.

**Connection pool:** `max_connections` is parsed but not yet applied to the pool.

## Running

Using Make targets:

```sh
make serve                 # API only,        http://0.0.0.0:8989
make admin                 # Dashboard + API, http://0.0.0.0:9898
```

`admin` serves its own API on the admin port (9898); the curl examples below use the standalone `serve` instance on 8989. Run both processes (they share Postgres) if you want both URLs.

Or with cargo directly:

```sh
cargo run -- serve --config crabbase.toml
cargo run -- admin --config crabbase.toml
```

If this is a fresh checkout, build the dashboard assets first (see above) before using `make admin`.

### The two commands

| Command | What it serves |
|---|---|
| `crabbase serve` | The REST API under `/api` |
| `crabbase admin` | The REST API under `/api` **plus** the embedded admin dashboard SPA at `/` |

Both commands run migrations and create any missing `initial_users` superusers before listening. You can run both at once against the same database; in practice `admin` alone gives you everything.

### What happens on boot

1. Logging initializes — pretty console logs filtered by `RUST_LOG`, plus daily-rotating JSON logs written to `logs/app.log`.
2. The TOML config is loaded.
3. A connection pool is created and connectivity is verified (`SELECT 1`) with a helpful error if Postgres is unreachable.
4. Embedded SQL migrations run automatically — no external migration step needed.
5. Any missing `[[initial_users]]` superusers are created.
6. The startup banner is printed.

## First steps

1. Open the dashboard at `http://localhost:9898` and log in with the email/password from `[[initial_users]]`.
2. Create a collection (e.g. `posts`) with a few fields using the **Create Collection** drawer. Collections start with no API rules, so set **List/View** to **Everyone (public)** in the **Edit Schema** drawer if you want unauthenticated reads.
3. Your API is live immediately:

```sh
curl http://localhost:8989/api/collections/posts/records
```

4. In a debug build, explore the generated API interactively at `http://localhost:9898/api/docs`.

The dashboard can run **separately** from your public API server — both keep application state in Postgres (use S3-compatible storage or a shared volume if you run multiple instances with local file storage). A common setup is one `crabbase admin` process on an internal host/port for you, and one or more `crabbase serve` processes facing users; they all share the same database, so changes made in the dashboard are instantly live on every API instance.

From here, everything (collections, records, user creation, schemas, rules, settings) can be managed through the UI — see [Collections & records](collections-and-records.md) and [Authentication](authentication.md) for the UI walkthroughs plus their REST equivalents.

## Makefile reference

| Target | Purpose |
|---|---|
| `make build` | Debug build |
| `make validate` | `cargo check && cargo build` |
| `make run` | Run `serve` with `crabbase.toml` |
| `make serve` | Start API server |
| `make admin` | Start admin dashboard + API |
| `make test` | Full suite: unit + DB + HTTP API (needs Postgres) |
| `make test-unit` | Unit tests only (no database) |
| `make test-db` | DB repository tests |
| `make test-integration` | HTTP API integration tests |
| `make test suite=records` | One API test suite (`records`, `rules`, `auth`, `validation`, `types`, `settings`, ...) |
| `make test-matrix` | Run all suites and print the feature matrix (`no-run=1` to skip execution) |
| `make test-raw` | Raw `cargo test --workspace` with full output |
| `make watch` | Auto-reload dev loop via bacon (`serve`) |
| `make watch-admin` | Auto-reload dev loop via bacon (`admin`) |
| `make watch-fe` | Trunk dev server for the admin UI |
| `make release` | Build admin UI (`trunk build --release`) then release binary |
| `make db-drop` | Drop Crabbase tables (add `force=1` to confirm) |

## Troubleshooting

- **Config file not found / invalid TOML** — check the path passed to `--config` and the `[database]`, `[server]`, and `[admin]` sections.
- **`env variable ... not set`** — a `$VAR` reference in a password field points at an unset environment variable.
- **Cannot connect to Postgres** — verify the `[database]` host/port/user/password/database and that the server is running.
- **Port already in use** — change `[server].port` or `[admin].port` in `crabbase.toml`.
- **Dashboard is blank / assets 404** — build the dashboard with `cd crates/admin-ui && trunk build` (or `make release`).
- **`/api/docs` returns 404** — Swagger/OpenAPI are excluded from release builds; run a debug build.
- **Login fails** — use the email/password configured in `[[initial_users]]` (or the environment variable it references); editing that file does not update an account created earlier.
