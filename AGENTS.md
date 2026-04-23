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
│   ├── agent/         # Autonomous agent (event loop, portfolio, memory, alerts, skills)
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
│   │   └── auth/      # Multi-protocol auth testing (ssh_auth, ftp_auth, smtp_auth)
│   ├── output/        # Report generation (JSON, HTML, SARIF, JUnit)
│   ├── wireless/      # Wireless security testing (WiFi scanning, auth testing)
│   ├── tool/          # Tool abstraction layer
│   │   ├── implementations/  # Tool implementations (recon, scanner, fuzzer, waf, search, etc.)
│   │   └── protocol/
│   │       └── mcp/   # MCP server (mod.rs, handlers.rs, routes.rs, types.rs, auth.rs, streaming.rs)
│   ├── scanner/       # Port scanning, endpoint discovery
│   │   ├── templates/ # Nuclei-style template engine
│   │   └── ports/     # Port scanning (mod.rs + spoofed.rs)
│   ├── proxy/         # Proxy modules
│   │   └── intercept/ # Intercepting proxy with dynamic SSL certs
│   ├── fuzzer/        # Fuzzing engine (32+ payload types)
│   │   └── payloads/
│   │       └── macros.rs  # payload_vec! macro
├── tests/             # Integration tests
└── Cargo.toml
```

**Note:** The `slapper_skills/` directory in the project root contains skill files for use with the autonomous agent. This is distinct from the codebase itself which agents work on.

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
- `ai-integration` - AI/LLM features (autonomous agent, skill system, payload generation)
- `full` - All features combined

Note: `mcp-server` feature has been removed. Use `rest-api` instead.

### PyO3 Dependency

- Current version: 0.28 (supports Python 3.14)
- In `crates/slapper-plugin/Cargo.toml`: `pyo3 = { version = "0.28", features = ["auto-initialize"], optional = true }`
- Breaking changes: `Python::with_gil` renamed to `Python::attach` in 0.26; `Bound` API introduced in 0.21 is now standard; GIL lifetime constraints tightened

## Codebase Health

### Current Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1107 passing | |
| Clippy | ~4 warnings | Pre-existing |
| Source files | 470+ | |
| Payload types | 39 | Added OAST |
| Tabs | 29 | + 10 new stubs |
| Skill files | 28 | |

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

The `tool/response.rs` module uses a separate `ResponseSeverity` enum with an extra `None` variant for API compatibility. **Note**: This is being phased out in favor of `Option<Severity>`.

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
- `len()` returns the length of the inner string
- `as_bytes()` returns raw bytes (for proxy auth encoding)
- `is_empty()` checks if empty

**Note:** Proxy credentials (`SocksProxy`, `HttpConnectProxy`) now use `SensitiveString` for secure storage.

### Circuit Breaker

`utils/circuit_breaker.rs` provides circuit breaker pattern for external API resilience:
- `CircuitBreaker` - individual breaker with state (Closed/Open/HalfOpen)
- `CircuitBreakerRegistry` - manages multiple breakers by name
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

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

### Current Codebase Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1107 passing | |
| Build | Clean compilation | |
| Clippy | ~4 warnings | Pre-existing (scan_ports 8 args, collapsible_if) |
| Doctests | 19 pass, 0 fail | All passing |
| `SlapperError` variants | 23 | |
| `once_cell` in slapper | 0 | Replaced with `std::sync::LazyLock` |
| MSRV | 1.80 | |
| `thiserror` | 2.x | |
| Ruby plugins | Zero warnings | With `--features ruby-plugins` |
| Largest file | `tui/app/mod.rs` (883 lines) | Decomposed from 1665 (46% reduction) |
| Source files | 470+ `.rs` files | |
| TUI files | 60 `.rs` files | |
| Tab variants | 29 | |
| Skill files | 28 | |
| Payload types | 38 | Added 6 new (nosql, xpath, expression, prototype, race, mass_assign) |
| Skill files | 28 | In `slapper_skills/` |
| ADRs | 5 | In `docs/adr/` |

## Planning

- `plans/plan.md` — Consolidated improvement plan (all planned work, organized by waves)

For new improvement work, add to the consolidated plan.md rather than creating new plan files.

## Lessons Learned

### Configuration

- `PathsConfig` fields are flattened via `#[serde(flatten)]` for backward compatibility
- Existing config files with top-level `custom_payloads_dir` etc. still work

### Testing

- Negative tests should use specific assertions, not `assert!(result.is_err() || result.is_ok())`
- Check actual error messages: `err.to_string().contains("expected substring")`
- Use `SpoofConfig::from_args()` with `Option<usize>` for decoy_count

### Common Pitfalls

1. **ScopeRule CIDR handling**: `ScopeRule::new()` creates rule with `pattern` but NOT `cidr`. CIDR matching only works via `ScopeRule::with_cidr()`. Using `ScopeRule::new("10.0.0.0/8")` will NOT match IPs correctly — use `with_cidr()` instead.
2. **Type mismatches**: `ScopeRule::new()` takes `String`, not `&str`
3. **Option types**: `decoy_count` is `Option<usize>`, not `usize`
4. **Unused imports**: Move feature-gated imports inside `#[cfg(...)]` blocks
5. **Feature-gated dead code**: Functions used only under `#[cfg(feature = "...")]` appear as dead code to the compiler. Gate the module declaration itself, not just callers.
6. **Clippy redundant closures**: `.map(|arr| func(arr))` should be `.map(func)` when the argument is passed directly
7. **Clippy needless borrows**: `.post(&format!(...))` should be `.post(format!(...))` when the format result implements the required traits
8. **`default_value = "None"` on Options**: Never use `#[arg(default_value = "None")]` on `Option<T>` fields — clap assigns the string `"None"` instead of `None`. Omit `default_value` entirely; `Option` defaults to `None` automatically.
9. **`fingerprint_services` signature**: Takes 5 args: `host`, `ports`, `timeout`, `tui_mode`, `concurrency` — don't forget `concurrency`

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
- `tui/app/mod.rs` contains the `App` struct (883 lines); split into submodules: `dispatch.rs`, `navigation.rs`, `command.rs`, `export.rs`, `state_update.rs`, `task_management.rs`
- `tui/workers/` directory contains 8 files: `runner.rs`, `scanner.rs`, `fuzzer.rs`, `network.rs`, `api.rs`, `recon.rs`, `security.rs`, `pipeline.rs`
- Tab dispatch uses match statements across ~18+ methods (29-arm matches)
- TUI uses ratatui 0.30 + crossterm 0.29 with immediate-mode rendering
- 29 tab variants exist (Recon=0 through Vuln=28); all 29 are fully functional
- `tui/app/mod.rs` contains 883 lines - uses `TabDispatcher` for tab delegation
- `tui/app/dispatch.rs` has `TabDispatcher` wrapper with 17 methods
- `tui/app/task_management.rs` contains `TaskBuilder` trait for task building logic
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

### Plugin Security (block_suspicious_plugins)

Both Python and Ruby plugins support suspicious pattern detection and blocking:

**Python Plugins** (`crates/slapper-plugin/src/python.rs`):
- `validate_python_plugin(content, block_suspicious_plugins)` checks for dangerous patterns
- Patterns detected: `os.system`, `subprocess`, `socket`, `eval(`, `exec`, `fork`, `__import__`, `open(`
- When `block_suspicious_plugins: true` (default), plugins with suspicious patterns are rejected

**Ruby Plugins** (`crates/slapper-ruby/src/bridge.rs`):
- `validate_ruby_plugin(content, block_suspicious_plugins)` checks for dangerous patterns
- Patterns detected: `eval(`, `exec(`, `system(`, `` ` ``, `IO.popen`, `Process.spawn`, `File.read(`, `File.write(`, `File.open(`, `Net::HTTP`, `Socket.open`, `TCPSocket`, `UDPSocket`, `Open3.`, `Shellwords.escape`
- Default behavior blocks suspicious plugins for security

**Configuration** (`PluginConfig`):
```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
}
```

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

### Skill System

The skill system defines agent capabilities via YAML+Markdown files, enabling AI assistants to understand how to use Slapper for security workflows.

**Skill Files Location:** `slapper_skills/` (root directory, NOT for working on the Slapper codebase)

**Skill File Format:**
```yaml
---
name: skill_name
description: "Brief description"
triggers:
  - trigger1
  - trigger2
metadata:
  category: category
  tools: [tool1, tool2]
  scope: targets
---

## Overview
<detailed description>

## Usage
<code examples>

## Triggers
Keywords that activate this skill
```

**Key Types:**
- `Skill` — Parsed skill with name, triggers, metadata, content
- `SkillLoader` — Loads skills from directories
- `SkillRegistry` — Indexes skills by trigger and tool

**Usage:**
```rust
let loader = SkillLoader::new(vec![PathBuf::from("slapper_skills")]);
let skills = loader.load_skills()?;
let registry = SkillRegistry::new();
registry.register(skill)?;

let matching = registry.find_by_trigger("sql injection");
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
- `tool/agents/communication.rs` — Multi-agent communication with `HealthStatus` enum

Feature gate: `#[cfg(feature = "rest-api")]` in `tool/mod.rs`.

**Note:** `HealthStatus` enum must derive `Copy, PartialEq, Eq` for use in test assertions.

#### MCP Prompts & Sampling

- `tool/protocol/mcp/prompts.rs` — 7 builtin prompt templates with `get_builtin_prompts()`
- `tool/protocol/mcp/sampling.rs` — Request/response types for AI completions

Feature gates: prompts always available, sampling gated on `ai-integration`.

#### Autonomous Agent (`agent/`)

The autonomous security agent provides continuous monitoring, scheduled scans, and AI-guided security testing:

**Module Structure:**
```
crates/slapper/src/agent/
├── mod.rs          # Agent core with event loop, CronScheduler
├── portfolio.rs    # TargetPortfolio for multi-target management
├── memory.rs       # LongitudinalMemory for file-based persistence
├── events.rs       # Event system with EventHandler trait
```

**Key Types:**
- `Agent` — Main orchestrator with `run()`, `stop()`, `execute_scan()`, `trigger_scan()`
- `AgentConfig` — Configuration with `portfolio_path`, `memory_dir`, `poll_interval_secs`
- `TargetPortfolio` — CRUD for monitored targets with scheduling support
- `TargetConfig` — Per-target settings (schedule, priority, alert_channels, baseline, scan_depth, off_peak_window)
- `LongitudinalMemory` — File-based storage in `~/.config/slapper/memory/`
- `AlertRouter` — Routes alerts via webhook with HMAC signing
- `EventHandler` — Trait for custom event handlers with `handles()` and `handle()`

**Trait Signature for Custom Handlers:**
```rust
impl EventHandler for MyHandler {
    fn handles(&self, event: &SecurityEvent) -> bool { true }
    fn handle<'a>(
        &'a self,
        event: &'a SecurityEvent,
        agent: &'a mut Agent,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a> {
        Box::pin(async move { Ok(()) })
    }
}
```

**CLI Commands:**
```bash
slapper agent run              # Run autonomous agent
slapper agent run --once       # Run once and exit
slapper agent targets list     # List monitored targets
slapper agent targets add <id> --target https://example.com --schedule "0 0 * * *"
slapper agent targets remove <id>
slapper agent skills list      # List available skills
slapper agent skills load /path/to/skills/
slapper agent status           # Show agent status
```

Feature gate: `#[cfg(feature = "rest-api")]` for core agent, `#[cfg(feature = "ai-integration")]` for skills.

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

#### OAST Integration

- `tool/implementations/oast.rs` — `OastTool` for Out-of-Band Application Security Testing
- Integrates with Interactsh API for blind vulnerability detection
- Feature gate: `#[cfg(feature = "rest-api")]`

#### Runtime Scripting Engine

- `tool/scripting.rs` — `ScriptEngine` trait for dynamic script execution
- Uses existing `pyo3` and `magnus` for plugin languages
- Implements sandbox restrictions for untrusted scripts
- Feature gate: `#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]`

#### Template Signing

- `scanner/templates/verify.rs` — Ed25519 signature verification for community templates
- `Template::verify(public_key)` checks signature before execution
- Prevents malicious template execution from untrusted sources

#### Session Management

- `tool/session.rs` — Extended `AgentSession` with auth methods, CSRF tokens, login sequences
- `AuthMethod` enum: `Basic`, `Bearer`, `OAuth2`, `APIKey`
- `AuthMethod::apply_to_request()` for flexible auth handling
- Feature gate: `#[cfg(feature = "rest-api")]`

#### Report Templating

- `output/template.rs` — `ReportTemplate` using `handlebars` crate
- Supports compliance templates (PCI-DSS, SOC2, HIPAA)
- CLI: `report render --template <path>`

#### Multi-Agent Communication

- `tool/agents/communication.rs` — `InterAgentChannel` for agent-to-agent messaging
- `AgentInfo` with health metrics and capability advertising
- Capability-based agent lookup and delegation

#### Network Utilities

- `utils/network.rs` — Helper functions for TCP connections with `TCP_NODELAY`
- `connect_with_nodelay()` and `connect_with_nodelay_timeout()` for efficient networking

## Security Patterns

### Authentication Middleware Pattern

When adding auth to new endpoints:
1. Add `Option<String>` to state
2. Create local `require_auth` function using constant-time comparison (`subtle::ConstantTimeEq`)
3. Apply to all handlers

### Formula Injection Prevention

Check for unsafe prefixes at START of string (`starts_with`) not just anywhere in string (`contains`):
```rust
// SAFE: Check first character
if content.starts_with('=') || content.starts_with('+') || content.starts_with('-') || content.starts_with('@') {
    // Handle formula injection
}
```

### Log Sanitization

When changing sanitization behavior, update corresponding tests that assert old behavior.

### TLS Certificate Verification Bypass

When creating HTTP clients that bypass TLS verification (`danger_accept_invalid_certs(true)`), use the centralized helpers that log warnings:

```rust
// In utils/http.rs - creates client and logs warning
pub fn create_insecure_http_client(timeout_secs: u64) -> Result<Client> {
    tracing::warn!(
        "Creating HTTP client with disabled TLS certificate verification. \
         This is insecure and should only be used in isolated testing environments."
    );
    // ... creates client with danger_accept_invalid_certs(true)
}
```

For custom options, use `create_insecure_client_with_options()`.

### MCP Auth Bypass

The `initialize` method bypass may be protocol-required, but auth MUST be enforced when api_key is configured (`Some`).

### NSE Sandbox

Default to `enabled: true` - security by default over convenience.

**Important**: The `socket` library is **NOT sandboxed** even when `nse-sandbox` is enabled. Scripts can still make arbitrary network connections. The `lfs` library IS sandboxed with path restrictions. See `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md` for details.

### Path Validation Pattern

Use `canonicalize()` to resolve symlinks, then check if result starts with allowed prefix. **Fail-secure**: If canonicalization fails (including symlink cycles), block the path rather than falling back to the unresolved path.

### ReDoS Prevention

Always use `RegexBuilder` with explicit `size_limit()` when building regexes from untrusted input.

### Race Condition with Atomics

When using both `Mutex` and atomic operations, ensure atomic operations happen inside the mutex lock to prevent inconsistent state reads.

## Performance Patterns

### DashMap for Concurrent Aggregation

Replace `Arc<Mutex<Vec>>` with `Arc<DashMap<K, V>>` for lock-free concurrent appends.

### FxHashMap for Hot Paths

Use `rustc_hash::FxHashMap` instead of `std::collections::HashMap` for 2-3x faster lookups in high-traffic areas.

### LazyLock for Static Regex

Pre-compile regex patterns at module level using `std::sync::LazyLock` to avoid repeated compilation.

### Single-Buffer Escape Functions

Use `write!` with pre-allocated `String` instead of chained `.replace()` calls to avoid intermediate allocations.

### HTTP Connection Pooling

Add `.pool_max_idle_per_host(20).pool_idle_timeout(Duration::from_secs(30)).tcp_nodelay(true)` to client builders.

### SmallVec for Stack-Allocated Buffers

Use `SmallVec<[u8; 256]>` instead of `Vec<u8>` for small fixed-size buffers to avoid heap allocation.

### contains_ignore_case Helper

For repeated case-insensitive substring checks, call `to_lowercase()` once before the loop instead of once per pattern.

### Watch Channel for Progress Updates

Use `tokio::sync::watch` channel instead of mutex-polling for progress updates:
```rust
let (tx, rx) = watch::channel<String>("initial".to_string());
// In worker:
tx.send("Processing step 1".to_string())?;
// In UI:
while rx.changed().await.is_ok() {
    println!("Progress: {}", *rx.borrow());
}
```

### TUI Render Caching (Dirty Flag)

Avoid unnecessary redraws by tracking whether the UI actually needs to be updated:
```rust
struct AppState {
    needs_redraw: bool,
}
loop {
    if app.needs_redraw {
        terminal.draw(|f| ui::draw(f, app))?;
        app.needs_redraw = false;
    }
    // Handle events
    app.needs_redraw = true;
}
```

### AtomicU64 for Lock-Free Counters

For simple counter operations, use `AtomicU64` instead of `Mutex<u64>`:
```rust
use std::sync::atomic::{AtomicU64, Ordering};
let counter = Arc::new(AtomicU64::new(0));
counter.fetch_add(1, Ordering::Relaxed);
```

### Agent Thread Safety

Agent modules use `Arc<Mutex<>>` or `Arc<RwLock<>>` for interior mutability:

- `AlertRouter` uses `Arc<Mutex<Vec<AlertChannel>>>` and `Arc<Mutex<HashMap<...>>>` for thread-safe alert routing
- `TargetPortfolio` uses `Arc<RwLock<PortfolioData>>` for thread-safe portfolio access
- `LongitudinalMemory` methods take `&self` (no internal mutation) for thread-safe memory access

Example pattern:
```rust
pub struct AlertRouter {
    channels: Arc<Mutex<Vec<AlertChannel>>>,
    recent_alerts: Arc<Mutex<HashMap<String, Instant>>>,
    dedup_window_secs: u64,
}

impl AlertRouter {
    pub fn add_channel(&self, channel: AlertChannel) -> Result<()> {
        self.channels.lock().map_err(|e| ...).push(channel);
        Ok(())
    }
}
```

### parking_lot vs std::sync Mutex

`parking_lot::Mutex::lock()` returns `MutexGuard` directly, NOT `Result<MutexGuard, PoisonError>` like `std::sync::Mutex`:
```rust
// parking_lot (correct):
let guard = mutex.lock();
guard.push(value);

// std::sync (returns Result):
let guard = mutex.lock().unwrap();
```

When converting from `std::sync::Mutex` to `parking_lot::Mutex`, remove `Ok()` pattern matching on lock results.

## Code Quality Patterns

### serde_yaml_neo Replacement

When updating from `serde_yaml` (deprecated), use `serde_yaml_neo` as drop-in replacement:
```toml
# Cargo.toml
serde_yaml_neo = "0.11"

# imports
use serde_yaml_neo::Value;  // instead of serde_yaml::Value
```

### pyo3 0.28 Migration

When upgrading pyo3 0.26+:
- `Python::with_gil` → `Python::attach`
- For `Vec<&str>` patterns, use `suspicious_found.push(*pattern)` not `push(pattern)`

### Plugin System Patterns

- `timeout_secs` in PluginConfig defaults to 300 seconds
- `max_file_size_bytes` for plugin validation (default 1MB)
- Use `LazyLock<Regex>` for compiled regex pattern detection
- `PluginRegistry::unregister()` removes plugin by name

### Test Feature Gating

Always gate integration tests with `#[cfg(feature = "...")]` when they depend on optional features.

### Doc Test Compilation

Doc examples must use correct types and function signatures - always verify against actual code.

### URL Encoding

Use `urlencoding::encode()` for any user-provided query string components in URLs.

### Error Conversion

When adding `From` impls for feature-gated error types, gate the entire impl block with the appropriate `#[cfg(feature = "...")]`.

### Dead Code Security

Code after an early return that can never execute is a security risk - remove it.

## Verification Best Practices

- Always verify plan items against actual codebase before assuming they still apply
- Use `rg` to confirm file paths, line numbers, and patterns exist
- Run `cargo test --lib -p slapper` after each change to catch regressions
- Use `cargo clippy --lib -p slapper` to verify no new warnings
- Check test counts: `cargo test --lib -p slapper -- --list 2>/dev/null | wc -l`
- Count source files: `find crates/slapper/src -name '*.rs' | wc -l`
- Run specific failing test: `cargo test --test negative_tests -- test_scope_cidr_edge_cases`
- Verify CLI build: `cargo build --release -p slapper && ./target/release/slapper --help`

## Architecture Decision Records

Located in `docs/adr/`:
- ADR-001: SensitiveString vs SecretString
- ADR-002: Feature flag design rationale
- ADR-003: rustls over native-tls (except nse)
- ADR-004: Error type separation

When making significant architectural decisions, document them here using the ADR template.
