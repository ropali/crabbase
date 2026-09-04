DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/crabbase

.PHONY: validate run release build test test-unit test-db test-integration test-matrix watch serve admin

validate:
	cargo check
	cargo build

# Run the default subcommand (serve)
run:
	cargo run -- serve --config crabbase.toml

# Start the API server
serve:
	cargo run -- serve --config crabbase.toml

# Start the admin dashboard
admin:
	cargo run -- admin --config crabbase.toml

release:
	CRABBASE_API_URL="http://0.0.0.0:8989/api" cd crates/admin-ui && trunk build --release
	cargo build --release

build:
	cargo build

# Run unit tests (instant, in-memory, no database required)
test-unit:
	@./scripts/run_tests.sh unit

# Run database repository tests (Postgres schema-isolated)
test-db:
	@DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh db

# Run HTTP API integration tests (requires PostgreSQL)
# Usage:
#   make test-integration
#   make test-integration suite=records
#   make test-integration suite=rules
#   make test-integration suite=auth
#   make test-integration suite=validation
#   make test-integration suite=types
test-integration:
	@if [ -n "$(suite)" ]; then \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh $(suite); \
	else \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh api; \
	fi

# Run Crabbase Feature Matrix report
# Usage:
#   make test-matrix           # runs all tests and prints live Feature Matrix
#   make test-matrix no-run=1  # prints Feature Matrix without running tests
test-matrix:
	@if [ "$(NO_RUN)" = "1" ] || [ "$(no-run)" = "1" ]; then \
		./scripts/run_tests.sh matrix --no-run; \
	else \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh matrix; \
	fi

# General test runner (all tests formatted beautifully)
test:
	@if [ -n "$(crate)" ] || [ -n "$(suite)" ]; then \
		TARGET="$(crate)$(suite)"; \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh $$TARGET; \
	else \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh all; \
	fi

watch:
	RUST_BACKTRACE=1 RUSTFLAGS=-Awarnings RUST_LOG=info bacon run -- serve --config crabbase.toml

watch-fe:
	@cd crates/admin-ui && trunk serve
