# Proxy Module Override

Specialized guidance for the proxy module.

## Intercepting Proxy

`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

## Web Proxy / Traffic Interception

`proxy/intercept/types.rs` - Core types: `WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`
`proxy/intercept/bridge.rs` - Bridge to `ScanReportData` via `to_scan_report_data_proxy()`

Feature flag: `web-proxy` (independent of `stress-testing`)
Policy: `OperationRisk::TrafficInterception`, requires `--allow-web-proxy` for real runs
Dry-run always safe; real interception is Phase 2

**Phase 3 (2026-06-12)**: Advanced protocols and enhanced rule engine.
- `intercept/protocols.rs`: WebSocket, HTTP/2, gRPC types and detection
- `intercept/correlation.rs`: Cross-loadout correlation hooks
- `intercept/rules.rs`: Enhanced rule engine with `EnhancedRule`, `EnhancedRuleSet`, `RuleCondition`, complex conditions (AND/OR/NOT), persistence (JSON), new actions (InjectResponse, Delay, Tag)
- New bridge categories: `proxy-websocket-session`, `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`
- `ProxyFlow` now has `protocol` field
- `WebProxySessionReport` now has `ws_sessions`, `http2_sessions`, `grpc_sessions`, `correlation` fields

## Safe Logging

`proxy` module uses `to_log_key()` for safe logging of sensitive data.
