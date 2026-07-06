.PHONY: lint fmt fmt-check install uninstall

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

install:
	cargo install --path ./zcd --locked

uninstall:
	cargo uninstall zcd

