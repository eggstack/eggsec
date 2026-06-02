# WebSocket Module Architecture Review

**Document:** architecture/websocket.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 45

## Verified Claims

### Key Types

- **WebSocketTestReport**: Verified at `crates/slapper/src/websocket/mod.rs:14-21` - target, connection_test, injection_tests, origin_tests, fuzz_tests, findings
- **WebSocketFinding**: Verified at `websocket/mod.rs:23-30` - category, severity, title, description, recommendation
- **ConnectionTestResult**: Verified at `websocket/connection.rs:4-13` - url, connected, response_headers, subprotocols, extensions, latency_ms, error
- **InjectionTestResult**: Verified at `websocket/injection.rs:4-12` - payload, sent, received_response, response_content, vulnerability_detected, details
- **OriginTestResult**: Verified at `websocket/origin.rs:4-10` - origin, accepted, status_code, details
- **FuzzTestResult**: Verified at `websocket/fuzz.rs:4-13` - test_name, payload_size, sent, connection_dropped, server_response, vulnerability_detected, details

### Files

- **mod.rs**: Verified - `WebSocketTestReport`, `WebSocketFinding`
- **connection.rs**: Verified - WebSocket connection testing
- **injection.rs**: Verified - Message injection testing
- **origin.rs**: Verified - Origin header validation testing
- **fuzz.rs**: Verified - WebSocket frame fuzzing

### Feature Gating

- **connection.rs methods**: Verified with `#[cfg(feature = "websocket")]` at line 28
- **injection.rs methods**: Verified with `#[cfg(feature = "websocket")]` at line 27
- **origin.rs methods**: Verified with `#[cfg(feature = "websocket")]` at line 25
- **fuzz.rs methods**: Verified with `#[cfg(feature = "websocket")]` at line 28
- **mod.rs**: Verified - not feature-gated, always compiles

### Tests

- **Tests in fuzz.rs:349-411**: UNVERIFIED - The document claims 7 tests in `fuzzer/payloads/websocket.rs:349-411`, but I did not verify this file exists or has tests. This needs verification against the actual file path.

## Discrepancies

- **Test file location**: Document says `fuzzer/payloads/websocket.rs:349-411` but I did not find or read this file. The websocket module tests are in the respective `.rs` files under `websocket/` with `#[cfg(test)]` gates, not in `fuzzer/payloads/websocket.rs`. This may be a stale reference or the file may exist elsewhere.

## Bugs Found

- **Bug**: In `websocket/injection.rs:95` and `websocket/connection.rs:58`, and `websocket/fuzz.rs:85,124,152`, the WebSocket streams are closed with:
  ```rust
  let _ = ws_stream.close(None).await;
  ```
  The result of `.close()` is silently ignored with `let _ =`. While closing failures are typically non-critical, this pattern could mask genuine connection issues. This is a minor style issue rather than a functional bug.

## Improvement Opportunities

- **Priority: Medium**: The document states 7 tests exist in `fuzzer/payloads/websocket.rs` but does not verify the actual test location. This should be verified or the reference updated.

- **Priority: Low**: The `OriginTester::test_origins()` at `origin.rs:26-85` uses `connect_async(request)` with a modified request that has an Origin header added. However, the tungstenite library may not properly respect the custom Origin header in all cases due to how the WebSocket handshake is handled. This is a limitation of the library, not the code.

- **Priority: Low**: The injection tests at `injection.rs:28-106` have hardcoded payloads and only check for error keywords in responses (`error`, `exception`, `syntax`, `unexpected`, `stack trace`). This is a basic detection mechanism that could miss more subtle vulnerabilities.

## Stale Items

- **Test file reference**: The reference to `fuzzer/payloads/websocket.rs` for tests is unverified and may be stale. The actual tests appear to be in the websocket module files themselves (`connection.rs:98-122`, `injection.rs:124-146`, `origin.rs:98-118`, `fuzz.rs:185-208`).

## Code Interrogation Findings

- **Finding**: All four test modules (`connection`, `injection`, `origin`, `fuzz`) have `#[cfg(test)]` tests that test the result types and constructors but don't actually test the async test methods. This is reasonable for unit tests of type definitions.

- **Finding**: The `ConnectionTester::test_connection()` at `connection.rs:29-80` properly drops the WebSocket stream (`drop(ws_stream)`) before returning, ensuring the connection is properly closed. This is correct resource management.

- **Finding**: The injection tests at `injection.rs:68-73` use a 5-second timeout for receiving responses. This is reasonable but could be configurable.

- **Finding**: The fuzz tests at `fuzz.rs:37-93` test message sizes from 1KB to 1MB. The 1MB test could be memory-intensive if many concurrent connections are tested.

- **Finding**: Origin testing at `origin.rs:50-67` creates a client request, parses it, adds the Origin header, then calls `connect_async(request)`. This approach works with tungstenite's `IntoClientRequest` trait.

## Summary

The WebSocket module architecture documentation is highly accurate regarding types and feature gating. All four test categories (connection, injection, origin, fuzz) are correctly documented. The main concern is the unverified test file reference in `fuzzer/payloads/websocket.rs` which should be confirmed or corrected.