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
│   └── utils/         # Common utilities
├── tests/             # Integration tests
└── Cargo.toml
```

### Key Types

- `SlapperConfig` - Main configuration (use `config::load_config()`)
- `PathsConfig` - Directory paths (flattened into SlapperConfig)
- `SpoofConfig` - IP spoofing settings
- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `PayloadType` - Enum of 22+ payload categories
- `Severity` - Canonical severity rating (in `types.rs`, re-exported everywhere)
- `SensitiveString` - Zeroized credential wrapper (in `types.rs`)

### Feature Flags

- `stress-testing` - Enables ICMP probing, IP spoofing, raw sockets
- `packet-inspection` - Packet capture features
- `python-plugins` / `ruby-plugins` - Plugin language support
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `nse-sandbox` - NSE sandbox mode (restricts `io.popen`, `os.setenv`, filesystem access)
- `full` - All features combined

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
- `Debug` and `Display` show `[REDACTED]`
- Constant-time equality (via `subtle::ConstantTimeEq`)
- Serializes transparently for config file compatibility

### Macros

`fuzzer/payloads/macros.rs` defines `payload_vec!` for building payload vectors from inline data. Reduces repetitive `for` loops in payload modules (e.g., sqli.rs went from 8 loops to 1 macro call).

### WAF Constants

`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

### Spoofed Scanner

`scanner/ports/spoofed.rs` contains raw socket scanning (feature-gated). `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled. Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing.

## Codebase Health

| Metric | Value |
|--------|-------|
| Tests | 350 passing |
| Build | Clean compilation |
| Clippy | 0 warnings |
| Doctests | 14 pass, 1 ignored, 0 fail |
| Ruby plugins | Zero warnings with `--features ruby-plugins` |
| Largest file | `waf/detector/detect.rs` (195 lines) |
| Improvement plan | See `plan.md` (consolidated from 5 source plans) |

## Planning

- `plan.md` — Consolidated improvement plan with sequential phases and parallelization strategy
- `CODE_REVIEW_PLAN.md` — Historical reference (anyhow migration + waf/detector split, completed)
- Work items in `plan.md` are ordered by priority; the parallelization section maps tasks to sub-agents
- Always check the "Already Complete" section in `plan.md` before starting work — some items were verified done during consolidation

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

### Severity Enum

- `Severity` derives `Ord` by declaration order (Critical < High), NOT semantic order
- Use `as_int()` for severity comparisons, not `>` or `<`
- `Display` outputs UPPERCASE ("CRITICAL"), `as_str()` outputs lowercase ("critical")
- `serde` serialization uses lowercase (due to `#[serde(rename_all = "lowercase")]`)

### SensitiveString

- Field is private; use `expose_secret()` (borrow) or `into_secret()` (consume)
- `into_secret()` uses `std::mem::take` internally to work with `ZeroizeOnDrop`
- `PartialEq` uses constant-time comparison; safe for credential checking
- Config deserialization works transparently — existing TOML files with plain strings still load

### Truncation Functions

Two truncation utilities exist with different behaviors:
- `utils::truncate` — strips control characters
- `utils::truncate_simple` — preserves all characters

Some modules alias `truncate_simple as truncate`, hiding the difference. When adding new truncation calls, choose explicitly.

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
