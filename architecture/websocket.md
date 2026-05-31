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

## Implementation Status

Fully implemented. All four test categories (connection, injection, origin, fuzz) are functional with structured result types.
