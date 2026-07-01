# Test Infrastructure for Eggsec
# ================================

.PHONY: test test-fast test-slow test-unit test-integration test-nse test-coverage test-ci test-feature-matrix check-no-default check-feature-profiles clean help

# Default: run unit tests only (fast feedback loop)
test: test-unit

# Run only unit tests (lib tests, no network, no wiremock)
test-unit:
	cargo nextest run -p eggsec --lib

# Run full test suite with no retries (CI-style)
test-ci:
	cargo nextest run -p eggsec --retries 0 --no-fail-fast

# Run integration tests (uses wiremock, may need network)
test-integration:
	cargo nextest run -p eggsec --test '*.rs'

# Run NSE tests (requires nse feature)
test-nse:
	cargo nextest run -p eggsec --features nse --test nse_tests --test nse_integration_tests

# Run slow/explicitly-ignored tests
test-slow:
	cargo nextest run -p eggsec --run-ignored ignored-only

# Run clippy
clippy:
	cargo clippy --lib -p eggsec -- -D warnings

# Run format check
fmt:
	cargo fmt --all -- --check

# Run code coverage
test-coverage:
	cargo llvm-cov -p eggsec --features rest-api,nse --lcov --output-dir coverage

# Build release
build:
	cargo build --release -p eggsec-cli

# Feature matrix and metadata validation
test-feature-matrix:
	cargo test -p eggsec --test feature_matrix
	cargo test -p eggsec --test metadata_consistency

# Validate no-default-features build
check-no-default:
	cargo check --workspace --no-default-features

# Representative feature profile checks (representative, not exhaustive)
check-feature-profiles:
	cargo check -p eggsec --no-default-features
	cargo check -p eggsec --features rest-api
	cargo check -p eggsec --features db-pentest
	cargo check -p eggsec --features mobile
	cargo check -p eggsec --features web-proxy
	cargo check -p eggsec --features wireless
	cargo check -p eggsec --features nse
	cargo check -p eggsec --features evasion
	cargo check -p eggsec --features postex
	cargo check -p eggsec --features c2

# Clean build artifacts
clean:
	cargo clean

# Help
help:
	@echo "Test targets:"
	@echo "  make test            - Run unit tests only (default)"
	@echo "  make test-fast       - Same as test"
	@echo "  make test-ci         - Full suite, no retries"
	@echo "  make test-integration - Integration tests"
	@echo "  make test-nse        - NSE tests (requires nse feature)"
	@echo "  make test-slow       - Run ignored tests"
	@echo "  make test-coverage   - Code coverage"
	@echo "  make clippy          - Lint"
	@echo "  make fmt             - Format check"
	@echo "  make build           - Release build"
	@echo "  make test-feature-matrix - Feature metadata validation tests"
	@echo "  make check-no-default   - Validate no-default-features build"
	@echo "  make check-feature-profiles - Representative feature profile checks"
	@echo "  make clean           - Clean artifacts"
