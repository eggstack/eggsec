# Proxy Module

## Purpose

Proxy pool management (SOCKS4/5, HTTP CONNECT, HTTPS, Tor) with health checking, rotation strategies, chain proxying, and a full MITM intercepting proxy for security testing.

## Dual-Mode Architecture

The proxy module spans two crates with a clean separation:

```
crates/eggsec/src/proxy/mod.rs          ← adapter layer (re-exports + stubs)
crates/eggsec-web-proxy/src/            ← domain crate (full implementation)
```

### Adapter Layer (`crates/eggsec/src/proxy/mod.rs`)

When `feature = "web-proxy"` is **enabled**: re-exports everything from `eggsec-web-proxy` — `pub use eggsec_web_proxy::*`.

When `feature = "web-proxy"` is **disabled**: provides stub/no-op types so downstream code compiles without the feature. Stubs return empty results or errors indicating the feature is unavailable.

Key adapter behavior:
- `ProxyType` always has the `Tor` variant (even in stub mode)
- `ProxyEntry` stubs include all fields (`name`, `weight`, `priority`, `timeout_ms`, `enabled`, `tags`)
- `HealthCheckConfig` stubs include all fields (`enabled`, `interval_secs`, `timeout_ms`, `test_url`, `max_failures`)
- `ProxyManager` stubs return `None` / empty `Vec` / default errors
- `intercept` module stubs provide minimal types (`WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `CorrelationId`, `ProtocolDetection`)

### Domain Crate (`crates/eggsec-web-proxy/src/`)

Standalone defense-lab surface for HTTP/HTTPS traffic interception, proxy pool management, and MITM security testing. Owns all domain logic, types, and tests but does NOT decide whether an operation is allowed — enforcement stays in the main `eggsec` crate.

## Files

### Domain Crate (`crates/eggsec-web-proxy/src/`)

| File | Description |
|------|-------------|
| `lib.rs` | Crate root: `ProxyManager`, `ProxiedConnection`, connection logic, private-IP blocking |
| `config.rs` | `ProxyConfig`, `ProxyEntry`, `ProxyType`, `RotationStrategy`, file loading (JSON/YAML/plaintext) |
| `error.rs` | `WebProxyError` enum and `Result<T>` type alias |
| `pool.rs` | `ProxyPool` — thread-safe proxy pool with stats tracking and health filtering |
| `rotator.rs` | `ProxyRotator` — rotation strategies (round-robin, random, weighted, least-used, lowest-latency) |
| `health.rs` | `HealthChecker` — async health checking with concurrent checks |
| `socks.rs` | SOCKS4/5 connection implementation (including `chain_connect` for multi-hop) |
| `http_connect.rs` | HTTP CONNECT tunnel implementation |
| `utils.rs` | Utility functions (insecure HTTP client builder, TCP connect with nodelay) |
| `mcp.rs` | MCP/Agent tool registration (gated behind `web-proxy-mcp` feature) |

### Intercept Submodule (`crates/eggsec-web-proxy/src/intercept/`)

HTTP/HTTPS intercepting proxy with dynamic SSL certificate generation, protocol support, and evidence packaging.

| File | Description |
|------|-------------|
| `mod.rs` | `ProxyServer` — TCP listener, CONNECT/HTTP handling, TLS termination, WebSocket/HTTP2 dispatch |
| `cert.rs` | `CertGenerator`, `CertMaterial` — on-the-fly per-host SSL certificates via `rcgen` |
| `interceptor.rs` | `InterceptProxy`, `InterceptConfig`, `InterceptMode` — request/response interception with pause/modify/continue |
| `rules.rs` | `InterceptRule`, `RuleAction`, `RuleSet`, `EnhancedRule`, `EnhancedRuleSet`, `RuleCondition` — configurable interception rules |
| `types.rs` | `WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `ManipulationRecord`, `InterceptSession`, `FlowAction` |
| `bridge.rs` | `to_scan_report_data_proxy()` — converts session report to `ScanReportData` for unified reporting |
| `correlation.rs` | Cross-loadout correlation hooks (`CorrelationEngine`, `CorrelationContext`, `CorrelationReference`, `CorrelationSource`, `ConfidenceScorer`, `BehavioralPattern`) |
| `protocols.rs` | WebSocket/HTTP2/gRPC protocol backends (`WebSocketSession`, `Http2Session`, `Http2Stream`, `GrpcSession`, `GrpcCall`, `ProtocolDetection`, `ProxyProtocol`) |
| `bundle.rs` | `EvidenceBundle`, `BundleManifest` — compressed JSON archive for session packaging (gzip, import/export/signed) |
| `narrative.rs` | `AttackNarrative`, `NarrativeEvent` — human-readable attack narrative generation |
| `plugins.rs` | `PluginRegistry`, `PluginSandbox`, `ProtocolHandler`, `PluginCapability` — extensible protocol handling with capability-based sandbox |
| `dynamic_plugins.rs` | `DynamicPluginRegistry` — shared-library plugin loading (gated behind `dynamic-plugins` feature) |
| `transparent.rs` | `TransparentProxyConfig` — iptables/nftables REDIRECT mode (gated behind `transparent-proxy` feature, Linux only) |
| `redteam.rs` | Adversarial security tests for proxy inputs (CRLF injection, header smuggling, etc.) |

## Key Types

### Proxy Pool

| Type | Location | Description |
|------|----------|-------------|
| `ProxyManager` | `lib.rs` | Central orchestrator: pool + rotator + health checker |
| `ProxiedConnection` | `lib.rs` | Connection routed through proxy chain (chain, local_addr, target_addr) |
| `ProxyPool` | `pool.rs` | Thread-safe proxy pool with stats tracking and health filtering |
| `ProxyRotator` | `rotator.rs` | Rotation strategy selector |
| `HealthChecker` | `health.rs` | Async health checking for proxy endpoints |
| `ProxyHealth` | `health.rs` | Aggregate health status (total, healthy, unhealthy, results) |

### Configuration

| Type | Location | Description |
|------|----------|-------------|
| `ProxyConfig` | `config.rs` | Pool configuration (rotation strategy, health check settings, chain settings) |
| `ProxyEntry` | `config.rs` | Individual proxy definition |
| `ProxyType` | `config.rs` | Enum: `Socks4`, `Socks5` (default), `Http`, `Https`, `Tor` |
| `RotationStrategy` | `config.rs` | Enum: `RoundRobin` (default), `Random`, `Weighted`, `LeastUsed`, `LowestLatency` |
| `HealthCheckConfig` | `config.rs` | Health check parameters |

### Intercept

| Type | Location | Description |
|------|----------|-------------|
| `ProxyServer` | `intercept/mod.rs` | TCP listener with CONNECT tunneling and TLS termination |
| `CertGenerator` | `intercept/cert.rs` | Per-host SSL certificate generation with 24-hour cache |
| `InterceptProxy` | `intercept/interceptor.rs` | Higher-level proxy with mode and channel-based modification |
| `InterceptMode` | `intercept/interceptor.rs` | Enum: `Monitor`, `Intercept`, `Allow` |
| `RuleSet` | `intercept/rules.rs` | Host/path pattern matcher with action evaluation |
| `EnhancedRuleSet` | `intercept/rules.rs` | Complex rule conditions (And, Or, HostMatches, BodyContains, etc.) |
| `RuleAction` | `intercept/rules.rs` | Enum: `Allow`, `Block`, `Intercept`, `Monitor`, `Modify`, `InjectResponse`, `Delay`, `Tag` |

### Reporting & Evidence

| Type | Location | Description |
|------|----------|-------------|
| `WebProxySessionReport` | `intercept/types.rs` | Complete session report with flows, manipulations, protocol sessions |
| `EvidenceBundle` | `intercept/bundle.rs` | Compressed JSON archive for session packaging |
| `BundleManifest` | `intercept/bundle.rs` | Session metadata for bundle header |
| `AttackNarrative` | `intercept/narrative.rs` | Chronological attack story from session data |

### Correlation & Plugins

| Type | Location | Description |
|------|----------|-------------|
| `CorrelationEngine` | `intercept/correlation.rs` | Cross-loadout correlation analysis |
| `CorrelationReference` | `intercept/correlation.rs` | Link between proxy flow and other loadout findings |
| `CorrelationSource` | `intercept/correlation.rs` | Enum: `DbPentest`, `AuthTest`, `MobileDynamic`, `Wireless`, `ProxyFlow`, `External` |
| `PluginRegistry` | `intercept/plugins.rs` | Extensible protocol handler registration |
| `PluginSandbox` | `intercept/plugins.rs` | Capability-based plugin isolation |

### Error Handling

| Type | Location | Description |
|------|----------|-------------|
| `WebProxyError` | `error.rs` | Domain error enum: `Proxy`, `Network`, `Config`, `Io`, `Tls`, `Intercept`, `Rule`, `Protocol`, `Timeout` |
| `Result<T>` | `error.rs` | `std::result::Result<T, WebProxyError>` alias |

## ProxyEntry Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `Option<String>` | `None` | Optional human-readable identifier |
| `proxy_type` | `ProxyType` | — | Protocol variant |
| `address` | `String` | — | Proxy host address |
| `port` | `u16` | `0` | Proxy port |
| `username` | `Option<String>` | `None` | Optional auth username |
| `password` | `Option<SensitiveString>` | `None` | Optional auth password (redacted in logs) |
| `weight` | `u32` | `1` | Weighted rotation priority |
| `priority` | `u8` | `0` | Higher-priority proxies selected first |
| `timeout_ms` | `u64` | `DEFAULT_PROXY_TIMEOUT_MS` | Connection timeout |
| `enabled` | `bool` | `true` | Whether proxy is active in pool |
| `tags` | `Vec<String>` | `[]` | User-defined tags for filtering |

## ProxyType Variants

| Variant | Serde | Description |
|---------|-------|-------------|
| `Socks4` | `"socks4"` | SOCKS4/4a |
| `Socks5` | `"socks5"` | SOCKS5 (default) |
| `Http` | `"http"` | HTTP CONNECT |
| `Https` | `"https"` | HTTPS CONNECT |
| `Tor` | `"tor"` | Tor via SOCKS5 |

`FromStr` also accepts `"socks4a"` → `Socks4`, `"socks"` → `Socks5`.

## HealthCheckConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Whether health checking is active |
| `interval_secs` | `u64` | `60` | Seconds between health check runs |
| `timeout_ms` | `u64` | `5000` | Timeout per individual proxy check |
| `test_url` | `String` | `"https://api.ipify.org"` | URL to request during health check |
| `max_failures` | `u32` | `3` | Consecutive failures before marking unhealthy |

## Key Methods on `ProxyManager`

| Method | Description |
|--------|-------------|
| `new(config)` | Creates manager with pool, rotator, and health checker |
| `add_proxy(proxy)` | Adds a single proxy to the pool |
| `add_proxies_from_file(path)` | Loads proxies from JSON/YAML/plaintext file |
| `get_next_proxy()` | Selects next proxy via rotator (all proxies) |
| `get_healthy_proxy()` | Selects next proxy from healthy subset only |
| `get_highest_priority_proxy(min_priority)` | Selects from highest-priority proxies, falls back to healthy |
| `get_all_healthy_proxies()` | Returns all healthy proxies |
| `check_health()` | Runs health check on all proxies |
| `create_connection(target)` | Creates single-proxy connection (auto-selects SOCKS/HTTP based on type) |
| `create_connection_to_domain(domain, port)` | Creates connection using SOCKS5 domain resolution |
| `create_chained_connection(target, chain_length)` | Creates multi-hop chain (SOCKS5/Tor only for chains > 1) |
| `pool_size()` | Returns current pool size |
| `start_background_health_check(interval_secs)` | Spawns periodic health check task |

## Intercept Submodule

### Architecture

```
Client → ProxyServer (TCP listener)
         ├─ CONNECT request → TLS termination → cert generation → client stream
         ├─ WebSocket upgrade → tokio-tungstenite bidirectional proxy
         ├─ HTTP/2 ALPN → h2 bidirectional proxy
         └─ HTTP/1.1 → request/response forwarding

Rule evaluation happens at each stage:
  RuleSet.evaluate(host, path) → RuleAction
  EnhancedRuleSet.evaluate_first(ctx) → EnhancedRule (with delay_ms)
```

### Protocol Support

| Protocol | Handler | Feature Gate |
|----------|---------|--------------|
| HTTP/1.1 | `handle_http_request` | always |
| HTTP/2 | `handle_http2_interception` | `web-proxy` |
| WebSocket | `handle_websocket_interception` | `web-proxy` |
| gRPC | via `protocols.rs` types | always (types) |
| Transparent | `transparent.rs` | `transparent-proxy` (Linux) |

### Evidence Bundles

`EvidenceBundle` packages a complete session into a `.json.gz` archive containing:
- All proxy flows
- WebSocket/HTTP2/gRPC session records
- Enhanced rule set snapshot
- Manipulation audit trail
- Cross-loadout correlation references

Supports import, export, signed export, and diff comparison.

### MCP Integration

Gated behind `web-proxy-mcp` marker feature. Registers `WebProxyToolSchema` with 12 actions:
`start`, `stop`, `status`, `list_flows`, `inspect_flow`, `forward_flow`, `drop_flow`, `replay_flow`, `add_rule`, `list_rules`, `remove_rule`, `export_session`.

Real runs require `EnforcementContext` + policy confirmation. Dry-run is always safe.

## Enforcement Boundary

The `eggsec-web-proxy` domain crate does not own enforcement decisions. The main-crate adapter (`crates/eggsec/src/tool/protocol/`) constructs `EnforcementContext` and verifies `ApprovedOperation` before calling domain crate functions. The domain crate receives pre-validated configuration and returns results.

## Feature Flags

| Feature | Effect |
|---------|--------|
| `web-proxy` | Full proxy implementation + intercept submodule + protocol backends |
| `web-proxy-mcp` | MCP tool registration for agent/REST surfaces |
| `dynamic-plugins` | Shared-library plugin loading at runtime |
| `transparent-proxy` | iptables/nftables REDIRECT mode (Linux only) |

Without `web-proxy`, the adapter stubs compile but provide no functional behavior.

## Implementation Status

Fully implemented. Complete proxy management with pool, rotation, health checking, SOCKS/HTTP CONNECT, proxy chaining, background health checks, MITM intercepting proxy with dynamic TLS, WebSocket/HTTP2/gRPC protocol support, evidence bundles, cross-loadout correlation, MCP integration, and transparent proxy mode. Includes an `AGENTS.override.md` for specialized guidance.

## References

- `architecture/web_proxy.md` — full web proxy feature details
- `docs/WEB_PROXY.md` — user-facing web proxy documentation
- `crates/eggsec-web-proxy/` — domain crate source
- `crates/eggsec/src/proxy/` — adapter layer source
- `crates/eggsec/src/proxy/AGENTS.override.md` — module-specific agent guidance
