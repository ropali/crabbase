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
	cargo test
