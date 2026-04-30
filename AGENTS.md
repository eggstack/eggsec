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
│   │   ├── chain.rs   # ChainExecutor (with LRU regex cache)
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
| Tests | 1120 passing | Base library tests |
| Tests | 1378 passing | With rest-api,ai-integration |
| Clippy | ~5 warnings | TUI-specific acceptable |
| Source files | 506 |
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

- `plans/plan.md` — Master Consolidated Improvement Plan (sole plan file, all others removed in 2026-04-29)
  - Plan is COMPLETED and pruned as of 2026-04-30
  - All 7 Waves verified complete
  - Only verification notes and completion summary remain

## Important Guidelines

### Codebase Verification Required

When implementing changes or reviewing plan items, verify actual state rather than assuming plan accuracy:
- Payload type count: 30 (verified via `fuzzer/payloads/mod.rs`)
- Recon module count: 31 (verified)
- Test count: 1120 base, 1378 with full features
- Use `rg` to confirm file paths and line numbers exist
- Run `cargo test --lib -p slapper` after each change

### notify-debouncer-mini API

When using `notify-debouncer-mini` in config watching code:
- Version 0.5+ uses callback-based API, NOT channel-based
- Use `new_debouncer(duration, |res: DebounceEventResult| { ... })` with callback
- Access watcher via `debouncer.watcher()` method
- Debouncer struct stores the RecommendedWatcher internally

### Agent Observability

The `agent/logging.rs` module provides `AgentLogger` for file-based logging:
- Call `AgentLogger::init(log_dir)` early in agent startup
- Logs written to `log_dir/agent.log` with daily rotation
- JSON format with thread IDs, file/line info

### Agent Config Hot-Reloading

The `agent/config_watcher.rs` module provides `ConfigWatcher`:
- Use `SlapperConfigReloader` for portfolio + main config paths
- Watcher gracefully handles missing files via `.ok()`
- Requires `ConfigReloader` trait implementation for custom reload logic

### Cookie Management

Reqwest handles cookies automatically when `cookies` feature is enabled:
- Enable via `features = [..., "cookies"]` in Cargo.toml
- Client's internal cookie jar manages Set-Cookie automatically
- Manual cookie header construction is NOT needed when using reqwest Client

### Regex Caching

Use LRU cache for regex patterns to prevent unbounded memory growth:
- `lru = "0.18"` crate available for use
- Recommended cache size: 100 entries (use `NonZeroUsize`)
- Access via `cache.put(key, value)` and `cache.get(&key)`

---

*End of AGENTS.md*