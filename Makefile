.PHONY: validate run release build test

validate:
	cargo check
	cargo build

run:
	cargo run

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
	RUSTFLAGS=-Awarnings RUST_LOG=info bacon run -- --quiet
