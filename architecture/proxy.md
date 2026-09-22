# Proxy Module

## Role & Responsibilities

Outbound upstream-proxy pooling for engine modules. Provides connection routing through SOCKS4/5, HTTP CONNECT, HTTPS CONNECT, and Tor proxies with health checking, rotation strategies, chain proxying, and private-IP blocking. Production dial execution runs on the listener-free `eggress-outbound 1.0.8` engine via `eggress_outbound.rs` (Eggsec owns selection/rotation/health/policy; Eggress executes the selected route; Reqwest remains the explicit health-only owner). The MITM intercepting proxy is in the `eggsec-web-proxy` domain crate (see [web_proxy.md](web_proxy.md)).

The module spans two crates with a clean adapter/domain separation:

```
crates/eggsec/src/proxy/mod.rs          ← adapter layer (re-exports + stubs)
crates/eggsec-web-proxy/src/            ← domain crate (full implementation)
```

## Location & Feature Gating

| Component | Crate | Path | Feature Gate |
|-----------|-------|------|-------------|
| Adapter layer | `eggsec` | `crates/eggsec/src/proxy/mod.rs` | always (stubs without `web-proxy`) |
| Domain crate root | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/lib.rs` | `web-proxy` |
| Proxy pool | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/pool.rs` | `web-proxy` |
| Rotation strategies | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/rotator.rs` | `web-proxy` |
| Health checking | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/health.rs` | `web-proxy` |
| Eggress adapter | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/eggress_outbound.rs` | `web-proxy` |
| SOCKS4/5 impl | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/socks.rs` | `web-proxy` |
| HTTP CONNECT impl | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/http_connect.rs` | `web-proxy` |
| Config types | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/config.rs` | `web-proxy` |
| Error types | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/error.rs` | `web-proxy` |
| Utilities | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/utils.rs` | `web-proxy` |
| Intercept submodule | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/intercept/` | `web-proxy` |
| MCP tool types | `eggsec-web-proxy` | `crates/eggsec-web-proxy/src/mcp.rs` | `web-proxy-mcp` |

When `feature = "web-proxy"` is **enabled**: the adapter re-exports everything from `eggsec-web-proxy` — `pub use eggsec_web_proxy::*` plus named re-exports (`mod.rs:7-15`).

When `feature = "web-proxy"` is **disabled**: provides stub/no-op types so downstream code compiles without the feature. Stubs return empty results or errors indicating the feature is unavailable.

## Architecture

### Adapter Layer (`crates/eggsec/src/proxy/mod.rs`, 288 lines)

When `web-proxy` is disabled, the adapter provides minimal stubs:

| Stub Type | Location | Behavior |
|-----------|----------|----------|
| `ProxyType` (5 variants) | `mod.rs:71-84` | Always has `Socks4`, `Socks5`, `Http`, `Https`, `Tor` variants |
| `ProxyEntry` | `mod.rs:118-184` | All fields present; `load_from_file()` returns `Ok(Vec::new())` |
| `HealthCheckConfig` | `mod.rs:206-228` | All fields present; `test_url` defaults to `""` |
| `ProxyManager` | `mod.rs:187-203` | `new()` returns `Ok(Self)`; `get_random_proxy()` returns `None`; `get_all_healthy_proxies()` returns empty `Vec` |
| `HealthChecker` | `mod.rs:231-254` | `check()` always returns `is_healthy: false` with error message; `check_all()` returns zero counts |
| `ProxyPool`, `ProxyRotator`, `ProxiedConnection`, `ProxyConfig` | `mod.rs:275-288` | Empty structs |
| `intercept::types` stubs | `mod.rs:19-68` | `WebProxySessionReport`, `ProxyFlow`, `ProxyFlowDirection`, `BudgetUsage` with minimal fields; `to_scan_report_data_proxy()` returns `serde_json::json!({})` |
| `intercept::correlation::CorrelationId` | `mod.rs:58-60` | Empty struct |
| `intercept::protocols::ProtocolDetection` | `mod.rs:61-63` | Empty struct |

### Domain Crate (`crates/eggsec-web-proxy/src/`, 24 Rust source files)

Standalone defense-lab surface for HTTP/HTTPS traffic interception, proxy pool management, and MITM security testing. Owns all domain logic, types, and tests but does NOT decide whether an operation is allowed — enforcement stays in the main `eggsec` crate.

#### Root files (10 files)

| File | Lines | Description |
|------|-------|-------------|
| `lib.rs` | 421 | `ProxyManager`, `ProxiedConnection`, connection logic, private-IP blocking, `is_private_ip()` |
| `config.rs` | 626 | `ProxyConfig`, `ProxyEntry`, `ProxyType`, `RotationStrategy`, `HealthCheckConfig`, file loading (JSON/YAML/plaintext) |
| `error.rs` | 93 | `WebProxyError` enum (9 variants: `Proxy`, `Network`, `Config`, `Io`, `Tls`, `Intercept`, `Rule`, `Protocol`, `Timeout`) and `Result<T>` type alias |
| `pool.rs` | 595 | `ProxyPool` (DashMap-backed), `ProxyStats`, `ProxyPoolBuilder` |
| `rotator.rs` | 418 | `ProxyRotator` — round-robin, random, weighted, least-used, lowest-latency strategies |
| `health.rs` | 416 | `HealthChecker` (config-only clone, SOCKS4 fail-closed, bounded `buffered` preserving enabled-input order), `HealthCheckResult`, `ProxyHealth` |
| `eggress_outbound.rs` | 341 | Eggress adapter: literal-gated `ProxyEntry`→`ProxyHopSpec` (`socket_addr()` validation, hostname entries rejected), chain spec, `OutboundConnector::from_chain`, centralized unknown-`local_addr` sentinel, redacted error mapping |
| `socks.rs` | 604 | Production `connect_through`/`connect_through_tor` delegate to Eggress; `SocksProxy`/handshake/`chain_connect`/`connect_through_with_domain` retained as `TcpStream` compatibility shims |
| `http_connect.rs` | 344 | Production `connect_through` delegates to Eggress; `HttpConnectProxy` framing retained as compatibility shim |
| `utils.rs` | 61 | `ensure_rustls_provider()`, `create_insecure_client_with_options()`, `connect_with_nodelay_timeout()` |
| `mcp.rs` | — | MCP/Agent tool registration (gated behind `web-proxy-mcp` feature) |

#### Intercept submodule (`crates/eggsec-web-proxy/src/intercept/`, 14 files)

| File | Lines | Description |
|------|-------|-------------|
| `mod.rs` | 1272 | `ProxyServer` TCP listener, `handle_connection()`, CONNECT/HTTP dispatch, TLS termination, WebSocket/HTTP2 dispatch, private IP validation |
| `cert.rs` | 180 | `CertGenerator` (per-host cache, 24h validity), `CertMaterial` (cert_der, key_der), rcgen on-the-fly CA generation |
| `interceptor.rs` | 251 | `InterceptProxy`, `InterceptConfig`, `InterceptMode` (Monitor/Intercept/Allow), request/response modification with CRLF validation |
| `rules.rs` | 1532 | `InterceptRule`, `RuleSet`, `EnhancedRule`, `EnhancedRuleSet`, `RuleCondition` (And/Or/Not/HostMatches/PathMatches/BodyContains etc.), `RuleAction` (8 variants), `InjectResponseConfig`, indexed prefix evaluation, async evaluation |
| `types.rs` | 1040 | `WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `RedactionPattern`, `ManipulationRecord`, `FlowAction`, `InterceptSession`, `FlowBuffer` (VecDeque O(1) eviction), `ProxyMetrics`, HAR export types |
| `protocols.rs` | 1868 | `WebSocketSession`, `WebSocketMessage`, `Http2Session`, `Http2Stream`, `GrpcSession`, `GrpcCall`, `GrpcStreamFrame`, `GrpcStreamingState`, `GrpcReflectionInfo`, `ProtocolDetection`, `detect_grpc_security_issues()` |
| `bridge.rs` | 493 | `to_scan_report_data_proxy()` — converts `WebProxySessionReport` to `ScanReportData` |
| `correlation.rs` | — | `CorrelationEngine`, `CorrelationContext`, `CorrelationReference`, `CorrelationSource`, `ConfidenceScorer`, `BehavioralPattern`, `TemporalCorrelation` |
| `bundle.rs` | 779 | `EvidenceBundle`, `BundleManifest`, gzip JSON archive, HMAC-SHA256 signing/verification, `compare_bundles()` diff |
| `narrative.rs` | — | `AttackNarrative`, `NarrativeEvent`, `build_narrative()` |
| `plugins.rs` | — | `PluginRegistry`, `PluginSandbox`, `ProtocolHandler` trait, `PluginCapability`, capability-based sandbox |
| `dynamic_plugins.rs` | — | `DynamicPluginRegistry` — shared-library plugin loading (gated behind `dynamic-plugins` feature) |
| `transparent.rs` | — | `TransparentProxyConfig` — iptables/nftables REDIRECT mode (gated behind `transparent-proxy`, Linux only) |
| `redteam.rs` | — | Adversarial security tests for proxy inputs (CRLF injection, header smuggling, etc.) |

## Behavior / Flow

### Health Checking / Rotation / Failover Cycle

Health is application-level HTTP(S)-through-proxy validation (2xx via the
configured `test_url`), never tunnel-only success. Reqwest is the explicit
health-only owner (Phase B Option B); production dialing is Eggress-backed.

1. **Background health check** (`lib.rs`): `start_background_health_check(interval_secs)` spawns a `tokio::spawn` loop that calls `check_concurrent()` with bounded concurrency (default 10).
2. **Concurrent checks** (`health.rs`): `check_concurrent()` uses `buffered(concurrency)` — O(concurrency) in-flight, one result per enabled proxy, results in enabled-input order, no spawn-per-proxy JoinHandle retention. Each proxy gets an independent reqwest client with the proxy's auth credentials.
3. **Result processing** (`lib.rs`): Healthy proxies call `pool.mark_healthy()` (resets `consecutive_failures` to 0); unhealthy call `pool.mark_unhealthy()`.
4. **Automatic demotion** (`pool.rs:177-194`): `record_failure()` increments `consecutive_failures`; when it reaches `config.max_failures_before_disable` (default 3), `is_healthy` is set to `false`.
5. **Selection** (`lib.rs`): `get_next_proxy()` and `get_healthy_proxy()` use `ProxyRotator::select_with_stats()` which queries pool stats for `LeastUsed` and `LowestLatency` strategies.
6. **Priority fallback** (`lib.rs`): `get_highest_priority_proxy(min_priority)` selects from highest-priority proxies; if none match, falls back to `get_healthy_proxy()`.
7. **Protocol disposition**: `Socks5`/`Tor`→SOCKS5, `Http`→HTTP CONNECT, `Https`→plaintext CONNECT (faithful to production dial naming debt) are validated; `Socks4` fails closed with an explicit unsupported error (Reqwest cannot represent SOCKS4 — testing SOCKS5 instead would be false equivalence; behavior fix with fixture, `tests/health_matrix.rs`).

### Connection Establishment

Production execution is Eggress-backed (`eggress_outbound.rs`); Eggsec
owns resolution/selection, Eggress executes the selected route (no direct
fallback, redacted errors, aggregate timeout, future-drop cancellation).

**Single-hop** (`lib.rs`):
- `create_connection(target)` resolves target (private-IP blocked), selects healthy proxy, passes the resolved IP literal + port to Eggress with the proxy's `timeout_ms`.
- Protocol mapping: `Socks4`→Socks4, `Socks5`/`Tor`→Socks5, `Http`/`Https`→Http plaintext CONNECT (`Https` naming debt: `tls=false` until a fixture proves TLS-to-proxy).
- Proxy hop endpoints are literal-address-only: `hop_from_entry()` validates each entry through `ProxyEntry::socket_addr()` and builds the Eggress endpoint from the IP literal (corrective pass 2026-09-22; hostname-valued proxy endpoints fail closed before any network behavior). SOCKS5/Tor remote-domain *targets* remain supported as a separate concern.
- `ProxiedConnection.local_addr` is the centralized unknown sentinel (`eggress_outbound::unknown_local_addr()`, `0.0.0.0:0`): `eggress-outbound 1.0.8` always reports `local_addr=None` (upstream-gated debt; no policy/routing/evidence path reads it, and it is never logged as a measured address). Removal condition: a published Eggress release exposes the established-socket local address.

**Remote-domain path**: `create_connection_to_domain(domain, port)` preserves SOCKS5/Tor remote-domain semantics (domain reaches the proxy; SOCKS4/HTTP fail closed as before).

**Chain proxying** (`lib.rs`):
- `create_chained_connection(target, chain_length)` selects `chain_length` healthy proxies via `rotator.select_chain()`.
- Chains > 1 hop still require all SOCKS5/Tor entries (public contract preserved; mixed chains exercised only internally by the Eggress parity fixture, never exposed).
- Aggregate timeout is the max per-hop `timeout_ms` around the complete Eggress establishment.

### CONNECT Tunnel Establishment (Intercept)

`handle_connect_request()` (`intercept/mod.rs:742-864`):
1. Parse host:port from CONNECT line.
2. Validate target via `validate_target()` (private-IP blocking).
3. Evaluate `RuleSet` — `Block` returns `403 Forbidden`.
4. Connect to upstream with 30s timeout.
5. Generate per-host TLS certificate via `CertGenerator`.
6. Create TLS acceptor with ALPN `[h2, http/1.1]`.
7. Accept TLS from client (30s timeout).
8. If ALPN negotiated `h2` and `proxy_http2_live` is true → `handle_http2_interception()`.
9. Read initial request from TLS stream; if WebSocket upgrade → `handle_websocket_interception()`.
10. Otherwise, forward raw bytes bidirectionally via `tokio::io::copy()`.

### CA / Certificate Generation

`CertGenerator` (`cert.rs:14-149`):
- Self-signed CA: `params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0))` (`cert.rs:65`).
- Key usages: `DigitalSignature` + `KeyEncipherment` (`cert.rs:67-70`).
- Extended key usages: `ServerAuth` + `ClientAuth` (`cert.rs:72-75`).
- Cert generation: `rcgen::KeyPair::generate()` → `params.self_signed(&key_pair)` → DER output (`cert.rs:77-85`).
- Cache: `Arc<RwLock<HashMap<String, CachedCert>>>`, keyed by hostname, default 24h validity (`cert.rs:36`).
- No trust anchor injection required — the CA is ephemeral and per-session.

### Protocol Upgrade Detection

- **WebSocket**: `is_websocket_upgrade()` (`intercept/mod.rs:191-195`) checks for `Upgrade: websocket` header (case-insensitive).
- **HTTP/2**: ALPN protocol `h2` in TLS negotiation (`intercept/mod.rs:819-827`).
- **gRPC**: `detect_protocol()` (`protocols.rs:1255-1300`) checks `Content-Type: application/grpc*` (confidence 0.95).
- `detect_grpc_method_type()` (`protocols.rs:1303-1321`) infers streaming from `TE: trailers` or `grpc-encoding` headers.

### Request / Response Capture into Evidence Bundles

1. Each connection produces a `ProxyFlow` (`types.rs:18-57`) with method, URL, headers, body, timing, and redaction info.
2. `WebProxySessionReport` (`types.rs:101-153`) accumulates flows, protocol sessions, manipulations, correlation refs.
3. `EvidenceBundle::from_report()` (`bundle.rs:81-117`) packages everything into a versioned structure.
4. `bundle.to_bytes()` (`bundle.rs:120-131`) serializes to JSON then gzips via flate2.
5. `bundle.sign()` (`bundle.rs:182-199`) computes HMAC-SHA256 over canonical manifest string for integrity.
6. `compare_bundles()` (`bundle.rs:397-472`) diffs two bundles by flow index, producing `BundleDiff`.

## Security Model

- **Private-IP blocking**: `is_private_ip()` in both `lib.rs:336-356` (outbound connections) and `intercept/mod.rs:157-174` (inbound interception) rejects RFC 1918, loopback, multicast, broadcast, link-local addresses.
- **SensitiveString for credentials**: `ProxyEntry.password` is `Option<SensitiveString>` (`config.rs:67`); `to_log_key()` masks password with `***` (`config.rs:149-157`); `to_url()` exposes via `pass.expose_secret()` only for actual connection (`config.rs:132-147`).
- **CRLF injection prevention**: `validate_header_value()` (`interceptor.rs:204-206`) rejects header values containing `\r`, `\n`, or `\0`.
- **Bundle integrity**: HMAC-SHA256 signing (`bundle.rs:182-199`) with canonical string representation; verification via `verify()` (`bundle.rs:205-226`).
- **In-memory only**: Certificate cache is in-memory (`cert.rs:15`), proxy pool stats are in-memory (`pool.rs:52-56`), no persistent state across restarts.
- **Health check auth**: `pass.expose_secret()` is called only within reqwest proxy setup (`health.rs:91`), not in logs.

## Public API

### `ProxyManager` methods (`lib.rs:50-330`)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(config: ProxyConfig) -> Result<Self>` | Creates manager with pool, rotator, and health checker |
| `add_proxy()` | `async (proxy: ProxyEntry) -> Result<()>` | Adds a single proxy to the pool |
| `add_proxies_from_file()` | `async (path: &str) -> Result<usize>` | Loads proxies from JSON/YAML/plaintext file |
| `get_next_proxy()` | `async () -> Option<ProxyEntry>` | Selects next proxy via rotator (all proxies) |
| `get_healthy_proxy()` | `async () -> Option<ProxyEntry>` | Selects next proxy from healthy subset only |
| `get_highest_priority_proxy()` | `async (min_priority: u8) -> Option<ProxyEntry>` | Selects from highest-priority proxies, falls back to healthy |
| `get_all_healthy_proxies()` | `async () -> Vec<ProxyEntry>` | Returns all healthy proxies |
| `check_health()` | `async () -> Result<ProxyHealth>` | Runs health check on all proxies |
| `create_connection()` | `async (target: &str) -> Result<ProxiedConnection>` | Creates single-proxy connection (auto-selects SOCKS/HTTP based on type) |
| `create_connection_to_domain()` | `async (domain: &str, port: u16) -> Result<ProxiedConnection>` | Creates connection using SOCKS5 domain resolution |
| `create_chained_connection()` | `async (target: &str, chain_length: usize) -> Result<ProxiedConnection>` | Creates multi-hop chain (SOCKS5/Tor only for chains > 1) |
| `pool_size()` | `async () -> usize` | Returns current pool size |
| `start_background_health_check()` | `async (interval_secs: u64) -> JoinHandle<()>` | Spawns periodic health check task |

### Intercept public API (re-exported from `intercept/mod.rs:25-55`)

`to_scan_report_data_proxy`, `EvidenceBundle`, `BundleManifest`, `CertGenerator`, `CertMaterial`, `InterceptConfig`, `InterceptMode`, `InterceptProxy`, `RuleSet`, `EnhancedRuleSet`, `InterceptRule`, `EnhancedRule`, `RuleAction`, `RuleCondition`, `WebSocketSession`, `Http2Session`, `GrpcSession`, `CorrelationEngine`, `AttackNarrative`, `PluginRegistry`, `ProtocolHandler`.

## Integration Points

- **Engine proxy facade**: `crates/eggsec/src/proxy/mod.rs` re-exports all types. Engine modules (scanner, recon) use `ProxyManager` for upstream proxy routing.
- **CLI `proxy-intercept`**: `crates/eggsec/src/commands/handlers/web_proxy.rs` uses `handle_proxy_intercept` which delegates to the intercept submodule.
- **MCP exposure**: `mcp.rs` gated behind `web-proxy-mcp` marker feature; registers 12 tools via `WebProxyToolSchema`.
- **Defense-lab framing**: All interception requires `EnforcementContext` + `OperationRisk::TrafficInterception` under `DefenseLab` mode; dry-run is always safe.
- **Reporting bridge**: `to_scan_report_data_proxy()` converts to `ScanReportData` for unified output consumers; auto-bridged in `report convert`.

## Testing

22 test-bearing files in the `eggsec-web-proxy` crate (every source file except `utils.rs` and `lib.rs` has tests, plus `lib.rs` has `test_proxy_type_parsing`). Test density is the highest in the workspace.

Key test categories:
- **Pool**: 18 tests in `pool.rs` covering add/remove, health filtering, stats tracking, consecutive failures, builder pattern.
- **Rotator**: 15 tests in `rotator.rs` covering all 5 strategies, chain selection, edge cases.
- **Health**: 12 tests in `health.rs` covering result fields, percentage calculation, disabled-proxy skipping, concurrent checks.
- **SOCKS**: 10 tests in `socks.rs` covering builder, auth, timeout, error mapping, wrong proxy type, invalid addresses.
- **HTTP CONNECT**: 13 tests in `http_connect.rs` covering request building, auth encoding, response parsing (200/201/403/500/empty/invalid).
- **Intercept rules**: 30+ tests in `rules.rs` covering all condition types, nesting (And/Or/Not), enhanced rules, indexed evaluation, file persistence, performance benchmarks (1000 rules <1ms/eval).
- **Protocols**: 40+ tests in `protocols.rs` covering WebSocket opcodes, HTTP/2 streams, gRPC calls, streaming state, security detection, reflection parsing.
- **Bundle**: 10 tests in `bundle.rs` covering roundtrip, export/import, signing/verification, diff comparison.
- **Bridge**: 9 tests in `bridge.rs` covering all finding categories, protocol session inclusion, correlation summary.

## Invariants & Gotchas

1. **Adapter stubs return errors/empty results**: Without `web-proxy`, `HealthChecker::check()` always returns `is_healthy: false` with error message. Code that depends on healthy proxies must handle the feature-gated case.
2. **Chain proxying limited to SOCKS5/Tor**: `create_chained_connection()` rejects chains with HTTP or SOCKS4 entries (public contract preserved; Eggress could compose mixed chains but the surface is not broadened silently).
3. **`socks::connect_through()` only accepts SOCKS types**: returns error for `ProxyType::Http` or `ProxyType::Https` (now Eggress-backed, signature preserved).
4. **SOCKS4 health fails closed**: `HealthChecker` returns an explicit unsupported error for `Socks4` instead of testing SOCKS5 (behavior fix 2026-09-22; previously a SOCKS5 result was presented as SOCKS4 health).
5. **`ProxiedConnection.local_addr` is the centralized unknown sentinel**: `eggress-outbound 1.0.8` never populates `OutboundInfo.local_addr`; all production call sites share `eggress_outbound::unknown_local_addr()` (`0.0.0.0:0`), documented as unknown metadata — never logged as measured, never read by policy/authorization/routing/evidence. Removal condition: a published Eggress release exposes the established-socket local address.
6. **Legacy handshake shims remain for `TcpStream` signatures only**: `SocksProxy`/`chain_connect`/`connect_through_with_domain`/`HttpConnectProxy` keep their implementations because Eggress `BoxStream` cannot satisfy `TcpStream` returns without a forbidden downcast. Do not extend them; do not claim all duplicate code is removed.
4. **Health check URL**: Defaults to `"https://api.ipify.org"` (`config.rs:332`); falls back to `"https://api.ipify.org"` if both `health_check_url` and `test_url` are None (`config.rs:397-401`).
5. **Cert cache is per-`CertGenerator` instance**: Two independent `CertGenerator` instances have separate caches; a cloned instance shares the cache via `Arc`.
6. **Background health check never terminates**: `start_background_health_check()` returns a `JoinHandle` but the loop has no break condition (`lib.rs:248-285`).
7. **`FlowBuffer::flows()` linearizes in place (Phase E)**: `flows(&mut self)` calls `make_contiguous()` and returns a real ordered slice over every buffered flow; `flows_vec()`/`iter()` remain for shared-access callers (`intercept/types.rs:605-650`).
8. **`is_private_ip` differs between layers**: The outbound `is_private_ip` (`lib.rs:336-356`) also blocks multicast and broadcast; the intercept `is_private_ip` (`intercept/mod.rs:157-174`) only blocks RFC 1918, loopback, link-local, and unspecified.

## References

- [overview.md](overview.md) — workspace ownership and module index
- [defense_lab.md](defense_lab.md) — defense-lab surface patterns
- [dispatch.md](dispatch.md) — runtime dispatch flow
- [websocket.md](websocket.md) — WebSocket protocol support
- [egress_reuse_decision.md](egress_reuse_decision.md) — Eggress 1.0.8 narrow adoption record (accepted edge, graph, Tokio widening)
- `architecture/web_proxy.md` — full web proxy feature details
- `crates/eggsec-web-proxy/` — domain crate source
- `crates/eggsec/src/proxy/` — adapter layer source
- `crates/eggsec/src/proxy/AGENTS.override.md` — module-specific agent guidance

*Last verified against source: 2026-09-22*
