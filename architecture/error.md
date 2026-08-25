# Error Module

## Overview

Unified error types for the entire Eggsec codebase. `EggsecError` is the primary error enum with **23 variants** covering configuration, network, HTTP, IO, proxy, and domain-specific failure modes. It uses `thiserror` for `Display`/`Error` derives and provides 22 `From` conversions (1 via `#[from]` derive + 21 manual impls) for ergonomic error propagation.

Related: [types.md](types.md), [constants.md](constants.md), [overview.md](overview.md).

---

## Location & Feature Gating

| Item | File | Notes |
|------|------|-------|
| `EggsecError` enum | `crates/eggsec/src/error/mod.rs:44` | 23 variants |
| `Result<T>` alias | `crates/eggsec/src/error/mod.rs:173` | `std::result::Result<T, EggsecError>` |
| Helper methods | `crates/eggsec/src/error/mod.rs:121-170` | `is_timeout()`, `is_network()`, `http_status()`, `with_timeout()` |
| Non-feature-gated `From` impls | `crates/eggsec/src/error/mod.rs:85-277` | 19 impls (1 `#[from]` + 18 manual) |
| Feature-gated `From` impls | `crates/eggsec/src/error/mod.rs:279-368` | 3 impls: `ai-integration`, `packet-inspection`, `web-proxy` |
| Sanitization utilities | `crates/eggsec/src/utils/error.rs` | 3 public functions |

No feature gating on the enum itself — `EggsecError` is always compiled.

---

## Architecture

### EggsecError Enum

**Derives**: `Debug`, `Error` (thiserror) — `:43`

**23 variants** (verified by counting, `:44-119`):

| # | Variant | Display format | Fields | Line |
|---|---------|---------------|--------|------|
| 1 | `Config(String)` | `"Configuration error: {0}"` | inner: `String` | `:46` |
| 2 | `InvalidTarget(String)` | `"Invalid target: {0}"` | inner: `String` | `:48` |
| 3 | `Network(String)` | `"Network error: {0}"` | inner: `String` | `:50` |
| 4 | `RequestFailed { method, url, error }` | `"Request failed: {method} {url} - {error}"` | 3 fields: `String` each | `:54-59` |
| 5 | `Timeout { timeout_ms, operation }` | `"Timeout after {timeout_ms}ms: {operation}"` | `timeout_ms: u64`, `operation: String` | `:63-64` |
| 6 | `RateLimited(String)` | `"Rate limited: {0}"` | inner: `String` | `:66` |
| 7 | `ScanFailed { stage, error }` | `"Scan failed: {stage} - {error}"` | 2 fields: `String` each | `:69-70` |
| 8 | `Payload(String)` | `"Payload error: {0}"` | inner: `String` | `:72` |
| 9 | `Output(String)` | `"Output error: {0}"` | inner: `String` | `:75` |
| 10 | `Internal(String)` | `"Internal error: {0}"` | inner: `String` | `:78` |
| 11 | `ScopeViolation(String)` | `"Scope violation: {0}"` | inner: `String` | `:81` |
| 12 | `Io(std::io::Error)` | `"IO error: {0}"` | inner: `std::io::Error` | `:84-85` |
| 13 | `HttpStatus { status, message }` | `"HTTP error {status}: {message}"` | `status: u16`, `message: String` | `:87-88` |
| 14 | `Http(String)` | `"HTTP error: {0}"` | inner: `String` | `:90` |
| 15 | `Parse(String)` | `"Parse error: {0}"` | inner: `String` | `:93` |
| 16 | `Validation(String)` | `"Validation error: {0}"` | inner: `String` | `:96` |
| 17 | `AddressParse(String)` | `"Address parse error: {0}"` | inner: `String` | `:99` |
| 18 | `Runtime(String)` | `"Runtime error: {0}"` | inner: `String` | `:102` |
| 19 | `Cancelled` | `"Cancelled"` | unit variant | `:105` |
| 20 | `Proxy(String)` | `"Proxy error: {0}"` | inner: `String` | `:108` |
| 21 | `Recon(String)` | `"Recon error: {0}"` | inner: `String` | `:111` |
| 22 | `LoadTest(String)` | `"Load test error: {0}"` | inner: `String` | `:114` |
| 23 | `Fingerprint(String)` | `"Fingerprint error: {0}"` | inner: `String` | `:117` |

### Result Alias

```rust
pub type Result<T> = std::result::Result<T, EggsecError>;
```

Defined at `:173`. Used throughout the engine crate as the standard return type for library code.

---

### Helper Methods

Defined at `:121-170`:

| Method | Source | Description |
|--------|--------|-------------|
| `is_timeout()` | `:123` | Returns `true` if variant is `Timeout` |
| `is_network()` | `:128` | Returns `true` if variant is `Network` |
| `http_status()` | `:133` | Returns `Some(status)` if variant is `HttpStatus`, else `None` |
| `with_timeout(timeout_ms)` | `:161` | If `Timeout`, replaces `timeout_ms` preserving `operation`; else returns self unchanged. Enables chaining: `.map_err(\|e\| e.with_timeout(5000))` |

---

## From Implementations

22 total `From` impls: 1 via `#[from]` attribute on `Io` (`:85`) + 21 manual impls.

### Non-Feature-Gated (19 total: 1 derive + 18 manual)

| # | Source Type | Target Variant | Location | Notes |
|---|-------------|----------------|----------|-------|
| 1 | `std::io::Error` | `Io` | `:85` | `#[from]` attribute — auto-generates `From` |
| 2 | `reqwest::Error` | `Timeout`, `Network`, `HttpStatus`, or `RequestFailed` | `:175-202` | Dispatches by kind: `is_timeout()` → `Timeout`, `is_connect()` → `Network`, `status()` → `HttpStatus`, else `RequestFailed`. Note: `timeout_ms` set to 0 (reqwest doesn't expose configured timeout). |
| 3 | `toml::de::Error` | `Parse` | `:205-208` | Prefixed `"TOML parse error: "` |
| 4 | `serde_json::Error` | `Parse` | `:211-213` | Prefixed `"JSON error: "` |
| 5 | `url::ParseError` | `Parse` | `:217-219` | Prefixed `"URL parse error: "` |
| 6 | `std::net::AddrParseError` | `AddressParse` | `:223-225` | Prefixed `"Invalid address: "` |
| 7 | `serde_yaml_neo::Error` | `Parse` | `:229-231` | Prefixed `"YAML error: "` |
| 8 | `toml::ser::Error` | `Parse` | `:235-237` | Prefixed `"TOML serialization error: "` |
| 9 | `std::string::FromUtf8Error` | `Parse` | `:241-243` | Prefixed `"UTF-8 error: "` |
| 10 | `tokio::time::error::Elapsed` | `Timeout` | `:247-253` | `timeout_ms: 0`, `operation: "async operation"` |
| 11 | `crate::config::ScopeError` | `ScopeViolation` | `:256-258` | Delegates to `Display` |
| 12 | `hickory_resolver::net::NetError` | `Network` | `:262-264` | Prefixed `"DNS resolution failed: "` |
| 13 | `anyhow::Error` | `Internal` | `:268-277` | Chains source: `"{error}: {source}"` if source exists |
| 14 | `std::num::ParseIntError` | `Parse` | `:333-336` | Prefixed `"Integer parse error: "` |
| 15 | `tokio::sync::AcquireError` | `Runtime` | `:339-342` | Prefixed `"Semaphore acquire error: "` |
| 16 | `quick_xml::Error` | `Output` | `:345-348` | Prefixed `"XML error: "` |
| 17 | `maxminddb::MaxMindDbError` | `Io` | `:351-354` | Wraps via `std::io::Error::other()` first |
| 18 | `reqwest::header::InvalidHeaderValue` | `Http` | `:357-360` | Prefixed `"Invalid header value: "` |
| 19 | `eggsec_web_proxy::WebProxyError` | `Network` | `:364-368` | **Feature-gated: `web-proxy`**. Prefixed `"Web proxy error: "` |

### Feature-Gated (3 manual impls)

| # | Source Type | Target Variant(s) | Feature | Location | Mapping |
|---|-------------|-------------------|---------|----------|---------|
| 1 | `crate::ai::AiError` | `RequestFailed`, `Config`, `Parse`, `Timeout`, or `RateLimited` | `ai-integration` | `:279-317` | `RequestFailed` → `RequestFailed`, `MissingApiKey`/`InvalidConfig` → `Config`, `ApiError` → `RequestFailed`, `ParseError`/`InvalidResponse` → `Parse`, `Timeout` → `Timeout`, `RateLimited`/`CircuitBreakerOpen` → `RateLimited` |
| 2 | `crate::packet::CaptureError` | `Network` | `packet-inspection` | `:319-324` | Prefixed `"Packet capture error: "` |
| 3 | `crate::packet::TracerouteError` | `Network` | `packet-inspection` OR `stress-testing` | `:326-331` | Prefixed `"Traceroute error: "` |

---

## Files

| File | Description |
|------|-------------|
| `crates/eggsec/src/error/mod.rs` | `EggsecError` enum (23 variants), `Result<T>` alias, `From` impls (22 total), helper methods |
| `crates/eggsec/src/utils/error.rs` | Error message sanitization utilities for external consumption |

---

## Utilities

Located in `crates/eggsec/src/utils/error.rs`.

| Function | Source | Description |
|----------|--------|-------------|
| `sanitize_error_message(error)` | `:31` | Strips stack traces, file paths, internal details, Rust/Python/Go panics, Windows paths. Truncates to 200 chars with `"..."` suffix. |
| `sanitize_rate_limit_error(error)` | `:71` | Calls `sanitize_error_message()` then additionally strips rate limiter implementation details (`RateLimiter`, `rate_limit`, `check_rate_limit`). |
| `sanitize_internal_error()` | `:78` | Returns static generic message: `"An internal error occurred. Please check logs for details."` |

**Implementation details**: Uses 8 compiled `LazyLock<Regex>` statics (`:9-29`) for pattern matching. Regex compilation is deferred to first access.

**Used by**: Tool layer, API handlers, TUI display — anywhere error messages are exposed to external consumers.

---

## Related Error Types

These domain-specific error types serve specialized purposes and intentionally do **not** convert to `EggsecError`. They are converted at module boundaries via `.map_err()`.

| Type | Location | Purpose | Converts to `EggsecError`? |
|------|----------|---------|-----------------------------|
| `ConfigError` | `config/settings.rs:707` | Config file IO/parse/serialize errors | No (config boundary) |
| `ScopeError` | `config/scope.rs:420` | Target scope validation errors | Yes (via `From` impl) |
| `AiError` | `ai/errors.rs:6` | AI/LLM API errors (9 variants) | Yes (feature-gated) |
| `CaptureError` | `packet/capture.rs:440` | Packet capture errors (7 variants) | Yes (feature-gated) |
| `TracerouteError` | `packet/traceroute.rs:543` | Traceroute errors (4 variants) | Yes (feature-gated) |
| `ProbeError` | `packet/traceroute.rs:555` | Traceroute probe errors (5 variants) | No (encapsulated by `TracerouteError`) |
| `ToolError` / `ToolErrorType` | `tool/tool_error.rs:4` / `:51` | Serializable API/MCP error (11 types) | No (serializable JSON schema) |
| `QueueError` | `distributed/queue.rs:155` | Distributed task queue errors | No (queue boundary) |
| `CallbackUrlValidationError` | `tool/protocol/agent_routes.rs:28` | MCP callback URL validation | No (validation boundary) |
| `PacketValidationError` | `packet/craft.rs:68` | Packet crafting validation | No (crafting boundary) |
| `CiError` | `commands/handlers/ci.rs:9` | CI exit code semantics | No (not `std::error::Error`) |
| `TabError` | `tui/app/tab_error.rs:4` | TUI tab error categorization | No (TUI boundary) |

### Design Rationale

- **`EggsecError`** is the canonical error for library code. All modules that are part of the core library return `Result<T, EggsecError>` (aliased as `crate::error::Result<T>`).
- **Domain-specific errors** exist where callers need structured error data (e.g., `ToolError` is serialized to JSON for MCP responses; `CiError` maps to process exit codes).
- **`anyhow::Result`** is used in binary entry points (command handlers, TUI workers, agent code) for convenience, with `.map_err()` bridges to `EggsecError` at boundaries.

---

## Integration Points

| Consumer | Usage |
|----------|-------|
| All engine modules (`scanner/`, `fuzzer/`, `recon/`, `waf/`, `auth/`, `loadtest/`, `pipeline/`, `hunt/`, `proxy/`, `stress/`, `evasion/`, `postex/`, `c2/`) | Return `Result<T>` from public functions |
| `commands/handlers/` (32 handler modules) | Bridge `anyhow::Result` → `EggsecError` at boundaries |
| `tool/dispatcher.rs` | `EnforcedDispatcher` propagates `EggsecError` |
| `runtime_bridge/` | Converts domain errors to `EggsecError` for runtime surfaces |
| `eggsec-python/src/error.rs:37` | `engine_error_to_pyerr()` converts `EggsecError` → Python `PyErr` |
| `eggsec-python/src/sbom.rs:305` | Raises `EggsecError::Config` for SBOM feature conflicts |
| `eggsec-python/src/websocket.rs:584` | Typed error returns for WebSocket tests |
| `eggsec-tui/` | Displays sanitized errors via `utils/error.rs` functions |

---

## Testing

Tests in `crates/eggsec/src/error/mod.rs:370-417`:

| Test | Source | What it verifies |
|------|--------|-----------------|
| `test_error_is_timeout` | `:374` | `is_timeout()` true for `Timeout`, false for others |
| `test_error_is_network` | `:384` | `is_network()` true for `Network`, false for others |
| `test_error_http_status` | `:392` | `http_status()` returns `Some(404)` for `HttpStatus`, `None` otherwise |
| `test_error_display` | `:404` | `Display` format: `"Invalid target: empty host"` |
| `test_result_type` | `:410` | `Result<T>` alias works, `Display` format: `"Runtime error: something went wrong"` |

Additional test in `crates/eggsec/tests/feature_tests.rs:10`: constructs `EggsecError::Config` to verify the type is accessible.

Sanitization tests in `crates/eggsec/src/utils/error.rs:82-112`:

| Test | What it verifies |
|------|-----------------|
| `test_sanitize_removes_stack_traces` | Java stack traces stripped |
| `test_sanitize_removes_paths` | Unix paths stripped |
| `test_sanitize_truncates_long_errors` | Output ≤ 200 chars |
| `test_rate_limit_sanitization` | Rate limiter details stripped |

---

## Invariants & Gotchas

1. **`reqwest::Error` conversion loses timeout value**: When converting from `reqwest::Error`, `timeout_ms` is set to `0` because reqwest doesn't expose the configured timeout value (`:179`). Use `with_timeout()` to set the actual value after conversion.

2. **`anyhow::Error` conversion chains source**: If `anyhow::Error` has a source, the message becomes `"{error}: {source}"` (`:270-274`). This preserves context but may produce long strings.

3. **`maxminddb::MaxMindDbError` wraps through `io::Error`**: The conversion first creates `std::io::Error::other()` then converts via the `#[from]` path (`:353`). This preserves the IO error variant rather than creating a raw `Internal`.

4. **`Cancelled` is a unit variant**: It carries no context. If the cancellation reason matters, use a different variant or add context via `.map_err()`.

5. **Many variants use bare `String`**: `Config`, `InvalidTarget`, `Network`, `Payload`, `Output`, `Internal`, `Validation`, `Runtime`, `Proxy`, `Recon`, `LoadTest`, `Fingerprint` all wrap a single `String`. This provides flexibility but loses structured data. Consider using typed inner errors for new code.

6. **`ScopeError` → `ScopeViolation` via `From`**: The only domain error that has a direct `From` conversion. All others use `.map_err()` at boundaries.

7. **Sanitization truncates to 200 chars**: `sanitize_error_message()` in `utils/error.rs:62-66` truncates with `"..."` suffix. Long error messages lose detail.

8. **No `Clone` on `EggsecError`**: The enum contains `std::io::Error` which is not `Clone`. If you need to duplicate an error, clone the message string instead.

---

*Last verified against source: 2026-08-25*
