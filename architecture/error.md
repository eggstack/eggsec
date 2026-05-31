# Error Module

## Purpose

Unified error types for the entire Slapper codebase. `SlapperError` is the primary error enum with 19+ variants covering configuration, network, HTTP, IO, proxy, and domain-specific failure modes.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `SlapperError` | `error/mod.rs` | Primary error enum with 19 variants |
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

## Implementation Status

Fully implemented. Comprehensive error enum with `thiserror` derives, `From` conversions for common third-party errors, and helper methods for error classification.
