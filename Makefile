.PHONY: validate run release build test watch serve admin

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

test:
	@if [ -n "$(crate)" ]; then \
		CRATE_NAME="$(crate)"; \
		case "$$CRATE_NAME" in \
			crabbase_*) ;; \
			*) CRATE_NAME="crabbase_$$CRATE_NAME" ;; \
		esac; \
		cargo test -p $$CRATE_NAME; \
	else \
		cargo test --workspace; \
	fi

watch:
	RUST_BACKTRACE=1 RUSTFLAGS=-Awarnings RUST_LOG=info bacon run -- serve --config crabbase.toml

watch-fe:
	@cd crates/admin-ui && trunk serve
