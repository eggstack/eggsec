# Proxy Module Override

Specialized guidance for the proxy module.

## Intercepting Proxy

`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

## Web Proxy / Traffic Interception

`proxy/intercept/types.rs` - Core types: `WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `FlowBuffer`, `ProxyMetrics`
`proxy/intercept/bundle.rs` - `EvidenceBundle`, `BundleManifest` for compressed gzip export/import of session evidence
`proxy/intercept/bridge.rs` - Bridge to `ScanReportData` via `to_scan_report_data_proxy()`
`proxy/mcp.rs` - `WebProxyToolSchema` / `WebProxyToolCall` MCP proxy tool types (requires `web-proxy-mcp`)

Feature flag: `web-proxy` (independent of `stress-testing`)
Policy: `OperationRisk::TrafficInterception`, requires `--allow-web-proxy` for real runs
Dry-run always safe; real interception is Phase 2

## Phase 2: TUI Integration

- `Tab::Intercept` — interactive TUI tab with live flow inspection
- Header/body detail panes, manipulation audit trail (`ManipulationRecord`)
- Session save/load (JSON), HAR export, intercept rules display
- Forward/drop/replay/pause actions

## Phase 3: Advanced Protocols (2026-06-12)

- `intercept/protocols.rs`: WebSocket, HTTP/2, gRPC types and detection
- `intercept/correlation.rs`: Cross-loadout correlation hooks
- `intercept/rules.rs`: Enhanced rule engine with `EnhancedRule`, `EnhancedRuleSet`, `RuleCondition`, complex conditions (AND/OR/NOT), persistence (JSON), new actions (InjectResponse, Delay, Tag)
- New bridge categories: `proxy-websocket-session`, `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`
- `ProxyFlow` now has `protocol` field
- `WebProxySessionReport` now has `ws_sessions`, `http2_sessions`, `grpc_sessions`, `correlation` fields

## Phase 4: Pipeline, MCP, Evidence, Performance (2026-06-12)

- **Pipeline**: `ScanProfile::WebProxy` / `Stage::WebProxy` — pipeline profile integration for automated proxy assessments
- **MCP**: 12 tools via `web-proxy-mcp` marker feature (list flows, inspect flow, edit request/response, manage rules, session save/load, HAR export, evidence bundle, flow actions)
- **Evidence**: `EvidenceBundle` / `BundleManifest` in `bundle.rs` for multi-loadout correlation export/import
- **Performance**: `FlowBuffer` (capacity-capped Vec, configurable max_size) and `ProxyMetrics` (runtime telemetry snapshot) in `types.rs`
- **Real protocols**: `tokio-tungstenite` (WebSocket), `h2` (HTTP/2) backends

## Safe Logging

`proxy` module uses `to_log_key()` for safe logging of sensitive data.
