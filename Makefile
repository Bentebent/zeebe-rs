build-debug:
	cargo build

build-release:
	cargo build --release

fmt-check:
	cargo +nightly fmt -- --check

test:
	cargo test --verbose

lint:
	cargo clippy -- -D warnings