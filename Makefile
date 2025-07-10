.PHONEY: lint

lint:
	cargo clippy --all-targets --all-features -- -D warnings

