# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Slapper is a Rust-based security testing toolkit. See `README.md` for features and `ARCHITECTURE.md` for design details.

## Quick Reference

### Build & Test Commands

```bash
# Check compilation
cargo check --lib -p slapper

# Run library tests
cargo test --lib -p slapper

# Run specific integration test
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper

# Lint
cargo clippy --lib -p slapper

# Build release
cargo build --release -p slapper
```

### Code Organization

```
crates/slapper/
├── src/
│   ├── cli/           # Command-line argument parsing
│   ├── commands/      # Command handlers
│   ├── config/        # Configuration (SlapperConfig, PathsConfig, Scope)
│   ├── constants.rs   # Centralized constants (WAF, HTTP, scan, etc.)
│   ├── types.rs       # Shared types (Severity, SensitiveString)
│   ├── fuzzer/        # Fuzzing engine (22+ payload types)
│   │   └── payloads/
│   │       └── macros.rs  # payload_vec! macro
│   ├── scanner/       # Port scanning, endpoint discovery
│   │   └── ports/     # Port scanning (mod.rs + spoofed.rs)
│   ├── waf/           # WAF detection and bypass
│   ├── recon/         # Reconnaissance modules
│   ├── output/        # Report generation (JSON, HTML, SARIF, JUnit)
│   ├── tool/          # Tool abstraction layer
│   │   └── protocol/
│   │       └── mcp/   # MCP server (mod.rs, handlers.rs, routes.rs, types.rs, auth.rs, streaming.rs)
│   ├── tui/           # Terminal UI
│   │   └── app/       # App state and logic (mod.rs, runner.rs, error.rs, input.rs, options.rs)
│   └── utils/         # Common utilities
├── tests/             # Integration tests
└── Cargo.toml
```

### Key Types

- `SlapperConfig` - Main configuration (use `config::load_config()`)
- `PathsConfig` - Directory paths (flattened into SlapperConfig)
- `SpoofConfig` - IP spoofing settings
- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `PayloadType` - Enum of 23 payload categories
- `Severity` - Canonical severity rating (in `types.rs`, re-exported everywhere)
- `SensitiveString` - Zeroized credential wrapper (in `types.rs`)

### Feature Flags

- `stress-testing` - Enables ICMP probing, IP spoofing, raw sockets
- `packet-inspection` - Packet capture features
- `python-plugins` / `ruby-plugins` - Plugin language support
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `nse-sandbox` - NSE sandbox mode (restricts `io.popen`, `os.setenv`, filesystem access)
- `ai-integration` - AI/LLM features (planned)
- `full` - All features combined

Note: `mcp-server` feature has been removed. Use `rest-api` instead.

### PyO3 Dependency

- Current version: 0.25 (supports Python 3.14)
- In `crates/slapper-plugin/Cargo.toml`: `pyo3 = { version = "0.25", features = ["auto-initialize"], optional = true }`
- When upgrading: check PyO3 CHANGELOG for breaking changes; `Python::with_gil` still works in 0.25 (renamed to `Python::attach` in 0.26)

## Codebase Health

### Severity Enum (Unified)

Single canonical definition in `types.rs`. All other modules re-export from it:

| Re-export path | Source |
|---------------|--------|
| `fuzzer::payloads::Severity` | `pub use crate::types::Severity` |
| `waf::types::Severity` | `pub use crate::types::Severity` |
| `config::Severity` | `pub use crate::types::Severity` |
| `recon::secrets::Severity` | `pub use crate::types::Severity` |
| `output::agent::Severity` | `pub use crate::types::Severity` |
| `output::trend::Severity` | `pub use crate::types::Severity` |

The `tool/response.rs` module uses a separate `ResponseSeverity` enum with an extra `None` variant for API compatibility.

**When adding new code:** re-export from `crate::types::Severity`. Do not create a new definition.

### Tool Abstraction Layer (Already Exists)

`tool/traits.rs:117` has `SecurityTool` trait, `tool/registry.rs:9` has `ToolRegistry`. These are feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`). Do not re-implement.

### SensitiveString

Credentials (API keys, passwords, PSKs, webhook secrets) use `SensitiveString` from `types.rs`:
- Zeroizes on drop
- `expose_secret()` borrows the inner string
- `into_secret()` consumes and returns the inner string
- `log_secret()` logs safely with redaction option
- `for_logging()` creates display-safe wrapper for logging
- `Debug` and `Display` show `[REDACTED]`
- Constant-time equality (via `subtle::ConstantTimeEq`)
- Serializes transparently for config file compatibility

### Circuit Breaker

`utils/circuit_breaker.rs` provides circuit breaker pattern for external API resilience:
- `CircuitBreaker` - individual breaker with state (Closed/Open/HalfOpen)
- `CircuitBreakerRegistry` - manages multiple breakers by name
- Tracks failure/success counts, total calls, failure rate
- Configurable failure threshold, success threshold, and timeout

### Truncation Functions

Two truncation utilities in `utils/formatting.rs`:
- `strip_controls` - removes control characters (recommended)
- `preserve_all` - preserves all characters

Both use `.chars().take()` for safe character-based truncation (no byte slicing panic risk).

### Macros

`fuzzer/payloads/macros.rs` defines `payload_vec!` for building payload vectors from inline data. Reduces repetitive `for` loops in payload modules (e.g., sqli.rs went from 8 loops to 1 macro call).

### WAF Constants

`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

### TLS

`rustls` (0.23) + `tokio-rustls` (0.26) is the only TLS backend for the main `slapper` crate. `native-tls` has been removed.
- `distributed/io.rs` — `StreamWrapper` enum with `Plain`, `TlsClient`, `TlsServer` variants
- `TlsServer::from_pem(cert_path, key_path)` — loads PEM cert + key files
- `TlsClient::new(domain)` — creates client with `NoVerifier` (insecure, for internal use)
- `recon/ssl.rs` uses `rustls_pki_types::CertificateDer` for cert extraction

**Exception:** `slapper-nse` retains `native-tls` (OpenSSL) for Nmap NSE script compatibility. This is intentional — Nmap scripts expect OpenSSL-based TLS behavior. Do not migrate `slapper-nse` to `rustls`.

### Spoofed Scanner

`scanner/ports/spoofed.rs` contains raw socket scanning (feature-gated). `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled. Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing.

| Metric | Value |
|--------|-------|
| Tests | ~974 passing, 2 failing (`negative_tests.rs`) |
| Build | Clean compilation |
| Clippy | 0 warnings (default features) |
| Doctests | 17 pass, 1 ignored, 0 fail |
| `SlapperError` variants | 23 |
| `once_cell` in slapper | 0 (replaced with `std::sync::LazyLock`) |
| MSRV | 1.80 |
| `thiserror` | 2.x |
| Ruby plugins | Zero warnings with `--features ruby-plugins` |
| Largest file | `tui/app/mod.rs` (664 lines — split into submodules) |
| Source files | 406 `.rs` files |
| TUI files | 60 `.rs` files |
| Tab variants | 29 |

## Planning

- `plan.md` — Consolidated improvement plan (10 waves, parallelizable in blocks)

## Lessons Learned

### Configuration

- `PathsConfig` fields are flattened via `#[serde(flatten)]` for backward compatibility
- Existing config files with top-level `custom_payloads_dir` etc. still work

### Testing

- Negative tests should use specific assertions, not `assert!(result.is_err() || result.is_ok())`
- Check actual error messages: `err.to_string().contains("expected substring")`
- Use `SpoofConfig::from_args()` with `Option<usize>` for decoy_count

### Common Pitfalls

1. **Type mismatches**: `ScopeRule::new()` takes `String`, not `&str`
2. **Option types**: `decoy_count` is `Option<usize>`, not `usize`
3. **Unused imports**: Move feature-gated imports inside `#[cfg(...)]` blocks
4. **Feature-gated dead code**: Functions used only under `#[cfg(feature = "...")]` appear as dead code to the compiler. Gate the module declaration itself, not just callers.
5. **Clippy redundant closures**: `.map(|arr| func(arr))` should be `.map(func)` when the argument is passed directly
6. **Clippy needless borrows**: `.post(&format!(...))` should be `.post(format!(...))` when the format result implements the required traits
7. **`default_value = "None"` on Options**: Never use `#[arg(default_value = "None")]` on `Option<T>` fields — clap assigns the string `"None"` instead of `None`. Omit `default_value` entirely; `Option` defaults to `None` automatically.
8. **`fingerprint_services` signature**: Takes 5 args: `host`, `ports`, `timeout`, `tui_mode`, `concurrency` — don't forget `concurrency`

### Severity Enum

- `Severity` has custom `Ord`/`PartialOrd` implementations using `as_int()` for correct semantic ordering (Critical > High > Medium > Low > Info)
- Use `as_int()` for numeric severity comparisons
- `Display` outputs UPPERCASE ("CRITICAL"), `as_str()` outputs lowercase ("critical")
- `serde` serialization uses lowercase (due to `#[serde(rename_all = "lowercase")]`)
- `Severity` implements `FromStr` trait; inherent method renamed to `parse_or_default`

### SensitiveString

- Field is private; use `expose_secret()` (borrow) or `into_secret()` (consume)
- `into_secret()` uses `std::mem::take` internally to work with `ZeroizeOnDrop`
- `PartialEq` uses constant-time comparison; safe for credential checking
- Config deserialization works transparently — existing TOML files with plain strings still load

### TUI-Specific Patterns

- `tui/app/runner.rs` contains the main event loop (`run_app`)
- `tui/app/mod.rs` contains the `App` struct (664 lines); split into submodules: `navigation.rs`, `command.rs`, `export.rs`, `state_update.rs`, `task_management.rs`
- `tui/workers/` directory contains 8 files: `runner.rs`, `scanner.rs`, `fuzzer.rs`, `network.rs`, `api.rs`, `recon.rs`, `security.rs`, `pipeline.rs`
- Tab dispatch uses match statements across ~18+ methods (29-arm matches)
- TUI uses ratatui 0.30 + crossterm 0.28 with immediate-mode rendering
- 29 tab variants exist (Recon=0 through Vuln=28); all 29 are fully functional
- `tui/app/mod.rs` contains ~664 lines - uses dispatch macros in `dispatch.rs` for tab delegation
- 6 dispatch macros exist: `dispatch!`, `dispatch_void!`, `dispatch_bool!`, `dispatch_page!`, `dispatch_is_at_edge!`, `dispatch_reset!`
- Tab cfg attributes: `Nse` and `Plugin` variants are always present in the Tab enum; use both `#[cfg(feature = "...")]` and `#[cfg(not(feature = "..."))]` arms for feature-gated dispatch

### Output Module

- `output/convert.rs` converts findings to HTML, JUnit, SARIF, JSON
- `output/junit.rs` generates JUnit XML reports

### Scope Module

- `config/scope.rs` handles target scope validation
- `ScopeRule` supports wildcard patterns (`*.example.com`)
- Wildcard matching **includes** apex domain (`*.example.com` matches `example.com`)
- `TargetScope` has `host` and `ip` fields (no `pinned_ip` — that field does not exist)

## Style Guidelines

- Use `anyhow::Result` for error handling in commands/TUI/tests
- Use `crate::error::Result` (`SlapperError`) in core library modules
- Add doc comments to public functions with `# Examples` and `# Errors`
- Keep modules focused - split files > 500 lines
- Follow existing patterns in neighboring code

## Plugin System

### Feature Flag Interactions

- `python-plugins` enables `slapper-plugin` (with `pyo3` + `dirs`) and exports `crate::plugin`
- `ruby-plugins` enables both `slapper-plugin` (with `magnus`) and `slapper-ruby` (with `magnus`)
- `commands/handlers/plugin.rs` is gated on `any(feature = "python-plugins", feature = "ruby-plugins")`
- The `crate::plugin` re-export in `lib.rs` is gated on `any(feature = "python-plugins", feature = "ruby-plugins")`
- `slapper-plugin` has separate feature flags: `python-plugins` (pyo3) and `ruby-plugins` (magnus)
- TUI plugin tab is gated on `any(feature = "python-plugins", feature = "ruby-plugins")` in all TUI files

### Ruby Plugin Thread Safety

`RubyBridge` is NOT `Send + Sync` (magnus `Ruby` type has `PhantomData<*mut ()>`). Thread safety is achieved via message-passing:

- `RubyPluginClient` spawns a dedicated `ruby-vm` thread that owns the `RubyBridge`
- Communication via `std::sync::mpsc` channels — each request gets its own response channel
- `RubyPluginAdapter` holds `Arc<RubyPluginClient>` — naturally `Send + Sync`, no unsafe code
- The `unsafe impl Send + Sync` on `RubyBridge` has been REMOVED — the bridge is now private

### Magnus 0.8 API (slapper-plugin/src/ruby.rs)

- `eval::<()>` is not valid — use `let _: Value = eval(...)` to discard result
- `funcall` returns `Result<Value>` — use explicit `Value` return type, not turbofish `::<_, Value>`
- Hash field access uses `RHash::lookup::<_, Value>(key)` + `String::try_convert(v)`, not `funcall("get", ...)` + `to_s()`
- Array iteration uses `RArray::each()` which yields `Result<Value>`

### ProgressStyle Template

Always use `.unwrap_or_else(|_| ProgressStyle::default_bar())` instead of `.unwrap()` on `ProgressStyle::template()`. The template method can fail on invalid format strings.

### Plugin Command Handler

`commands/handlers/plugin.rs` uses `slapper_plugin::Plugin` trait methods (e.g., `run_check`). The trait must be imported:
```rust
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
use slapper_plugin::Plugin;
```

### Output Patterns

Use the appropriate output method based on the context:

- **`eprintln!`** — Progress messages (user-initiated operations, step-by-step feedback)
- **`tracing::warn!`** — Recoverable logged issues (retries, degraded functionality)
- **`tracing::error!`** — Unrecoverable errors (failures that halt operations)
- **`println!`** — Final output only (scan results, reports, completion messages)

The TUI has its own rendering layer; use `tracing` for logging from background workers.

### TUI Feature-Gated Dispatch

When writing match arms for feature-gated tab variants (`Nse`, `Plugin`), always provide BOTH arms:
```rust
#[cfg(feature = "nse")]
Tab::Nse => self.nse.method(),
#[cfg(not(feature = "nse"))]
Tab::Nse => { /* fallback */ },
```
Without the `#[cfg(not(...))]` arm, compilation fails when the feature is disabled because the enum variant still exists but has no matching arm.

### New Feature Modules

#### AI Integration Module (`ai/`)

The AI module provides integration with LLM APIs for security testing:
- `ai/client.rs` — HTTP client for OpenAI-compatible APIs with `apply_auth()` helper
- `ai/payloads.rs` — `AiPayloadGenerator` with HashMap cache for payload suggestions
- `ai/waf_bypass.rs` — `SmartWafBypass` with knowledge base persistence to `~/.config/slapper/waf_bypasses.json`
- `ai/adaptive.rs` — `AdaptiveScanEngine` with strategy adjustment based on findings

Feature gate: `#[cfg(feature = "ai-integration")]` in `lib.rs`.

#### Agent Registry (`tool/agents/`)

The agent module provides multi-agent orchestration:
- `tool/agents/registry.rs` — `AgentRegistry` with Arc<RwLock<HashMap>> for async CRUD
- `tool/agents/delegation.rs` — `DelegationRequest`/`DelegationResponse` types

Feature gate: `#[cfg(feature = "rest-api")]` in `tool/mod.rs`.

#### MCP Prompts & Sampling

- `tool/protocol/mcp/prompts.rs` — 7 builtin prompt templates with `get_builtin_prompts()`
- `tool/protocol/mcp/sampling.rs` — Request/response types for AI completions

Feature gates: prompts always available, sampling gated on `ai-integration`.

#### OpenAI Protocol Module

- `tool/protocol/openai/` — Chat completions endpoint at `/v1/chat/completions`
- Auto-generates tool definitions from `ToolRegistry`

Feature gate: `#[cfg(feature = "rest-api")]` in `tool/protocol/mod.rs`.

#### CI/CD Module

- `cli/ci.rs` — CI-specific command with `--fail-on`, `--baseline`, `--quiet` flags
- `commands/handlers/ci.rs` — Handler with exit codes (0=pass, 1=fail, 2=error, 3=scope violation)
- `output/baseline.rs` — `BaselineComparison` struct for regression detection

#### Plan Command

- `cli/plan.rs` — Preview execution plans without running them
- `commands/handlers/plan.rs` — Handler that outputs JSON or formatted table

#### Deduplication Engine

- `output/dedup.rs` — `DedupEngine` with `Strict`, `Fuzzy`, `Disabled` strategies

#### AI Output Schema

- `output/ai_schema.rs` — `AiOutput`, `AiFinding`, `AiEvidence`, `AiRemediation`, `AiSummary` types

### Lessons Learned (Session 2026-04-05)

#### Test coverage improvements

- Added `#[cfg(test)]` modules to 10 recon modules (asn, cve, cve_lookup, dns_enhanced, ssl, subdomain, techdetect, threatintel, wayback, runner) — added 77 new tests
- Test count: 851 → 974 (verified with `cargo test --lib -p slapper -- --list 2>/dev/null | wc -l`)

#### Stub module implementation

- All 8 stub modules (`container`, `storage`, `supply_chain`, `hunt`, `compliance`, `integrations`, `workflow`, `vuln`) are now implemented with real functionality:
  - `TaskConfig` enum variants include config/mode parameters
  - `TaskResult` enum variants use real result types (`StorageListScans`, `StorageListFindings`, `IntegrationsCreateIssue`, `IntegrationsSearchIssues`, `Workflow(WorkflowReport)`, `Vuln(VulnAssessment)`)
  - `VulnAssessment` struct added to `vuln/mod.rs`
  - `Issue` struct updated with `id`, `status`, `url`, `created_at` fields — required fixing all 6 construction sites in github.rs, gitlab.rs, jira.rs
  - Tab `get_mode()` methods added for Storage, Integrations, Workflow, Vuln tabs
  - `build_*_task()` methods pass config from tab state to worker

#### Compliance checks expansion

- `run_compliance_task()` expanded from ~7 to 15 checks:
  - HTTPS enforcement, HSTS, X-Content-Type-Options, X-Frame-Options/CSP frame-ancestors
  - server/X-Powered-by, status codes, CSP, Referrer-Policy, Permissions-Policy
  - cache-control on sensitive pages, HttpOnly/Secure/SameSite cookies
  - CORS wildcard, X-XSS-Protection

#### Performance optimizations

- `ScrollableText::render()` — replaced `Vec::with_capacity` + loop with `.cloned().collect()` iterator pattern

### Lessons Learned (Session 2026-04-07)

#### Plan consolidation

- Consolidated 3 plan files (plan2.md, plan3.md, plan4.md) into single `plans/plan.md`
- Organized into 6 waves with parallelizable blocks
- Before implementing, verify file paths using `glob` or `rg` - some planned features already implemented (TabState, TabRender, TabInput traits exist)
- Count actual source files with `find crates/slapper/src -name '*.rs' | wc -l` (406 files)
- Count tests with `cargo test --lib -p slapper -- --list 2>/dev/null | wc -l` (976 tests)
- Before implementing any plan item, verify file paths exist using `glob` or `rg`
- Verify codebase metrics (test counts, file sizes, line counts) against actual code before referencing in plans
- The plan uses waves organized into parallelizable blocks where items within each block are independent

#### Known bugs identified (not yet fixed)

- UTF-8 panic in `InputField::delete()` and `backspace()` — uses byte indices instead of char boundaries
- Grammar fuzzer payloads all tagged as `PayloadType::Xss` regardless of actual grammar type
- CSV export doesn't escape `severity` and `cve_ids` fields
- `PortScanResults::ports_scanned` is `u16` — overflows at 65,536
- `ConfigError::Io(String)` wraps string instead of `std::io::Error` — loses error chain
- `SlapperConfig::load`/`save` take `&PathBuf` instead of `impl AsRef<Path>`
- `ResponseSeverity` lacks `Ord`/`PartialOrd` implementations

#### Verification best practices

- Always verify plan items against actual codebase before assuming they still apply
- Use `rg` to confirm file paths, line numbers, and patterns exist
- Run `cargo test --lib -p slapper` after each change to catch regressions
- Use `cargo clippy --lib -p slapper` to verify no new warnings
- Check test counts with `cargo test --lib -p slapper -- --list 2>/dev/null | wc -l`
- Count source files with `find crates/slapper/src -name '*.rs' | wc -l`

### Lessons Learned (Session 2026-04-03)

#### Plan consolidation

- Multiple plan files should be consolidated into a single `plan.md` in the `plans/` directory
- Waves should be organized into parallelizable blocks (A, B, C, etc.) where items within each block are independent
- Before implementing any plan item, verify file paths exist using `glob` or `rg`

#### Known bugs FIXED (2026-04-03)

- ✅ WebSocket payloads mislabeled as `PayloadType::GraphQL` in `fuzzer/payloads/websocket.rs` — Fixed by adding `PayloadType::Websocket` variant and updating `websocket::get_payloads()` to use correct type
- ✅ `AiConfig` field is `base_url`, not `api_url` — Fixed `ai/client.rs` to use `self.config.base_url`
- ✅ `AiConfig` missing `temperature` field — Added `pub temperature: Option<f64>` to `config/settings.rs`
- ✅ `AiConfig.api_key` should be `Option<SensitiveString>` — Changed from `SensitiveString` to `Option<SensitiveString>` with `#[serde(default)]`

#### Feature flag patterns

- New feature flags follow the pattern: optional dep in `Cargo.toml` + `#[cfg(feature = "...")]` in code
- The `full` feature should include all new optional flags
- `grpc-api` and `nse-sandbox` are intentionally excluded from `full`

### Verification Best Practices

When working with improvement plans or code reviews:
- Verify every item against the actual codebase before assuming it still applies
- Use `rg` to confirm file paths, line numbers, and patterns exist
- Plans may describe issues from earlier code states that have since been resolved
- Run `cargo test --lib -p slapper` after each change to catch regressions
- Use `cargo clippy --lib -p slapper` to verify no new warnings

### Lessons Learned (Session 2026-04-02)

#### Clippy warnings can be auto-fixed

Many clippy warnings can be automatically fixed with:
```bash
cargo clippy --fix --lib -p slapper --allow-dirty
```

#### Doc tests must compile as standalone examples

Doc test examples in `error/mod.rs` and similar files:
- Must use actual types and values that compile
- Cannot use `reqwest::Error::from(std::io::Error::new(...))` because `reqwest::Error` doesn't have such a constructor
- Use direct `SlapperError::Timeout {...}` construction instead

#### TUI imports matter

When adding `set_error()` implementations to tabs:
- `ratatui::text::Line` and `ratatui::style::Style` may need to be imported
- Use `#[allow(unused_imports)]` temporarily if unsure, then run clippy to identify what's actually needed
- For `set_error()` in tabs like ResumeTab, only `Line` was needed (not `Span` and `Style` which were already in scope)

#### Dead code removal impact

- Removing a private helper function used by multiple callers in the same module: add the import first, then remove the duplicate
- When removing duplicate `centered_rect()` from `tui/ui.rs`, needed to:
  1. Add import: `use crate::tui::components::popup::centered_rect;`
  2. Export it from `tui/components/mod.rs`: `pub use popup::centered_rect;`
  3. Update import in `tui/ui.rs` to use the re-export path

#### Function signature changes

When adding a new parameter to a public async function like `scan_endpoints()`:
- All callers must be updated (TUI, CLI, tests, benchmarks)
- The new parameter should default to a safe value to minimize breaking changes
- The `verify_tls` parameter was already properly implemented and used
