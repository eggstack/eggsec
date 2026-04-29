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

- `plans/plan.md` — Master Consolidated Improvement Plan (sole plan file, all others removed in 2026-04-29)
  - Organized into 7 Waves for parallel execution.
  - **Wave 1**: Critical & Security (vulnerabilities, blockers)
  - **Wave 2**: TUI UX & Features (Search, Clipboard, Pause/Resume)
  - **Wave 3**: Core Quality & Refactor (splitting large files, error handling)
  - **Wave 4**: Performance & Hardening (FxHashMap, Regex LruCache)
  - **Wave 5**: Feature Enhancements (Agent, Plugin, WAF/Fuzzer gaps)
  - **Wave 6**: Long-term Capabilities (Exploitation, Cloud, Mobile)
  - **Wave 7**: Documentation (Full updates)

**On Using This Guide**: When working on items from `plan.md`, always verify claims against the actual codebase. Line numbers and file paths in plans may become outdated as code evolves. Use `rg` to confirm before implementing.

## Lessons Learned

### Codebase Verification Required

When implementing plan items, verify actual state rather than assuming plan accuracy:
- Payload type count: 30 (verified via `fuzzer/payloads/mod.rs`)
- Recon module count: 31 (verified)
- Test count: 1115 passing base, 1364 with full features
- Use `rg` to confirm file paths and line numbers exist
- Run `cargo test --lib -p slapper` after each change

### Wave-Based Parallelization

The master plan in `plan.md` is structured for parallel implementation by domain. Independent waves can be assigned to separate agents.

### Session Learnings (2026-04-29)

**Consolidation Results:**
- Merged all source plans (`code_quality_review.md`, `tui_improvements.md`, etc.) into a single `plans/plan.md`.
- Reorganized plan into 7 waves supporting parallel execution by domain.
- Verified critical items like `unwrap_u8` auth pattern and `TOCTOU` config fixes are already implemented.
- Identified and documented 4 false positives in original plan claims.
- Verified that `Global Search`, `Clipboard`, and `Pause/Resume` are currently stubs/missing and added them to Wave 2.

**Verified False Positives:**
| Item | Claim | Actual State |
|------|-------|--------------|
| k8s-openapi | full feature doesn't compile | Already fixed - version is 0.22 |
| SSRF/Private IP blocking | No blocking in executor | Partially protected via template loader |
| Intercept proxy TLS | No TLS validation | NOT A BUG - by design (intercepting proxy) |
| HistoryTab Search | missing | Feature EXISTS in `tui/tabs/history.rs` |

**Pre-existing AI Test Failures (7)** - These are known and should be addressed separately:
1. `ai::client::tests::test_extract_content_valid_response` - expects 3 lines, gets 4
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage`
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration`
4. `ai::planner::tests::test_parse_modifications_multiple_types`
5. `ai::planner::tests::test_planner_cache_clear`
6. `ai::planner::tests::test_record_outcome_updates_success_rate`
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base`

---

*End of AGENTS.md*