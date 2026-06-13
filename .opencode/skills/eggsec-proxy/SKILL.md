# Eggsec Proxy Skill

Intercepting proxy module workflows and patterns for traffic inspection.

## Key Types and Patterns

### Intercepting Proxy
`proxy/intercept/` - Intercepting proxy with dynamic SSL certificates.

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
- 12 MCP tools via `web-proxy-mcp` marker feature (list flows, inspect flow, edit, rules, session, HAR, evidence bundle)
- `EvidenceBundle` / `BundleManifest` — Evidence bundle export/import for multi-loadout correlation (`proxy/intercept/bundle.rs`)
- `FlowBuffer` — Capacity-capped flow buffer (`proxy/intercept/types.rs`)
- `ProxyMetrics` — Runtime performance telemetry snapshot (`proxy/intercept/types.rs`)
- `WebProxyToolSchema` / `WebProxyToolCall` — MCP proxy tool types (`proxy/mcp.rs`)
- Real WebSocket (`tokio-tungstenite`) and HTTP/2 (`h2`) protocol backends

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
Follow existing test patterns in `proxy/` modules, testing interception and safe logging.

## Common Tasks

### Adding a New Proxy Feature
1. Implement logic in `proxy/` modules
2. Use `to_log_key()` for logging sensitive data
3. Add tests for new proxy feature

### Adding Dynamic SSL Certificate Support
1. Update `proxy/intercept/` with certificate generation logic
2. Test certificate handling

### Working with Phase 4
1. Pipeline profile: `ScanProfile::WebProxy` in pipeline module
2. MCP tools: `proxy/mcp.rs` (requires `web-proxy-mcp` feature)
3. Evidence bundles: `proxy/intercept/bundle.rs` (`EvidenceBundle`/`BundleManifest`)
4. Performance: `proxy/intercept/types.rs` (`FlowBuffer`, `ProxyMetrics`)

## Bug Fixes (2026-05-30)

- **health.rs:158-170**: Changed `filter_map(|r| r.ok())` to explicit `match` with `is_panic()` detection and `tracing::warn!` for JoinErrors. Previously, panics in health check tasks were silently dropped.

## Resources
- `crates/eggsec/src/proxy/AGENTS.override.md` - Detailed proxy patterns
- `architecture/web_proxy.md` - Full web proxy architecture
- `AGENTS.md` - General project guidelines
- `architecture/overview.md` - Overall design
