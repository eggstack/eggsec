# Web Proxy Module (MITM Interception)

## Role & Responsibilities

Inbound MITM (Man-in-the-Middle) interception domain for authorized defense-lab security testing. Intercepts HTTP/HTTPS traffic with on-the-fly dynamic TLS certificate generation, configurable request/response interception rules, flow capture with budget controls, protocol-aware session recording (HTTP/1.1, HTTP/2, WebSocket, gRPC), evidence bundling with HMAC integrity, and cross-loadout correlation. Gated behind `web-proxy` feature.

The MITM proxy is the *interception* side; the *outbound* upstream-proxy pooling is in the `proxy/` facade (see [proxy.md](proxy.md)).

## Location & Feature Gating

| Component | Path | Feature Gate |
|-----------|------|-------------|
| Domain crate root | `crates/eggsec-web-proxy/src/lib.rs` | always |
| Config types | `crates/eggsec-web-proxy/src/config.rs` | always |
| Error types | `crates/eggsec-web-proxy/src/error.rs` | always |
| Proxy pool | `crates/eggsec-web-proxy/src/pool.rs` | always |
| Rotator | `crates/eggsec-web-proxy/src/rotator.rs` | always |
| Health checker | `crates/eggsec-web-proxy/src/health.rs` | always |
| SOCKS impl | `crates/eggsec-web-proxy/src/socks.rs` | always |
| HTTP CONNECT | `crates/eggsec-web-proxy/src/http_connect.rs` | always |
| Utilities | `crates/eggsec-web-proxy/src/utils.rs` | always |
| Intercept submodule | `crates/eggsec-web-proxy/src/intercept/mod.rs` | always (body gated) |
| Certificate generation | `crates/eggsec-web-proxy/src/intercept/cert.rs` | always |
| Intercept proxy engine | `crates/eggsec-web-proxy/src/intercept/interceptor.rs` | always |
| Rule engine | `crates/eggsec-web-proxy/src/intercept/rules.rs` | always |
| Core types | `crates/eggsec-web-proxy/src/intercept/types.rs` | always |
| Bridge to ScanReportData | `crates/eggsec-web-proxy/src/intercept/bridge.rs` | always |
| Evidence bundles | `crates/eggsec-web-proxy/src/intercept/bundle.rs` | always |
| Narrative generation | `crates/eggsec-web-proxy/src/intercept/narrative.rs` | always |
| Protocol types | `crates/eggsec-web-proxy/src/intercept/protocols.rs` | always (types); `web-proxy` (detection logic) |
| Correlation engine | `crates/eggsec-web-proxy/src/intercept/correlation.rs` | always |
| Plugin system | `crates/eggsec-web-proxy/src/intercept/plugins.rs` | always |
| Dynamic plugins | `crates/eggsec-web-proxy/src/intercept/dynamic_plugins.rs` | `dynamic-plugins` |
| Transparent proxy | `crates/eggsec-web-proxy/src/intercept/transparent.rs` | `transparent-proxy` (Linux) |
| Red team tests | `crates/eggsec-web-proxy/src/intercept/redteam.rs` | always |
| MCP tool types | `crates/eggsec-web-proxy/src/mcp.rs` | `web-proxy-mcp` |

### Feature flags (`Cargo.toml:93-106`)

| Feature | Effect | Dependencies |
|---------|--------|-------------|
| `web-proxy` | Core intercept + real protocol backends | `tokio-tungstenite`, `h2`, `http`, `prost`, `prost-types` |
| `web-proxy-mcp` | MCP tool schema types | implies `web-proxy` |
| `transparent-proxy` | iptables/nftables REDIRECT mode (Linux only) | implies `web-proxy` |
| `dynamic-plugins` | Shared-library plugin loading at runtime | implies `web-proxy` |

TLS dependencies use ring-only per workspace convention: `tokio-rustls = { default-features = false, features = ["ring", "tls12"] }`, `rustls = { default-features = false, features = ["ring", "std", "tls12"] }` (`Cargo.toml:22-23`). `reqwest` uses `features = ["rustls-no-provider"]` (`Cargo.toml:42`).

## Architecture

### File inventory (24 Rust source files, 10 root + 14 intercept)

#### Root files

| File | Lines | Key types |
|------|-------|-----------|
| `lib.rs` | 376 | `ProxyManager`, `ProxiedConnection`, `resolve_target()`, `is_private_ip()` |
| `config.rs` | 626 | `ProxyType` (5 variants), `ProxyEntry` (11 fields), `ProxyConfig`, `RotationStrategy` (5 variants), `HealthCheckConfig` |
| `error.rs` | 93 | `WebProxyError` (9 variants), `Result<T>` |
| `pool.rs` | 595 | `ProxyPool` (DashMap-backed), `ProxyStats`, `ProxyPoolBuilder` |
| `rotator.rs` | 418 | `ProxyRotator` — 5 rotation strategies |
| `health.rs` | 382 | `HealthChecker`, `HealthCheckResult`, `ProxyHealth` |
| `socks.rs` | 584 | `SocksProxy`, SOCKS4/4a/5 connection, `chain_connect()` |
| `http_connect.rs` | 336 | `HttpConnectProxy`, HTTP CONNECT tunnel |
| `utils.rs` | 61 | `ensure_rustls_provider()`, `create_insecure_client_with_options()`, `connect_with_nodelay_timeout()` |
| `mcp.rs` | — | `WebProxyToolSchema`, `WebProxyToolCall` (12 MCP tools) |

#### Intercept submodule

| File | Lines | Key types |
|------|-------|-----------|
| `mod.rs` | 1272 | `ProxyServer` (TCP listener), `handle_connection()`, `handle_connect_request()`, `handle_http_request()`, `handle_websocket_interception()`, `handle_http2_interception()`, `create_tls_acceptor()` |
| `cert.rs` | 180 | `CertGenerator` (Arc<RwLock<HashMap>> cache, 24h default), `CertMaterial` |
| `interceptor.rs` | 251 | `InterceptProxy`, `InterceptConfig`, `InterceptMode` (Monitor/Intercept/Allow), `InterceptRequest`, `InterceptResponse`, `InterceptEvent`, `InterceptDecision`, `RequestModification`, `ResponseModification`, `validate_header_value()` |
| `rules.rs` | 1532 | `InterceptRule`, `RuleSet`, `EnhancedRule` (id, name, condition, action, priority, modifications), `EnhancedRuleSet` (prefix-indexed, async eval), `RuleCondition` (14 variants), `RuleAction` (8 variants: Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag), `RuleContext`, `InjectResponseConfig` |
| `types.rs` | 1040 | `WebProxySessionReport` (20+ fields), `ProxyFlow` (15 fields), `BudgetUsage` (12 fields), `RedactionPattern`, `ManipulationRecord`, `FlowAction` (4 variants), `InterceptSession`, `FlowBuffer` (VecDeque O(1) eviction), `ProxyMetrics`, HAR 1.2 export types |
| `protocols.rs` | 1868 | `ProxyProtocol` (4 variants), `WebSocketSession`, `WebSocketMessage`, `WebSocketOpcode`, `Http2Session`, `Http2Stream`, `Http2StreamState`, `GrpcSession`, `GrpcCall`, `GrpcMethodType`, `GrpcStreamFrame`, `GrpcStreamingState`, `GrpcReflectionInfo`, `GrpcSecurityFinding`, `ProtocolDetection`, `detect_grpc_security_issues()` |
| `bridge.rs` | 493 | `to_scan_report_data_proxy()` — finding categories: `proxy-intercept-flow`, `proxy-websocket-session`, `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`, `proxy-manipulation-*`, `web-traffic-summary` |
| `correlation.rs` | — | `CorrelationEngine`, `CorrelationContext`, `CorrelationReference`, `CorrelationSource` (6 variants), `ConfidenceScorer`, `BehavioralPattern`, `TemporalCorrelation`, `CorrelationSummary` |
| `bundle.rs` | 779 | `EvidenceBundle` (version "2"), `BundleManifest`, HMAC-SHA256 signing, `compare_bundles()` → `BundleDiff` |
| `narrative.rs` | — | `AttackNarrative`, `NarrativeEvent`, `build_narrative()` |
| `plugins.rs` | — | `PluginRegistry`, `PluginSandbox`, `ProtocolHandler` trait, `PluginCapability`, `CapabilitySet`, `PluginFinding`, `PluginError` |
| `dynamic_plugins.rs` | — | `DynamicPluginRegistry` (shared-library loading, `dynamic-plugins` feature) |
| `transparent.rs` | — | `TransparentProxyConfig` (iptables/nftables REDIRECT, `transparent-proxy` feature, Linux only) |
| `redteam.rs` | — | 33 adversarial tests (CRLF injection, header smuggling, etc.) |

### Key types with file:line references

| Type | Location | Description |
|------|----------|-------------|
| `ProxyServer` | `intercept/mod.rs:71-78` | TCP listener with addr, cert_generator, rules, enhanced_rules, mode, proxy_http2_live |
| `CertGenerator` | `intercept/cert.rs:14-17` | Arc<RwLock<HashMap>> cache, validity_duration (default 86400s) |
| `CertMaterial` | `intercept/cert.rs:130-134` | cert_der + key_der (both `Vec<u8>`) |
| `InterceptProxy` | `intercept/interceptor.rs:43-48` | Config, rules, optional event/decision channels |
| `InterceptMode` | `intercept/interceptor.rs:15-22` | Monitor (default), Intercept, Allow |
| `RuleSet` | `intercept/rules.rs:613-616` | Vec<InterceptRule> with priority-sorted evaluation |
| `EnhancedRuleSet` | `intercept/rules.rs:217-223` | Rules + host/path prefix indices for fast lookup |
| `RuleCondition` | `intercept/rules.rs:51-67` | 14 variants: HostMatches, PathMatches, MethodMatches, HeaderContains, BodyContains, ProtocolIs, WebSocketOpcodeIs, GrpcMethodIs, And, Or, Not, BodySizeGt, BodySizeLt |
| `RuleAction` | `intercept/rules.rs:16-28` | 8 variants: Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag |
| `WebProxySessionReport` | `intercept/types.rs:101-153` | 20+ fields including flows, budget, protocol sessions, correlation |
| `ProxyFlow` | `intercept/types.rs:18-57` | 15 fields including method, url, host, path, headers, body, status, timing, protocol |
| `BudgetUsage` | `intercept/types.rs:64-98` | 12 fields: max_flows, flows_captured, max_bytes_per_flow, max_duration_secs, max_concurrent, peak_concurrent, protocol-specific counters |
| `FlowBuffer` | `intercept/types.rs:600-650` | VecDeque-backed capacity-capped buffer with O(1) eviction |
| `ProxyMetrics` | `intercept/types.rs:653-689` | flows_per_second, rule_eval_time_ms, memory_usage_bytes, active_connections |
| `EvidenceBundle` | `intercept/bundle.rs:18-38` | Version "2", manifest, flows, protocol sessions, rules, manipulations, correlations |
| `BundleManifest` | `intercept/bundle.rs:41-79` | Target, scope, timestamps, counts, HMAC-SHA256 signature fields |
| `BundleDiff` | `intercept/bundle.rs:307-328` | flows_added/removed/modified, manipulations/rules/correlations added/removed, manifest_changed |

## Behavior / Flow

### TLS MITM: CA/Cert Generation and Trust Requirements

1. `CertGenerator::generate_for_host(host)` (`cert.rs:45-53`): checks cache first, then calls `generate_cert()`.
2. `generate_cert(host)` (`cert.rs:55-91`):
   - Creates `CertificateParams` with host as SAN (`cert.rs:62`).
   - Sets `is_ca = IsCa::Ca(BasicConstraints::Constrained(0))` — intermediate CA with zero path length (`cert.rs:65`).
   - Key usages: `DigitalSignature` + `KeyEncipherment` (`cert.rs:67-70`).
   - Extended key usages: `ServerAuth` + `ClientAuth` (`cert.rs:72-75`).
   - Generates `KeyPair` with rcgen, creates self-signed cert, returns DER bytes (`cert.rs:77-85`).
3. `create_tls_acceptor(material)` (`intercept/mod.rs:983-996`):
   - Builds `ServerConfig` with `CertificateDer` and `PrivateKeyDer::Pkcs8`.
   - Sets ALPN to `[b"h2", b"http/1.1"]` for HTTP/2 negotiation (`intercept/mod.rs:993`).
4. **Trust requirement**: Clients must trust the ephemeral CA. The CA is per-session and not persisted. No system trust store modification.

### Protocol Upgrade Detection

| Protocol | Detection | Confidence | Location |
|----------|-----------|------------|----------|
| WebSocket | `Upgrade: websocket` header (case-insensitive) | 0.99 | `intercept/mod.rs:191-195` |
| HTTP/2 | ALPN `h2` negotiated in TLS | — | `intercept/mod.rs:819-820` |
| gRPC | `Content-Type: application/grpc*` | 0.95 | `protocols.rs:1272-1280` |
| HTTP/2 (pseudo) | `:scheme` pseudo-header present | 0.90 | `protocols.rs:1283-1292` |
| HTTP/1.1 | Default fallback | 1.0 | `protocols.rs:1295-1299` |

### Request / Response Capture into Evidence Bundles

1. Each connection produces `ProxyFlow` records (`types.rs:18-57`) indexed sequentially within the session.
2. `WebProxySessionReport` (`types.rs:101-153`) accumulates: flows, protocol sessions (`ws_sessions`, `http2_sessions`, `grpc_sessions`), manipulation audit trail, correlation context.
3. `EvidenceBundle::from_report()` (`bundle.rs:81-117`) packages all data into version "2" structure with `BundleManifest`.
4. `bundle.to_bytes()` (`bundle.rs:120-131`) serializes to JSON, gzips via flate2 with default compression.
5. `bundle.sign(key, key_id)` (`bundle.rs:182-199`) computes HMAC-SHA256 over canonical manifest string.
6. `compare_bundles(baseline, other)` (`bundle.rs:397-472`) produces `BundleDiff` comparing flows by index, counts for manipulations/rules/correlations.
7. `to_scan_report_data_proxy()` (`bridge.rs:10-190`) converts to `ScanReportData` with finding categories: `proxy-intercept-flow` (per flow), `proxy-websocket-session` (per WS session), `proxy-http2-session` (per H2 session), `proxy-grpc-session` (per gRPC session), `proxy-correlation-summary`, `proxy-manipulation-*` (per manipulation), `web-traffic-summary` (session metadata).

### Enhanced Rule Evaluation

`EnhancedRuleSet` (`rules.rs:217-459`) provides multiple evaluation strategies:

1. **Linear scan** (`evaluate()`, `rules.rs:270-275`): Filters all rules by enabled + condition match.
2. **Indexed evaluation** (`evaluate_indexed()`, `rules.rs:285-323`): Uses prefix indices for fast candidate selection, then applies full condition evaluation. Falls back to linear scan if no prefix matches.
3. **Async evaluation** (`evaluate_async()`, `rules.rs:329-341`): Offloads to `spawn_blocking` for CPU-intensive conditions.
4. **Indexed async** (`evaluate_indexed_async()`, `rules.rs:347-393`): Combines prefix indexing with async evaluation.
5. **Rebuild index** (`rebuild_index()`, `rules.rs:253-268`): Extracts host/path prefixes from `RuleCondition::HostMatches` and `RuleCondition::PathMatches` (including through `And`/`Or`/`Not` combinators).

Rules are sorted by priority (descending) on insertion (`rules.rs:237`). `evaluate_first()` returns the highest-priority matching rule (`rules.rs:277-279`).

### gRPC Security Detection

`detect_grpc_security_issues()` (`protocols.rs:1155-1241`) checks:
- Missing `Authorization` metadata on successful calls (severity 5).
- Request payloads > 10MB (severity 4).
- Non-zero gRPC status codes with named status mapping (severity 3; 6 for `PERMISSION_DENIED`/`UNAUTHENTICATED`).
- Sensitive path patterns (`/admin`, `/debug`, `/internal`, `/test`, `/swagger`) — first match only (severity 6).

### gRPC Streaming State Tracking

`GrpcStreamingState` (`protocols.rs:779-938`):
- Tracks `client_frames` and `server_frames` with flow control window management.
- `flow_control_window` defaults to 65535 (HTTP/2 spec default).
- `bytes_in_flight` tracks unacknowledged data; server responses reduce it.
- `prepare_window_update()` triggers at >50% window consumption.
- `is_complete()` checks end-of-stream frames per method type (unary always true, server-streaming requires server end, client-streaming requires client end, bidi requires both).
- `create_frame()` validates flow control before sending, returns `FlowControlError::WindowExceeded` if exceeded.

## Security Model

- **In-memory only**: Certificate cache (`cert.rs:15`), proxy pool stats (`pool.rs:52-56`), session data — no persistent state across restarts.
- **SensitiveString for credentials**: `ProxyEntry.password` is `Option<SensitiveString>` (`config.rs:67`); `to_log_key()` masks with `***` (`config.rs:149-157`); `expose_secret()` called only during actual connection (`socks.rs:356`, `http_connect.rs:220`, `health.rs:91`).
- **CRLF injection prevention**: `validate_header_value()` (`interceptor.rs:204-206`) rejects `\r`, `\n`, `\0` in header values. Applied in both `modify_request()` and `modify_response()`.
- **Bundle integrity**: HMAC-SHA256 signing with canonical string representation; verification via `verify()` (`bundle.rs:205-226`). Signed bundles include `signature`, `signed_at`, and `signing_key_id` fields.
- **Private-IP blocking**: `is_private_ip()` (`intercept/mod.rs:157-174`) blocks RFC 1918, loopback, link-local, unspecified addresses. Applied in `validate_target()` before upstream connection.
- **Budget enforcement**: `BudgetUsage` tracks flows, bytes, duration, concurrency, and protocol-specific counters against configured maximums.
- **HTTP response size limit**: `read_response()` (`http_connect.rs:118-181`) caps accumulated header bytes at 64KB to prevent hostile upstream proxy DoS.
- **Initial request timeout**: `handle_connection()` (`intercept/mod.rs:718-723`) uses 30s timeout for the initial request to prevent slowloris-style connection holding.
- **WebSocket pump termination**: After one direction closes, the sibling pump gets a 30s drain window before `abort()` (`intercept/mod.rs:366-384`).

## Public API

### ProxyServer (`intercept/mod.rs:71-155`)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(addr: SocketAddr) -> Result<Self>` | Creates server with default Monitor mode |
| `with_mode()` | `(self, mode: InterceptMode) -> Self` | Sets interception mode |
| `with_cert_generator()` | `(self, gen: CertGenerator) -> Self` | Custom cert generator |
| `with_proxy_http2_live()` | `(self, live: bool) -> Self` | Enable real HTTP/2 interception |
| `add_rule()` | `(&self, rule: InterceptRule)` | Add a basic rule |
| `add_enhanced_rule()` | `(&self, rule: EnhancedRule)` | Add an enhanced rule |
| `start()` | `async (&self) -> Result<()>` | Bind TCP listener and accept connections |

### InterceptProxy (`interceptor.rs:43-159`)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(config: InterceptConfig) -> Self` | Creates proxy with config |
| `with_rules()` | `(self, rules: RuleSet) -> Self` | Attach rule set |
| `with_event_channel()` | `(self, tx: Sender<InterceptEvent>) -> Self` | Attach event channel |
| `with_decision_channel()` | `(self, rx: Receiver<InterceptDecision>) -> Self` | Attach decision channel |
| `should_intercept()` | `(&self, host, path) -> bool` | Check if rule action is Intercept |
| `should_monitor()` | `(&self, host, path) -> bool` | Check if rule action is Monitor or Intercept |
| `modify_request()` | `(&self, request, modification)` | Apply request modification with CRLF validation |
| `modify_response()` | `(&self, response, modification)` | Apply response modification with CRLF validation |

### EvidenceBundle (`bundle.rs:18-249`)

| Method | Signature | Description |
|--------|-----------|-------------|
| `from_report()` | `(report, rules) -> Self` | Build from session report |
| `to_bytes()` | `(&self) -> Result<Vec<u8>>` | Serialize to gzipped JSON |
| `from_bytes()` | `(data: &[u8]) -> Result<Self>` | Deserialize from gzipped JSON |
| `to_session_report()` | `(&self) -> WebProxySessionReport` | Reconstruct session report |
| `sign()` | `(&mut self, key, key_id) -> Result<()>` | HMAC-SHA256 sign manifest |
| `verify()` | `(&self, key) -> Result<bool>` | Verify HMAC-SHA256 signature |

## Integration Points

- **Engine proxy facade**: Re-exported via `crates/eggsec/src/proxy/mod.rs` for engine modules (scanner, recon) that need upstream proxy routing.
- **CLI `proxy-intercept`**: `crates/eggsec/src/commands/handlers/web_proxy.rs` uses `handle_proxy_intercept` which constructs `ProxyServer` with rules, starts listening, and produces `WebProxySessionReport`.
- **CLI `report convert`**: Auto-bridges native `WebProxySessionReport` JSON to `ScanReportData` via `to_scan_report_data_proxy()` when `web-proxy` feature is enabled.
- **MCP proxy surface**: 12 tools via `web-proxy-mcp` marker feature: `proxy_list_flows`, `proxy_inspect_flow`, `proxy_edit_request`, `proxy_edit_response`, `proxy_manage_rules`, `proxy_session_save`, `proxy_session_load`, `proxy_har_export`, `proxy_evidence_bundle`, plus flow action tools.
- **TUI Intercept tab**: `Tab::Intercept` with live flow inspection, header/body editing, forward/drop/replay/pause, rules display, session management, HAR export, manipulation audit trail.
- **Defense-lab framing**: All interception requires `EnforcementContext` + `OperationRisk::TrafficInterception` under `DefenseLab` mode; `--allow-web-proxy` override for real runs; dry-run always safe.
- **Cross-loadout correlation**: `CorrelationContext` / `CorrelationReference` link proxy flows to findings from other loadouts (db-pentest, auth-test, mobile-dynamic, wireless).

## Testing

22 test-bearing files (highest density in the workspace). 14 files in `intercept/` alone have tests.

Key test areas:
- **Cert generation**: 3 tests (`cert.rs`) — generation, caching, material validation.
- **Intercept proxy**: 5 tests (`interceptor.rs`) — config defaults, intercept/monitor decisions, CRLF validation, null byte rejection.
- **Rule engine**: 30+ tests (`rules.rs`) — all 14 condition types, And/Or/Not nesting, enhanced rules, indexed evaluation (1000 rules <1ms/eval), file persistence, YAML parsing.
- **Protocols**: 40+ tests (`protocols.rs`) — WebSocket opcodes, session tracking, HTTP/2 streams/sessions, gRPC calls/streaming state, security detection (auth, large payloads, errors, sensitive paths), reflection parsing, flow control window management.
- **Bundle**: 10 tests (`bundle.rs`) — roundtrip, export/import, signing/verification, diff comparison, empty reports, WS/HTTP2/gRPC counts.
- **Bridge**: 9 tests (`bridge.rs`) — all finding categories, protocol session inclusion, correlation summary, roundtrip serialization, empty report structure.
- **Types**: 15 tests (`types.rs`) — ProxyFlow roundtrip, session report lifecycle, FlowBuffer eviction (O(1) VecDeque), ProxyMetrics recording.
- **Red team**: 33 adversarial tests (`redteam.rs`) — CRLF injection, header smuggling, input validation.

## Invariants & Gotchas

1. **Cert cache is ephemeral and per-instance**: Two independent `CertGenerator` instances have separate caches; a cloned instance shares via `Arc`. Cache is cleared on drop.
2. **HTTP/2 interception is opt-in**: `proxy_http2_live` defaults to `false` (`intercept/mod.rs:88`). Without it, HTTP/2 connections fall through to HTTP/1.1 handling.
3. **WebSocket interception fallback**: Without `web-proxy` feature, `handle_websocket_interception()` is a no-op passthrough (`intercept/mod.rs:399-410`).
4. **`FlowBuffer::flows()` returns empty slice**: Due to `VecDeque` non-contiguity, `flows()` always returns `&[]`; callers must use `flows_vec()` or `iter()` (`types.rs:623-641`).
5. **`is_private_ip` inconsistency**: The intercept version (`intercept/mod.rs:157-174`) does NOT block multicast/broadcast; the outbound version (`lib.rs:336-356`) does.
6. **`handle_http_request()` always returns 400**: Non-CONNECT HTTP requests after rule evaluation always return `400 Bad Request` (`intercept/mod.rs:951-953`). This is a stub for Phase 2.
7. **gRPC method type detection is heuristic**: `detect_grpc_method_type()` (`protocols.rs:1303-1321`) infers streaming from `TE: trailers` or `grpc-encoding` headers — not definitive without deep inspection.
8. **Bundle canonical string for signing**: Fixed field order (`bundle.rs:232-248`); changes to field set require version bump.

## References

- [overview.md](overview.md) — workspace ownership and module index
- [defense_lab.md](defense_lab.md) — defense-lab surface patterns
- [dispatch.md](dispatch.md) — runtime dispatch flow
- [websocket.md](websocket.md) — WebSocket protocol support
- [proxy.md](proxy.md) — outbound upstream-proxy pooling
- `crates/eggsec-web-proxy/` — domain crate source
- `crates/eggsec/src/proxy/AGENTS.override.md` — module-specific agent guidance

*Last verified against source: 2026-08-25*
