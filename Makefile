.PHONY: help build test lint fmt clean check release docs

help:
	@echo "GXX Development Tasks"
	@echo "===================="
	@echo "  make build       - Build the project"
	@echo "  make test        - Run all tests"
	@echo "  make lint        - Run clippy linter"
	@echo "  make fmt         - Format code with rustfmt"
	@echo "  make fmt-check   - Check formatting without changes"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make check       - Run cargo check"
	@echo "  make release     - Build release binary"
	@echo "  make docs        - Generate documentation"
	@echo "  make all         - Run all checks and tests"

build:
	cargo build

test:
	cargo test --verbose

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt -- --check

clean:
	cargo clean

check:
	cargo check --all-features

release:
	cargo build --release

docs:
	cargo doc --no-deps --open

all: check lint test

install:
	cargo install --path .

uninstall:
	cargo uninstall gxx
