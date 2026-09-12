DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/crabbase

.PHONY: validate run release build test test-unit test-db test-integration test-matrix test-raw test-raw-unit test-raw-integration cargo-test watch serve admin

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

# ==============================================================================
# 🦀 FORMATTED TEST RUNNER & FEATURE MATRIX (Human-Readable Output)
# ==============================================================================

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

# General formatted test runner
# Usage:
#   make test
#   make test suite=records
#   make test suite=rules
test:
	@if [ -n "$(crate)" ] || [ -n "$(suite)" ]; then \
		TARGET="$(crate)$(suite)"; \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh $$TARGET; \
	else \
		DATABASE_URL="$(DATABASE_URL)" ./scripts/run_tests.sh all; \
	fi

# ==============================================================================
# 🔍 RAW CARGO TEST RUNNERS (Full Rust assertion failures, left/right diffs & backtraces)
# ==============================================================================

# Run raw unit tests directly via cargo test
# Usage:
#   make test-raw-unit
#   make test-raw-unit test=test_compile_logical_expressions
test-raw-unit:
	RUST_BACKTRACE=1 cargo test -p crabbase_core $(if $(test),-- $(test),-- --nocapture)

# Run raw HTTP API integration tests directly via cargo test
# Usage:
#   make test-raw-integration
#   make test-raw-integration suite=access_rules_test
#   make test-raw-integration suite=records_api_test test=test_update_record_returns_full_record_json
#   make test-raw-integration test=test_list_rule_null_requires_admin
test-raw-integration:
	@if [ -n "$(suite)" ]; then \
		RUST_BACKTRACE=1 TEST_DATABASE_URL="$(TEST_DATABASE_URL)" cargo test -p crabbase_api --test $(suite) $(if $(test),-- $(test),-- --nocapture); \
	elif [ -n "$(test)" ]; then \
		RUST_BACKTRACE=1 TEST_DATABASE_URL="$(TEST_DATABASE_URL)" cargo test -p crabbase_api --tests --no-fail-fast -- $(test) --nocapture; \
	else \
		RUST_BACKTRACE=1 TEST_DATABASE_URL="$(TEST_DATABASE_URL)" cargo test -p crabbase_api --tests --no-fail-fast -- --nocapture; \
	fi

# Run raw workspace tests with full Rust panic messages & backtraces
# Usage:
#   make test-raw
#   make test-raw test=test_name
#   make test-raw crate=crabbase_core
#   make test-raw crate=crabbase_db
#   make test-raw crate=crabbase_auth
#   make test-raw suite=access_rules_test
test-raw:
	@if [ -n "$(crate)" ]; then \
		RUST_BACKTRACE=1 DATABASE_URL="$(DATABASE_URL)" cargo test -p $(crate) $(if $(test),-- $(test),-- --nocapture); \
	elif [ -n "$(suite)" ]; then \
		RUST_BACKTRACE=1 DATABASE_URL="$(DATABASE_URL)" cargo test -p crabbase_api --test $(suite) $(if $(test),-- $(test),-- --nocapture); \
	elif [ -n "$(test)" ]; then \
		RUST_BACKTRACE=1 DATABASE_URL="$(DATABASE_URL)" cargo test --workspace --no-fail-fast -- $(test) --nocapture; \
	else \
		RUST_BACKTRACE=1 DATABASE_URL="$(DATABASE_URL)" cargo test --workspace --no-fail-fast -- --nocapture; \
	fi

# Alias for test-raw
cargo-test: test-raw

watch:
	RUST_BACKTRACE=1 RUSTFLAGS=-Awarnings RUST_LOG=info bacon run -- serve --config crabbase.toml

watch-fe:
	@cd crates/admin-ui && trunk serve
