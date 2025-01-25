build-debug:
	cargo build

build-release:
	cargo build --release

examples:
	cargo build --examples

fmt-check:
	cargo +nightly fmt -- --check

test:
	cargo test --verbose

lint:
	cargo clippy -- -D warnings