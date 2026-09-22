---
name: eggsec-proxy
description: "Intercepting proxy for traffic inspection - use when working with HTTP/HTTPS/WebSocket/HTTP2/gRPC interception, MCP proxy tools, enhanced rules, evidence bundles, or session management."
---

# Eggsec Proxy Skill

Intercepting proxy module workflows and patterns for traffic inspection.

## Key Types and Patterns

### Intercepting Proxy
The intercepting proxy domain code lives in `crates/eggsec-web-proxy/src/intercept/` (separate domain crate), not in `crates/eggsec/src/proxy/`. The main engine's `proxy/` module contains stubs and re-exports gated behind the `web-proxy` feature.

### Phase 2: Interactive TUI
- `Tab::Intercept` — TUI tab with live flow inspection, header/body detail panes
- `ManipulationRecord` — immutable audit trail of request/response edits
- `InterceptSession` — saveable session with flows, manipulations, and flow actions
- `FlowAction` — per-flow actions (Forward/Drop/Replay/Paused)
- Session save/load (JSON), HAR export, intercept rules display

### Phase 3: Advanced Protocols
- `EnhancedRule` / `EnhancedRuleSet` — Enhanced rule engine with complex conditions
- `RuleCondition` — AND/OR/NOT condition combinators
- `RuleContext` — Context for rule evaluation
- `WebSocketSession` / `WebSocketMessage` — WebSocket interception types
- `Http2Session` / `Http2Stream` — HTTP/2 stream tracking
- `GrpcSession` / `GrpcCall` — gRPC call interception
- `CorrelationContext` / `CorrelationReference` — Cross-loadout correlation
- `ProxyProtocol` — Protocol detection enum

### Phase 4: Pipeline, MCP, Evidence, Performance
- `ScanProfile::WebProxy` / `Stage::WebProxy` — Pipeline profile integration
- `EvidenceBundle` / `BundleManifest` — Evidence bundle export/import for multi-loadout correlation (`crates/eggsec-web-proxy/src/intercept/bundle.rs`)
- `FlowBuffer` — Capacity-capped flow buffer (`crates/eggsec-web-proxy/src/intercept/types.rs`)
- `ProxyMetrics` — Runtime performance telemetry snapshot (`crates/eggsec-web-proxy/src/intercept/types.rs`)
- `WebProxyToolSchema` / `WebProxyToolCall` — MCP proxy tool types (`crates/eggsec-web-proxy/src/mcp.rs`)
- `ProxyTool` — MCP tool handler implementation (`crates/eggsec/src/tool/implementations/proxy.rs`)
- Real WebSocket (`tokio-tungstenite`) and HTTP/2 (`h2`) protocol backends

### MCP Proxy Tools (12 tools via `web-proxy-mcp` feature)

The following 12 tools are available when the `web-proxy-mcp` feature is enabled:

| Tool ID | Action | Description |
|---------|--------|-------------|
| `proxy-start` | Start proxy | Start the intercepting proxy on a listen address |
| `proxy-stop` | Stop proxy | Stop the running proxy and clear session |
| `proxy-status` | Status | Get session status, flow count, and budget usage |
| `proxy-list-flows` | List flows | List intercepted flows with pagination |
| `proxy-inspect-flow` | Inspect flow | Get full detail of a specific flow by index |
| `proxy-forward-flow` | Forward | Forward a paused flow to upstream |
| `proxy-drop-flow` | Drop | Drop a paused flow without forwarding |
| `proxy-replay-flow` | Replay | Replay a flow |
| `proxy-add-rule` | Add rule | Add an intercept rule with pattern and action |
| `proxy-list-rules` | List rules | List all configured intercept rules |
| `proxy-remove-rule` | Remove rule | Remove a rule by ID |
| `proxy-export-session` | Export | Export session data as JSON or HAR |

**Policy enforcement:** All proxy tools require `EnforcementContext::evaluate()` before dispatch. Real runs need `--allow-web-proxy` + policy confirmation. Dry-run is always safe.

**Truthfulness contract (Phase E):** `proxy-start` serves labeled synthetic fixture flows in dry-run only; live mode fails explicitly. `proxy-export-session` builds a real `WebProxySessionReport` so counters reflect the flows. Python `run_intercept_session()` runs a real timed listener but per-exchange capture into its result is not yet wired (empty `exchanges` = "not captured by this binding"). Python proxy credentials (`ProxyEntry`/`ProxyRoutePy` passwords) are `[REDACTED]` in every readout, mirroring `DbProbeRequest`. `FlowBuffer::flows(&mut self)` returns a real ordered slice (linearized in place).

**Tool implementation:** `tool/implementations/proxy.rs` implements the `SecurityTool` trait with all 12 actions. Tools use a shared `PROXY_SESSION` static for session state.

### Outbound proxy engine (Eggress 1.0.8, 2026-09-22; corrective pass 2026-09-22)
Production SOCKS/HTTP-CONNECT/chain dialing runs on listener-free
`eggress-outbound 1.0.8` via `crates/eggsec-web-proxy/src/eggress_outbound.rs`
(pinned `=1.0.8`, `default-features = false`; plus `eggress-uri =1.0.8`).
Eggsec owns pool/rotation/health/policy/selection; Eggress executes the
already-selected route (no direct fallback, redacted errors). Do NOT rebuild
SOCKS handshakes or CONNECT framing: `socks.rs`/`http_connect.rs` production
helpers delegate to the adapter; the remaining handshake code exists only for
`TcpStream`-returning compatibility shims. Reqwest stays as the explicit
health-only owner (application-level 2xx-through-proxy checks; SOCKS4 fails
closed as unsupported). `Https` means plaintext CONNECT (naming debt).
Proxy hop endpoints are literal-address-only: `hop_from_entry()` validates
through `ProxyEntry::socket_addr()` and rejects hostname-valued endpoints
before any network behavior (SOCKS5/Tor remote-domain *targets* stay
supported as a separate concern). `check_concurrent()` uses `buffered`
(enabled-input result order, still O(concurrency)). `ProxiedConnection.local_addr`
is the centralized unknown sentinel (`unknown_local_addr()`, upstream-gated
until Eggress exposes the real socket address) — never a measured address,
never a routing/policy input. Guard Check 106 encodes the exact two-crate
allowlist (`eggress-outbound` + `eggress-uri` only).

### Safe Logging
`proxy` module uses `to_log_key()` for safe logging of sensitive data.

## Testing

### Running Proxy Tests
```bash
cargo test --lib -p eggsec proxy::
```

### Running with Features
```bash
# Dry-run (no hardware required)
cargo test --lib -p eggsec --features web-proxy

# MCP proxy surface
cargo test --lib -p eggsec --features web-proxy-mcp
```

### Writing Tests
Follow existing test patterns in `crates/eggsec-web-proxy/src/intercept/` modules, testing interception and safe logging.

## Common Tasks

### Adding a New Proxy Feature
1. Implement logic in `crates/eggsec-web-proxy/src/intercept/` (domain crate)
2. Use `to_log_key()` for logging sensitive data
3. Add tests for new proxy feature

### Adding Dynamic SSL Certificate Support
1. Update `crates/eggsec-web-proxy/src/intercept/` with certificate generation logic
2. Test certificate handling

### Working with Phase 4
1. Pipeline profile: `ScanProfile::WebProxy` in pipeline module
2. MCP tools: `tool/implementations/proxy.rs` (requires `web-proxy-mcp` feature)
3. Evidence bundles: `proxy/intercept/bundle.rs` (`EvidenceBundle`/`BundleManifest`)
4. Performance: `proxy/intercept/types.rs` (`FlowBuffer`, `ProxyMetrics`)
5. gRPC protobuf: `crates/eggsec-web-proxy/src/intercept/protocols.rs` (prost-based encoding/decoding)
6. Async rules: `EnhancedRuleSet::evaluate_async()` and `evaluate_indexed_async()`
7. Session resume: `WebProxySessionReport::save_to_file()` / `load_from_file()`

### Adding a New MCP Proxy Tool
1. Add the tool action to `ProxyAction` enum in `crates/eggsec-web-proxy/src/mcp.rs`
2. Add the handler in `tool/implementations/proxy.rs` `execute()` method
3. Add tool ID to `classify_tool_risk()` and `infer_tool_category()` in `tool/protocol/mcp/policy.rs`
4. Add policy entries in `McpProfilePolicy` for tool visibility per profile
5. Add capability definition in `SecurityTool::capabilities()`
6. Add tests in both `proxy/mcp.rs` and `tool/implementations/proxy.rs`

## Bug Fixes (2026-05-30)

- **health.rs:158-170**: Changed `filter_map(|r| r.ok())` to explicit `match` with `is_panic()` detection and `tracing::warn!` for JoinErrors. Previously, panics in health check tasks were silently dropped.

## Resources
- `crates/eggsec/src/proxy/AGENTS.override.md` - Detailed proxy patterns
- `architecture/web_proxy.md` - Full web proxy architecture
- `AGENTS.md` - General project guidelines
- `architecture/overview.md` - Overall design
