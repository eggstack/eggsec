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

## Safe Logging

`proxy` module uses `to_log_key()` for safe logging of sensitive data.
