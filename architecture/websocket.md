# WebSocket Module

## Purpose

WebSocket security testing including real connection testing, message injection, authentication bypass, origin validation, and frame fuzzing.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WebSocketTestReport` | `websocket/mod.rs` | Aggregated WebSocket test results |
| `WebSocketFinding` | `websocket/mod.rs` | WebSocket security finding |
| `ConnectionTestResult` | `websocket/connection.rs` | Connection test results |
| `InjectionTestResult` | `websocket/injection.rs` | Message injection test results |
| `OriginTestResult` | `websocket/origin.rs` | Origin validation test results |
| `FuzzTestResult` | `websocket/fuzz.rs` | Frame fuzzing test results |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `WebSocketTestReport`, `WebSocketFinding` |
| `connection.rs` | WebSocket connection testing (upgrade, handshake) |
| `injection.rs` | Message injection testing (XSS, SQLi via WebSocket) |
| `origin.rs` | Origin header validation testing |
| `fuzz.rs` | WebSocket frame fuzzing |

## Feature Gating

The public API methods in `connection.rs`, `injection.rs`, `origin.rs`, and `fuzz.rs` are each gated behind `#[cfg(feature = "websocket")]`. The module root (`mod.rs`) is not feature-gated; it always compiles.

## Tests

7 tests in `fuzzer/payloads/websocket.rs:349-411`, all under `#[cfg(test)]` (none feature-gated):
- `test_get_payloads_returns_non_empty`
- `test_get_payloads_count_reasonable`
- `test_payloads_are_non_empty_strings`
- `test_payloads_contain_expected_patterns`
- `test_subprotocol_tests_generation`
- `test_subprotocol_tests_empty_when_no_protocols`
- `test_all_tests_includes_subprotocol`

## Implementation Status

Fully implemented. All four test categories (connection, injection, origin, fuzz) are functional with structured result types.
