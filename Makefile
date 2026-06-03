.PHONY: validate run release build test watch serve admin

validate:
	cargo check
	cargo build

# Run the default subcommand (serve)
run:
	cargo run -- serve

# Start the API server
# Usage:
#   make serve
#   make serve port=9000 host=127.0.0.1
serve:
	cargo run -- serve $(if $(port),--port $(port)) $(if $(host),--host $(host))

# Start the admin dashboard
# Usage:
#   make admin
#   make admin port=8181 host=127.0.0.1
admin:
	cargo run -- admin $(if $(port),--port $(port)) $(if $(host),--host $(host))

release:
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
	RUSTFLAGS=-Awarnings RUST_LOG=info bacon run -- serve
