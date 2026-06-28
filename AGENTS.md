# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Eggsec is a Rust-based security testing toolkit organized as a workspace with 8 crates: `eggsec-core`, `eggsec-tool-core`, `eggsec`, `eggsec-nse`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`, and `eggsec-agent`. See `README.md` for features and `architecture/overview.md` for design details.

## Quick Reference

### Build & Test Commands

```bash
cargo check -p eggsec-core
cargo check -p eggsec-tool-core
cargo check --lib -p eggsec
cargo check -p eggsec --features mobile
cargo test --lib -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-nse
cargo check -p eggsec-output
cargo test -p eggsec-core
cargo test -p eggsec-tool-core
cargo test -p eggsec-output
cargo test --lib -p eggsec
cargo test --test negative_tests -p eggsec
cargo test --test scanner_tests -p eggsec
cargo clippy --lib -p eggsec
cargo build --release -p eggsec-cli
```

#### Feature-Specific Build & Test

```bash
# db-pentest
cargo check -p eggsec --features db-pentest
cargo test --lib -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest

# Wireless
cargo check -p eggsec --features wireless
cargo test --lib -p eggsec --features wireless
cargo clippy --lib -p eggsec --features wireless

# wireless-advanced (deauth; requires wireless feature)
cargo check -p eggsec --features wireless-advanced
cargo test --lib -p eggsec --features wireless-advanced
cargo clippy --lib -p eggsec --features wireless-advanced

# mobile-dynamic
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
cargo clippy --lib -p eggsec --features mobile-dynamic

# web-proxy
cargo check -p eggsec --features web-proxy
cargo test --lib -p eggsec --features web-proxy
cargo clippy --lib -p eggsec --features web-proxy

# web-proxy-mcp
cargo check -p eggsec --features web-proxy-mcp
cargo test --lib -p eggsec --features web-proxy-mcp
cargo clippy --lib -p eggsec --features web-proxy-mcp

# Evasion, postex, c2
cargo check -p eggsec --features evasion
cargo test --lib -p eggsec --features evasion
cargo check -p eggsec --features postex
cargo test --lib -p eggsec --features postex
cargo check -p eggsec --features c2
cargo test --lib -p eggsec --features c2
cargo check -p eggsec --features c2-mcp
cargo test --lib -p eggsec --features c2-mcp
```

#### Make Targets

Requires `cargo-nextest` (`cargo install cargo-nextest`). Uses `cargo-nextest` instead of `cargo test`.

```bash
make test          # unit tests only (default, fast)
make test-ci       # full suite, no retries (CI-style)
make test-integration  # integration tests (wiremock, may need network)
make test-nse      # NSE tests (requires nse feature)
make test-slow     # run ignored tests
make clippy        # lint (-D warnings)
make fmt           # format check
make test-coverage # llvm-cov with rest-api,nse features
make build         # release build
```

> **Note**: CI uses `cargo-tarpaulin` for coverage, while the Makefile uses `cargo llvm-cov`. Both measure the same thing but with different tools.

### Module Override Files

For specialized guidance on specific modules, see `AGENTS.override.md` in each module directory:

| Module | Override File |
|--------|---------------|
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

### Architecture Index

Canonical reference points when updating guidance or skills:

- `architecture/overview.md` - System-wide architecture, module index, data flow
- `architecture/tui.md` - TUI event loop, key handling, overlays, tab routing, session persistence
- `architecture/config.md` - Config loading, scope enforcement, TUI settings save semantics
- `architecture/cli_commands.md` - CLI parsing, command dispatch, handler patterns
- `architecture/output.md` - Report formatting, exports, and rendering integration
- `architecture/pipeline.md` - Security assessment pipeline, 18 profiles
- `architecture/scanner.md` - Port scanning and endpoint discovery
- `architecture/fuzzer.md` - Fuzzing engine and payload generation
- `architecture/waf.md` - WAF detection and bypass
- `architecture/recon.md` - Reconnaissance module
- `architecture/distributed.md` - Distributed coordinator/worker architecture
- `architecture/compile_time_baseline.md` - Workspace crate layout and compile-time baseline
- `architecture/mobile.md` - Mobile app static + dynamic analysis
- `architecture/auth.md` - Authentication testing module
- `architecture/c2.md` - C2 framework

### Feature Flags

**Feature-gated modules (require system deps or root for real scans):**

| Feature | Module | System Dep | Notes |
|---------|--------|------------|-------|
| `wireless` | WiFi recon | `wireless-tools` (iwlist) | Passive scans; root/CAP_NET_ADMIN for real; TUI tab present |
| `wireless-advanced` | WiFi active | (needs wireless) | deauth/disassoc; `--allow-active-wireless`; policy gated `Intrusive` |
| `mobile` | APK/IPA static | none | Pure-Rust parsers; local file only |
| `mobile-dynamic` | Mobile dynamic | ADB + device | Phase 1-4a complete; `--allow-dynamic-mobile` for real |
| `db-pentest` | DB security | none (drivers) | Postgres/MySQL/MSSQL/MongoDB/Redis; `--allow-db-pentest` for real |
| `web-proxy` | MITM proxy | none | `--allow-web-proxy` + policy for real interception |
| `evasion` | Evasion detection | none | `--allow-evasion-testing` for real |
| `postex` | Post-ex simulation | none | `--allow-postex` + scope for real |
| `c2` | C2 simulation | none | `--allow-c2`; depends on postex + evasion |
| `stress-testing` | Flood testing | none | Raw sockets, IP spoofing |
| `packet-inspection` | Packet capture | `libpcap-dev` | |
| `nse` | NSE scripts | `libssl-dev` | |

**Marker-only features (no deps, just build gating):**

`tool-api`, `insecure-tls`, `rest-api`, `grpc-api`, `ws-api`, `nse-ssh2`, `nse-sandbox`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `web-proxy-mcp`, `c2-mcp`, `transparent-proxy`, `dynamic-plugins`, `pdf`, `api-schema`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`, `full`

### Key Types

- `Severity` - Canonical definition in `eggsec-core::types`, re-exported by `types.rs`. Don't recreate.
- `SensitiveString` - Zeroized credential wrapper (defined in `eggsec-core::types`)
- `EggsecConfig` - Main configuration (`config::load_config()`)
- `EnforcementContext` - Central policy evaluator (`config/policy_decision.rs`); constructors: `cli`, `mcp_strict`, `agent_strict`, `ci_strict`
- `LoadedScope` - Scope with provenance (`DefaultEmpty`, `ConfigFile`, `CliScopeFile`, `GeneratedPreset`) in `config/scope.rs`
- `ExecutionProfile` - Trust boundary enum: `ManualPermissive`, `ManualGuarded`, `McpStrict`, `AgentStrict`, `CiStrict`
- `ConfirmationClass` - Kebab-case strings for policy confirmations; use `as_str()` for stable IDs
- `TabError` - Structured error type with `is_recoverable()` in `eggsec-tui`
- `PayloadType` - Enum of 40 payload categories; lives in `fuzzer/payloads/mod.rs`, NOT `types.rs`
- `McpProfile` / `McpProfilePolicy` - MCP agent profiles and per-profile tool visibility in `tool/protocol/mcp/`

### Important Patterns

- **Severity Enum**: Single canonical definition in `eggsec-core::types`. Re-export, don't recreate.
- **Tool Abstraction**: `tool/traits.rs` has `SecurityTool` trait, `tool/registry.rs` has `ToolRegistry`
- **Regex Caching**: Use `lru = "0.18"` with cache size 100 (NonZeroUsize)
- **Circuit Breaker**: `utils/circuit_breaker.rs` - `CircuitBreaker` with configurable thresholds
- **Truncation**: `utils/formatting.rs` - `strip_controls` (recommended) and `preserve_all`
- **Visual Regression Testing**: Use `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()` to verify rendered content
- **AI Cache Keys**: Always use `CacheKeyBuilder` for cache keys in AI module to avoid collisions
- **Hash Collections**: Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of std collections for performance
- **Error Handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead
- **Shared Policy Evaluator**: Use `EnforcementContext::evaluate()` (central) in `config/policy_decision.rs` instead of building policy checks inline
- **MCP/Agent Invariant**: For MCP/agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope must come from `LoadedScope`. See `docs/ENFORCEMENT_MODES.md` for the canonical dual-mode enforcement contract.
- **eggsec-output Re-exports**: Use `eggsec_output::Severity` rather than reaching into `eggsec_output::agent::Severity`

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | ~4470 (includes #[test] + #[tokio::test]) |
| Clippy | ~54 warnings (pre-existing, none in ai module) |
| Source files | 878 (.rs files in crates/) |
| Tabs | 33 (Tab enum variants 0-32) |
| Pipeline profiles | 18 |
| Output formats | 8 |
| Themes | 50 packaged + 3 built-in |
| CLI commands | 26 base, 45 total with all features |

### Security Notes

- **Scope Enforcement**: Private IP checks are deferred to scope rule evaluation in `is_target_allowed()` (`config/scope.rs`). Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback private-IP block.
- **MCP Coding Agent**: Default deny posture; stress/load/packet tools are hidden from coding-agent profile
- **Manual Overrides**: `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-*` flags required for others. Strict profiles/MCP/agent never honor overrides.

### Key Patterns (Lessons Learned)

- **TUI bounds checking**: Always use `.get(i)` pattern instead of direct `chunks[i]` indexing
- **TUI is_running() guards**: All input/navigation handlers must check `!self.is_running()` before processing
- **TUI reset() methods**: Must reset all state (selectors, checkboxes, fields, focus areas)
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` - always log with tracing
- **Timeout wrappers**: All spawned tokio tasks should have timeout wrappers (30-300s depending on operation)
- **FxHashMap migration**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` in performance-critical paths
- **Verification before claims**: Always verify line numbers, file paths, and whether issues still exist before including in plans
- **File path conventions**: Use `commands/handlers/` not `cli/handlers/` - the latter directory does not exist
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top - many items flagged in reviews may already be resolved
- **PayloadType location**: `PayloadType` enum is in `fuzzer/payloads/mod.rs`, not `types.rs`
- **`.ok()` vs `if let Ok`**: Not all `.ok()` calls are bugs - `if let Ok` is proper error handling. Verify the context.
- **Count verification**: Always verify statistical claims (file counts, enum variants) against actual source
- **Packaged themes**: Run `python3 scripts/package_themes.py` after modifying `themes/*.toml` to regenerate `crates/eggsec-tui/src/theme/packaged.rs`
- **Theme system**: 50 Halloy-format themes packaged via LZMA. `cyber-red` fallback always available in-code. `Theme::default()` returns `cyber-red`.
- **Theme loader**: `theme/loader.rs` parses Halloy `.toml` themes. Background thread loading via `std::thread::spawn` + `std::sync::mpsc`.

## Skills Directory

Skills are located in `.opencode/skills/`:

| Skill | Purpose |
|-------|---------|
| `eggsec-agent/` | Agent-specific workflows |
| `eggsec-ai/` | AI module workflows |
| `eggsec-architecture-review/` | Architecture document review methodology |
| `eggsec-auth/` | Authentication security testing workflows |
| `eggsec-browser/` | Headless browser security testing |
| `eggsec-cli/` | CLI parsing, command dispatch, handler patterns |
| `eggsec-config/` | Config module workflows |
| `eggsec-distributed/` | Distributed module workflows |
| `eggsec-evasion/` | Evasion technique detection workflows |
| `eggsec-fuzzer/` | Fuzzer module workflows |
| `eggsec-hunt/` | Vulnerability hunting workflows |
| `eggsec-loadtest/` | Loadtest module workflows |
| `eggsec-nse/` | NSE/Lua module workflows |
| `eggsec-output/` | Output module workflows |
| `eggsec-packet/` | Packet capture/crafting/parsing workflows |
| `eggsec-pipeline/` | Pipeline module workflows |
| `eggsec-proxy/` | Proxy module workflows |
| `eggsec-recon/` | Reconnaissance module workflows |
| `eggsec-scanner/` | Scanner module workflows |
| `eggsec-security/` | Security testing skill workflows |
| `eggsec-stress/` | Stress module workflows |
| `eggsec-tool/` | Tool module workflows |
| `eggsec-tui/` | TUI module workflows (includes `tui_testing.md` for visual regression) |
| `eggsec-waf/` | WAF module workflows |

Use the `skill` tool to load relevant skills when tackling tasks in their domain.

## Planning Notes for Future Agents

1. **Plan lifecycle**: Implementation plans in `plans/` are executed and deleted after completion. Focus on the current codebase state rather than plan files.
2. **Verify before implementing**: Always verify file paths, line numbers, and whether issues still exist before implementing.
3. **Error pattern verification**: Some `let _ =` patterns are followed by proper error logging via `tracing::warn!`. Verify the full context before claiming silent suppression.
4. **Wave plan verification**: Plans may contain stale assertions. Use subagents to check actual codebase state.
5. **Orphan directories**: `crates/eggstack-tui/` and `crates/slapper/` are orphan directories not in the workspace. Do not reference or depend on them.
