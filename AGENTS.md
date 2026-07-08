# AGENTS.md

Guidelines for AI agents working on this codebase.

## Quick Verification

Before claiming code is correct, run these in order:

```bash
cargo fmt --all --check          # format
cargo clippy --lib -p eggsec     # lint (pre-existing warnings OK)
cargo test --lib -p eggsec       # unit tests
cargo test -p eggsec --test feature_matrix   # feature metadata
cargo test -p eggsec --test enforcement_matrix
bash scripts/check-architecture-guards.sh    # requires ripgrep
```

Or use the Makefile (requires `cargo-nextest`): `make check-architecture-ci`

Feature-gated crates need explicit features: `cargo check -p eggsec --features mobile`, `cargo check -p eggsec --features db-pentest`, etc.

## Project Overview

Eggsec is a Rust security testing toolkit organized as a Cargo workspace with 14 crates:

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
| `eggsec-ui-model` | Frontend-neutral view DTOs |

## Build & Test Commands

### Essential verification (run before any change)

```bash
cargo fmt --all --check
cargo clippy --lib -p eggsec -- -D warnings
cargo test --lib -p eggsec
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
bash scripts/check-architecture-guards.sh
```

### Full architecture CI reproduction

```bash
make check-architecture-ci    # or the individual commands in scripts/check-architecture-guards.sh
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

# Domain crates (standalone)
cargo check -p eggsec-db-lab
cargo check -p eggsec-web-proxy
cargo check -p eggsec-mobile-lab
cargo check -p eggsec-nse --features nse

# CLI variants
cargo check -p eggsec-cli                          # default (TUI + daemon-client)
cargo check -p eggsec-cli --no-default-features    # headless
cargo check -p eggsec-cli --no-default-features --features daemon-client  # daemon client only

# Workspace build (CI baseline)
cargo check --workspace --no-default-features
```

### Makefile targets

Requires `cargo-nextest` (`cargo install cargo-nextest`):

```bash
make test                  # unit tests only (default)
make test-ci               # full suite, no retries
make clippy                # lint (-D warnings)
make fmt                   # format check
make test-feature-matrix   # feature + metadata validation
make check-architecture-ci # full architecture guard CI reproduction
make check-no-default      # no-default-features workspace build
make build                 # release build
```

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
4. **eggsec-runtime** must be dependency-light (serde, tokio, tracing only). No TUI, no transport deps. Enforced by architecture guards.
5. **eggsec-output** must not depend on `eggsec` (engine) or `eggsec-runtime`. Only depends on `eggsec-core`.
6. **eggsec-daemon** must not depend on TUI or engine crates. Only depends on `eggsec-runtime`.

### Runtime dispatch flow

```
TUI → TuiTaskDispatcher → eggsec::dispatch::dispatch_inner() → TaskResult
CLI → CLI dispatch → eggsec::dispatch::dispatch_inner() → direct output
REST/MCP/Agent → EnforcementContext::evaluate() → EnforcedDispatcher::dispatch_checked() → tool execution
```

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
  eggsec-ui-model/  # frontend view DTOs
  eggsec-db-lab/    # database pentest domain
  eggsec-web-proxy/ # web proxy domain
  eggsec-mobile-lab/ # mobile analysis domain
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
| `nse` | `libssl-dev` | Nmap NSE scripts |
| `evasion` | none | Evasion detection |
| `postex` | none | Post-exploitation simulation |
| `c2` | none | C2 simulation (depends on postex+evasion) |
| `stress-testing` | none | Raw sockets, IP spoofing |
| `packet-inspection` | `libpcap-dev` | Packet capture |
| `http-api` | none | Daemon HTTP transport (axum) |

Marker features (no deps): `rest-api`, `grpc-api`, `tool-api`, `insecure-tls`, `api-schema`, `sbom`, `container`, `ai-integration`, `websocket`, `headless-browser`, `database`, `cloud`, `git-secrets`, `pdf`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`, `c2-mcp`, `web-proxy-mcp`, `transparent-proxy`, `dynamic-plugins`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`

CLI features: `tui` (default), `daemon-client`, `headless`

Aggregate: `full` — all non-default features. Not conservative/production.

## Key Patterns

- **Severity Enum**: Canonical in `eggsec-core::types`. Re-export, don't recreate.
- **FxHashMap**: Use `rustc_hash::FxHashMap`/`FxHashSet` in performance paths, not std collections.
- **Regex Caching**: `lru = "0.18"` with cache size 100 (NonZeroUsize).
- **Truncation**: `utils/formatting.rs` — `strip_controls` (recommended), `preserve_all`.
- **Error Handling**: Avoid `unwrap_or_default()` on async ops; use explicit match with tracing.
- **Hash Collections**: `rustc_hash::FxHashMap` for hot paths.
- **PayloadType location**: `fuzzer/payloads/mod.rs`, NOT `types.rs`.
- **Visual Regression**: `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()`.
- **AI Cache Keys**: Always use `CacheKeyBuilder` to avoid collisions.
- **Themes**: 50 packaged via LZMA. Run `python3 scripts/package_themes.py` after modifying `themes/*.toml`.

## Lessons Learned

- **TUI bounds checking**: Always use `.get(i)`, not `chunks[i]`.
- **TUI is_running()**: All input/navigation handlers must check `!self.is_running()`.
- **TUI reset()**: Must reset all state (selectors, checkboxes, fields, focus areas).
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` — always log with tracing.
- **Timeout wrappers**: All spawned tokio tasks need timeout wrappers (30-300s).
- **File paths**: Use `commands/handlers/`, not `cli/handlers/` (doesn't exist).
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top before flagging.
- **Count verification**: Always verify statistical claims against actual source.
- **Orphan directories**: `crates/eggstack-tui/` and `crates/slapper/` are orphan dirs — do not reference.
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

See `docs/CI_ARCHITECTURE_GUARDS.md` for the full inventory.

## Module Override Files

Each module has specialized guidance in `AGENTS.override.md`:

| Module | File |
|--------|------|
| `agent/` | `crates/eggsec/src/agent/AGENTS.override.md` |
| `ai/` | `crates/eggsec/src/ai/AGENTS.override.md` |
| `fuzzer/` | `crates/eggsec/src/fuzzer/AGENTS.override.md` |
| `scanner/` | `crates/eggsec/src/scanner/AGENTS.override.md` |
| `tui/` | `crates/eggsec-tui/src/AGENTS.override.md` |
| `waf/` | `crates/eggsec/src/waf/AGENTS.override.md` |
| `recon/` | `crates/eggsec/src/recon/AGENTS.override.md` |
| `tool/` | `crates/eggsec/src/tool/AGENTS.override.md` |
| `config/` | `crates/eggsec/src/config/AGENTS.override.md` |
| `output/` | `crates/eggsec/src/output/AGENTS.override.md` |
| `proxy/` | `crates/eggsec/src/proxy/AGENTS.override.md` |
| `stress/` | `crates/eggsec/src/stress/AGENTS.override.md` |
| `distributed/` | `crates/eggsec/src/distributed/AGENTS.override.md` |
| `packet/` | `crates/eggsec/src/packet/AGENTS.override.md` |
| `loadtest/` | `crates/eggsec/src/loadtest/AGENTS.override.md` |
| `mobile/` | `crates/eggsec/src/mobile/AGENTS.override.md` |
| `pipeline/` | `crates/eggsec/src/pipeline/AGENTS.override.md` |
| `nse/` | `crates/eggsec-nse/AGENTS.override.md` |
| `container/` | `crates/eggsec/src/container/AGENTS.override.md` |
| `db_pentest/` | `crates/eggsec/src/db_pentest/AGENTS.override.md` |
| `wireless/` | `crates/eggsec/src/wireless/AGENTS.override.md` |
| `evasion/` | `crates/eggsec/src/evasion/AGENTS.override.md` |
| `c2/` | `crates/eggsec/src/c2/AGENTS.override.md` |
| `postex/` | `crates/eggsec/src/postex/AGENTS.override.md` |

## Architecture Docs

Canonical references for system design:

| Document | Covers |
|----------|--------|
| `docs/ARCHITECTURE.md` | Workspace ownership, enforcement model, execution flows |
| `docs/ARCHITECTURE_INVARIANTS.md` | 30 normative invariants |
| `docs/FEATURE_MATRIX.md` | Feature inventory, naming, build profiles |
| `docs/ENFORCEMENT_MODES.md` | Dual-mode enforcement contract |
| `docs/COMMAND_REGISTRY.md` | Command registry inventory and dispatch |
| `docs/TOOL_REGISTRATION.md` | Tool registration for MCP/REST/gRPC/agent |
| `docs/EXTENSIBILITY.md` | Contributor guide for adding operations, domains, commands |
| `architecture/overview.md` | System-wide architecture, module index |
| `architecture/tui.md` | TUI event loop, key handling, overlays (33 tabs) |
| `architecture/nse_integration.md` | NSE/Lua integration, milestones, capability wrappers |
| `architecture/domain_contract.md` | DomainDescriptor contract |
| `architecture/report_envelope.md` | Normalized report/evidence envelope |
| `architecture/config.md` | EggsecConfig, Scope, EnforcementContext, ExecutionProfile |
| `architecture/cli_commands.md` | CLI Commands enum, handler dispatch, policy enforcement |
| `architecture/scanner.md` | Port scanning, service fingerprinting, endpoint discovery |
| `architecture/fuzzer.md` | FuzzEngine, payloads, detection algorithms, response filtering |
| `architecture/waf.md` | WAF detection (34 products), bypass techniques, profiles |
| `architecture/recon.md` | 17-module recon pipeline, subdomain enum, tech detection |
| `architecture/auth.md` | Auth testing (brute force, MFA, lockout, session), defense-lab |
| `architecture/web_proxy.md` | Web proxy domain crate, intercept, MCP proxy surface |
| `architecture/distributed.md` | Coordinator/worker architecture, command protocol |
| `architecture/ai_agents.md` | AI client, adaptive fuzzing, WAF bypass, planner |
| `architecture/evasion.md` | 16 evasion techniques, MITRE ATT&CK mapping |
| `architecture/c2.md` | C2 simulation, campaign profiles, agent lifecycle |
| `architecture/mobile.md` | Mobile static/dynamic analysis, Frida integration |
| `architecture/wireless.md` | WiFi recon, active attacks (root required) |
| `architecture/daemon.md` | DaemonStore, SQLite schema, lifecycle persistence |
| `architecture/database_pentest.md` | Database pentest domain, correlation engine |

## Skills

Load relevant skills via the `skill` tool when working in specific domains. Skills are in `.opencode/skills/`:

`eggsec-agent`, `eggsec-ai`, `eggsec-architecture-review`, `eggsec-auth`, `eggsec-browser`, `eggsec-cli`, `eggsec-config`, `eggsec-distributed`, `eggsec-evasion`, `eggsec-fuzzer`, `eggsec-hunt`, `eggsec-loadtest`, `eggsec-nse`, `eggsec-output`, `eggsec-packet`, `eggsec-pipeline`, `eggsec-proxy`, `eggsec-recon`, `eggsec-scanner`, `eggsec-security`, `eggsec-stress`, `eggsec-tool`, `eggsec-tui`, `eggsec-waf`

## Planning Notes

- **Plan lifecycle**: Implementation plans in `plans/` are retained (with `Status: Executed` header) for NSE milestones and multi-phase correctness efforts. Don't delete phase plan files ad hoc.
- **Verify before implementing**: Always check file paths, line numbers, and whether issues still exist.
- **Error pattern verification**: Some `let _ =` patterns are followed by proper `tracing::warn!`. Verify full context before claiming silent suppression.
- **Wave plan verification**: Plans may contain stale assertions. Check actual codebase state.
