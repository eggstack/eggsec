# Test Infrastructure for Eggsec
# ================================

.PHONY: test test-fast test-slow test-unit test-integration test-nse test-coverage test-ci test-feature-matrix test-architecture-guards check-no-default check check-python check-full check-feature-profiles release-check clippy fmt build clean help

# Default: run unit tests only (fast feedback loop)
test: test-unit

# Compatibility alias for the default fast unit-test loop
test-fast: test-unit

# Run only unit tests (lib tests, no network, no wiremock)
test-unit:
	cargo test --lib -p eggsec

# Run the eggsec package test suite without Cargo fail-fast
test-ci:
	cargo test -p eggsec --features rest-api --tests --no-fail-fast

# Run integration tests (alias for test-ci; uses wiremock, may need network)
test-integration: test-ci

# Run NSE tests (requires nse feature)
test-nse:
	cargo test -p eggsec --features nse --test nse_tests --test nse_integration_tests

# Run slow/explicitly-ignored tests
test-slow:
	cargo test -p eggsec --features rest-api --tests -- --ignored

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

# Architecture drift guards (static grep checks)
test-architecture-guards:
	bash scripts/check-architecture-guards.sh

# Validate no-default-features build
check-no-default:
	cargo check --workspace --no-default-features

# Validate MSRV (requires `rustup toolchain install 1.88`)
check-msrv:
	cargo +1.88 check --workspace --no-default-features
	cargo +1.88 check -p eggsec-cli --no-default-features

# Full mandatory Rust CI contract (no cargo-nextest required)
check:
	cargo fmt --all --check
	cargo check --workspace --no-default-features
	cargo clippy --lib -p eggsec -- -D warnings
	cargo test -p eggsec --doc
	cargo test -p eggsec --no-default-features --test tool_registration --test loadtest_tests --no-fail-fast
	cargo test -p eggsec --features rest-api --tests --no-fail-fast
	cargo test -p eggsec-output --tests
	bash scripts/check-architecture-guards.sh

# Optional broad validation (pre-release, not required for merge)
check-full: check
	cargo deny check
	$(MAKE) check-feature-profiles

# Representative feature profile checks (representative, not exhaustive)
check-feature-profiles:
	cargo check -p eggsec --features tool-api,rest-api
	cargo check -p eggsec --features grpc-api
	cargo check -p eggsec --features db-pentest
	cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
	cargo check -p eggsec --features mobile
	cargo check -p eggsec --features mobile-dynamic
	cargo check -p eggsec --features web-proxy
	cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
	cargo check -p eggsec --features c2-mcp,tool-api,rest-api
	# Marker-feature profiles that previously regressed silently (stale
	# TaskResult match arms); keep these compiling.
	cargo check -p eggsec --features compliance
	cargo check -p eggsec --features finding-workflow
	cargo check -p eggsec --features vuln-management

# Release validation (local only, no publication)
release-check:
	bash scripts/release-check.sh

# Clean build artifacts
clean:
	cargo clean

# ── Python checks ─────────────────────────────────────────────────────────

# Unified Python CI check (one venv, one maturin develop, all retained checks)
check-python:
	bash scripts/check-python.sh

# Help
help:
	@echo "Primary (run these):"
	@echo "  make check           - Full mandatory Rust CI contract (format, lint, test, guards)"
	@echo "  make check-python    - Python CI check (one build, all checks)"
	@echo "  make test            - Unit tests only (default)"
	@echo "  make check-full      - Optional broad validation (pre-release)"
	@echo "  make release-check   - Release validation (no publication)"
	@echo "  make clean           - Clean artifacts"
	@echo ""
	@echo "Specialist diagnostics (as needed):"
	@echo "  make clippy          - Lint"
	@echo "  make fmt             - Format check"
	@echo "  make build           - Release build"
	@echo "  make test-ci         - Full package tests with rest-api"
	@echo "  make test-fast       - Fast unit-test alias"
	@echo "  make test-integration - Integration tests (alias for test-ci)"
	@echo "  make test-nse        - NSE tests (requires nse feature)"
	@echo "  make test-slow       - Run ignored tests"
	@echo "  make test-coverage   - Code coverage"
	@echo "  make test-feature-matrix - Feature metadata validation tests"
	@echo "  make test-architecture-guards - Static grep checks for invariant regressions"
	@echo "  make check-no-default   - Validate no-default-features build"
	@echo "  make check-msrv         - Validate MSRV (requires rustup toolchain install 1.88)"
	@echo "  make check-feature-profiles - Representative feature profile checks"
