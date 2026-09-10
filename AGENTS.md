# AGENTS.md

Guidelines for AI agents working on this codebase.

**Minimum Rust version: 1.88** (workspace `rust-version` in `Cargo.toml`). CI tests the exact MSRV via the `msrv` job in `.github/workflows/deep-checks.yml`. Verify locally with `make check-msrv` (requires `rustup toolchain install 1.88`).

## Quick Verification

Before claiming code is correct, run:

```bash
make check                  # Rust CI contract (format, lint, test, architecture guards)
make check-python           # Python CI (when Python-facing code, bindings, stubs, or docs change)
```

Prerequisites: `ripgrep` (`rg`) for architecture guards. No `cargo-nextest` required.

Scope notes:
- `make test` runs `cargo test --lib -p eggsec` only (engine lib unit tests). Use `make test-ci` for the full package suite.
- `make clippy` lints the engine lib plus dependency-light leaf crates (`eggsec-core`, `eggsec-tool-core`, `eggsec-output`, `eggsec-runtime`, `eggsec-ui-model`, `eggsec-agent`); `make clippy-domain` covers domain/platform crates (deep checks only).
- `make check-python` builds into `.venv-ci/` (override with `EGGSEC_PYTHON_VENV`). Pytest excludes tests marked `network` by default (`-m 'not network'`).

`make check-full` is optional; run before broad feature/release work. See [`docs/VERIFICATION.md`](docs/VERIFICATION.md) for the full verification contract.

## Project Overview

Eggsec is a Rust security testing toolkit organized as a Cargo workspace with 16 crates:

| Crate | Purpose |
|-------|---------|
| `eggsec-core` | Shared types, constants (Severity, SensitiveString) |
| `eggsec-tool-core` | Tool abstraction layer types |
| `eggsec` | Main engine library (no binary) |
| `eggsec-nse` | Optional Nmap NSE/Lua compatibility |
| `eggsec-tui` | Terminal UI (ratatui/crossterm) |
| `eggsec-cli` | CLI binary entry point |
| `eggsec-output` | Report formatting (JSON/SARIF/JUnit/HTML/CSV/MD) |
| `eggsec-agent` | Agent coordination primitives |
| `eggsec-db-lab` | Database pentest domain crate |
| `eggsec-web-proxy` | Web proxy/MITM domain crate |
| `eggsec-mobile-lab` | Mobile app analysis domain crate |
| `eggsec-runtime` | Frontend-neutral task lifecycle (Runtime, RuntimeTaskExecutor) |
| `eggsec-daemon` | Persistent session host (SQLite, Unix socket, optional HTTP) |
| `eggsec-daemon-protocol` | Daemon IPC protocol types and client registry |
| `eggsec-ui-model` | Frontend-neutral view DTOs |
| `eggsec-python` | Python bindings (PyO3/maturin; scoped pre-1.0 stable-core, broader domains provisional/experimental; Release 5 Phases A–F complete) |

## Build & Test Commands

### Full architecture CI reproduction

```bash
make check    # or the individual commands in scripts/check-architecture-guards.sh
```

### Feature-specific checks

```bash
# Feature-gated crates
cargo check -p eggsec --features mobile
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features wireless
cargo check -p eggsec --features nse
cargo check -p eggsec --features evasion
cargo check -p eggsec --features postex
cargo check -p eggsec --features c2
cargo check -p eggsec --features rest-api
cargo check -p eggsec --features grpc-api

# Domain crates (standalone)
cargo check -p eggsec-db-lab
cargo check -p eggsec-web-proxy
cargo check -p eggsec-mobile-lab
cargo check -p eggsec-nse --features nse

# CLI variants
cargo check -p eggsec-cli                          # default (TUI only; default = ["tui"])
cargo check -p eggsec-cli --no-default-features    # headless
cargo check -p eggsec-cli --no-default-features --features daemon-client  # daemon client only

# Workspace build (CI baseline)
cargo check --workspace --no-default-features
```

### Makefile targets

```bash
make check                  # full mandatory Rust CI contract (no nextest required)
make check-python           # Python CI check (one build, all checks)
make check-full             # optional broad validation (advisories + feature profiles)
make release-check          # release validation (no publication)

# Release graph validation
python scripts/release-package-graph.py list      # package set inventory
python scripts/release-package-graph.py validate   # publishability checks
python scripts/release-package-graph.py order      # topological publication order
python scripts/release-package-graph.py version-locations # internal version inventory
python scripts/release-package-graph.py package-workspace <target-dir> # Cargo-native archives + JSONL inventory
python scripts/release-package-graph.py inspect-archive <crate> # archive checks
python scripts/release-package-graph.py inspect-inventory <inventory> # exact archive/content/standalone checks
make test                   # unit tests only (default; alias of test-unit)
make test-ci                # full package tests with rest-api
make test-nse               # NSE crate tests (feature nse)
make clippy                 # lint (-D warnings)
make fmt                    # format check
make test-feature-matrix    # feature + metadata validation
make check-no-default       # no-default-features workspace build
make check-msrv             # MSRV compile check (requires rustup toolchain install 1.88)
make check-feature-profiles # representative feature profile checks
make check-features-individual # exhaustive per-feature compile sweep (deep checks only)
make clippy-domain          # lint domain/platform crates (deep checks only)
make build                  # release build
make clean                  # remove build artifacts
make help                   # authoritative target list
```

### Python bindings

```bash
# Development build (installs into active venv)
cd crates/eggsec-python
maturin develop

# Release wheel
maturin build --release

# Tests
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/

# Unified CI check (one build, all checks)
make check-python

# Validation infrastructure
python scripts/validate_python_profiles.py   # validates profile manifest
python scripts/run_python_profile.py --profile <name>   # runs a specific profile
python scripts/check_python_compatibility.py             # semantic compatibility checker

# Rust-side tests
cargo test -p eggsec-python
```

### CI workflows

GitHub Actions (`.github/workflows/`):
- `ci.yml` — mandatory Rust (`make check`) and Python (`make check-python`) checks on ubuntu
- `deep-checks.yml` — optional diagnostic workflow (weekly schedule or manual trigger): `make check-full` + cargo-deny, `make check-features-individual` sweep, exact-MSRV 1.88 job, macOS/Windows portability job

Consumer GitLab CI example: `examples/ci/gitlab/eggsec-scan.yml` (not wired to repository triggers).

## Architecture

### Enforcement model (critical)

`EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for ALL surfaces (CLI, TUI, REST, MCP, agent, gRPC). Never bypass it.

- **Manual CLI/TUI**: `ManualPermissive` — operator-directed, supports overrides
- **REST/MCP**: `McpStrict` — no manual overrides, scope required
- **Agent**: `AgentStrict` — explicit scope manifest, no overrides
- **CI**: `CiStrict` — hard enforcement

Scope must come from `LoadedScope` (not raw `Scope`) for automated surfaces.

### Key invariants

1. **OperationMetadata** is the single source of truth for operation policy. Don't build policy checks inline.
2. **DomainDescriptor** in `domain/mod.rs` groups operations under domains. Always present; check `required_feature` before use.
3. **ApprovedOperation** token required for strict surfaces. `EnforcedDispatcher::dispatch_checked()` verifies tool+target match.
4. **eggsec-runtime** must stay dependency-light (serde/serde_json, thiserror, tokio, tokio-util, tracing, uuid only). No TUI, transport, or persistence deps. Enforced by architecture guards.
5. **eggsec-output** must not depend on `eggsec` (engine) or `eggsec-runtime`. Only depends on `eggsec-core`.
6. **eggsec-daemon** must never depend on TUI crates. Engine dep (`eggsec`) is optional behind `full-executor`; transport deps (axum etc.) are optional behind `http-api`. Default deps: `eggsec-runtime` + `eggsec-daemon-protocol` only. Guard rejects non-optional engine/transport deps.
7. **Operation request contracts**: canonical defaults/validation live in `eggsec-tool-core::operation_request`; engine facade in `eggsec::operation_request`; `TaskKind::operation_id`/`canonical_target` exhaustive; `ToolRequest.params` validated via `validate_tool_request_params`; command handlers use `describe_from_registry`.

### Runtime dispatch flow

```
TUI → TuiTaskDispatcher → eggsec::dispatch::dispatch_inner() → TaskResult
CLI → CLI dispatch → eggsec::dispatch::dispatch_inner() → direct output
REST/MCP/Agent → EnforcementContext::evaluate() → EnforcedDispatcher::dispatch_checked() → tool execution
Daemon/Runtime → runtime_bridge (RuntimeSurface→ExecutionSurface, TaskKind→OperationDescriptor) → EnforcementContext → dispatch
```

The `runtime_bridge` module (`crates/eggsec/src/runtime_bridge/`) bridges `eggsec-runtime` DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) to the engine enforcement model (`ExecutionSurface`, `OperationDescriptor`, `EnforcementContext`). It provides `preflight_run_request()` for policy preview and `approve_run_request()` for pre-dispatch authorization.

### Workspace structure

```
crates/
  eggsec/           # main engine (lib only, no binary)
  eggsec-core/      # shared types
  eggsec-tool-core/ # tool abstraction types
  eggsec-cli/       # CLI binary (features: tui, daemon-client, headless)
  eggsec-tui/       # terminal UI
  eggsec-nse/       # Nmap NSE compatibility
  eggsec-output/    # report formatting
  eggsec-agent/     # agent coordination
  eggsec-runtime/   # frontend-neutral runtime
  eggsec-daemon/    # persistent session host
  eggsec-daemon-protocol/ # daemon IPC protocol types and client registry
  eggsec-ui-model/  # frontend view DTOs
  eggsec-db-lab/    # database pentest domain
  eggsec-web-proxy/ # web proxy domain
  eggsec-mobile-lab/ # mobile analysis domain
  eggsec-python/    # Python bindings (PyO3/maturin)
```

### Feature flags

Feature-gated modules require explicit build flags:

| Feature | System Dep | Notes |
|---------|------------|-------|
| `wireless` | `wireless-tools` | WiFi recon; root for real scans |
| `wireless-advanced` | (needs wireless) | deauth/disassoc; policy gated Intrusive |
| `mobile` | none | APK/IPA static; pure-Rust parsers |
| `mobile-dynamic` | ADB + device | Android runtime testing |
| `db-pentest` | none (drivers) | Postgres/MySQL/MSSQL/MongoDB/Redis |
| `web-proxy` | none | MITM proxy |
| `nse` | `libssl-dev` | Nmap NSE scripts; `native-tls` and `openssl` behind this feature |
| `evasion` | none | Evasion detection |
| `postex` | none | Post-exploitation simulation |
| `c2` | none | C2 simulation (depends on postex+evasion) |
| `stress-testing` | none | Raw sockets, IP spoofing |
| `packet-inspection` | `libpcap-dev` | Packet capture |
| `grpc-api` | `protobuf-compiler` | gRPC protobuf reflection descriptor (protoc only for descriptor set; Rust code is checked-in) |
| `nse-ssh2` | `libssh2-dev` | NSE with SSH2/libssh2 support |
| `nse-sandbox` | (needs nse) | Sandboxed NSE execution |
| `email-notifications` | (none) | SMTP email via lettre; enables `rest-api` email transport |
| `logging-subscriber` | (none) | tracing subscriber/appender setup for process-host crates |
| `cli` | `clap`, `clap_complete` | CLI types, command dispatch, and argument parsing |
| `config-watch` | (none) | File watching (notify + debouncer) for config hot-reload |

Empty gates (empty feature arrays; compile-time gate only): `tool-api`, `insecure-tls`, `api-schema`, `cloud`, `git-secrets`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `wireless`, `evasion`, `postex`, `daemon-client`, `test-helpers`.

Feature-coupled markers (base/domain refs, no new third-party closure): `wireless-advanced` (→`wireless`), `c2` (→`postex`+`evasion`), `c2-mcp`/`db-pentest-mcp`/`web-proxy-mcp` (exposure markers), `transparent-proxy`/`dynamic-plugins` (→`web-proxy`), `nse-sandbox` (→`nse`), `mobile-dynamic` (→`mobile`).

Dependency-backed feature flags: `cli`, `rest-api`, `grpc-api`, `ws-api`, `sbom`, `container`, `websocket`, `headless-browser`, `database`, `db-pentest`, `db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis`, `mobile`, `web-proxy`, `nse`, `nse-ssh2`, `ai-integration`, `pdf`, `stress-testing`, `packet-inspection`, `email-notifications`, `logging-subscriber`, `config-watch`.

Note: `http-api` is a feature on `eggsec-daemon` (not `eggsec`), enabling HTTP/SSE transport.

CLI features: `tui` (default), `daemon-client`, `headless`

Platform integration (Phase F): prerequisite detection lives in
`crates/eggsec/src/platform/` (`PlatformReport`, `skip_reason_for`) and is
surfaced via `eggsec doctor`. Fixture tests (mock ADB, Frida simulation,
`packet::fixture`, `wireless::fixture`) run hermetically with no root or
hardware; live legs are isolated scripts (`scripts/setup_packet_netns.sh`,
`scripts/setup_android_emulator.sh`) that SKIP with the named prerequisite.
Never require the full suite to run as root. See `docs/PLATFORM.md` and
`bash scripts/check_platform.sh`.

Python bindings (`eggsec-python`): Build with `maturin develop` from `crates/eggsec-python/`. The stable-core boundary is the twenty-two-operation engine registry: the original ten (`scan_ports`, `scan_endpoints`, `fingerprint_services`, `recon_dns`, `inspect_tls`, `detect_technology`, `detect_waf`, `validate_waf`, `fuzz_http`, `load_test`) plus twelve promoted domains (`scan_git_secrets`, `generate_sbom`, `run_consolidated_recon`, `graphql_test`, `oauth_test`, `auth_test`, `db_probe`, `nse_run`, `scan_docker_image`, `scan_kubernetes`, `analyze_apk`, `analyze_ipa`). Daemon-client APIs remain provisional. Release fixtures use `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`. See `docs/python/domain-maturity.md` for provisional/experimental boundary and `crates/eggsec-python/README.md` for examples.

Provisional subsystems (scope-checked, policy-gated, not stable-core): network types (`eggsec.net`, `eggsec.sessions`, `eggsec.storage`), NSE runtime, interception proxy, database assessment. Experimental: raw packet injection (feature: `packet-inspection`). Package layout: stable core at top-level `eggsec`, provisional under `eggsec.net`/`eggsec.sessions`/`eggsec.storage`/`eggsec.reporting`/`eggsec.daemon`, experimental under `eggsec.experimental`. Feature introspection via `eggsec._feature_guard`.

Python pip extras (installable via `pip install eggsec[extra]`) are the subset
`db-pentest`, `web-proxy`, `mobile`, `mobile-dynamic` (requires `mobile`),
`packet-inspection`, `stress-testing`, `nse`, `wireless`, `headless-browser`,
plus the `full-no-system` aggregate (`websocket`, `git-secrets`, `sbom`,
`container`). The remaining engine features are compile-time Cargo features on
the binding crate only (build with `maturin develop --features ...`), e.g.
`daemon-client`, `advanced-hunting`, `compliance`. Authoritative extras list
lives in `[project.optional-dependencies]` of
`crates/eggsec-python/pyproject.toml`; system-dependent ones need
libpcap/libssl/wireless-tools/Chromium at build or run time.

Aggregates: engine `full` = curated 28-member lab set (pinned in `FULL_MEMBERS`, `crates/eggsec/src/config/feature_registry.rs`; not exhaustive — the oracle is `make check-features-individual`). Python `full-no-system` = `websocket` + `git-secrets` + `sbom` + `container` only. Neither is conservative/production.

## Key Patterns

- **Severity Enum**: Canonical in `eggsec-core::types`. Re-export, don't recreate.
- **FxHashMap**: Use `rustc_hash::FxHashMap`/`FxHashSet` in performance paths, not std collections.
- **Regex Caching**: `lru = "0.18"` with cache size 100 (NonZeroUsize).
- **Truncation**: `utils/formatting.rs` — `strip_controls` (recommended), `preserve_all`.
- **Error Handling**: Avoid `unwrap_or_default()` on async ops; use explicit match with tracing.
- **PayloadType location**: `fuzzer/payloads/mod.rs`, NOT `types.rs`.
- **Visual Regression**: `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()`.
- **AI Cache Keys**: Always use `CacheKeyBuilder` to avoid collisions.
- **Themes**: 50 packaged via LZMA. Run `python3 scripts/package_themes.py` after modifying `themes/*.toml`.
- **Enum from_str**: All public enums raise `ValueError` on unknown strings. Never silently default.
- **Context managers**: All sink/callback classes support `with` statements. Use them for automatic cleanup.
- **DTO round-trip**: `OperationError`, `ExecutionStats`, `Artifact` support `from_dict()`/`from_json()` for serialization round-trip.
- **Descriptor construction**: Use `OperationMetadata::try_descriptor_for_target()` for validated construction. The unchecked `descriptor_for_target()` remains for backward compatibility but should not be used for new strict-surface code.
- **Approval binding**: `ApprovedOperation` is the only valid dispatch token. Use `EnforcementContext::approve()` or `approve_manual()`. The surface must match the context profile. Construction is private/controlled (`policy_approval.rs`); adapters obtain tokens via `approve()`, never `ApprovedOperation::new`.
- **Approval-cache binding (Phase 0.1)**: Frontend reuse requires exact `ApprovedOperation::matches_descriptor()` (derived `PartialEq` on `OperationDescriptor`, so future fields participate automatically) plus unchanged scope fingerprint, policy hash, surface/profile, and manual-override generation. Never compare `descriptor().operation` names alone. Invalidate via `clear_cached_approval()`/`invalidate_cached_approval()` on posture/scope/policy/override changes; `toggle_posture()` clears. Runtime bundle dispatch uses the same predicate (`bundle.rs`).
- **Dispatch binding**: Use `validate_request_binding()` to verify request matches approval before dispatch. Fails closed on any mismatch.
- **Surface support matrix (Phase 0.2)**: Test-owned matrix in `crates/eggsec/tests/frontend_surface_matrix.rs` (engine: CLI registry/Clap/canonical/runtime) plus `crates/eggsec-tui/src/parity.rs` (TUI tabs). No new production registry. Classify every entry as operation-backed/multiplexer/helper/lifecycle with explicit exception lists; new drift must fail loudly. TUI `waf`→`waf-detect`, `scan-pipeline`→`pipeline` are normalized; `waf`/`scan-pipeline` remain tested aliases. Stress/Packet TUI tabs are always-visible availability shells (`stress-testing`/`packet-inspection`).
- **Clap reflection (Phase 0.3)**: Tests use `clap::CommandFactory` on the real `Cli` type; registry `cli_visible` presence tracks the compiled feature set. `oauth` primary is `oauth` (`o-auth` alias). `mobile-dynamic`/`wireless-deauth` are subcommand identities; `proxy`/`daemon`/`session`/`task`/`codegg-mcp` are explicit Clap-only exceptions.
- **Runtime mapping (Phase 0.5)**: `TaskKind::operation_id()` and engine `operation_id_for_task_kind()` must agree exhaustively (29 kinds); target extraction must agree; `RuntimeSurface` conversion is explicit and round-trippable (`Unknown` rejected).
- **Engine services (Phase D)**: Protocol/agent adapters depend on narrow traits, not concrete engine internals. `tool::service::{OperationCatalog, CheckedExecutor, PreflightService, EngineServices}` is the adapter boundary; `agent::services::AgentExecutionService` is the agent boundary; `mcp::bridge::McpEngineBridge` is the MCP narrow bridge. Adapters take `EngineServices` via `with_services`/`router_with_services`; only composition roots call `EngineServices::new`. `CheckedExecutor` exposes only `dispatch_checked` — raw dispatch is unrepresentable. Authorization stays in `EnforcementContext`; adapters never duplicate scope/policy evaluation and never call `Scope::is_target_allowed` or direct `tool.execute()`.
- **Hotspot modules (Phase D)**: Policy/target/catalog/approval live in `config/policy.rs` (facade) + `policy_target.rs` + `policy_catalog.rs` + `policy_approval.rs`. Scope address/resolver live in `scope_address.rs`/`scope_resolver.rs` (facade `scope.rs`). Runtime config/sink live in `eggsec-runtime/src/runtime_config.rs`/`runtime_sink.rs` (facade `runtime.rs`). Daemon RBAC/persistence live in `eggsec-daemon/src/host_auth.rs`/`host_persistence.rs` (facade `host.rs`). Public paths are stable via re-exports; new code imports from the cohesive module, not the facade internals.
- **Address classification**: Use `classify_address()` from `config::scope` to determine address class (Public, Private, Loopback, etc.). The resolver (`HostResolver` trait) reports facts; policy decides authorization.
- **DNS resolution**: Use `TargetScope::parse_with_resolver()` with `HostResolver` trait for deterministic testing. `SystemResolver` is the default. Never reject address classes in the resolver — defer to policy.
- **Scope contract**: `eggsec-tool-core::ScopeSpec` (`ToolScopeSpec`; legacy alias `Scope`) is a declarative transport DTO with no authorization method. Convert via `eggsec::config::scope_from_spec` (fail-closed) and evaluate through `EnforcementContext`/engine `Scope`. Effective authorization is the intersection of engine scope and converted spec. Never add `is_allowed()`/`authorize()` to the DTO layer.
- **Scope evaluation**: `TargetScope::evaluate_addresses()` checks all resolved addresses against CIDR rules. For strict surfaces, every address must be authorized. Use `resolved_addresses` field (not just `ip`) for scope decisions.
- **TLS provider**: All crates use ring-only (no aws-lc-rs). When declaring `rustls` or `tokio-rustls`, use `default-features = false` and explicitly enable `features = ["ring", "std", "tls12"]`. When declaring `reqwest`, use `features = ["rustls-no-provider"]` instead of `features = ["rustls"]` to avoid pulling in aws-lc-rs.

## Lessons Learned

- **TUI bounds checking**: Always use `.get(i)`, not `chunks[i]`.
- **TUI is_running()**: All input/navigation handlers must check `!self.is_running()`.
- **TUI reset()**: Must reset all state (selectors, checkboxes, fields, focus areas).
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` — always log with tracing.
- **Timeout wrappers**: All spawned tokio tasks need timeout wrappers (30-300s).
- **File paths**: CLI command handlers live in the engine crate at `crates/eggsec/src/commands/handlers/`; there is no `crates/eggsec/src/cli/handlers/` (`crates/eggsec/src/cli/` holds command types only). `crates/eggsec-cli/src/` is just the binary shell (main, daemon client, logging).
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top before flagging.
- **Count verification**: Always verify statistical claims against actual source.
- **`cargo install`**: Use `cargo install --path crates/eggsec-cli` (workspace root is virtual manifest).

## Architecture Guards

CI enforces invariants via `scripts/check-architecture-guards.sh` (requires ripgrep `rg`). Run before every PR:

```bash
bash scripts/check-architecture-guards.sh
```

Key checks:
- No stale `manual_only` in docs (use `cli_interactive_only`)
- MCP exposure terminology split (`mcp_metadata_exposable` vs `mcp_default_visible`)
- Strict surfaces don't call raw dispatch
- Required plan files exist
- Required docs exist (COMMAND_REGISTRY.md, TOOL_REGISTRATION.md, FEATURE_MATRIX.md, METADATA_OWNERSHIP.md, CI_ARCHITECTURE_GUARDS.md)
- No TUI workers directory (dispatch moved to `eggsec::dispatch`)
- `eggsec-runtime` has no TUI or transport dependencies
- `eggsec-output` has no engine/runtime dependencies
- NSE script/module loading flows through `ScriptResolver`
- NSE `ManualPermissive` stays in manual surfaces only
- NSE automated surfaces use `with_profile()` not `with_policy()`
- `NseRunReport.libraries` is per-run require activity, not registry dump
- HTTP library routes through `check_network_tcp()` before reqwest
- Runtime has no persistence dependencies (rusqlite/sqlx)
- Phase D service boundaries: adapters use `dispatch_checked` only (no raw `.dispatch` or direct `tool.execute` in `tool/protocol/`); no `is_target_allowed` DTO auth in adapters; `ApprovedOperation::new` only in `config/policy_approval.rs` + `policy_decision.rs`; protocol free of `tool::implementations::`; hotspot facades exist (`policy_target`, `policy_catalog`, `policy_approval`, `scope_address`, `scope_resolver`, `runtime_config`, `runtime_sink`, `host_auth`, `host_persistence`, `tool/service`, `mcp/bridge`, `agent/services`)
- Documentation reference consistency checked by `scripts/check_doc_references.py`

See `docs/CI_ARCHITECTURE_GUARDS.md` for the full inventory.

## Module Index

Each engine module has specialized guidance in an `AGENTS.override.md`, a deep-dive doc under `architecture/`, and usually a loadable skill. Load all three when working in a module:

| Module | Override file | Architecture doc | Skill |
|--------|---------------|------------------|-------|
| `agent/` | `crates/eggsec/src/agent/AGENTS.override.md` | `architecture/ai_agents.md` | `eggsec-agent` |
| `ai/` | `crates/eggsec/src/ai/AGENTS.override.md` | `architecture/ai_agents.md` | `eggsec-ai` |
| `fuzzer/` | `crates/eggsec/src/fuzzer/AGENTS.override.md` | `architecture/fuzzer.md` | `eggsec-fuzzer` |
| `scanner/` | `crates/eggsec/src/scanner/AGENTS.override.md` | `architecture/scanner.md` | `eggsec-scanner` |
| TUI | `crates/eggsec-tui/src/AGENTS.override.md` | `architecture/tui.md` | `eggsec-tui` |
| `waf/` | `crates/eggsec/src/waf/AGENTS.override.md` | `architecture/waf.md` | `eggsec-waf` |
| `recon/` | `crates/eggsec/src/recon/AGENTS.override.md` | `architecture/recon.md` | `eggsec-recon` |
| `tool/` | `crates/eggsec/src/tool/AGENTS.override.md` | `architecture/dispatch.md`, `docs/TOOL_REGISTRATION.md` | `eggsec-tool` |
| `config/` | `crates/eggsec/src/config/AGENTS.override.md` | `architecture/config.md` | `eggsec-config` |
| `output/` | `crates/eggsec/src/output/AGENTS.override.md` | `architecture/output.md` | `eggsec-output` |
| `proxy/` | `crates/eggsec/src/proxy/AGENTS.override.md` | `architecture/web_proxy.md` | `eggsec-proxy` |
| `stress/` | `crates/eggsec/src/stress/AGENTS.override.md` | `architecture/stress.md` | `eggsec-stress` |
| `distributed/` | `crates/eggsec/src/distributed/AGENTS.override.md` | `architecture/distributed.md` | `eggsec-distributed` |
| `packet/` | `crates/eggsec/src/packet/AGENTS.override.md` | `architecture/networking.md` | `eggsec-packet` |
| `loadtest/` | `crates/eggsec/src/loadtest/AGENTS.override.md` | `architecture/loadtest.md` | `eggsec-loadtest` |
| `mobile/` | `crates/eggsec/src/mobile/AGENTS.override.md` | `architecture/mobile.md` | `eggsec-mobile` |
| `pipeline/` | `crates/eggsec/src/pipeline/AGENTS.override.md` | `architecture/pipeline.md` | `eggsec-pipeline` |
| NSE | `crates/eggsec-nse/AGENTS.override.md` | `architecture/nse_integration.md` | `eggsec-nse` |
| `container/` | `crates/eggsec/src/container/AGENTS.override.md` | `architecture/container.md` | `eggsec-container` |
| `db_pentest/` | `crates/eggsec/src/db_pentest/AGENTS.override.md` | `architecture/database_pentest.md` | `eggsec-db-pentest` |
| `wireless/` | `crates/eggsec/src/wireless/AGENTS.override.md` | `architecture/wireless.md` | `eggsec-wireless` |
| `evasion/` | `crates/eggsec/src/evasion/AGENTS.override.md` | `architecture/evasion.md` | `eggsec-evasion` |
| `c2/` | `crates/eggsec/src/c2/AGENTS.override.md` | `architecture/c2.md` | `eggsec-c2` |
| `postex/` | `crates/eggsec/src/postex/AGENTS.override.md` | `architecture/postex.md` | `eggsec-postex` |
| `eggsec-python/` | `crates/eggsec-python/AGENTS.override.md` | `architecture/python_api.md` | `eggsec-python` |

Cross-cutting skills without a single owning module: `eggsec-security` (end-user capability tour), `eggsec-cli` (command dispatch patterns), `eggsec-daemon` (daemon/runtime/runtime_bridge crates), `eggsec-architecture-review` (doc-vs-code review methodology), plus per-module testing skills (`eggsec-auth`, `eggsec-browser`, `eggsec-hunt`).

## Dependency Ownership

Major direct dependency families, owning crate/domain, and suggested review cadence:

| Dependency Family | Owning Crate/Domain | Review Cadence | Notes |
|-------------------|---------------------|----------------|-------|
| PyO3/maturin | `eggsec-python` | Each PyO3 release cycle | Python bindings; 0.29 currently used |
| TLS (rustls, tokio-rustls) | `eggsec`, `eggsec-web-proxy` | Monthly or advisory-driven | Security-critical transport |
| reqwest | `eggsec`, `eggsec-agent` | Monthly or advisory-driven | HTTP client; security-critical |
| SQLx | `eggsec-db-lab` | Quarterly or compatibility-driven | Postgres/MySQL drivers; 0.8 blocks rusqlite 0.40 upgrade (libsqlite3-sys conflict) |
| Tiberius | `eggsec-db-lab` | Quarterly or compatibility-driven | MSSQL driver; 0.12 (current minor) |
| MongoDB/BSON | `eggsec-db-lab` | Quarterly or compatibility-driven | MongoDB driver; upgraded to 3.x |
| Redis | `eggsec-db-lab` | Quarterly or compatibility-driven | Redis driver; upgraded to 1.x |
| kube/k8s-openapi | `eggsec` (container) | Quarterly or compatibility-driven | Kubernetes client; upgraded to kube 4.2/k8s-openapi 0.28 |
| Rusqlite | `eggsec-daemon` | Quarterly or advisory-driven | SQLite; daemon-only; 0.31 (blocked by sqlx 0.8 libsqlite3-sys conflict) |
| mlua | `eggsec-nse` | Quarterly | Lua VM for NSE |
| native-tls/openssl | `eggsec-nse` | Monthly or advisory-driven | NSE TLS; optional, behind `nse` feature |
| ssh2/libssh2 | `eggsec-nse` | Quarterly or advisory-driven | NSE SSH; optional, behind `nse-ssh2` |
| prost/tonic | `eggsec` (grpc-api) | Quarterly | gRPC; generated code checked-in |
| ratatui/crossterm | `eggsec-tui` | Advisory/feature-driven | TUI libraries |
| pnet/nix/libc | `eggsec` (stress/packet) | Advisory | Raw networking; feature-platform-gated |
| printpdf | `eggsec` (pdf) | Advisory/feature-driven | PDF output; optional |

Manual grouped updates are acceptable for this repository's size. Dependabot/Renovate automation is not required.

## Architecture Docs

Canonical references live in `docs/` and `architecture/` directories. Key entry points:

- `docs/ARCHITECTURE.md` — workspace ownership, enforcement model, execution flows
- `docs/ARCHITECTURE_INVARIANTS.md` — 39 normative invariants
- `docs/FEATURE_MATRIX.md` — feature inventory, naming, build profiles
- `docs/ENFORCEMENT_MODES.md` — dual-mode enforcement contract
- `docs/COMMAND_REGISTRY.md` — command registry inventory and dispatch
- `docs/TOOL_REGISTRATION.md` — tool registration for MCP/REST/gRPC/agent
- `docs/EXTENSIBILITY.md` — contributor guide for adding operations, domains, commands
- `architecture/overview.md` — system-wide architecture, module index
- `architecture/nse_integration.md` — NSE/Lua integration, milestones, capability wrappers
- `architecture/daemon.md` — daemon persistence, session lifecycle, transport
- `architecture/runtime.md` — eggsec-runtime core types and invariants
- `architecture/runtime_bridge.md` — surface conversion, preflight/approval flow
- `architecture/config.md` — enforcement model, LoadedScope, policy system
- `architecture/cli_commands.md` — CLI parsing, command registry, handlers
- `architecture/tui.md` — TUI tabs, themes, enforcement facade
- `architecture/python_api.md` — Python bindings contract, stable operations

## Skills

Load relevant skills via the `skill` tool when working in specific domains. The canonical skills directory is `.opencode/skills/`; `.skills/`, `.claude/skills/`, and `.agents/skills/` are symlinks to it (update skills in one place, all platforms see them):

`eggsec-agent`, `eggsec-ai`, `eggsec-architecture-review`, `eggsec-auth`, `eggsec-browser`, `eggsec-c2`, `eggsec-cli`, `eggsec-config`, `eggsec-container`, `eggsec-daemon`, `eggsec-db-pentest`, `eggsec-distributed`, `eggsec-evasion`, `eggsec-fuzzer`, `eggsec-hunt`, `eggsec-loadtest`, `eggsec-mobile`, `eggsec-nse`, `eggsec-output`, `eggsec-packet`, `eggsec-pipeline`, `eggsec-postex`, `eggsec-proxy`, `eggsec-python`, `eggsec-recon`, `eggsec-scanner`, `eggsec-security`, `eggsec-stress`, `eggsec-tool`, `eggsec-tui`, `eggsec-waf`, `eggsec-wireless`

See the Module Index table above for the module-to-skill mapping.

## Planning Notes

- **Plan lifecycle**: Implementation plans in `plans/` are retained (with `Status: Executed` header) for NSE milestones and multi-phase correctness efforts. Don't delete phase plan files ad hoc.
- **Verify before implementing**: Always check file paths, line numbers, and whether issues still exist.
- **Error pattern verification**: Some `let _ =` patterns are followed by proper `tracing::warn!`. Verify full context before claiming silent suppression.
- **Wave plan verification**: Plans may contain stale assertions. Check actual codebase state.
