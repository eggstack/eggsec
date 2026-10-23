# Test Infrastructure for Eggsec
# ================================

.PHONY: test test-fast test-slow test-unit test-integration test-nse test-coverage test-ci test-feature-matrix test-architecture-guards test-tui-pty check-no-default check check-deps check-gui check-gui-prereqs gui-release check-daemon-profile build-daemon-profile check-python check-full check-feature-profiles check-features-individual clippy clippy-domain release-check fmt build clean help

# ── Desktop GUI shell profile ─────────────────────────────────────────────
#
# The `shell` feature carries the Tauri v2 dependency, so compiling it needs
# the platform webview SDK. Linux needs the webkit2gtk development packages;
# macOS and Windows ship the webview with the OS, so the gate is open there.
#
# The gate follows the discipline of scripts/check-features-individual.sh:
# an ABSENT prerequisite is a SKIP with the reason named, while a PRESENT
# prerequisite whose build breaks is a hard FAIL -- the build error is never
# swallowed. `make check-gui` is the ungated escape hatch: it always runs and
# reports cargo's own error.
GUI_SHELL_PREREQ_HINT = Linux: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf pkg-config

# Output directory for the GUI release build (M007). Git-ignored.
GUI_RELEASE_OUT ?= dist/gui

# Runs "$(1)" when the platform webview prerequisite is present; otherwise
# prints a SKIP line naming the prerequisite and succeeds.
define GUI_SHELL_GATE
@if [ "$$(uname -s)" != "Linux" ]; then \
	$(1); \
elif command -v pkg-config >/dev/null 2>&1 \
	&& { pkg-config --exists webkit2gtk-4.1 2>/dev/null || pkg-config --exists webkit2gtk-4.0 2>/dev/null; }; then \
	$(1); \
else \
	echo "SKIP: desktop GUI shell profile (prerequisite absent: $(GUI_SHELL_PREREQ_HINT))."; \
fi
endef

# Default: run unit tests only (fast feedback loop)
test: test-unit

# Compatibility alias for the default fast unit-test loop
test-fast: test-unit

# Run only unit tests (lib tests, no network, no wiremock)
test-unit:
	cargo test --lib -p eggsec

# Run the eggsec package test suite without Cargo fail-fast
# Phase E WS2: engine default is empty, so integration suites that exercise
# CLI types/dispatch (via default in earlier phases) request `cli` explicitly.
test-ci:
	cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast

# Run integration tests (alias for test-ci; uses wiremock, may need network)
test-integration: test-ci

# Run NSE tests (requires nse feature)
test-nse:
	cargo test -p eggsec --features nse,cli --test nse_tests --test nse_integration_tests

# Run slow/explicitly-ignored tests
test-slow:
	cargo test -p eggsec --features rest-api,cli --tests -- --ignored

# Run clippy (routine: engine + dependency-light leaf crates)
# Phase E WS2: engine default is empty; lint both the empty library default
# and the `cli` process-host surface (used by eggsec-cli/eggsec-tui).
#
# eggsec-tui / eggsec-cli (the process hosts) and the knowledge-corpus leaves
# are listed explicitly: they had no clippy coverage at all, so `-D warnings`
# had never gated them — including the crate with the most guard rules. Keep
# this list in sync with the workspace members `make check` builds.
clippy:
	cargo clippy --lib -p eggsec -- -D warnings
	cargo clippy --lib -p eggsec --features cli -- -D warnings
	cargo clippy -p eggsec-core -p eggsec-tool-core -p eggsec-report-model -p eggsec-output -p eggsec-runtime -p eggsec-ui-model -p eggsec-agent -p eggsec-transport -p eggsec-transport-eggfetch -p eggsec-policy -- -D warnings
	cargo clippy -p eggsec-tui -- -D warnings
	# The CLI's `daemon` / `session` / `task` subcommands live behind the
	# `daemon-client` feature, so the plain `-p eggsec-cli` lint above never
	# compiles them. Lint the feature on.
	cargo clippy -p eggsec-cli --features daemon-client -- -D warnings
	cargo clippy -p eggsec-secrets -p eggsec-payloads -p eggsec-service-db -p eggsec-udp-scan -- -D warnings
	# The operation-schema module is behind a feature, so the plain
	# `-p eggsec-runtime` lint above never compiles it. Lint the feature on.
	cargo clippy -p eggsec-runtime --features schema -- -D warnings
	# The daemon protocol and the frontend client are the wire contract for
	# every frontend; they carry published types, so they are linted too.
	cargo clippy -p eggsec-daemon-protocol -p eggsec-frontend-client -- -D warnings
	# The desktop GUI shell (Tauri) is a separate profile that needs the
	# platform webview SDK, so it is gated on that prerequisite.
	$(call GUI_SHELL_GATE,cargo clippy -p eggsec-gui --features shell -- -D warnings)

# Lint domain/platform crates with their relevant features (deep checks only:
# heavier closures; the `nse` closure needs a C toolchain for its vendored OpenSSL).
# The standalone NSE runtime owns its own lint gate in eggstack/eggsec-nse CI.
clippy-domain:
	cargo clippy -p eggsec-db-lab --features db-drivers -- -D warnings
	cargo clippy -p eggsec-web-proxy --features web-proxy -- -D warnings
	cargo clippy -p eggsec-mobile-lab -- -D warnings
	cargo clippy -p eggsec-daemon -- -D warnings

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

# TUI PTY regression smoke (Phase B WS6, specialist/deep-checks only).
# Runs the real eggsec binary in a pseudo-terminal (stdlib pty, no Rust
# runtime deps) and proves a representative tracing warning no longer leaks
# outside the frame plus terminal restoration on the daemon-attach path.
# Unix/Linux only; the script reports SKIP on Windows. Never part of the
# normal unit-test loop (depends on terminal timing).
test-tui-pty:
	cargo build -p eggsec-cli
	python3 scripts/tui_pty_smoke.py --binary ./target/debug/eggsec

# Validate no-default-features build
check-no-default:
	cargo check --workspace --no-default-features

# Validate MSRV (requires `rustup toolchain install 1.89`)
check-msrv:
	cargo +1.89 check --workspace --no-default-features
	cargo +1.89 check -p eggsec-cli --no-default-features
	cargo +1.89 check -p eggsec-transport
	cargo +1.89 check -p eggsec-transport-eggfetch
	cargo +1.89 check -p eggsec --no-default-features
	# The desktop GUI shell profile under the declared MSRV. Same
	# prerequisite gate as `make check`: absent -> SKIP, present but
	# broken -> FAIL.
	$(call GUI_SHELL_GATE,cargo +1.89 check -p eggsec-gui --features shell)

# Dependency advisory/license/source policy (Phase F supply-chain hardening).
# Canonical gate: `deny.toml` via cargo-deny over the full workspace closure
# (`--workspace --all-features`; deny.toml also sets [graph] all-features).
# Fails closed when cargo-deny is absent — install with
# `cargo install cargo-deny` (CI installs a SHA-pinned version). Never silently
# skip: a missing tool is a failed check, not a pass.
check-deps:
	@command -v cargo-deny >/dev/null 2>&1 || (echo "FAIL: cargo-deny not found. Install with 'cargo install cargo-deny' (CI uses a SHA-pinned install-action)." >&2; exit 1)
	cargo deny --workspace --all-features check
	cargo deny --workspace --all-features check advisories
	cargo deny --workspace --all-features check bans
	cargo deny --workspace --all-features check licenses
	cargo deny --workspace --all-features check sources

# Full mandatory Rust CI contract (no cargo-nextest required)
check:
	cargo fmt --all --check
	cargo check --workspace --no-default-features
	# The desktop GUI shell profile (Tauri). The workspace check above
	# covers the GUI's non-shell profile only; the shell profile needs the
	# platform webview SDK, so it is gated on that prerequisite.
	$(call GUI_SHELL_GATE,cargo check -p eggsec-gui --features shell)
	# Build the *test* targets for the no-default-features profile too:
	# `cargo check --workspace` only compiles library/binary targets, so a
	# feature-gated `use eggsec::commands` in tests/ can break unnoticed.
	cargo check -p eggsec --all-targets --no-default-features
	cargo check -p eggsec
	cargo check -p eggsec-cli
	cargo check -p eggsec-cli --no-default-features
	# The CLI's daemon client surface lives behind `daemon-client`, so the
	# checks above never compile the `daemon` / `session` / `task` subcommands.
	cargo check -p eggsec-cli --features daemon-client
	cargo test -p eggsec-cli --features daemon-client
	$(MAKE) check-deps
	$(MAKE) clippy
	cargo test -p eggsec --doc
	cargo test -p eggsec --no-default-features --test tool_registration --test loadtest_tests --no-fail-fast
	cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast
	cargo test -p eggsec-output --tests
	cargo test -p eggsec-report-model --tests
	cargo test -p eggsec-policy --tests
	cargo test -p eggsec-service-db --tests
	cargo test -p eggsec-secrets --tests
	cargo test -p eggsec-payloads --tests
	cargo test -p eggsec-udp-scan --tests
	cargo test -p eggsec-transport-eggfetch --tests --no-fail-fast
	cargo test -p eggsec-tui --lib --no-fail-fast
	bash scripts/check-architecture-guards.sh

# Desktop GUI shell profile (Tauri), UNGATED: it always runs, so a broken
# shell build is a hard failure and a host without the webview SDK gets
# cargo's own error message instead of a silent skip.
check-gui:
	@echo "Desktop GUI shell profile (prerequisite: $(GUI_SHELL_PREREQ_HINT))."
	cargo check -p eggsec-gui --features shell
	cargo clippy -p eggsec-gui --features shell -- -D warnings

# Desktop GUI prerequisite probe: reports whether the shell profile can be
# built on this host. It REPORTS, it does not install anything and it does
# not fail the build -- `make check-gui` is the gate that fails.
check-gui-prereqs:
	bash scripts/check_gui_prereqs.sh

# Desktop GUI release build (M007). Produces the binary plus its build
# metadata sidecar in $(GUI_RELEASE_OUT). It performs no signing, no
# bundling, no installer and no publication -- the distribution workstream
# owns those.
gui-release:
	bash scripts/build_gui_release.sh --out $(GUI_RELEASE_OUT)

# Daemon feature profile `service` (M008): the GUI-serving profile. The
# static half pins the manifest to the documented list; the runtime half
# pins the COMPILED feature set to the same list. The profile needs a C
# toolchain (vendored OpenSSL under `nse`), so it is a deep check rather
# than part of `make check`.
DAEMON_PROFILE_OUT ?= dist/daemon-profile

.PHONY: build-daemon-profile
build-daemon-profile:
	bash scripts/build-daemon-profile.sh --profile service --out $(DAEMON_PROFILE_OUT)

.PHONY: check-daemon-profile
# This gate is `cargo check`, not `clippy -D warnings`. The profile compiles
# engine modules the default build never reaches (packet, stress, wireless,
# database, tool-api), and those modules carry pre-existing clippy debt that
# no milestone has cleaned up. The profile BUILD must be green; its lint debt
# is tracked separately rather than silently gated here.
check-daemon-profile:
	cargo check -p eggsec-daemon --features service
	cargo test -p eggsec --features cli --test feature_matrix
	cargo test -p eggsec-daemon --features service --test feature_profile

# Optional broad validation (pre-release, not required for merge)
# Note: dependency policy (`check-deps`) already runs inside `check`; this
# target adds domain lint and representative feature profiles only.
check-full: check
	$(MAKE) clippy-domain
	$(MAKE) check-feature-profiles
	$(MAKE) test-tui-pty

# Exhaustive per-feature compilation sweep (weekly/manual, not per-PR).
# `full` is a curated aggregate, so this sweep is the completeness oracle.
check-features-individual:
	bash scripts/check-features-individual.sh

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
	# Broad TUI profile (dependency-light, deterministic): exercises the
	# feature-gated runtime request builders (db-pentest, intercept,
	# C2) plus adjacent tabs for cross-feature compile drift. Fast
	# detection between weekly deep sweeps; NOT a substitute for the
	# eggsec-tui/full aggregate in Deep Checks.
	cargo check -p eggsec-tui --features db-pentest,web-proxy,c2
	cargo test -p eggsec-tui --features db-pentest,web-proxy,c2 --lib --no-fail-fast
	# The daemon's GUI-serving profile (`service`). It carries `nse`, which
	# vendors OpenSSL and needs a C toolchain, so it belongs here rather
	# than in `make check`.
	$(MAKE) check-daemon-profile

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
	@echo "  make check           - Full mandatory Rust CI contract (format, lint, test, guards, deps)"
	@echo "  make check-deps      - Dependency advisory/license/source policy (cargo deny)"
	@echo "  make check-gui       - Desktop GUI shell profile (check + clippy, ungated)"
	@echo "  make check-gui-prereqs - Report whether the GUI shell can be built here"
	@echo "  make gui-release     - Desktop GUI release build + metadata sidecar"
	@echo "  make check-daemon-profile - Daemon service profile conformance (deep check)"
	@echo "  make build-daemon-profile - Build the daemon service profile"
	@echo "  make check-python    - Python CI check (one build, all checks)"
	@echo "  make test            - Unit tests only (default)"
	@echo "  make check-full      - Optional broad validation (pre-release)"
	@echo "  make release-check   - Release validation (no publication)"
	@echo "  make clean           - Clean artifacts"
	@echo ""
	@echo "Specialist diagnostics (as needed):"
	@echo "  make clippy          - Lint (engine + leaf crates)"
	@echo "  make clippy-domain   - Lint domain/platform crates (deep checks)"
	@echo "  make check-features-individual - Per-feature compile sweep (deep checks)"
	@echo "  make fmt             - Format check"
	@echo "  make build           - Release build"
	@echo "  make test-ci         - Full package tests with rest-api"
	@echo "  make test-fast       - Fast unit-test alias"
	@echo "  make test-integration - Integration tests (alias for test-ci)"
	@echo "  make test-nse        - NSE tests (requires nse feature)"
	@echo "  make test-slow       - Run ignored tests"
	@echo "  make test-coverage   - Code coverage"
	@echo "  make test-feature-matrix - Feature metadata validation tests"
	@echo "  make test-tui-pty - TUI PTY regression smoke (specialist/deep-checks)"
	@echo "  make test-architecture-guards - Static grep checks for invariant regressions"
	@echo "  make check-no-default   - Validate no-default-features build"
	@echo "  make check-msrv         - Validate MSRV (requires rustup toolchain install 1.89)"
	@echo "  make check-feature-profiles - Representative feature profile checks"
