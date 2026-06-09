# WebSocket Module

## Purpose

WebSocket security testing including message injection, DoS, cross-site WebSocket hijacking, origin validation, and frame fuzzing.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WebSocketTestReport` | `websocket/mod.rs` | Aggregated WebSocket test results |
| `WebSocketFinding` | `websocket/mod.rs` | WebSocket security finding |
| `WebSocketTestConfig` | `websocket/mod.rs` | Configuration for live tests (feature-gated) |
| `ConnectionTestResult` | `websocket/connection.rs` | Connection test results |
| `InjectionTestResult` | `websocket/injection.rs` | Message injection test results |
| `OriginTestResult` | `websocket/origin.rs` | Origin validation test results |
| `FuzzTestResult` | `websocket/fuzz.rs` | Frame fuzzing test results |
| `WebSocketFuzzer` | `fuzzer/payloads/websocket.rs` | Advanced fuzzer with payload generation |
| `WebSocketVulnerability` | `fuzzer/payloads/websocket.rs` | Vulnerability type enum (7 variants) |

## Files

| File | Description |
|------|-------------|
| `websocket/mod.rs` | Module root: `WebSocketTestReport`, `WebSocketFinding`, `run_live_tests()` |
| `websocket/connection.rs` | `ConnectionTestResult`, `test_connection()` with real WS connect |
| `websocket/injection.rs` | `InjectionTestResult`, `test_injection()` with message send/receive |
| `websocket/origin.rs` | `OriginTestResult`, `test_origins()` with custom Origin headers |
| `websocket/fuzz.rs` | `FuzzTestResult`, `test_dos()`, `test_message_fuzz()` |
| `fuzzer/payloads/websocket.rs` | `WebSocketFuzzer` payload generation and test cases |

## Feature Gating

The `websocket` feature enables real WebSocket connections via `tokio-tungstenite`:

- **With `websocket` feature**: `WebSocketFuzzer.fuzz()` runs both local payload generation AND live connection tests
- **Without `websocket` feature**: Only local payload generation (no network I/O)

The `tokio-tungstenite` dependency is gated behind `#[cfg(feature = "websocket")]`.

## Integration

The `WebSocketFuzzer` integrates via the `AdvancedFuzzer` trait in `fuzzer/advanced.rs`. When invoked via `eggsec fuzz <url> -t websocket`, it:

1. Generates local test cases (always available)
2. When `websocket` feature is enabled, runs live connection tests:
   - **Connection test**: Connect, measure latency, capture headers
   - **Origin validation**: Test with malicious origins (CSWSH detection)
   - **Injection**: Send payloads, detect vulnerability indicators
   - **DoS**: Large messages, ping floods, rapid close frames
   - **Message Fuzzing**: Empty messages, null bytes, control chars

## OWASP Mapping

| Vulnerability | OWASP Category |
|---------------|----------------|
| Injection | A03:2021 - Injection |
| DoS | A05:2021 - Security Misconfiguration |
| CSWSH / Origin Bypass | A01:2021 - Broken Access Control |
| Message/Frame Fuzzing | A03:2021 - Injection |
| Auth Bypass | A07:2021 - Identification and Authentication Failures |

## Tests

7 tests in `fuzzer/payloads/websocket.rs`, plus unit tests in each `websocket/*.rs` file, all under `#[cfg(test)]`:
- `test_get_payloads_returns_non_empty`
- `test_get_payloads_count_reasonable`
- `test_payloads_are_non_empty_strings`
- `test_payloads_contain_expected_patterns`
- `test_subprotocol_tests_generation`
- `test_subprotocol_tests_empty_when_no_protocols`
- `test_all_tests_includes_subprotocol`
- `test_connection_result_creation`
- `test_injection_result_creation`
- `test_origin_result_creation`
- `test_fuzz_result_creation`
- `test_finding_creation`

## Implementation Status

Fully implemented. All six test categories (injection, DoS, CSWSH, message fuzz, frame fuzz, subprotocol) generate structured test cases via `WebSocketFuzzer`. When the `websocket` feature is enabled, real WebSocket connections are made using `tokio-tungstenite` for live security testing. OWASP category mapping is implemented in `FuzzerResultConverter`.
