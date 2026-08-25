# WebSocket Module

## Role & Responsibilities

WebSocket security testing including connection validation, message injection, cross-site WebSocket hijacking (CSWSH), origin validation, DoS resilience, and frame/message fuzzing.

**Non-responsibilities**: Does not perform WebSocket-based chat or application-level protocol testing. Does not proxy or intercept WebSocket traffic (that is the `eggsec-web-proxy` domain). Does not integrate with MCP/agent/pipeline surfaces.

## Location & Feature Gating

| Item | Location | Gate |
|------|----------|------|
| Module declaration | `crates/eggsec/src/lib.rs:153-154` | `#[cfg(feature = "websocket")]` |
| No stub module | — | When disabled, the module simply does not exist |
| `WebSocketTestConfig` | `websocket/mod.rs:34-44` | `#[cfg(feature = "websocket")]` |
| `run_live_tests()` | `websocket/mod.rs:46-79` | `#[cfg(feature = "websocket")]` |
| `test_connection()` | `websocket/connection.rs:14` | `#[cfg(feature = "websocket")]` |
| `test_injection()` | `websocket/injection.rs:13` | `#[cfg(feature = "websocket")]` |
| `test_origins()` | `websocket/origin.rs:11` | `#[cfg(feature = "websocket")]` |
| `test_dos()`, `test_message_fuzz()` | `websocket/fuzz.rs:14`, `:26` | `#[cfg(feature = "websocket")]` |
| `tokio-tungstenite` dep | `crates/eggsec/Cargo.toml:155-158` | `version = "0.26"`, `features = ["rustls-tls-native-roots"]`, optional |
| Feature flag | `crates/eggsec/Cargo.toml:333` | `websocket = ["dep:tokio-tungstenite"]` |

**Important asymmetry note**: The task description claims `WebSocketTestReport`/`WebSocketFinding` are NOT cfg-gated and always available. This is **incorrect** per source: the entire `websocket` module is gated at `lib.rs:153-154` with no `#[cfg(not(...))]` stub. When the `websocket` feature is disabled, none of these types exist in the public API.

The `WebSocketTestReport`, `WebSocketFinding`, `ConnectionTestResult`, `InjectionTestResult`, `OriginTestResult`, and `FuzzTestResult` structs are all defined inside the feature-gated module and therefore only available with `--features websocket`. The only always-available WebSocket types live in the fuzzer (`fuzzer/payloads/websocket.rs`), which is not gated.

## Architecture

### Files (5 total)

| File | Lines | Description |
|------|-------|-------------|
| `websocket/mod.rs` | 214 | `WebSocketTestReport`, `WebSocketFinding`, `WebSocketTestConfig`, `run_live_tests()`, `run_live_tests_inner()` |
| `websocket/connection.rs` | 137 | `ConnectionTestResult`, `test_connection()` with real WS connect + close |
| `websocket/injection.rs` | 337 | `InjectionTestResult`, `test_injection()`, `test_single_injection()`, `detect_injection_vulnerability()` |
| `websocket/origin.rs` | 134 | `OriginTestResult`, `test_origins()`, `test_single_origin()` with malicious origin testing |
| `websocket/fuzz.rs` | 440 | `FuzzTestResult`, `test_dos()`, `test_message_fuzz()`, `test_large_message()`, `test_ping_flood()`, `test_rapid_close()`, `test_single_message_fuzz()` |
| `fuzzer/payloads/websocket.rs` | 485 | `WebSocketFuzzer`, `WebSocketVulnerability` (7 variants), `WebSocketTestResult`, `get_payloads()` (14 payloads) — **always compiled** |

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WebSocketTestReport` | `mod.rs:16` | `target`, `connection_test` (Option), `injection_tests`, `origin_tests`, `fuzz_tests`, `findings` |
| `WebSocketFinding` | `mod.rs:25` | `category`, `severity`, `title`, `description`, `recommendation` |
| `WebSocketTestConfig` | `mod.rs:35` | `url`, `timeout_secs`, `injection_payloads`, `test_connection`, `test_origins`, `test_injection`, `test_dos`, `test_message_fuzz` |
| `ConnectionTestResult` | `connection.rs:4` | `url`, `connected`, `response_headers`, `subprotocols`, `extensions`, `latency_ms`, `error` |
| `InjectionTestResult` | `injection.rs:4` | `payload`, `sent`, `received_response`, `response_content`, `vulnerability_detected`, `details` |
| `OriginTestResult` | `origin.rs:4` | `origin`, `accepted`, `status_code`, `details` |
| `FuzzTestResult` | `fuzz.rs:4` | `test_name`, `payload_size`, `sent`, `connection_dropped`, `server_response`, `vulnerability_detected`, `details` |
| `WebSocketFuzzer` | `fuzzer/payloads/websocket.rs:41` | `url`, `subprotocols` — local payload generation (always compiled) |
| `WebSocketVulnerability` | `fuzzer/payloads/websocket.rs:7` | 7 variants: `Injection`, `DoS`, `CrossSiteWebSocketHijacking`, `OriginBypass`, `MessageFuzzing`, `FrameFuzzing`, `AuthBypass` |

### Variant Counts

| Enum | Variants | Source |
|------|----------|--------|
| `WebSocketVulnerability` | 7 | `fuzzer/payloads/websocket.rs:7-15` |

## Behavior / Flow

### `run_live_tests(config)` — `mod.rs:47-79`

Global timeout: `config.timeout_secs * 10` seconds. Wraps `run_live_tests_inner()` in `tokio::time::timeout()`. On timeout, returns a report with a single `Timeout` finding at `Severity::Medium`.

### `run_live_tests_inner(config)` — `mod.rs:82-197`

Sequential execution of test categories based on config flags:

1. **Connection test** (`:88-105`): If `test_connection`, calls `connection::test_connection()`. Adds `Connection` finding on failure.
2. **Origin tests** (`:107-127`): If `test_origins`, calls `origin::test_origins()` with 4 malicious origins (`evil.com`, `localhost`, `null`, `target.com.evil.com`). Adds `CSWSH` finding for each accepted origin.
3. **Injection tests** (`:129-152`): If `test_injection` and payloads non-empty, calls `injection::test_injection()`. Adds `Injection` finding for each detected vulnerability.
4. **DoS tests** (`:154-169`): If `test_dos`, calls `fuzz::test_dos()` (large message, ping flood, rapid close). Adds `DoS` finding for each detected vulnerability.
5. **Message fuzzing** (`:171-187`): If `test_message_fuzz`, calls `fuzz::test_message_fuzz()` (7 fuzz cases: empty, null bytes, control chars, template-like, XSS-like, SQLi-like, empty object). Adds `Message Fuzzing` finding for each.

### Vulnerability Detection

**Injection** (`injection.rs:163-233`): `detect_injection_vulnerability()` checks response for SQL error indicators (10 patterns when payload contains `'`), XSS reflection (payload `<script>` reflected), Java/Python exceptions (8 patterns), path traversal indicators (7 patterns when payload contains `../`).

**DoS resilience** (`fuzz.rs:49-298`): Tests 65KB large message, 50 rapid pings (100ms intervals), 10 rapid close frames. Detects connection drops and error responses.

**Message fuzz** (`fuzz.rs:300-421`): 7 fuzz cases. Detects server error responses (12 patterns: "unhandled exception", "internal server error", stack traces, etc.).

### Fuzzer Integration — `fuzzer/payloads/websocket.rs`

`WebSocketFuzzer` (always compiled) generates local test cases across 6 categories:
- `generate_injection_tests()`: 8 payloads (SQLi, XSS, SSTI, JNDI, path traversal, prototype pollution, MSSQL injection, SQLi bypass)
- `generate_dos_tests()`: 6 payloads (large ping, large text, large binary, rapid close, rapid ping, message flood)
- `generate_cswsh_tests()`: 4 malicious origins
- `generate_message_fuzz_tests()`: 16 fuzz cases (empty, null, undefined, booleans, JSON variants, null bytes, control chars, etc.)
- `generate_frame_fuzz_tests()`: 7 frame-level tests (continuation, text/binary with FIN=0, close with status, ping, pong, fragmented with nulls)
- `generate_subprotocol_tests()`: 4 common subprotocols + configured subprotocol validation

`get_payloads()` returns 14 `Payload` entries with tags (`websocket`, `injection`, `xss`, `ssti`, `jndi`, `dos`, `cswsh`, `fuzzing`, `frame-fuzz`, `subprotocol`).

## Public API

| Function | Signature | Description |
|----------|-----------|-------------|
| `run_live_tests` | `pub async fn run_live_tests(config: &WebSocketTestConfig) -> WebSocketTestReport` | Main entry point for live testing |
| `test_connection` | `pub async fn test_connection(url: &str, timeout_secs: u64) -> ConnectionTestResult` | Connection + header capture |
| `test_injection` | `pub async fn test_injection(url: &str, payloads: &[String], timeout_secs: u64) -> Vec<InjectionTestResult>` | Message injection testing |
| `test_origins` | `pub async fn test_origins(url: &str, timeout_secs: u64) -> Vec<OriginTestResult>` | Origin validation testing |
| `test_dos` | `pub async fn test_dos(url: &str, timeout_secs: u64) -> Vec<FuzzTestResult>` | DoS resilience testing |
| `test_message_fuzz` | `pub async fn test_message_fuzz(url: &str, timeout_secs: u64) -> Vec<FuzzTestResult>` | Message fuzzing |

## Integration Points

### Fuzzer (`fuzzer/payloads/websocket.rs`)

`WebSocketFuzzer` integrates via the `AdvancedFuzzer` trait in `fuzzer/advanced.rs`. When invoked via `eggsec fuzz <url> -t websocket`, it:
1. Generates local test cases (always available)
2. When `websocket` feature is enabled, runs live connection tests via `run_live_tests()`

### CLI

No dedicated `WebSocket` CLI command. WebSocket testing is accessed through the fuzzer: `eggsec fuzz <url> -t websocket`.

### No MCP/Agent/Pipeline Integration

The websocket module is standalone. It does not register as an MCP tool and is not wired into the dispatch layer.

## Testing

### Always-compiled tests (`fuzzer/payloads/websocket.rs`)
7 tests: `test_get_payloads_returns_non_empty`, `test_get_payloads_count_reasonable`, `test_payloads_are_non_empty_strings`, `test_payloads_contain_expected_patterns`, `test_subprotocol_tests_generation`, `test_subprotocol_tests_empty_when_no_protocols`, `test_all_tests_includes_subprotocol`, `minimum_payload_count`

### Feature-gated tests
- `mod.rs:1`: `test_finding_creation`
- `connection.rs:1`: `test_connection_result_creation`
- `injection.rs:1`: `test_injection_result_creation` + 8 additional detection logic tests
- `origin.rs:1`: `test_origin_result_creation`
- `fuzz.rs:1`: `test_fuzz_result_creation`

## Invariants & Gotchas

1. **No stub module**: Unlike `browser`, there is no `#[cfg(not(...))]` fallback. When `websocket` is disabled, the types simply don't exist. Callers that reference `WebSocketTestReport` must be gated.
2. **Global timeout**: `run_live_tests()` applies `timeout_secs * 10` as a global ceiling (`mod.rs:50`).
3. **Per-operation timeouts**: Each connection/send/receive uses `timeout_secs` as an individual deadline.
4. **Close frame cleanup**: `test_connection()` and `test_single_origin()` send a close frame before returning (`connection.rs:66-76`, `origin.rs:72-82`). This prevents resource leaks.
5. **Injection detection false positives**: `detect_injection_vulnerability()` checks for exact phrases ("syntax error", "unhandled exception") rather than generic substrings, reducing false positives.
6. **Ping flood is bounded**: 50 pings at 100ms intervals (`fuzz.rs:169`), not unbounded.

## Bugs / Observations

| Location | Issue | Severity |
|----------|-------|----------|
| `injection.rs:235-236` | Test module gated `#[cfg(feature = "websocket")]` — tests for `detect_injection_vulnerability()` (a pure function) are behind the feature gate even though the function itself is also gated. This means unit tests for detection logic can't run without the feature | Low |
| `fuzz.rs:97` | `test_large_message()` sleeps 500ms before checking for server response — arbitrary delay, not timeout-based | Low |
| `mod.rs:50` | Global timeout is `timeout_secs * 10` — if `timeout_secs` is 0, the global timeout is also 0, causing immediate timeout | Medium |
| `origin.rs:18-20` | Malicious origins are hardcoded (4 values). Not configurable via `WebSocketTestConfig` | Informational |

*Last verified against source: 2026-08-25*
