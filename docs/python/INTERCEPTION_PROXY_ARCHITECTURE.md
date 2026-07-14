# Interception Proxy Architecture

## Overview

The eggsec interception proxy enables man-in-the-middle (MITM) inspection of HTTP/HTTPS traffic for security testing. It is implemented in the `eggsec-web-proxy` Rust crate and exposed to Python via the `eggsec-python` bindings (feature: `web-proxy`).

## Listener Lifecycle

1. **Bind**: The proxy binds a TCP listener on `listen_addr:listen_port` (default `127.0.0.1:8080`).
2. **Accept**: Incoming connections are accepted and dispatched to handler tasks.
3. **CONNECT handling**: For HTTPS, the proxy reads the HTTP `CONNECT` method to determine the target host/port, then establishes a TLS tunnel.
4. **TLS interception**: When `ssl_intercept` is enabled, the proxy dynamically generates certificates per hostname using `CertGenerator` and terminates TLS on behalf of the client.
5. **Upstream connection**: The proxy opens a separate connection to the upstream server and relays traffic.
6. **Shutdown**: The listener stops on timeout (`timeout_secs`), max flows (`max_flows`), or explicit stop.

## TLS Interception and Certificate Issuance

- `CertGenerator` maintains an in-memory cache of generated certificates keyed by hostname.
- Certificates are self-signed with the CA's key, valid for 24 hours by default.
- Each generated `CertMaterial` contains DER-encoded certificate and private key.
- The CA fingerprint is recorded in session reports for verification.
- Custom CA certificates can be provided via `CertificateAuthorityConfigPy`.

## Request/Response Capture

Every proxied exchange produces a `ProxyFlow` containing:
- HTTP method, URL, host, path
- Request/response headers (HashMap)
- Request/response bodies (optional, truncated)
- HTTPS flag, protocol version
- Timing: `started_at`, `completed_at`, `duration_ms`
- Body sizes (before truncation)
- Redaction notes (if applied)

Flows are collected in a `FlowBuffer` (LRU eviction, configurable capacity) and summarized in `WebProxySessionReport`.

## WebSocket Upgrades

WebSocket traffic is detected during the CONNECT tunnel phase. Captured WebSocket sessions are stored as `WebSocketSession` objects in the session report, including individual `WebSocketMessage` frames with opcode, payload, and direction.

## HTTP/2 and gRPC

- HTTP/2 streams are tracked as `Http2Stream` within `Http2Session`.
- gRPC calls are decoded from HTTP/2 frames into `GrpcCall` with method, fields, and streaming state.
- Both are included in the session report for comprehensive protocol analysis.

## Mutation Hooks

Rules can modify requests and responses in flight:
- `RequestModification`: add/remove/replace headers, modify body
- `ResponseModification`: add/remove/replace headers, modify body, inject response
- `InjectResponseConfig`: return a synthetic response with custom status, headers, and body
- Manipulations are recorded in `ManipulationRecord` for audit trail

## Replay

The `InterceptSession` supports flow replay:
- `FlowAction::Replay` re-sends the original unmodified request
- `FlowAction::Forward` sends the (possibly modified) request
- `FlowAction::Drop` discards the request
- `FlowAction::Paused` holds the flow for operator inspection

## HAR Export

`InterceptSession::to_har()` converts captured flows to HAR 1.2 format:
- `HarExport` → `HarLog` → `Vec<HarEntry>`
- Each entry includes `HarRequest`, `HarResponse`, `HarCache`, `HarTimings`
- Headers are stored as `HarNameValuePair` (name/value pairs)
- Content includes size, MIME type, and optional text body

## Evidence Bundles

For multi-session correlation:
- `EvidenceBundle` packages flows, manipulations, and metadata
- `BundleManifest` provides a manifest of included artifacts
- `CorrelationContext` links findings across sessions
- `compare_bundles()` diffs two bundles for regression testing

## Limits and Budgets

`BudgetUsage` tracks resource consumption:
- `max_flows` / `flows_captured`: total flow count
- `max_bytes_per_flow`: body truncation threshold
- `max_duration_secs` / `elapsed_secs`: session time limit
- `max_concurrent` / `peak_concurrent`: connection concurrency
- Protocol-specific: `max_ws_messages`, `max_http2_streams`, `max_grpc_calls`

## CLI Assumptions

- The proxy is designed for interactive CLI/TUI use (`ManualPermissive` enforcement).
- Real interception requires `--allow-web-proxy` flag and policy confirmation.
- Dry-run mode is always safe (no upstream connections).
- REST/MCP surfaces use `McpStrict` enforcement with explicit scope.

## Python API Surface

### Existing Types (Release 1)

| Type | Purpose |
|------|---------|
| `ProxyTypePy` | Proxy protocol enum (Socks4/5, HTTP, HTTPS, Tor) |
| `RotationStrategyPy` | Pool rotation strategy |
| `ProxyConfigPy` | Pool management configuration |
| `ProxyEntryPy` | Individual proxy entry |
| `ProxyManagerPy` | Pool manager with health checking |
| `HealthCheckResultPy` | Per-proxy health result |
| `ProxyHealthPy` | Aggregated pool health |
| `InterceptConfigPy` | Intercept session configuration |
| `CapturedExchangePy` | Single request/response exchange |
| `InterceptSessionResultPy` | Session result with all exchanges |

### Existing Functions (Release 1)

| Function | Purpose |
|----------|---------|
| `create_proxy_manager` | Create a proxy pool manager |
| `async_add_proxy` | Add proxy to pool (async) |
| `async_proxy_health_check` | Check pool health (async) |

### New Types (Release 3)

| Type | Purpose |
|------|---------|
| `InterceptSessionStatePy` | Session lifecycle state enum |
| `InterceptStatsPy` | Session statistics snapshot |
| `InterceptFilterPy` | Traffic filtering configuration |
| `InterceptRulePy` | Simplified interception rule |
| `CertificateAuthorityConfigPy` | CA certificate configuration |
| `IssuedCertificatePy` | Issued certificate metadata |
| `HarEntryPy` | Single HAR 1.2 entry |
| `HarDocumentPy` | Complete HAR 1.2 document |

### New Functions (Release 3)

| Function | Purpose |
|----------|---------|
| `run_intercept_session` | Run an interception proxy session (sync) |
| `async_run_intercept_session` | Run an interception proxy session (async) |
