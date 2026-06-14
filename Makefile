.PHONY: build release test fmt lint run clean

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

run:
	cargo run -- $(ARGS)

clean:
	cargo clean
