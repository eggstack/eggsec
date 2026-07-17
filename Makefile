# Test Infrastructure for Eggsec
# ================================

.PHONY: test test-fast test-slow test-unit test-integration test-nse test-coverage test-ci test-feature-matrix test-architecture-guards check-no-default check-architecture-ci check-feature-profiles test-python-phase-f test-python-compatibility test-python-resource-budgets test-python-redaction build-python-evidence clean help

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

# Architecture drift guards (static grep checks)
test-architecture-guards:
	bash scripts/check-architecture-guards.sh

# Validate no-default-features build
check-no-default:
	cargo check --workspace --no-default-features

# Full architecture guard CI reproduction (single target for contributors)
check-architecture-ci:
	cargo fmt --all --check
	cargo check --workspace --no-default-features
	cargo test -p eggsec --lib
	cargo test -p eggsec --test metadata_consistency
	cargo test -p eggsec --test command_registry
	cargo test -p eggsec --test tool_registration --features rest-api
	cargo test -p eggsec --test feature_matrix
	cargo test -p eggsec --test enforcement_matrix
	cargo test -p eggsec --test enforced_dispatch_regression
	cargo test -p eggsec-output --test report_envelope
	bash scripts/check-architecture-guards.sh

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

# Clean build artifacts
clean:
	cargo clean

# ── Phase F: Python release closure targets ──────────────────────────────

# Run semantic compatibility checker against baseline
test-python-compatibility:
	python scripts/check_python_compatibility.py

# Run resource budget tests (FD, thread, memory, socket, temp-dir, repo scale)
test-python-resource-budgets:
	rtk python -m pytest crates/eggsec-python/tests/test_resource_budgets.py -v --tb=short

# Run comprehensive redaction test suite
test-python-redaction:
	rtk python -m pytest crates/eggsec-python/tests/test_redaction_comprehensive.py -v --tb=short

# Run all Phase F Python gates (compatibility + resource budgets + redaction)
test-python-phase-f: test-python-compatibility test-python-resource-budgets test-python-redaction

# Generate commit-bound evidence bundle for release validation
build-python-evidence:
	python scripts/build_python_release_evidence.py --commit $$(git rev-parse HEAD)

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
	@echo "  make test-architecture-guards - Static grep checks for invariant regressions"
	@echo "  make check-no-default   - Validate no-default-features build"
	@echo "  make check-architecture-ci  - Full architecture guard CI reproduction"
	@echo "  make check-feature-profiles - Representative feature profile checks"
	@echo "  make test-python-phase-f - All Phase F Python gates (compat + budgets + redaction)"
	@echo "  make test-python-compatibility - Semantic compatibility checker vs baseline"
	@echo "  make test-python-resource-budgets - Resource budget tests (FD, thread, memory)"
	@echo "  make test-python-redaction - Comprehensive redaction test suite"
	@echo "  make build-python-evidence - Generate commit-bound evidence bundle"
	@echo "  make clean           - Clean artifacts"
