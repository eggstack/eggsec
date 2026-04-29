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
| Tests | 1115 passing | Base library tests |
| Tests | 1238 passing | With rest-api,ai-integration (7 pre-existing AI test failures) |
| Clippy | ~21 warnings | TUI-specific acceptable, some dead code warnings remain |
| Source files | 503 |
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

- `plans/plan.md` — Consolidated improvement plan (sole plan file, all others removed in 2026-04-29)
  - Contains all pending items organized into 7 Waves
  - **Wave 1**: Critical Fixes (compilation, security vulnerabilities)
  - **Wave 2**: Code Quality (dead code, error handling, refactoring)
  - **Wave 3**: Performance (FxHashMap, TUI renders, ReDoS)
  - **Wave 4**: TUI Improvements (LoadTest, AuthTab, CSRF, Formula injection)
  - **Wave 5**: Feature Enhancements (Agent, Plugin, Fuzzer, WAF, CLI, Config)
  - **Wave 6**: New Capabilities (Exploitation, Cloud, Mobile - long term)
  - **Wave 7**: Documentation (README, CAPABILITIES, ARCHITECTURE updates)
  - **Test count: 1115 passing (base), 1364 with full features**
  - **Pre-existing: 7 AI test failures (ai::planner, ai::waf_bypass, ai::client)**

**On Using This Guide**: When working on items from plan.md, always verify claims against the actual codebase. Line numbers and file paths in plans may become outdated as code evolves. Use `rg` to confirm before implementing.

## Lessons Learned

### Codebase Verification Required

When implementing plan items, verify actual state rather than assuming plan accuracy:
- Payload type count: 30 (verified via `fuzzer/payloads/mod.rs`)
- Recon module count: ~30 (more than previously documented)
- Test count: 1115 passing base, 1364 with full features
- Use `rg` to confirm file paths and line numbers exist
- Run `cargo test --lib -p slapper` after each change
- Check test counts: `cargo test --lib -p slapper -- --list 2>/dev/null | grep -c "test$"`

### Wave-Based Parallelization

The consolidated plan.md is structured for parallel implementation by wave. See plan.md's "Parallelization Strategy" section for the full agent assignment tables. Key principle: items in different files can always be parallelized; items touching the same file need sequential execution within a sub-agent.

### Session Learnings (2026-04-29)

**Consolidation Results:**
- Merged 9 plan files into single consolidated plan.md
- Verified 20+ plan items against actual codebase
- Identified 4 false positives (items claimed as bugs were not bugs)
- Marked 1 item as already fixed (k8s-openapi version 0.22)
- Corrected 1 count (12 clones, not 13)

**Newly Verified False Positives:**
| Item | Claim | Actual State |
|------|-------|--------------|
| k8s-openapi | full feature doesn't compile | Already fixed - version is 0.22 |
| SSRF/Private IP blocking | No blocking in executor | Partially protected via template loader |
| Intercept proxy TLS | No TLS validation | NOT A BUG - by design (intercepting proxy) |
| Clone count | 13 clones | Actually **12 clones** |

**Pre-existing AI Test Failures (7)** - will be addressed separately:
1. `ai::client::tests::test_extract_content_valid_response` - expects 3 lines, gets 4
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage` - keyword extraction
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration` - keyword matching
4. `ai::planner::tests::test_parse_modifications_multiple_types` - keyword matching
5. `ai::planner::tests::test_planner_cache_clear` - cache behavior
6. `ai::planner::tests::test_record_outcome_updates_success_rate` - cache entry creation
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base` - knowledge base state

**Note on reqwest 0.13:**
The `cookies()` method is not available in reqwest 0.13 by default. Use manual Set-Cookie header parsing instead.

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

All completed implementation items from historical work sessions are consolidated in `plans/plan.md`. The plan contains:
- 69 completed items across Waves A-G (2026-04-28 session)
- Additional items from 2026-04-29 consolidation
- Full verification details and file references

See `plans/plan.md` for the complete list of all verified completed items.

**Additional Waves Completed** (2026-04-29):
- **Wave 1.1**: Security hardening - path traversal validation, restrictive CORS config, plaintext password redaction with SensitiveString, privilege checks in stress modules
- **Wave 1.2**: Compilation fixes - DashMap deadlocks (spoofed.rs), removed missing `parse` module, added `TracerouteError` export, fixed `ipnet::Ipv4Net` → `ipnetwork::Ipv4Network`, fixed reqwest errors, fixed Arc/DashMap issues, fixed libc::in_addr wrapping
- **Wave 2.1**: TUI bug fixes - mouse redraw now sets `needs_redraw = true`, settings reset calls `reset()`, WafStress parameters properly passed via `run_waf_stress`
- **Wave 2.2**: Orphaned tab removal - icmp.rs, mcp.rs, traceroute.rs deleted
- **Wave 3.1**: Network performance - `connect_with_nodelay_timeout` in fingerprint.rs, HTTP client pooling with TCP_NODELAY in lifecycle.rs and alerts routing.rs
- **Wave 3.2**: Async memory - Converted `agent/memory.rs` from blocking `std::fs` to async `tokio::fs`, updated callers in `agent/mod.rs` to use `.await`, converted tests to `#[tokio::test]`

**Test Count**: 1115 passing (base library tests)

**Deferred Items**:
- **Wave 4.1 (Tab Integration)**: Requires adding 6 new tabs (Auth, Plan, Ci, Serve, Sbom, Notify) to the 29-tab Tab enum - complex architectural change deferred
- **Wave 4.2 (TUI Refactoring)**: History wrapper and animation fix - verified as false positives (features already exist)
- **Waves 5-8**: Many items already complete, false positives, or require large new features (Auto-Calibration, Subdomain Enumeration)

### Session Learnings (2026-04-29)

**AI-integration Compilation Fixes:**
- `tool/implementations/search.rs`: Error conversion to `SlapperError::Network`/`Parse` instead of `String`
- `tool/session.rs`: Manual Set-Cookie header parsing instead of `response.cookies()` (reqwest 0.13 lacks cookies feature)
- `tool/session.rs`: Fixed `TargetType` import and csrf token lookup using `request.target.value`
- `tool/session.rs`: Fixed `StatusCode` parse type annotation with explicit `u16`
- `tool/session.rs`: Fixed `form_re` duplicate `ok()?` bug
- `tool/session.rs`: Fixed `has_field_named` to use `find_field_name` (regex-based string matching was broken)
- `tool/session.rs`: Fixed `NotFound` error - used `Runtime(format!(...))` instead
- `tool/implementations/fuzzer.rs`: Added missing `FuzzArgs` fields (calibrate, fc, fs, fw, fl, ft, fr)

**Test Results:**
- 1115 base tests pass
- 1238 ai-integration tests pass (7 pre-existing AI test failures remain)

**Pre-existing AI Test Failures (7)** - will be addressed separately:
1. `ai::client::tests::test_extract_content_valid_response` - expects 3 lines, gets 4
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage` - keyword extraction
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration` - keyword matching
4. `ai::planner::tests::test_parse_modifications_multiple_types` - keyword matching
5. `ai::planner::tests::test_planner_cache_clear` - cache behavior
6. `ai::planner::tests::test_record_outcome_updates_success_rate` - cache entry creation
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base` - knowledge base state

**Clippy Auto-fix Applied:**
- Added `Default` impl for 8 types: CargoScanner, NpmScanner, GoScanner, ClusterTab, GraphQlTab, OAuthTab, ReportTab, StressTab

**Note on reqwest 0.13:**
The `cookies()` method is not available in reqwest 0.13 by default. Use manual Set-Cookie header parsing instead.

### Session Learnings (2026-04-29)

**AI-integration Compilation Fixes:**
- `tool/implementations/search.rs`: Error conversion to `SlapperError::Network`/`Parse` instead of `String`
- `tool/session.rs`: Manual Set-Cookie header parsing instead of `response.cookies()` (reqwest 0.13 lacks cookies feature)
- `tool/session.rs`: Fixed `TargetType` import and csrf token lookup using `request.target.value`
- `tool/session.rs`: Fixed `StatusCode` parse type annotation with explicit `u16`
- `tool/session.rs`: Fixed `form_re` duplicate `ok()?` bug
- `tool/session.rs`: Fixed `has_field_named` to use `find_field_name` (regex-based was broken)
- `tool/session.rs`: Fixed `NotFound` error - used `Runtime(format!(...))` instead
- `tool/implementations/fuzzer.rs`: Added missing `FuzzArgs` fields (calibrate, fc, fs, fw, fl, ft, fr)

**Test Results:**
- 1115 base tests pass
- 1238 ai-integration tests pass (7 pre-existing AI test failures remain)

**Pre-existing AI Test Failures (7)** - will be addressed separately:
1. `ai::client::tests::test_extract_content_valid_response` - expects 3 lines, gets 4
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage` - keyword extraction
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration` - keyword matching
4. `ai::planner::tests::test_parse_modifications_multiple_types` - keyword matching
5. `ai::planner::tests::test_planner_cache_clear` - cache behavior
6. `ai::planner::tests::test_record_outcome_updates_success_rate` - cache entry creation
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base` - knowledge base state

**Clippy Auto-fix Applied:**
- Added `Default` impl for 8 types: CargoScanner, NpmScanner, GoScanner, ClusterTab, GraphQlTab, OAuthTab, ReportTab, StressTab

**Note on reqwest 0.13:**
The `cookies()` method is not available in reqwest 0.13 by default. Use manual Set-Cookie header parsing instead.

---

### Session Learnings (2026-04-28)

**All Waves Complete**: Waves A through G are now fully implemented (121 items total).
- Test count updated: 1115 passing (base), 1364 with full features
- Source files: 503 (updated from 470+)
- Clippy warnings: ~19 (reduced from ~28)
- See plan.md for verification commands

**Newly Documented Features:**
- `docs/VULNERABILITY_GUIDE.md` created (G-5 through G-9)
- `docs/SCAN_STRATEGY.md` created (G-6)
- API.md deprecation notice added (G-4)
- README.md, CAPABILITIES.md, USAGE.md expanded (G-7 through G-17)

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