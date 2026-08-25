# Getting started

This guide gets Crabbase running locally so you can use it as a ready-made backend — no custom REST code required. The only infrastructure Crabbase needs is a PostgreSQL server (yours): everything else — API, admin dashboard, migrations — ships inside the single binary.

> **Evolving project:** flag names, defaults, and response shapes may change between versions. When in doubt, check the live spec at `GET /api/openapi.json` or the Swagger UI at `GET /api/docs`.

## Prerequisites

- **Rust** (stable toolchain) — <https://rustup.rs>
- **PostgreSQL 14+** running locally or reachable via network
- `psql` on PATH (used by helper scripts; not required to run the server)

Only rebuild the admin dashboard if you modify it (requires [Trunk](https://trunkrs.dev) and the `wasm32-unknown-unknown` target). The release binary embeds a prebuilt dashboard.

## Configuration

Crabbase reads environment variables from the process environment, loading an optional `.env` file from the working directory first.

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/crabbase` | PostgreSQL connection string |
| `ADMIN_PASSWORD` | *(none)* | Password for the superuser. **Required on first boot** — startup fails without it when creating the initial superuser |
| `ADMIN_USERNAME` | `admin@crabbase.local` | Superuser email |
| `SERVER_BIND_ADDR` | `0.0.0.0:8989` | API bind address (overridden by `serve --host/--port`) |
| `ADMIN_BIND_ADDR` | `0.0.0.0:8181` | Admin dashboard bind address (overridden by `admin --host/--port`) |
| `MAX_DB_CONN` | `10` | Max DB pool connections *(parsed but not yet applied to the pool)* |
| `RUST_LOG` | `info` | Log filter for console output (`debug`, `trace`, ...) |

Minimal `.env`:

```dotenv
DATABASE_URL=postgres://postgres:postgres@localhost:5432/crabbase
ADMIN_PASSWORD=change-me
```

## Running

Using Make targets:

```sh
make serve                 # API only,        http://0.0.0.0:8989
make admin                 # Dashboard + API, http://0.0.0.0:8181
make serve port=9000 host=127.0.0.1
```

Or with cargo directly:

```sh
cargo run -- serve --port 8989 --host 0.0.0.0
cargo run -- admin --port 8181
```

### The two commands

| Command | What it serves |
|---|---|
| `crabbase serve` | The REST API under `/api` |
| `crabbase admin` | The REST API under `/api` **plus** the embedded admin dashboard SPA at `/` |

Both commands run migrations and ensure the superuser exists before listening. You can run both at once against the same database; in practice `admin` alone gives you everything.

### What happens on boot

1. Logging initializes — pretty console logs filtered by `RUST_LOG`, plus daily-rotating JSON logs written to `logs/app.log`.
2. Config is loaded (`.env` → environment).
3. A connection pool is created and connectivity is verified (`SELECT 1`) with a helpful error if Postgres is unreachable.
4. Embedded SQL migrations run automatically — no external migration step needed.
5. The superuser is created/repaired from `ADMIN_USERNAME`/`ADMIN_PASSWORD`.

## First steps

1. Open the dashboard at `http://localhost:8181` and log in as the superuser.
2. Create a collection (e.g. `posts`) with a few fields using the **Create Collection** drawer.
3. Your API is live immediately:

```sh
curl http://localhost:8989/api/collections/posts/records
```

4. Explore the generated API interactively at `http://localhost:8181/api/docs`.

The dashboard can run **separately** from your public API server — both are stateless against Postgres. A common setup is one `crabbase admin` process on an internal host/port for you, and one or more `crabbase serve` processes facing users; they all share the same database, so changes made in the dashboard are instantly live on every API instance.

From here, everything (collections, records, user creation, schemas, rules, settings) can be managed through the UI — see [Collections & records](collections-and-records.md) and [Authentication](authentication.md) for the UI walkthroughs plus their REST equivalents.

## Makefile reference

| Target | Purpose |
|---|---|
| `make build` | Debug build |
| `make validate` | `cargo check && cargo build` |
| `make run` | Run the default subcommand (`serve`) |
| `make serve [port= host=]` | Start API server |
| `make admin [port= host=]` | Start admin dashboard + API |
| `make test` | Test whole workspace |
| `make test crate=db` | Test one crate (`crabbase_` prefix added automatically) |
| `make watch` | Auto-reload dev loop via bacon (`RUST_LOG=info`) |
| `make watch-fe` | Trunk dev server for the admin UI |
| `make release` | Build admin UI (`trunk build --release`) then release binary |

## Troubleshooting

- **"panicked ... ADMIN_PASSWORD"** — set `ADMIN_PASSWORD` in `.env` before first boot.
- **Cannot connect to Postgres** — verify `DATABASE_URL` and that the database exists: `createdb crabbase`.
- **Port already in use** — pass `--port`/`port=` to move the server.
