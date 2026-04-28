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

# Test specific features
cargo check --lib -p slapper --features rest-api,ai-integration
cargo check --lib -p slapper --features python-plugins,ruby-plugins
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
│   ├── fuzzer/        # Fuzzing engine (30 payload types)
│   │   ├── chain.rs   # ChainExecutor (with regex caching)
│   │   ├── detection/ # TimingAnalyzer (lock-free with atomics)
│   │   └── payloads/
│   │       └── macros.rs  # payload_vec! macro
│   ├── scanner/       # Port scanning, endpoint discovery
│   │   ├── templates/ # Nuclei-style template engine
│   │   └── ports/     # Port scanning (mod.rs + spoofed.rs)
│   ├── waf/           # WAF detection and bypass
│   ├── recon/         # Reconnaissance modules
│   │   ├── auth/      # Multi-protocol auth testing (ssh_auth, ftp_auth, smtp_auth)
│   │   └── dependency_scan/  # Split by ecosystem (npm, cargo, go)
│   ├── output/        # Report generation (JSON, HTML, SARIF, JUnit)
│   ├── wireless/      # Wireless security testing (WiFi scanning, auth testing)
│   ├── tool/          # Tool abstraction layer
│   │   ├── implementations/  # Tool implementations (recon, scanner, fuzzer, waf, search, etc.)
│   │   └── protocol/
│   │       ├── mcp/   # MCP server (handlers/server.rs, handlers/helpers.rs)
│   │       ├── openai/  # OpenAI-compatible chat completions
│   │       ├── rest.rs  # REST API (scope validation implemented)
│   │       └── grpc.rs  # gRPC service
│   ├── proxy/         # Proxy modules (to_log_key() for safe logging)
│   │   └── intercept/ # Intercepting proxy with dynamic SSL certs
│   ├── stress/        # Stress testing (raw_udp module integrated)
│   ├── tui/           # Terminal UI (ratatui 0.30 + crossterm 0.29)
│   │   ├── app/       # App struct split into submodules (dispatch, navigation, command, export, state_update, task_management)
│   │   ├── tabs/      # 29 tab implementations (settings/ split into main.rs, render.rs, input.rs)
│   │   └── workers/   # Background task workers
│   └── utils/         # Utilities (circuit_breaker, http, formatting, network)
├── tests/             # Integration tests
└── Cargo.toml
```

**Note:** The `slapper_skills/` directory in the project root contains skill files for use with the autonomous agent. This is distinct from the codebase itself which agents work on.

### Key Types

- `SlapperConfig` - Main configuration (use `config::load_config()`)
- `PathsConfig` - Directory paths (flattened into SlapperConfig)
- `SpoofConfig` - IP spoofing settings
- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `PayloadType` - Enum of 30 payload categories
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
- `ws-api` - WebSocket support for pub/sub
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
| Tests | 1107 passing | Library tests |
| Tests | 1345 passing | With rest-api,ai-integration (pre-existing AI test failures fixed) |
| Clippy | ~28 warnings | TUI-specific acceptable, some dead code warnings remain |
| Source files | 470+ |
| Payload types | 30 |
| Tabs | 29 |

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
- `len()` returns the length of the inner string
- `as_bytes()` returns raw bytes (for proxy auth encoding)
- `is_empty()` checks if empty

**Note:** Proxy credentials (`SocksProxy`, `HttpConnectProxy`) now use `SensitiveString` for secure storage.

### Circuit Breaker

`utils/circuit_breaker.rs` provides circuit breaker pattern for external API resilience:
- `CircuitBreaker` - individual breaker with state (Closed/Open/HalfOpen)
- `CircuitBreakerRegistry` - manages multiple breakers by name (each AI client creates its own breaker directly)
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

**Dependency Versions (as of 2026-04-24):**
- axum: 0.8.x
- tonic: 0.14.x
- prost: 0.14.x

**Exception:** `slapper-nse` retains `native-tls` (OpenSSL) for Nmap NSE script compatibility. This is intentional — Nmap scripts expect OpenSSL-based TLS behavior. Do not migrate `slapper-nse` to `rustls`.

### Spoofed Scanner

`scanner/ports/spoofed.rs` contains raw socket scanning (feature-gated). `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled. Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing.

**Note:** The `raw_udp` module in `stress/udp.rs:20-117` is integrated — `run_udp_flood()` calls `run_udp_flood_spoofed()` which uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix.

## Planning

- `plans/plan.md` — Consolidated improvement plan (sole plan file, all others removed)
  - 121 verified items across 7 waves (A through G)
  - Wave A: Critical bugs (15 items) — build errors, security vulns, broken features
  - Wave B: Code quality (15 items) — dead code, duplication, CLI fixes
  - Wave C: Security improvements (11 items) — credential exposure, sandbox gaps, design notes
  - Wave D: Performance (16 items) — regex caching, HashMap migrations, string optimization
  - Wave E: Feature enhancements (32 items) — plugin system, TUI, agent, fuzzing capabilities
  - Wave F: New capabilities (9 items) — gRPC testing, AD security, OS detect, distributed, NSE, REST API, MCP, WebSocket, flow persistence
  - Wave G: Documentation (18 items) — accuracy fixes, VULNERABILITY_GUIDE expansion, new guides
  - Waves execute in order A→G. Within each wave, sub-agents work in parallel on non-overlapping file groups
  - See plan.md for implementation details, parallelization tables, and verification commands

**On Using This Guide**: When working on items from plan.md, always verify claims against the actual codebase. Line numbers and file paths in plans may become outdated as code evolves. Use `rg` to confirm before implementing.

## Lessons Learned

### Codebase Verification Required

When implementing plan items, verify actual state rather than assuming plan accuracy:
- Payload type count: 30 (verified via `fuzzer/payloads/mod.rs`)
- Recon module count: ~30 (more than previously documented)
- Test count: 1107 passing base, 1345 with full features
- Use `rg` to confirm file paths and line numbers exist
- Run `cargo test --lib -p slapper` after each change
- Check test counts: `cargo test --lib -p slapper -- --list 2>/dev/null | grep -c "test$"`

### Wave-Based Parallelization

The consolidated plan.md is structured for parallel implementation by wave. See plan.md's "Parallelization Strategy" section for the full agent assignment tables. Key principle: items in different files can always be parallelized; items touching the same file need sequential execution within a sub-agent.

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
10. **Test assertion logic**: Always verify tests actually exercise the expected code path. Use specific assertions and test documentation to verify behavior.
11. **UTF-8 byte slicing**: `InputField` stores cursor as character count (not byte offset). Always use character-based indexing for UTF-8 strings to avoid panics.
12. **File path verification**: Always verify file paths with `rg` or `ls` before referencing. Several plan items referenced non-existent files (e.g., `slapper-plugin/src/ruby.rs` doesn't exist — Ruby is in `slapper-ruby/src/loader.rs`).
13. **parking_lot lock scoping**: When checking if operations are inside a lock, note that `parking_lot::MutexGuard` lives until end of scope (function return), unlike `std::sync::MutexGuard` which can be dropped early. Always trace the actual scope.
14. **Feature-gated compilation**: Always verify build errors with the actual feature flag enabled. Some modules (e.g., `packet/`, `stress/`) only fail to compile when their respective features are active.

### Severity Enum

- `Severity` has custom `Ord`/`PartialOrd` implementations using `as_int()` for correct semantic ordering (Critical > High > Medium > Low > Info)
- Use `as_int()` for numeric severity comparisons
- `Display` outputs UPPERCASE ("CRITICAL"), `as_str()` outputs lowercase ("critical")
- `serde` serialization uses lowercase (due to `#[serde(rename_all = "lowercase")]`)
- `Severity` implements `FromStr` trait; inherent method renamed to `parse_or_default`
- Use `eq_ignore_ascii_case()` instead of `to_lowercase().as_str()` for comparisons

### SensitiveString

- Field is private; use `expose_secret()` (borrow) or `into_secret()` (consume)
- `into_secret()` uses `std::mem::take` internally to work with `ZeroizeOnDrop`
- `PartialEq` uses constant-time comparison; safe for credential checking
- Config deserialization works transparently — existing TOML files with plain strings still load
- Use `to_log_key()` for logging/display, NOT `to_url()` which exposes credentials

### TUI-Specific Patterns

- `tui/app/runner.rs` contains the main event loop (`run_app`)
- `tui/app/mod.rs` contains the `App` struct; split into submodules: `dispatch.rs`, `navigation.rs`, `command.rs`, `export.rs`, `state_update.rs`, `task_management.rs`
- `tui/workers/` directory contains 8 files: `runner.rs`, `scanner.rs`, `fuzzer.rs`, `network.rs`, `api.rs`, `recon.rs`, `security.rs`, `pipeline.rs`
- Tab dispatch uses match statements across ~18+ methods (29-arm matches)
- TUI uses ratatui 0.30 + crossterm 0.29 with immediate-mode rendering
- 29 tab variants exist (Recon=0 through Vuln=28); all 29 are fully functional
- `tui/app/dispatch.rs` has `TabDispatcher` wrapper with 27 methods for input delegation
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

Both Python and Ruby plugins support suspicious pattern detection and blocking via consolidated `slapper-plugin/src/security.rs`:

**Python Plugins**:
- `validate_python_plugin(content, block_suspicious_plugins)` checks for dangerous patterns
- Patterns detected: `os.system`, `subprocess`, `socket`, `eval(`, `exec`, `fork`, `__import__`, `open(`, `pty.spawn`, `os.popen`, `multiprocessing.Process`, `ctypes`, `importlib`, `getattr(`, `chr(`, hex/unicode/octal escapes
- When `block_suspicious_plugins: true` (default), plugins with suspicious patterns are rejected

**Ruby Plugins**:
- `validate_ruby_plugin(content, block_suspicious_plugins)` checks for dangerous patterns
- Patterns detected: `eval(`, `exec(`, `system(`, `` ` ``, `IO.popen`, `Process.spawn`, `File.read(`, `File.write(`, `File.open(`, `Net::HTTP`, `Socket.open`, `TCPSocket`, `UDPSocket`, `Open3.`, `Shellwords.escape`, `Kernel.exec`, `\bopen\b`, `(?i)\beval\b`
- Default behavior blocks suspicious plugins for security

**Configuration** (`PluginConfig`):
```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
    pub timeout_secs: u64,               // default: 300
}
```

### Plugin Path Validation

Use `validate_plugin_path()` from `slapper-plugin/src/validation.rs` for safe path handling in plugin loading. This prevents path traversal attacks by canonicalizing paths and checking they start with the base directory.

### Plugin Lifecycle Methods

The `Plugin` trait (`slapper-plugin/src/lib.rs:144-154`) includes lifecycle methods:
```rust
fn init(&self) -> Result<()>;
fn shutdown(&self) -> Result<()>;
fn health_check(&self) -> Result<HealthStatus>;
fn priority(&self) -> u32;
```

### Ruby Plugin Thread Safety

`RubyBridge` is NOT `Send + Sync` (magnus `Ruby` type has `PhantomData<*mut ()>`). Thread safety is achieved via message-passing:

- `RubyPluginClient` spawns a dedicated `ruby-vm` thread that owns the `RubyBridge`
- Communication via `std::sync::mpsc` channels — each request gets its own response channel
- `RubyPluginAdapter` holds `Arc<RubyPluginClient>` — naturally `Send + Sync`, no unsafe code
- The `unsafe impl Send + Sync` on `RubyBridge` has been REMOVED — the bridge is now private
- Ruby API now only exposes safe reporting methods (HTTP, Scanner, Fuzzer, Metasploit removed)

### Magnus 0.8 API (slapper-ruby/src/loader.rs)

Note: Ruby plugin code is in `crates/slapper-ruby/src/loader.rs`, NOT `slapper-plugin/src/ruby.rs`.

- `eval::<()>()` is not valid — use `let _: Value = eval(...)` to discard result
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
├── alerts/         # Alert routing (AlertRouter, AlertChannel, AlertRoutingRules)
└── skills.rs       # SkillRegistry for agent capabilities
```

**Key Types:**
- `Agent` — Main orchestrator with `run()`, `stop()`, `execute_scan()`, `trigger_scan()`
- `AgentConfig` — Configuration with `portfolio_path`, `memory_dir`, `poll_interval_secs`
- `TargetPortfolio` — CRUD for monitored targets with scheduling support
- `TargetConfig` — Per-target settings (schedule, priority, alert_channels, baseline, scan_depth, off_peak_window)
- `LongitudinalMemory` — File-based storage in `~/.config/slapper/memory/`
- `AlertRouter` — Routes alerts via webhook with HMAC signing (uses `to_log_key()` for safe credential handling)
- `EventHandler` — Trait for custom event handlers with `handles()` and `handle()`

**Trait Signature for Custom Handlers:**
```rust
impl EventHandler for MyHandler {
    fn handles(&self, event: &SecurityEvent) -> bool { true }
    fn handle<'a>(
        &'a self,
        event: &'a SecurityEvent,
        agent: &'a mut Agent,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
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

**Constant-time comparison**: Use `bool::from(key.as_bytes().ct_eq(v.as_bytes()))` instead of `.unwrap_u8() == 1`. The `unwrap_u8()` pattern degrades `Choice` to `u8` which enables side-channel attacks through branch prediction.

### Formula Injection Prevention

Check for unsafe prefixes at START of string (`starts_with`) not just anywhere in string (`contains`):
```rust
// SAFE: Check first character
if content.starts_with('=') || content.starts_with('+') || content.starts_with('-') || content.starts_with('@') {
    // Handle formula injection
}
```

Also use NFKC normalization to prevent fullwidth character bypass:
```rust
use unicode_normalization::UnicodeNormalization;
let normalized: String = s.nfkc().collect();
```

### Log Sanitization

`utils/error.rs` sanitizes error messages by removing:
- Stack traces (Rust panics, Python tracebacks, Go panics)
- File paths and Windows paths
- Long error messages (>500 chars truncated)

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

**Socket library network restrictions**: The `socket` library has conditional restrictions via `allowed_networks` configuration in `SandboxConfig`. When `allowed_networks` is configured, connections are validated against the CIDR blocklist. The `lfs` library IS sandboxed with path restrictions. See `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md` for details.

### Path Validation Pattern

Use `canonicalize()` to resolve symlinks, then check if result starts with allowed prefix. **Fail-secure**: If canonicalization fails (including symlink cycles), block the path rather than falling back to the unresolved path.

### ReDoS Prevention

Always use `RegexBuilder` with explicit `size_limit()` when building regexes from untrusted input.

### Race Condition with Atomics

When using both `Mutex` and atomic operations, ensure atomic operations happen inside the mutex lock to prevent inconsistent state reads.

### IMAP Injection Prevention (slapper-nse)

The IMAP library in `slapper-nse` requires careful string escaping. Use `escape_imap_quoted()` function per RFC 3501 to prevent command injection:
```rust
fn escape_imap_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\r' | '\n' => {}  // Strip these
            c => result.push(c),
        }
    }
    result
}
```

## Performance Patterns

### DashMap for Concurrent Aggregation

Replace `Arc<Mutex<Vec>>` with `Arc<DashMap<K, V>>` for lock-free concurrent appends.

### FxHashMap for Hot Paths

Use `rustc_hash::FxHashMap` instead of `std::collections::HashMap` for 2-3x faster lookups in high-traffic areas. The `FxHasher` deliberately trades DoS-resistance for speed — acceptable for local tools with no untrusted input.

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
- `TargetPortfolio` uses `parking_lot::RwLock` (not `std::sync::RwLock`) for thread-safe portfolio access
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
- `reload_plugin()` method available for hot reload of plugins
- Python plugin classes prefixed with `Slapper_` for namespace isolation

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

## Implementation Notes

### Test Behavior Notes

When fixing failing tests in integration scenarios:
- **Circuit breaker tests**: The breaker requires BOTH failure_threshold (5) AND success_threshold (3) transitions to close. After 5 failures, one success only moves to HalfOpen, not Closed. Tests should reflect actual state machine behavior.
- **WAF bypass knowledge_base**: Pre-populated from `~/.config/slapper/waf_bypasses.json` - tests may have non-empty state. Use unique identifiers for test payloads to avoid collisions.
- **Skills extract_triggers**: Pattern matching is case-insensitive. Line must contain "trigger", "keyword", or "example" AND ":" for YAML frontmatter format, or start with these words for other formats.

### UTF-8 Byte Slicing

`InputField` stores cursor as byte offset. All methods consistently use `value.len()` for cursor position:
- `with_value()`, `apply_autocomplete()`, `move_end()` use `value.len()` (byte length)
- `insert()` uses `str.insert(self.cursor_pos, c)` for byte-based insertion

### Large File Reference

| File | Lines | Status |
|------|-------|--------|
| `tui/app/mod.rs` | ~900 | Split into submodules |
| `tool/protocol/mcp/handlers.rs` | ~1000 | Split into handlers/ subdirectory |
| `recon/dependency_scan.rs` | ~1000 | Split into dependency_scan/ subdirectory |
| `tui/tabs/settings.rs` | ~800 | Split into settings/ subdirectory |
| `tui/tabs/packet.rs` | ~743 | Not split |

### Completed Implementation Items

The following items have been implemented and verified:

1. **Plugin timeout enforcement**: Python uses `tokio::time::timeout`, Ruby uses `rx.recv_timeout()`
2. **Path traversal prevention**: `validate_plugin_path()` in `slapper-plugin/src/validation.rs`
3. **TUI Plugin Tab**: `TaskResult::PluginsLoaded` variant and proper integration
4. **Constant-time comparison**: All auth uses `bool::from(ct_eq())` instead of `unwrap_u8()`
5. **IMAP injection prevention**: `escape_imap_quoted()` in slapper-nse
6. **Private IP blocking**: `resolve_host()` blocks loopback and private IPs
7. **Ruby sandbox**: Only safe reporting methods exposed
8. **Shared security patterns**: `slapper-plugin/src/security.rs` consolidates pattern detection
9. **AI client integration**: `analyze_findings_typed()` used in Agent
10. **HMAC webhook signing**: `X-Signature-256` header with HMAC-SHA256
11. **Error sanitization**: Rust panics, Python tracebacks, Go panics, Windows paths caught
12. **UTF-8 handling**: Character-based indexing in InputField
13. **Theme system**: `ThemeManager` integrated with `tc!()` macro
14. **Session persistence**: `SessionManager` integrated with `auto_save_if_due()`
15. **WebSocket support**: `ws-api` feature with `/ws` endpoint
16. **Clipboard paste**: Ctrl+V integration with `arboard`
17. **Alert CLI flags**: `--alert-webhook`, `--alert-slack`, `--alert-email`
18. **Dedup key includes finding_ids**: `make_dedup_key()` in AlertRouter
19. **TUI tabs/mod.rs dual arms**: Feature-gated tabs already have proper `#[cfg(feature)]` and `#[cfg(not(feature))]` arms
20. **HistoryTab search**: `search()` method already exists and is functional
21. **PluginRegistry thread safety**: Uses `parking_lot::RwLock` for thread-safe concurrent access
22. **Community templates**: `scanner/templates/` exists with full implementation
23. **Subdomain sources**: Has multiple sources (crt.sh, threatminer, dns-bruteforce)
24. **gRPC compilation fixed**: Prostad types conversion and ResponseStatus fixed
25. **TUI color theme system**: Uses `tc!()` macro for light/dark theme
26. **Cron expression matching**: `field_matches()` handles wildcards correctly
27. **Agent registry field used**: Passed to Agent constructor
28. **WAF fingerprinting active**: Integrated in production
29. **TUI unused import fixed**: `Modifier` now used
30. **Pipeline depth params**: Now reads concurrency, timeout_ms, max_rate, payload_types
31. **InterAgentChannel subscriber notification**: Notifies via webhook callbacks
32. **AlertRouter routing_rules**: Integrates AlertRoutingRules with set_routing_rules() method
33. **PSK output to eprintln**: PSK now sent to stderr to avoid shell history capture
21. **PluginRegistry thread safety**: Uses `parking_lot::RwLock` for thread-safe concurrent access (`Arc<RwLock<Vec<Arc<dyn Plugin>>>>`)
22. **Community templates**: `scanner/templates/` already exists with full implementation
23. **Subdomain sources**: Has 3 sources (crt.sh, alexa, threatminer) but alexa is a stub (returns empty). dns-bruteforce also exists as 4th source.
24. **gRPC server**: Proto file, `GrpcServerArgs` CLI, `grpc_server.rs` handler, conditional build, proper feature gating. FIXED (2026-04-26): compilation errors resolved.
25. **Feature flag documentation**: Comprehensive grouping documented in ARCHITECTURE.md (Core, Protocol, Plugin, Network, AI/Agent, Specialized, Infrastructure, Aggregation)
26. **TUI color theme system**: ~300 hardcoded `Color::*` replaced with `tc!()` macro for light/dark theme support
27. **Cron expression matching**: `output/schedule.rs:field_matches()` correctly handles wildcards (pattern 0 acts as wildcard, matches any value)
28. **Agent registry field used**: `registry` field at `agent/mod.rs:76` IS used after initialization (passed to Agent constructor)
29. **WAF fingerprinting active**: `fuzzer/waf_fingerprint.rs` has 16 integration references and IS used in production
30. **TUI unused import fixed**: `tabs/scan_endpoints.rs` `Modifier` is now used at line 161

### Wave A/B/C/D/E/F/G Implementation Items (2026-04-27)

**Wave A - Critical Bugs:**
- A-1: gRPC API key timing attack fixed with constant-time comparison
- A-2: WebSocket authentication enforced (was missing auth check)
- A-3: Private IP blocking in stress modules (udp, icmp, syn)
- A-4: packet/parse.rs module added (was missing)
- A-5: container feature k8s-openapi version fix
- A-6: stress module ProxyCommand import fix
- A-7: Portfolio persistence - TargetPortfolio::load_from_file used
- A-8: Pipeline tool reads depth params from request
- A-9: Event system wired into agent scan completion
- A-10: InterAgentChannel notifies subscribers
- A-11: AlertRoutingRules integrated into AlertRouter  
- A-12: cleanup_stale_entries async/sync fix
- A-13: O(n^2) dedup replaced with HashSet
- A-14: Mutex guard scope fixed (not held across await)
- A-15: Unused SHA256 body hashing removed

**Wave B - Code Quality:**
- B-1: TUI unused imports removed (5 files)
- B-2: full feature includes grpc-api
- B-3: Agent routes feature gate fixed (ai-integration → rest-api)
- B-5: WAF request_error field added
- B-6: ChainExecutor error_kind field added
- B-7: MCP health endpoint auth check added
- B-8: TimingAnalyzer Clone semantics documented
- B-10: TaskScheduler respects scheduled_for timestamp
- B-11: list_tasks() returns actual tasks
- B-13: Agent CLI fields for target configuration added

**Wave C - Security:**
- C-1/C-4: Startup warning when API key not configured
- C-2: RubySandboxConfig with allowed_dir/networks
- C-3: Proxy to_dedup_key() method added
- C-5: TUI notify secret → SensitiveString
- C-6: PSK output to eprintln! (was println!)
- C-7: Python chr() pattern removed (caused false positives)
- C-8: cleanup_expired_messages synchronous
- C-10: AI cache serialization fixed (was losing entries)
- C-11: Skills feature gate → rest-api

**Wave D - Performance:**
- D-1: LazyLock<Regex> in tool/session.rs
- D-3: HTTP connection pooling added
- D-4: HashMap → FxHashMap in ai/cache, ai/planner, utils/cache, tool/state, distributed/remote
- D-5: contains_ignore_case optimization (single to_lowercase allocation)
- D-6: TUI Selector render_with_focus method (eliminates clones)
- D-10: AtomicU64 replacements in scanner modules

**Wave E - Features:**
- E-5: Plugin reload validation with security checks
- E-8: JWT brute_force_weak_key now performs actual HMAC-SHA256 verification
- E-13: WAF detection enhanced (34 products, up from 26)
- E-15: TUI spinner animation added
- E-16: TUI error handling improved (no more 10-char truncation)
- E-17: TUI confirmation dialogs with Destructive variant
- E-20: TUI spinner tick in event loop
- E-24: LongitudinalMemory max_scans_per_target config
- E-25: Agent status shows actual data
- E-27: Cancellation token for scan execution
- E-28: TargetsCommand Update variant

**Wave B - Code Quality (Additional):**
- B-14: AlertRouter dedup_window_secs now configurable via with_dedup_window()
- B-15: TimeRange unified with OffPeakWindow (midnight wrap-around support)

**Wave F - New Capabilities:**
- F-6: REST API CORS, validation, pagination, metrics
- F-7: MCP roots/list, shutdown notifications
- F-8: WebSocket subprotocol testing

**Wave G - Documentation:**
- G-1: mcp-server → rest-api in docs
- G-2: Payload count 22→30 in docs
- G-3: Recon modules 18→30+ in docs

**gRPC Compilation Fixes:**
- prost_types::Value to serde_json::Value conversion added
- ResponseStatus struct usage fixed
- Option<String> fields properly unwrapped

### Session Learnings (2026-04-27)

**Key bugs fixed in Wave A-remaining (2026-04-27):**
- **PipelineTool depth params**: `execute()` now reads `concurrency`, `timeout_ms`, `max_rate`, `payload_types` from request params (A-8)
- **InterAgentChannel subscribers**: `send_message()` now iterates subscriptions and calls webhook callbacks (A-10)
- **AlertRouter routing_rules**: Added `routing_rules: Option<AlertRoutingRules>` field, `set_routing_rules()` method, and channel filtering in `send()` (A-11)
- **cleanup_stale_entries**: Verified as synchronous function - no await needed (A-12)

### Session Learnings (2026-04-26)

During plan consolidation and verification:
- Verified 121 items against actual codebase using sub-agents
- Discovered that ~25% of planned items from original plans were already fixed, incorrectly documented, or false positives
- All corrections incorporated into consolidated plan.md

**Key bugs confirmed in codebase:**
- `cleanup_stale_entries()` bug: async fn called without `.await` at `agent/alerts/routing.rs:50` — best fix is removing `async` keyword since body is synchronous (uses `parking_lot::Mutex`)
- InterAgentChannel is incomplete: `send_message()` only appends, never notifies subscribers
- AlertRouter missing routing_rules field — `AlertRoutingRules` exists in `agent/alerts/mod.rs:41-47` with tests but is never integrated
- Skills system lacks versioning, proper trigger matching, and validation
- Portfolio persistence broken: `TargetPortfolio::new()` creates `file_path: None`, affecting all 5 call sites in `agent.rs`

**Verified false positives (removed from plan):**
- **C-8 (CircuitBreaker atomic reset)**: Verified FALSE POSITIVE — atomic stores at `circuit_breaker.rs:75-76` ARE inside the `parking_lot::MutexGuard` scope (acquired at line 68). The lock is held until function return. This item was removed from the plan.

**File path corrections for future agents:**
- Ruby plugin code is in `crates/slapper-ruby/src/loader.rs`, NOT `slapper-plugin/src/ruby.rs` (that file does not exist)
- `reload_plugin()` is at `slapper-plugin/src/lib.rs:433-461`, NOT `slapper-plugin/src/plugin.rs` (that file does not exist)
- Skills feature gate is at `agent/mod.rs:16-17`, NOT `agent/skills.rs:16`
- `fuzzer/redos_integrator.rs` does NOT exist — ReDoS integration should modify `fuzzer/redos_detect.rs` and engine config files
- `CsrfExtractor::extract_from_html()` is the correct method name (not `extract_tokens_from_html()`)
- `cmd.rs` has 38 payloads, not 35

**Verified design notes (not bugs):**
- SensitiveString plaintext serialization is intentional (config file compatibility)
- API auth bypass when key unconfigured is intentional (development mode)
- `CircuitBreaker` atomic operations are correctly inside the lock scope

### Verified Incorrect Plan Items

The following items were found to be incorrect during verification (note: the consolidated plan.md now reflects verified state):

- **D.7 (HistoryTab search)**: Plan claimed search was unavailable, but `search()` method EXISTS
- **D.8 (SettingsTab progress)**: Plan claimed missing, but 0.0 is correct - SettingsTab has no async work
- **E.2 Issue 2 (tab dual arms)**: Plan claimed tabs/mod.rs was missing dual arms, but they exist
- **E.4 (AST-based security)**: Plan described AST-based but code is regex-based (regex is current implementation)
- **E.8 (Templates)**: Plan treated as capability gap, but `scanner/templates/` already exists
- **E-12 (Command Palette)**: Plan treated as new feature, but command palette already EXISTS in `tui/app/command.rs` — frame as enhancement
- **E-20 (ReDoS integrator)**: Referenced non-existent `redos_integrator.rs` — should modify existing `redos_detect.rs`
- **Wave A items (A-9, A-12, A-14)**: Originally marked as bugs but verification showed they're already correct:
  - A-9: `trigger_event()` IS called in process_scheduled_scans() at line 220
  - A-12: cleanup_stale_entries() is synchronous, no async issue
  - A-14: MutexGuard drops immediately after recording (scope block)

---

*End of AGENTS.md*