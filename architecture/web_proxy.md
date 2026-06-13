# Interactive Web Proxy Module

## Purpose

Standalone defense-lab surface for interactive MITM (Man-in-the-Middle) HTTP/HTTPS traffic interception and logging in authorized lab environments. Provides on-the-fly dynamic TLS certificate generation for HTTPS decryption, configurable request/response interception rules, flow capture with budget controls, and PII/token redaction. Produces local `WebProxySessionReport` / `ProxyFlow` types. Gated behind `web-proxy` feature. Follows the consolidated standalone defense-lab pattern (like wireless, mobile, auth-test, db-pentest). Phase 1 delivers dry-run reporting and policy integration; real traffic interception is deferred to Phase 2.

## CLI Behavior

- Build with `--features web-proxy` (or `--features full`).
- `eggsec proxy-intercept [--listen ADDR] [--dry-run] [--json] [--max-flows N] [--max-bytes-per-flow N] [--max-duration S] [--max-concurrent N] [--allow-web-proxy] [--manual-override-reason "..."] [--intercept-rule "host:path:action"]`.
- Dry-run (default/safe) always produces a complete, valid `WebProxySessionReport` JSON with synthetic flows and full audit trail — zero network activity.
- Real runs require explicit `--allow-web-proxy` (audited narrow override) + policy gate (`OperationRisk::TrafficInterception` under `DefenseLab` mode).
- Human output: concise session summary + flow list + budget usage + prominent "lab-only" disclaimer. `--json` emits native `WebProxySessionReport`.
- Prominent pre-execution banner and help text emphasizing authorized lab use only.
- Repeatable `--intercept-rule` flag for rule-based traffic matching (format: `host:path:action`).
- Optional `--upstream-proxy` for chaining through an existing proxy.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `WebProxySessionReport` | `proxy/intercept/types.rs` | Complete session report (listen_addr, ca_fingerprint, dry_run, flows, budget, policy_decision, actions_performed, manifest_matched, timestamps, duration_ms, counters for https_intercepted/http_logged/blocked/redacted, errors) |
| `ProxyFlow` | `proxy/intercept/types.rs` | Single captured HTTP/HTTPS flow (index, method, url, host, path, request/response headers/body, is_https, duration_ms, body sizes, timestamps, redaction_applied) |
| `BudgetUsage` | `proxy/intercept/types.rs` | Budget tracking (max_flows, flows_captured, max_bytes_per_flow, max_duration_secs, elapsed_secs, max_concurrent, peak_concurrent) |
| `RedactionPattern` | `proxy/intercept/types.rs` | PII/token redaction pattern (name, regex pattern, replacement) |
| `ProxyServer` | `proxy/intercept/mod.rs` | Core MITM proxy server (addr, cert_generator, rules, mode; bind, accept, handle HTTP/CONNECT, private IP validation, rule-based dispatch) |
| `CertGenerator` | `proxy/intercept/cert.rs` | Dynamic TLS certificate generation with per-host caching (rcgen-based, self-signed CA, DER output, validity duration) |
| `CertMaterial` | `proxy/intercept/cert.rs` | Generated certificate + key pair (cert_der, key_der) |
| `InterceptProxy` | `proxy/intercept/interceptor.rs` | Request/response interception engine (config, rules, event/decision channels; should_intercept, should_monitor, modify_request, modify_response) |
| `InterceptConfig` | `proxy/intercept/interceptor.rs` | Interception configuration (mode, pause_on_match, timeout, buffer_size) |
| `InterceptMode` | `proxy/intercept/interceptor.rs` | Enum: Monitor (passive), Intercept (active), Allow (passthrough) |
| `InterceptEvent` | `proxy/intercept/interceptor.rs` | Enum: Request(InterceptRequest), Response(InterceptResponse, InterceptRequest) |
| `InterceptDecision` | `proxy/intercept/interceptor.rs` | Enum: Allow, Block, Drop |
| `InterceptRequest` | `proxy/intercept/interceptor.rs` | Mutable request view (method, path, headers, body, host) |
| `InterceptResponse` | `proxy/intercept/interceptor.rs` | Mutable response view (status_code, headers, body) |
| `RequestModification` | `proxy/intercept/interceptor.rs` | Runtime request modification (headers, path, body) |
| `ResponseModification` | `proxy/intercept/interceptor.rs` | Runtime response modification (headers, body, status_code) |
| `InterceptRule` | `proxy/intercept/rules.rs` | Single rule (host_pattern, path_pattern, action, request/response modifications, priority) |
| `RuleAction` | `proxy/intercept/rules.rs` | Enum: Allow, Block, Intercept, Monitor, Modify |
| `RuleSet` | `proxy/intercept/rules.rs` | Ordered rule collection (add, evaluate, get_modifications, remove, clear) |
| `RequestModification` (rules) | `proxy/intercept/rules.rs` | Declarative YAML rule modification (header_name, header_value, new_path, new_body) |
| `ResponseModification` (rules) | `proxy/intercept/rules.rs` | Declarative YAML rule modification (header_name, header_value, new_body, new_status) |

## Files

| File | Description |
|------|-------------|
| `proxy/intercept/mod.rs` | Core: `ProxyServer` (bind, accept, HTTP/CONNECT dispatch, private IP validation, TLS acceptor, bidirectional copy), re-exports |
| `proxy/intercept/types.rs` | Core types (`WebProxySessionReport`, `ProxyFlow`, `BudgetUsage`, `RedactionPattern`) + serde roundtrip tests |
| `proxy/intercept/cert.rs` | `CertGenerator` with per-host caching (rcgen self-signed CA, DER output, validity duration) + cert tests |
| `proxy/intercept/interceptor.rs` | `InterceptProxy` engine (Monitor/Intercept/Allow modes, event/decision channels, request/response modification with CRLF validation) + interceptor tests |
| `proxy/intercept/rules.rs` | `InterceptRule` / `RuleSet` (host/path matching, wildcard support, priority sorting, YAML parsing, modification extraction) + rule tests |
| `proxy/intercept/bridge.rs` | `to_scan_report_data_proxy` bridge: `WebProxySessionReport` → `ScanReportData` (findings with `proxy-intercept-flow` + `web-traffic-summary` categories) + bridge tests |
| `cli/web_proxy.rs` | `ProxyInterceptArgs` + clap definitions (listen, ca-dir, dry-run, budget flags, intercept-rule, upstream-proxy) |
| `commands/handlers/web_proxy.rs` | `handle_proxy_intercept`: `evaluate_and_enforce_operation` (DefenseLab + TrafficInterception/SafeActive), `--allow-web-proxy` gate, dry-run synthetic report, real interception stub |
| `commands/handlers/mod.rs` | Dispatch routing (feature-gated `web-proxy` module + `handle_proxy_intercept` call) |
| `commands/handlers/report.rs` | Auto-bridge for native `WebProxySessionReport` JSON when `web-proxy` feature enabled (via `to_scan_report_data_proxy`) |
| `config/policy.rs` | `OperationRisk::TrafficInterception`, `Capability::TrafficInterception`, `allow_traffic_interception` in `ExecutionPolicy` |
| `config/policy_decision.rs` | `ConfirmationClass::TrafficInterception` handling, `allow_web_proxy` override field |
| `cli/mod.rs` | `ManualOverride.allow_web_proxy` flag, `proxy-intercept` subcommand registration (feature-gated) |

## Feature Flag

`web-proxy` — marker-only feature (no new runtime dependencies). Enables the `proxy-intercept` CLI subcommand, handler, types, bridge, and policy integration. Included in `full` feature set. Phase 4 adds `web-proxy-mcp` marker feature for optional MCP tool exposure.

## Status

**Phase 5 (complete, 2026-06-12)**. Documentation updated, stale file references corrected, TUI integration documented. All core types, cert generation, intercept engine, rule engine, dry-run report path, policy integration (`OperationRisk::TrafficInterception`, `ConfirmationClass::TrafficInterception`, `--allow-web-proxy`), CLI args, handler with `evaluate_and_enforce_operation`, and `to_scan_report_data_proxy` bridge are implemented and tested. TUI tab (`Tab::Intercept`) with live flow inspection, editing, HAR export, and manipulation audit trail complete (Phase 2). Advanced protocols (WebSocket/HTTP/2/gRPC) and enhanced rule engine complete (Phase 3). Pipeline integration (`ScanProfile::WebProxy`), MCP proxy surface (12 tools via `web-proxy-mcp`), evidence bundle v2 (`proxy/intercept/bundle.rs`), performance optimizations (`FlowBuffer`, `ProxyMetrics` in `types.rs`), and real WebSocket/HTTP2 backends complete (Phase 4). Phase 5 polish: documentation accuracy, cross-references, AGENTS.md alignment complete.

## Policy Integration

- `EnforcementContext` central gate via `OperationDescriptor` (`operation: "proxy-intercept"`, `mode: DefenseLab`, `risk: TrafficInterception` for real / `SafeActive` for dry-run, `required_features: ["web-proxy"]`).
- `--allow-web-proxy` (narrow, audited; only for real non-dry-run operations) + `--manual-override-reason`; dry-run bypasses.
- `ConfirmationClass::TrafficInterception` (`as_str()`: "traffic-interception") triggers `RequireConfirmation` under `ManualPermissive`; the handler maps this to `--allow-web-proxy` (audited narrow override).
- Private IP validation (`is_private_ip` / `validate_target`) blocks connections to RFC 1918, loopback, multicast, and broadcast addresses.
- Strict budgets (`--max-flows`, `--max-bytes-per-flow`, `--max-duration`, `--max-concurrent`); redaction on captured bodies.
- Prominent banners, help disclaimers, and output notes: "Use ONLY on lab systems you own and are authorized to intercept."
- Policy decision records + audit trail in every report (even dry-run).

## Integration with Reporting Pipeline

Emits local `WebProxySessionReport` / `ProxyFlow` directly (human + `--json`). Optional `to_scan_report_data_proxy()` bridge (in `bridge.rs`) converts to `ScanReportData` (findings with `proxy-intercept-flow` category per captured flow + `web-traffic-summary` metadata finding; evidence/remediation carried through). Empty findings are valid.

The CLI `report convert` handler auto-bridges native `WebProxySessionReport` JSON when `web-proxy` feature is enabled, so `eggsec proxy-intercept --dry-run --json -o report.json ; eggsec report convert report.json -f sarif` works directly (mirrors wireless/mobile/db-pentest).

Bridged categories: `proxy-intercept-flow` (per flow: method, host, path, https, status, redaction), `web-traffic-summary` (session metadata: listen_addr, total_flows, counters, dry_run, duration). Timestamp is report `ended_at` time. Native `scan_type: "web-proxy-intercept"` in `ScanReportData`.

See `docs/USAGE.md` (Output Models), `crates/eggsec/src/proxy/intercept/`, `commands/handlers/report.rs`, AGENTS.md.

## Standalone Defense-Lab Surface

Web proxy follows the consolidated standalone defense-lab pattern:

- **CLI primary**: `eggsec proxy-intercept` subcommand (feature-gated behind `web-proxy`).
- **TUI tab**: `Tab::Intercept` (Phase 2 complete) with live flow inspection, header/body editing, forward/drop/replay/pause, rules display, session management, HAR export, and manipulation audit trail.
- **MCP proxy surface**: 12 tools via `web-proxy-mcp` marker feature (list flows, inspect flow, edit request/response, manage rules, session save/load, HAR export, evidence bundle). See Phase 4 below.
- **Pipeline integration**: `ScanProfile::WebProxy` / `Stage::WebProxy` (Phase 4). See Phase 4 below.
- **Optional bridge**: `to_scan_report_data_proxy()` for unified reporting consumers (auto-bridged in `report convert`).
- Policy enforcement uses the same `CommandContext::evaluate_and_enforce_operation` + `EnforcementContext` path as wireless/mobile/auth-test/db-pentest.

See `architecture/defense_lab.md`, `architecture/cli_commands.md` (Special Cases), `architecture/output.md`, `docs/USAGE.md` (Output Models block), AGENTS.md.

## Safety Model

- **Dry-run always safe**: Produces a complete, valid `WebProxySessionReport` with synthetic flows. Zero network interaction. Zero CA generation. No `--allow-web-proxy` required.
- **Real interception gated**: Requires `--allow-web-proxy` (audited narrow override) + `--manual-override-reason` + policy confirmation (`OperationRisk::TrafficInterception` under `DefenseLab` mode). Phase 2 item.
- **Private IP blocking**: `is_private_ip()` in `proxy/intercept/mod.rs` rejects connections to loopback, RFC 1918, link-local, multicast, and broadcast addresses before upstream connection.
- **CRLF injection prevention**: `validate_header_value()` in `interceptor.rs` rejects header values containing `\r`, `\n`, or `\0`.
- **Budget enforcement**: `BudgetUsage` tracks flows, bytes, duration, and concurrency against configured maximums.
- **Cert caching**: `CertGenerator` caches per-host certificates with configurable validity duration; cache is in-memory and cleared on drop.
- **Prominent disclaimers**: Pre-execution banners, help text, and output notes emphasize authorized lab use only.

## Phase 3: Advanced Protocols & Enhanced Rule Engine (2026-06-12)

### New Types

| Type | Location | Description |
|------|----------|-------------|
| `ProxyProtocol` | `intercept/protocols.rs` | Protocol enum (Http1, Http2, WebSocket, Grpc) |
| `WebSocketMessage` | `intercept/protocols.rs` | Captured WebSocket message with opcode, payload, direction |
| `WebSocketSession` | `intercept/protocols.rs` | Complete WebSocket session with all messages |
| `WebSocketOpcode` | `intercept/protocols.rs` | WebSocket frame opcode (Text, Binary, Close, Ping, Pong) |
| `Http2Stream` | `intercept/protocols.rs` | HTTP/2 stream with ID, state, headers, body |
| `Http2Session` | `intercept/protocols.rs` | HTTP/2 connection with multiplexed streams |
| `Http2StreamState` | `intercept/protocols.rs` | Stream state (Idle, Open, HalfClosed*, Closed) |
| `GrpcCall` | `intercept/protocols.rs` | gRPC call with path, method type, metadata, body |
| `GrpcSession` | `intercept/protocols.rs` | gRPC session with all captured calls |
| `GrpcMethodType` | `intercept/protocols.rs` | gRPC method type (Unary, ServerStreaming, ClientStreaming, Bidirectional) |
| `ProtocolDetection` | `intercept/protocols.rs` | Protocol detection result with confidence |
| `EnhancedRule` | `intercept/rules.rs` | Rule with complex conditions, ID, and additional actions |
| `EnhancedRuleSet` | `intercept/rules.rs` | Rule collection with persistence and evaluation |
| `RuleCondition` | `intercept/rules.rs` | Complex condition with AND/OR/NOT combinators |
| `RuleContext` | `intercept/rules.rs` | Context for rule evaluation |
| `RuleId` | `intercept/rules.rs` | Rule identifier newtype |
| `InjectResponseConfig` | `intercept/rules.rs` | Inject-response action configuration |
| `CorrelationContext` | `intercept/correlation.rs` | Cross-loadout correlation aggregation |
| `CorrelationReference` | `intercept/correlation.rs` | Reference to a finding in another loadout |
| `CorrelationSource` | `intercept/correlation.rs` | Source loadout enum |
| `CorrelationHook` | `intercept/correlation.rs` | Hook definition for cross-loadout linking |

### New Files

| File | Description |
|------|-------------|
| `intercept/protocols.rs` | WebSocket, HTTP/2, gRPC protocol types and detection |
| `intercept/correlation.rs` | Cross-loadout correlation hooks and context |

### Updated Files

| File | Changes |
|------|---------|
| `intercept/types.rs` | `ProxyFlow.protocol`, `WebProxySessionReport` protocol session fields and correlation |
| `intercept/rules.rs` | Enhanced rule engine with complex conditions, persistence, new actions |
| `intercept/mod.rs` | New module declarations and re-exports |
| `intercept/bridge.rs` | New finding categories for protocols and correlation |

### Bridge Finding Categories

| Category | Description |
|----------|-------------|
| `proxy-websocket-session` | Per WebSocket session |
| `proxy-http2-session` | Per HTTP/2 session |
| `proxy-grpc-session` | Per gRPC session |
| `proxy-correlation-summary` | Session correlation summary |

## Phase 4: Pipeline, MCP, Evidence Bundles & Performance (2026-06-12)

### Pipeline Profile Integration

- `ScanProfile::WebProxy` — pipeline profile for web proxy interception assessments.
- `Stage::WebProxy` — pipeline stage that runs the proxy interception workflow within the standard assessment pipeline.
- Enables `eggsec scan --profile web-proxy` for automated proxy-based assessments.

### MCP Proxy Surface

- 12 MCP tools exposed via `web-proxy-mcp` marker feature:
  - `proxy_list_flows` — list captured flows with filtering
  - `proxy_inspect_flow` — inspect full request/response details
  - `proxy_edit_request` / `proxy_edit_response` — modify request/response headers and body
  - `proxy_manage_rules` — add/remove/update intercept rules
  - `proxy_session_save` / `proxy_session_load` — persist and restore sessions
  - `proxy_har_export` — export session as HAR format
  - `proxy_evidence_bundle` — export evidence bundle for multi-loadout correlation
  - Additional tools for flow actions (forward/drop/replay) and session management
- Tools are registered via `WebProxyToolSchema` / `WebProxyToolCall` types in `proxy/mcp.rs`.
- MCP exposure is gated by `web-proxy-mcp` marker feature (requires `web-proxy`).

### Evidence Bundle v2

- `EvidenceBundle` / `BundleManifest` types in `proxy/intercept/bundle.rs` support export/import of session evidence for multi-loadout correlation.
- Bundles include flows, manipulations, rules, and protocol session data (WebSocket/HTTP2/gRPC).
- Cross-loadout correlation via `CorrelationContext` / `CorrelationReference` hooks (Phase 3).

### Performance Optimizations

- `FlowBuffer` — Capacity-capped flow buffer with configurable max size for high-throughput flow capture (`proxy/intercept/types.rs`).
- `ProxyMetrics` — runtime performance telemetry snapshot (`proxy/intercept/types.rs`).
- Both types are public under `web-proxy` feature.

### Real Protocol Support

- Real WebSocket interception via `tokio-tungstenite` backend.
- Real HTTP/2 interception via `h2` backend.
- Phase 3 introduced type definitions; Phase 4 delivers real protocol backends.

## Future

- **Phase 5+**: Extensibility — plugin-style rule authors, scriptable modifiers, PCAP export, session replay, expanded MCP tool surface.
