HOST_TARGET := $(shell rustc +stable -vV | sed -n 's/host: //p')

.PHONY: test build fmt fmt-check clippy

test:
	cargo +stable test --no-default-features --target $(HOST_TARGET)

build:
	cargo build

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings
