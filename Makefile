.PHONY: build test lint clean check run

MANIFEST := src-tauri/Cargo.toml

build:
	cargo build --manifest-path $(MANIFEST) --locked --release

check:
	cargo check --manifest-path $(MANIFEST) --locked

test:
	cargo test --manifest-path $(MANIFEST) --locked

lint:
	cargo clippy --manifest-path $(MANIFEST) --locked -- -D warnings

run:
	cargo run --manifest-path $(MANIFEST) --locked

clean:
	cargo clean --manifest-path $(MANIFEST)
