# WebSocket Module

## Purpose

WebSocket security testing including message injection, DoS, cross-site WebSocket hijacking, origin validation, and frame fuzzing.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WebSocketTestReport` | `websocket/mod.rs` | Aggregated WebSocket test results |
| `WebSocketFinding` | `websocket/mod.rs` | WebSocket security finding |
| `ConnectionTestResult` | `websocket/connection.rs` | Connection test results |
| `InjectionTestResult` | `websocket/injection.rs` | Message injection test results |
| `OriginTestResult` | `websocket/origin.rs` | Origin validation test results |
| `FuzzTestResult` | `websocket/fuzz.rs` | Frame fuzzing test results |
| `WebSocketFuzzer` | `fuzzer/payloads/websocket.rs` | Advanced fuzzer with payload generation |
| `WebSocketVulnerability` | `fuzzer/payloads/websocket.rs` | Vulnerability type enum (7 variants) |

## Files

| File | Description |
|------|-------------|
| `websocket/mod.rs` | Module root: `WebSocketTestReport`, `WebSocketFinding` |
| `websocket/connection.rs` | `ConnectionTestResult` type definition |
| `websocket/injection.rs` | `InjectionTestResult` type definition |
| `websocket/origin.rs` | `OriginTestResult` type definition |
| `websocket/fuzz.rs` | `FuzzTestResult` type definition |
| `fuzzer/payloads/websocket.rs` | `WebSocketFuzzer` payload generation and test cases |

## Feature Gating

The `WebSocketFuzzer` in `fuzzer/payloads/websocket.rs` always compiles (no feature gate). It generates test cases without making real WebSocket connections.

The `tokio-tungstenite` dependency is gated behind `#[cfg(feature = "websocket")]` but is not currently used by the fuzzer.

## Integration

The `WebSocketFuzzer` integrates via the `AdvancedFuzzer` trait in `fuzzer/advanced.rs`. When invoked via `slapper fuzz <url> -t websocket`, it generates test cases for:

- **Injection**: SQL injection, XSS, template injection, JNDI, path traversal, prototype pollution
- **DoS**: Large frames, ping floods, message floods, rapid close frames
- **CSWSH**: Cross-site WebSocket hijacking with malicious origins
- **Message Fuzzing**: Empty messages, null bytes, control characters, malformed JSON
- **Frame Fuzzing**: Invalid opcodes, fragmented frames, close status codes
- **Subprotocol**: GraphQL-WS, SOAP, MQTT, WAMP protocol testing

## OWASP Mapping

| Vulnerability | OWASP Category |
|---------------|----------------|
| Injection | A03:2021 - Injection |
| DoS | A05:2021 - Security Misconfiguration |
| CSWSH / Origin Bypass | A01:2021 - Broken Access Control |
| Message/Frame Fuzzing | A03:2021 - Injection |
| Auth Bypass | A07:2021 - Identification and Authentication Failures |

## Tests

7 tests in `fuzzer/payloads/websocket.rs:406-474`, all under `#[cfg(test)]`:
- `test_get_payloads_returns_non_empty`
- `test_get_payloads_count_reasonable`
- `test_payloads_are_non_empty_strings`
- `test_payloads_contain_expected_patterns`
- `test_subprotocol_tests_generation`
- `test_subprotocol_tests_empty_when_no_protocols`
- `test_all_tests_includes_subprotocol`

Additional tests in `websocket/*.rs` for result type construction.

## Implementation Status

Fully implemented. All six test categories (injection, DoS, CSWSH, message fuzz, frame fuzz, subprotocol) generate structured test cases via `WebSocketFuzzer`. OWASP category mapping is implemented in `FuzzerResultConverter`.
