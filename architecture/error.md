# Error Module

## Purpose

Unified error types for the entire Eggsec codebase. `EggsecError` is the primary error enum with 23 variants covering configuration, network, HTTP, IO, proxy, and domain-specific failure modes.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `EggsecError` | `error/mod.rs` | Primary error enum with 23 variants |
| `Result<T>` | `error/mod.rs` | Type alias for `Result<T, EggsecError>` |

### EggsecError Variants

| Variant | Description |
|---------|-------------|
| `Config(String)` | Configuration errors |
| `InvalidTarget(String)` | Invalid target specification |
| `Network(String)` | Network/connection errors |
| `RequestFailed { method, url, error }` | HTTP request failures |
| `Timeout { timeout_ms, operation }` | Operation timeouts |
| `RateLimited(String)` | Rate limiting encountered |
| `ScanFailed { stage, error }` | Scan stage failures |
| `Payload(String)` | Payload generation errors |
| `Internal(String)` | Internal errors |
| `Output(String)` | Output formatting errors |
| `ScopeViolation(String)` | Target scope violations |
| `Io(std::io::Error)` | IO errors (via `From` impl) |
| `HttpStatus { status, message }` | HTTP status code errors |
| `Http(String)` | General HTTP errors |
| `Parse(String)` | Parse errors |
| `Validation(String)` | Validation errors |
| `AddressParse(String)` | Address parsing errors |
| `Runtime(String)` | Runtime errors |
| `Cancelled` | Operation cancelled |
| `Proxy(String)` | Proxy errors |
| `Recon(String)` | Reconnaissance errors |
| `LoadTest(String)` | Load test errors |
| `Fingerprint(String)` | Fingerprint errors |

## Files

| File | Description |
|------|-------------|
| `error/mod.rs` | `EggsecError` enum, `From` impls for 21 error types, helper methods (`is_timeout()`, `is_network()`, `http_status()`, `with_timeout()`) |
| `utils/error.rs` | Error message sanitization utilities (`sanitize_error_message()`, `sanitize_rate_limit_error()`, `sanitize_internal_error()`) |

## From Implementations

`EggsecError` implements `From` for 21 error types. One is via `#[from]` attribute on the enum variant; 20 are manual impls.

### Non-Feature-Gated (18)

| Source Type | Target Variant | Location |
|-------------|----------------|----------|
| `std::io::Error` | `Io` | `mod.rs:85` (via `#[from]` attribute) |
| `reqwest::Error` | `Timeout`, `Network`, `HttpStatus`, or `RequestFailed` | `mod.rs:175-202` (dispatches based on error kind) |
| `toml::de::Error` | `Parse` | `mod.rs:205-208` |
| `serde_json::Error` | `Parse` | `mod.rs:211-213` |
| `url::ParseError` | `Parse` | `mod.rs:217-219` |
| `std::net::AddrParseError` | `AddressParse` | `mod.rs:223-225` |
| `serde_yaml_neo::Error` | `Parse` | `mod.rs:229-231` |
| `toml::ser::Error` | `Parse` | `mod.rs:235-237` |
| `std::string::FromUtf8Error` | `Parse` | `mod.rs:241-243` |
| `tokio::time::error::Elapsed` | `Timeout` | `mod.rs:247-253` |
| `crate::config::ScopeError` | `ScopeViolation` | `mod.rs:256-258` |
| `hickory_resolver::net::NetError` | `Network` | `mod.rs:262-264` |
| `anyhow::Error` | `Internal` | `mod.rs:268-277` |
| `std::num::ParseIntError` | `Parse` | `mod.rs:333-336` |
| `tokio::sync::AcquireError` | `Runtime` | `mod.rs:339-342` |
| `quick_xml::Error` | `Output` | `mod.rs:345-348` |
| `maxminddb::MaxMindDbError` | `Io` (via `std::io::Error::other`) | `mod.rs:351-354` |
| `reqwest::header::InvalidHeaderValue` | `Http` | `mod.rs:357-360` |

### Feature-Gated (3)

| Source Type | Target Variant | Feature Gate | Location |
|-------------|----------------|--------------|----------|
| `crate::ai::AiError` | `RequestFailed`, `Config`, `Parse`, `Timeout`, or `RateLimited` | `ai-integration` | `mod.rs:279-317` |
| `crate::packet::CaptureError` | `Network` | `packet-inspection` | `mod.rs:319-324` |
| `crate::packet::TracerouteError` | `Network` | `packet-inspection` OR `stress-testing` | `mod.rs:326-331` |

## Related Error Types

These domain-specific error types serve specialized purposes and intentionally do **not** convert to `EggsecError`. They are used within their respective modules and converted at module boundaries via `.map_err()`.

| Type | Location | Purpose | Converts to `EggsecError`? |
|------|----------|---------|-----------------------------|
| `ConfigError` | `config/settings.rs:707` | Config file IO/parse/serialize errors | No (config boundary) |
| `ScopeError` | `config/scope.rs:420` | Target scope validation errors | Yes (via `From` impl) |
| `AiError` | `ai/errors.rs:6` | AI/LLM API errors (9 variants) | Yes (feature-gated) |
| `CaptureError` | `packet/capture.rs:440` | Packet capture errors (7 variants) | Yes (feature-gated) |
| `TracerouteError` | `packet/traceroute.rs:543` | Traceroute errors (4 variants) | Yes (feature-gated) |
| `ProbeError` | `packet/traceroute.rs:555` | Traceroute probe errors (5 variants) | No (encapsulated by `TracerouteError`) |
| `ToolError` / `ToolErrorType` | `tool/tool_error.rs:4` / `tool/tool_error.rs:51` | Serializable API/MCP error (11 types) | No (serializable JSON schema) |
| `QueueError` | `distributed/queue.rs:155` | Distributed task queue errors | No (queue boundary) |
| `CallbackUrlValidationError` | `tool/protocol/agent_routes.rs:28` | MCP callback URL validation | No (validation boundary) |
| `PacketValidationError` | `packet/craft.rs:68` | Packet crafting validation | No (crafting boundary) |
| `CiError` | `commands/handlers/ci.rs:9` | CI exit code semantics | No (not `std::error::Error`) |
| `TabError` | `tui/app/tab_error.rs:4` | TUI tab error categorization | No (TUI boundary) |

### Design Rationale

- **`EggsecError`** is the canonical error for library code. All modules that are part of the core library return `Result<T, EggsecError>` (aliased as `crate::error::Result<T>`).
- **Domain-specific errors** (`ConfigError`, `ToolError`, `QueueError`, etc.) exist where callers need structured error data (e.g., `ToolError` is serialized to JSON for MCP responses; `CiError` maps to process exit codes).
- **`anyhow::Result`** is used in binary entry points (command handlers, TUI workers, agent code) for convenience, with `.map_err()` bridges to `EggsecError` at boundaries.

## Utilities

| Function | File | Description |
|----------|------|-------------|
| `sanitize_error_message()` | `utils/error.rs:31` | Strips stack traces, file paths, internal details from error strings; truncates to 200 chars |
| `sanitize_rate_limit_error()` | `utils/error.rs:70` | Sanitizes and additionally strips rate limiter implementation details |
| `sanitize_internal_error()` | `utils/error.rs:77` | Returns a generic "internal error" message for external consumption |

These utilities prevent information leakage when error messages are exposed to clients (e.g., API responses, TUI display). Used by the tool layer and API handlers.

## Implementation Status

Fully implemented. Comprehensive error enum with `thiserror` derives, `From` conversions for common third-party errors, and helper methods for error classification.
