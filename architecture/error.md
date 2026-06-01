# Error Module

## Purpose

Unified error types for the entire Slapper codebase. `SlapperError` is the primary error enum with 22 variants covering configuration, network, HTTP, IO, proxy, and domain-specific failure modes.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `SlapperError` | `error/mod.rs` | Primary error enum with 22 variants |
| `Result<T>` | `error/mod.rs` | Type alias for `Result<T, SlapperError>` |

### SlapperError Variants

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
| `mod.rs` | `SlapperError` enum, `From` impls for `reqwest::Error`/`anyhow::Error`, helper methods (`is_timeout()`, `is_network()`, `http_status()`, `with_timeout()`) |

## From Implementations

`SlapperError` implements `From` for 21 error types. One is via `#[from]` attribute on the enum variant; 20 are manual impls.

### Non-Feature-Gated (18)

| Source Type | Target Variant | Location |
|-------------|----------------|----------|
| `std::io::Error` | `Io` | `mod.rs:82` (via `#[from]` attribute) |
| `reqwest::Error` | `Timeout`, `Network`, `HttpStatus`, or `RequestFailed` | `mod.rs:172-200` (dispatches based on error kind) |
| `toml::de::Error` | `Parse` | `mod.rs:202-206` |
| `serde_json::Error` | `Parse` | `mod.rs:208-212` |
| `url::ParseError` | `Parse` | `mod.rs:214-218` |
| `std::net::AddrParseError` | `AddressParse` | `mod.rs:220-224` |
| `serde_yaml_neo::Error` | `Parse` | `mod.rs:226-230` |
| `toml::ser::Error` | `Parse` | `mod.rs:232-236` |
| `std::string::FromUtf8Error` | `Parse` | `mod.rs:238-242` |
| `tokio::time::error::Elapsed` | `Timeout` | `mod.rs:244-251` |
| `crate::config::ScopeError` | `ScopeViolation` | `mod.rs:253-257` |
| `hickory_resolver::net::NetError` | `Network` | `mod.rs:259-263` |
| `anyhow::Error` | `RequestFailed` | `mod.rs:265-273` |
| `std::num::ParseIntError` | `Parse` | `mod.rs:329-333` |
| `tokio::sync::AcquireError` | `Runtime` | `mod.rs:335-339` |
| `quick_xml::Error` | `Output` | `mod.rs:341-345` |
| `maxminddb::MaxMindDbError` | `Io` (via `std::io::Error::other`) | `mod.rs:347-351` |
| `reqwest::header::InvalidHeaderValue` | `Http` | `mod.rs:353-357` |

### Feature-Gated (3)

| Source Type | Target Variant | Feature Gate | Location |
|-------------|----------------|--------------|----------|
| `crate::ai::AiError` | `RequestFailed`, `Config`, `Parse`, `Timeout`, or `RateLimited` | `ai-integration` | `mod.rs:275-313` |
| `crate::packet::CaptureError` | `Network` | `packet-inspection` | `mod.rs:315-320` |
| `crate::packet::TracerouteError` | `Network` | `packet-inspection` OR `stress-testing` | `mod.rs:322-327` |

## Implementation Status

Fully implemented. Comprehensive error enum with `thiserror` derives, `From` conversions for common third-party errors, and helper methods for error classification.
