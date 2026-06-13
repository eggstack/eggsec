# Interactive Web Proxy / Traffic Interception

Eggsec provides an interactive MITM (man-in-the-middle) proxy for intercepting and inspecting HTTP/HTTPS traffic in authorized lab environments. The `web-proxy` feature enables dynamic SSL certificate generation, request/response capture, rule-based interception, and budget-constrained session recording for security assessment and defense validation.

**This is a standalone defense-lab surface.** The proxy is intended for use on lab systems you own and are authorized to intercept. Real traffic interception requires explicit operator confirmation and is gated by policy.

**Phase 1 (complete)**: MITM server + CA + CLI + dry-run + policy + bridge.
**Phase 2 (complete)**: Interactive TUI tab (`Tab::Intercept`) with live flow inspection, header/body editing, forward/drop/replay actions, intercept rules, session save/load, HAR export, and full manipulation audit trail.

## Feature Gate

Build with `--features web-proxy` (or `--features full`).

```bash
cargo build --release -p eggsec-cli --features web-proxy
```

## Safety & Policy

- Use **only on systems you own or are explicitly authorized to intercept** (lab, authorized defensive validation).
- **Defense-lab only**: The proxy operates in `OperationMode::DefenseLab` with `OperationRisk::TrafficInterception` (real) or `OperationRisk::SafeActive` (dry-run).
- **`--allow-web-proxy` required** for non-dry-run execution (audited; same pattern as `wireless deauth --allow-active-wireless`).
- **Dry-run is always safe**: Produces a complete `WebProxySessionReport` with synthetic flows, zero network activity, and no port binding.
- **Policy integration**: `CommandContext::evaluate_and_enforce_operation()` with `OperationDescriptor` gate; `EnforcementContext` enforces scope, feature presence, and risk tier.
- **Budget enforcement**: Configurable limits on flows, bytes per flow, session duration, and concurrent connections (defaults: 1000 flows, 64 KiB/flow, 300s, 100 concurrent).
- **CA management**: Dynamic self-signed CA generated on first use; per-host leaf certificates cached with configurable validity.
- **Scope validation**: Private/internal IP addresses are blocked by the proxy server to prevent unintended LAN interception.

See also: [docs/SAFETY.md](SAFETY.md), `architecture/proxy.md`, and the central `EnforcementContext` policy gate.

## Quick Start

```bash
# Dry-run: complete report, no network activity, no privileges required
eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json -o report.json

# Dry-run with custom budgets
eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json \
  --max-flows 100 --max-duration 60 -o report.json

# Interactive TUI: launch eggsec-tui, navigate to Intercept tab, configure and press Enter
```

## CLI Reference

| Flag | Default | Description |
|------|---------|-------------|
| `--listen` | `127.0.0.1:8080` | Address and port to listen on |
| `--ca-dir` | (auto) | Directory for CA certificate storage |
| `--generate-ca-if-missing` | `true` | Generate CA if missing in `ca-dir` |
| `--ca-cert` | (none) | Path to user-provided CA certificate (PEM) |
| `--ca-key` | (none) | Path to user-provided CA private key (PEM) |
| `--dry-run` | `false` | Produce complete report without binding server |
| `--json` | `false` | Output in JSON format |
| `-o`, `--output` | (stdout) | Output file path |
| `--max-flows` | `1000` | Maximum number of flows to capture |
| `--max-bytes-per-flow` | `65536` | Maximum bytes per flow body (0 = unlimited) |
| `--max-duration` | `300` | Maximum session duration in seconds |
| `--max-concurrent` | `100` | Maximum concurrent connections |
| `--allow-web-proxy` | `false` | Allow traffic interception (required for non-dry-run) |
| `--manual-override-reason` | (none) | Manual override reason for audit trail |
| `--quiet` | `false` | Suppress non-essential output |
| `--intercept-rule` | (none) | Intercept rule (repeatable; format: `host:path:action`) |
| `--upstream-proxy` | (none) | Upstream proxy URL (chain through existing proxy) |

### Intercept Rule Format

Rules use the format `host:path:action` where:
- `host`: Exact hostname, `*.domain.com` wildcard, or `*` for all
- `path`: Exact path, `/prefix/*` for subtree, or `*` for all
- `action`: `allow`, `block`, `intercept`, `monitor`, or `modify`

Examples:
```bash
# Block all traffic to evil.com
--intercept-rule "evil.com:*:block"

# Intercept API endpoints on example.com
--intercept-rule "example.com:/api/*:intercept"

# Monitor all traffic
--intercept-rule "*:*:monitor"
```

Rules are evaluated by priority (higher first); first match wins. Default action when no rule matches is `Allow`.

## HTTPS Interception

### How CA Generation Works

The proxy uses `rcgen` to dynamically generate a self-signed CA certificate on first run. When a client sends a `CONNECT` request (HTTPS), the proxy:

1. Validates the target host against scope rules and private-IP blocking
2. Evaluates intercept rules for the target host/path
3. Generates a leaf certificate signed by the cached CA, with the target hostname as a Subject Alternative Name (SAN)
4. Accepts the client TLS connection using the generated certificate
5. Opens a separate TLS connection to the upstream server
6. Relays traffic between client and server, logging the flow

Leaf certificates are cached per-host with a configurable validity period (default 24 hours) to avoid repeated generation.

### Browser/OS Trust Store Installation

For HTTPS interception to work without certificate errors, the CA certificate must be trusted by the client:

**Firefox:**
1. Go to Settings → Privacy & Security → Certificates → View Certificates
2. Go to Authorities → Import
3. Select the CA PEM file from `--ca-dir` (or the path printed in session output)
4. Check "Trust this CA to identify websites" → OK

**Chrome / Chromium:**
1. Go to Settings → Privacy and Security → Security → Manage certificates
2. Go to Authorities → Import
3. Select the CA PEM file
4. Check all trust purposes → Close

**macOS Keychain:**
```bash
# Import CA into system keychain
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ca-cert.pem
```

**Windows (certutil):**
```powershell
certutil -addstore -f "Root" ca-cert.pem
```

**Linux (Debian/Ubuntu):**
```bash
sudo cp ca-cert.pem /usr/local/share/ca-certificates/eggsec-proxy.crt
sudo update-ca-certificates
```

### Limitations

- **Certificate pinning**: Applications that use certificate pinning (HPKP, custom trust stores, network security config) will reject intercepted connections. This is expected behavior — the proxy cannot bypass pinning without additional instrumentation.
- **Client certificate authentication**: Mutual TLS (mTLS) endpoints will fail unless the client is configured to present certificates to the proxy.
- **Transparent proxy**: The proxy requires explicit client configuration (manual or PAC file). Transparent proxy mode (iptables redirect) is not supported.
- **Streaming body capture**: Only complete request/response bodies are captured; streaming uploads/downloads are not progressively logged.

## What It Captures

### Flow Structure

Each captured flow (`ProxyFlow`) records:

| Field | Description |
|-------|-------------|
| `index` | Monotonically increasing flow index within the session |
| `method` | HTTP method (GET, POST, CONNECT, etc.) |
| `url` | Full URL |
| `host` | Host header value |
| `path` | Request path |
| `request_headers` | Request headers (key-value pairs) |
| `request_body` | Request body (truncated to `--max-bytes-per-flow`) |
| `response_status` | Response status code |
| `response_headers` | Response headers |
| `response_body` | Response body (truncated to `--max-bytes-per-flow`) |
| `is_https` | Whether this was an HTTPS CONNECT tunnel |
| `duration_ms` | Flow duration in milliseconds |
| `request_body_size` | Original request body size (before truncation) |
| `response_body_size` | Original response body size (before truncation) |
| `started_at` | Timestamp when the flow started (RFC 3339) |
| `completed_at` | Timestamp when the flow completed (RFC 3339) |
| `redaction_applied` | Redaction type applied (if any) |

### Redaction

Request and response bodies are truncated to `--max-bytes-per-flow` (default 64 KiB). Bodies exceeding this limit are truncated with a `[TRUNCATED]` marker. In future phases, configurable redaction patterns (PII, tokens, secrets) will be applied to headers and bodies.

### Budget Limits

The session enforces configurable resource limits:

| Budget | Default | Description |
|--------|---------|-------------|
| `max_flows` | 1000 | Total flows captured before session stops |
| `max_bytes_per_flow` | 65536 | Per-flow body size cap |
| `max_duration` | 300 | Session timeout in seconds |
| `max_concurrent` | 100 | Peak concurrent connections |

Budget status is tracked in `BudgetUsage` and included in the session report. When a budget limit is reached, the session gracefully shuts down and finalizes the report.

## Output & Integration

### WebProxySessionReport JSON Structure

```json
{
  "listen_addr": "127.0.0.1:8080",
  "ca_fingerprint": "sha256-hex-fingerprint",
  "dry_run": true,
  "flows": [ { "index": 1, "method": "GET", "url": "...", "..." : "..." } ],
  "budget": {
    "max_flows": 1000,
    "flows_captured": 2,
    "max_bytes_per_flow": 65536,
    "max_duration_secs": 300,
    "elapsed_secs": 0,
    "max_concurrent": 100,
    "peak_concurrent": 0
  },
  "policy_decision": null,
  "actions_performed": ["dry-run-execution", "synthetic-flows-generated"],
  "manifest_matched": true,
  "started_at": "2026-06-12T00:00:00Z",
  "ended_at": "2026-06-12T00:00:01Z",
  "duration_ms": 100,
  "https_intercepted": 1,
  "http_logged": 1,
  "blocked": 0,
  "redacted": 1,
  "errors": []
}
```

### Reporting Bridge

Native `--json` output from `eggsec proxy-intercept` is auto-bridged by `eggsec report convert` when the `web-proxy` feature is enabled:

```bash
# Dry-run → JSON → SARIF report
eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json -o proxy.json
eggsec report convert proxy.json -f sarif -o proxy.sarif

# HTML report
eggsec report convert proxy.json -f html -o proxy.html

# Markdown
eggsec report convert proxy.json -f markdown -o proxy.md
```

### Finding Categories

Bridged findings use these categories:

| Category | Description |
|----------|-------------|
| `proxy-intercept-flow` | One finding per captured flow (method, host, path, status, redaction) |
| `web-traffic-summary` | Session metadata (total flows, HTTPS count, redacted count, budget usage) |

The bridge is produced by `to_scan_report_data_proxy()` in `proxy/intercept/bridge.rs` and auto-wired in `commands/handlers/report.rs` when the feature is present.

## MCP Proxy Tools

When built with `--features web-proxy-mcp` (requires `web-proxy`), 12 MCP tools are available for agent and automation integration:

| Tool | Description |
|------|-------------|
| `proxy_list_flows` | List captured flows with filtering |
| `proxy_inspect_flow` | Inspect full request/response details |
| `proxy_edit_request` | Modify request headers and body |
| `proxy_edit_response` | Modify response headers and body |
| `proxy_manage_rules` | Add/remove/update intercept rules |
| `proxy_session_save` | Save session to JSON file |
| `proxy_session_load` | Load session from JSON file |
| `proxy_har_export` | Export session as HAR 1.2 |
| `proxy_evidence_bundle` | Export evidence bundle for multi-loadout correlation |
| `proxy_forward_flow` | Forward a specific flow |
| `proxy_drop_flow` | Drop a specific flow |
| `proxy_replay_flow` | Replay a specific flow |

Tools are defined in `proxy/mcp.rs` as `WebProxyToolSchema` / `WebProxyToolCall` types. MCP exposure is gated by the `web-proxy-mcp` marker feature.

```bash
# Build with MCP proxy tools
cargo build --release -p eggsec-cli --features web-proxy-mcp
```

## Evidence Bundles

The proxy supports evidence bundle export/import for multi-loadout correlation analysis. Bundles are compressed gzip JSON files containing flows, manipulations, rules, and protocol session data.

### Exporting

From the TUI:
1. Navigate to the Intercept tab
2. Select "Export HAR" or use the evidence bundle action
3. The bundle is saved as `intercept_session_YYYYMMDD_HHMMSS.json.gz`

From the CLI (via bridge):
```bash
# Generate dry-run session
eggsec proxy-intercept --dry-run --json -o proxy.json

# Convert to evidence bundle format
eggsec report convert proxy.json -f json -o evidence.json
```

### Bundle Contents

| Field | Description |
|-------|-------------|
| `version` | Bundle format version |
| `manifest` | Session metadata (target, scope, timestamps, dry_run, counts) |
| `flows` | All captured HTTP/HTTPS flows |
| `ws_sessions` | WebSocket sessions with messages |
| `http2_sessions` | HTTP/2 sessions with streams |
| `grpc_sessions` | gRPC sessions with calls |
| `rules` | Intercept rules (legacy + enhanced) |
| `manipulations` | All request/response manipulations |
| `correlations` | Cross-loadout correlation references |

### Cross-Loadout Correlation

Evidence bundles can be correlated with other Eggsec loadouts:

- **Database pentest**: JWT tokens intercepted in proxy → correlated with SQL injection findings
- **Auth testing**: Authentication tokens → correlated with proxy session flows
- **Mobile dynamic**: Mobile app traffic → correlated with proxy captures

Built-in correlation hooks: `jwt_to_db_query_hook()`, `proxy_auth_hook()`, `proxy_mobile_hook()`.

## Troubleshooting

- **"Traffic interception requires --allow-web-proxy"**: Real (non-dry-run) paths are intentionally gated. Use `--dry-run` for planning/safe validation, or pass `--allow-web-proxy --manual-override-reason "..."` for authorized lab runs. The flag is audited on the policy decision.
- **Connection refused**: If `--listen` address is already in use, the proxy will fail to bind. Choose a different port or stop the conflicting process.
- **Budget exhausted**: When `--max-flows`, `--max-duration`, or other limits are reached, the session stops gracefully. Increase limits if needed, or review the budget section of the report for what was captured before exhaustion.
- **CA not trusted**: Install the generated CA certificate into your browser/OS trust store (see "HTTPS Interception" above). The CA PEM file path is included in the session report.
- **"Dynamic SSL certificate generation failed"**: Usually indicates an issue with the `rcgen` library or key generation. Ensure the `web-proxy` feature is enabled and dependencies are resolved.
- **Feature not enabled**: Rebuild with `--features web-proxy` (or `--features full`).

## Limitations

- **No request/response modification via CLI**: The CLI handler supports dry-run; real interception with request/response modification is available through the TUI tab.
- **No transparent proxy**: The proxy requires explicit client configuration (manual or PAC file). Transparent proxy mode (iptables redirect) is not supported.
- **No streaming body capture**: Only complete request/response bodies are captured; streaming uploads/downloads are not progressively logged.
- **Binary protobuf editing**: gRPC binary protobuf editing is best-effort for common services; complex or unknown schemas may not round-trip correctly.

## Phase 3: Advanced Protocols & Enhanced Rule Engine (2026-06-12)

Phase 3 extends the interactive web proxy with modern protocol support and a powerful rule engine.

### Protocol Support

#### WebSocket Interception

WebSocket traffic is detected via the `Upgrade: websocket` header and tracked separately from HTTP flows.

- **Detection**: Automatic via `detect_protocol()` in `proxy/intercept/protocols.rs`
- **Types**: `WebSocketSession`, `WebSocketMessage`, `WebSocketOpcode`
- **Features**: Message list with direction, opcode, payload, masking info; manipulation audit trail; close frame handling; ping/pong tracking
- **TUI**: Protocol selector in detail pane; WebSocket-specific message stream view

#### HTTP/2 Support

HTTP/2 streams are tracked via pseudo-headers (`:scheme`, `:method`, `:path`).

- **Detection**: Automatic via HTTP/2 pseudo-header presence
- **Types**: `Http2Session`, `Http2Stream`, `Http2StreamState`
- **Features**: Stream multiplexing tracking, priority, window updates, per-stream request/response bodies
- **TUI**: HTTP/2 stream list view with stream ID and state

#### gRPC / Protobuf

gRPC traffic is detected via `Content-Type: application/grpc` headers.

- **Detection**: Automatic via content-type header matching
- **Types**: `GrpcSession`, `GrpcCall`, `GrpcMethodType`
- **Features**: Unary, server-streaming, client-streaming, bidirectional detection; JSON-transcoded or text-protobuf inspection where feasible
- **Limitations**: Binary protobuf editing is partial in Phase 3; best-effort for common services
- **TUI**: gRPC call inspector view

### Enhanced Rule Engine

The rule engine has been significantly enhanced with complex conditions, persistence, and new actions.

- **Types**: `EnhancedRule`, `EnhancedRuleSet`, `RuleCondition`, `RuleContext`, `RuleId`, `InjectResponseConfig`
- **Conditions**: AND/OR/NOT combinators, regex, body size thresholds, protocol-specific matching (WebSocket opcode, gRPC method type)
- **Actions**: `Allow`, `Block`, `Intercept`, `Monitor`, `Modify`, `InjectResponse`, `Delay`, `Tag`
- **Persistence**: JSON file-based save/load with `save_to_file()`/`load_from_file()`
- **Import/Export**: JSON format via `export_json()`/`import_json()`
- **Rule Management**: Enable/disable rules by ID, tag-based grouping, priority ordering
- **TUI**: Legacy/Enhanced rule view toggle in Rules detail pane

### Cross-Loadout Correlation

Lightweight correlation hooks link proxy flows with other Eggsec loadouts.

- **Types**: `CorrelationContext`, `CorrelationReference`, `CorrelationSource`, `CorrelationHook`
- **Sources**: `DbPentest`, `AuthTest`, `MobileDynamic`, `Wireless`, `ProxyFlow`, `External`
- **Built-in Hooks**: `jwt_to_db_query_hook()`, `proxy_auth_hook()`, `proxy_mobile_hook()`
- **Features**: Per-flow correlation references, confidence scoring, summary statistics
- **Reporting**: Correlation data included in `web-traffic-summary` finding and dedicated `proxy-correlation-summary` finding

### Reporting Bridge Extensions

The `to_scan_report_data_proxy()` bridge now produces additional finding categories:

- `proxy-websocket-session` — per WebSocket session
- `proxy-http2-session` — per HTTP/2 session
- `proxy-grpc-session` — per gRPC session
- `proxy-correlation-summary` — session correlation summary (when present)

### TUI Enhancements

- **Protocol Selector**: Toggle between Legacy and Enhanced rule views in the Rules detail pane
- **Protocol Detail Panes**: New WebSocket, HTTP/2, and gRPC detail pane views
- **Keyboard**: `r` key toggles rule view mode when in Rules detail pane

## Phase 2: Interactive TUI & Manipulation (Complete - 2026-06-13)

Phase 2 adds the interactive TUI tab for live traffic inspection and manual manipulation.

### TUI Tab

Launch the interactive intercept tab from the TUI:
- Navigate to the "Intercept" tab (feature-gated under `web-proxy`)
- Configure listen address and dry-run mode
- Press Enter to start/stop the intercept session

### Flow List & Detail Panes

The TUI displays:
- **Flow List**: Table of captured flows with method, host, path, status, size, HTTPS indicator
- **Detail Panes** (cycle with ↑/↓ when detail is focused):
  - **Headers**: Request and response headers
  - **Body**: Request and response body content
  - **Manipulations**: Audit trail of all edits performed
  - **Rules**: Information about intercept rule actions

### Actions

Select a flow and use the action bar (←/→ to navigate, Enter to execute):
- **Forward**: Forward the (possibly modified) request
- **Drop**: Drop the request without forwarding
- **Replay**: Replay the original unmodified request
- **Pause All / Resume All**: Global flow control
- **Save**: Save session as JSON
- **Export HAR**: Export session in HAR 1.2 format

### Session Management

Sessions can be saved and loaded:
```bash
# Sessions are saved as JSON with full flow data and manipulation history
# HAR export produces standard HAR 1.2 format for browser DevTools import
```

### Manipulation Audit Trail

Every edit (header change, body modification) is recorded as a `ManipulationRecord` with:
- Flow index and direction (request/response)
- Field modified (e.g., "header:Authorization", "body")
- Before/after values
- Reason for the change
- Timestamp

### Key Types

- `ManipulationRecord` - Immutable record of a request/response manipulation
- `InterceptSession` - Complete saveable session with flows, manipulations, and actions
- `FlowAction` - Per-flow action (Forward/Drop/Replay/Paused)
- `HarExport` - HAR 1.2 export structure

## TUI Keybindings (Phase 2 Interactive)

The Intercept tab provides keyboard-driven traffic inspection and manipulation.

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate flow list / cycle detail panes |
| `←` / `→` | Navigate action bar / move within focused input |
| `Tab` | Cycle focus: Flow List → Detail View → Action Bar |
| `g` / `G` | Jump to first / last flow (vim-style, if enabled) |
| `Home` / `End` | Jump to start / end of flow list |
| `PgUp` / `PgDn` | Page up / down through flows |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Execute selected action in action bar / apply edit in modal |
| `Esc` | Close modal / cancel edit / return to flow list focus |
| `e` | Open edit modal for selected flow's detail pane |
| `d` | Toggle dry-run mode (default on) |
| `r` | Refresh / reload flows |
| `s` | Quick-save session |
| `x` | Export HAR |

### Action Bar Actions (←/→ to select, Enter to execute)

| Index | Action | Description |
|-------|--------|-------------|
| 0 | Forward | Forward the (possibly modified) request upstream |
| 1 | Drop | Drop the request without forwarding (logged only) |
| 2 | Replay | Replay the original unmodified request (logged only) |
| 3 | Pause | Pause all flow interception |
| 4 | Resume | Resume flow interception |
| 5 | Save | Save session to JSON file |
| 6 | HAR | Export session to HAR 1.2 format |

### Edit Modal

| Key | Action |
|-----|--------|
| Type | Add characters to edit buffer |
| `Backspace` | Remove character from edit buffer |
| `Enter` | Apply edit and close modal |
| `Esc` | Cancel and close modal without applying |

### Detail Panes (when Detail View is focused)

| Key | Action |
|-----|--------|
| `↑` / `↓` | Cycle: Headers → Body → Manipulations → Rules |

## Example: Session Artifacts

### JSON Session Format

Sessions are saved as `intercept_session_YYYYMMDD_HHMMSS.json`:

```json
{
  "listen_addr": "127.0.0.1:8080",
  "ca_fingerprint": "SHA256:...",
  "dry_run": false,
  "started_at": "2026-06-13T10:00:00Z",
  "ended_at": "2026-06-13T10:30:00Z",
  "target": "https://example.com",
  "flows": [
    {
      "index": 0,
      "method": "GET",
      "url": "https://example.com/api/user",
      "host": "example.com",
      "path": "/api/user",
      "request_headers": {"Authorization": "Bearer token123", "Content-Type": "application/json"},
      "response_status": 200,
      "is_https": true,
      "duration_ms": 150
    }
  ],
  "manipulations": [
    {
      "flow_index": 0,
      "direction": "request",
      "field": "header:Authorization",
      "before": "Bearer old-token",
      "after": "Bearer new-token",
      "reason": "Token refresh test",
      "timestamp": "2026-06-13T10:15:00Z"
    }
  ],
  "flow_actions": [
    {"flow_index": 0, "action": "forward", "timestamp": "2026-06-13T10:15:01Z"}
  ],
  "budget": {
    "max_flows": 100,
    "flows_captured": 42
  }
}
```

### HAR Export

Exported HAR files (`intercept_session_YYYYMMDD_HHMMSS.har`) follow the HAR 1.2 specification and can be imported into browser DevTools or tools like Postman.

## Current Status

**Phase 1 (dry-run, complete)**:
- CLI command `proxy-intercept` with full policy integration
- Dry-run produces complete `WebProxySessionReport` with synthetic flows, budget metadata, and audit trail
- `CertGenerator` with per-host caching and configurable validity
- `ProxyServer` with TCP listener, CONNECT handling, dynamic TLS, rule evaluation, and private-IP blocking
- `InterceptRule` / `RuleSet` with host/path pattern matching, priority, and YAML parsing
- `to_scan_report_data_proxy()` bridge with `proxy-intercept-flow` and `web-traffic-summary` categories
- Auto-bridge in `report convert` when `web-proxy` feature is present
- Budget enforcement (flows, bytes, duration, concurrent)
- `--intercept-rule` CLI flag for runtime rule injection
- `--upstream-proxy` flag for proxy chaining

**Phase 2 (Interactive TUI & Manipulation, complete)**:
- Interactive TUI tab `Tab::Intercept` with live flow inspection
- Flow list with method, host, path, status, size, HTTPS indicator
- Header/body detail panes with cycling navigation
- Forward/drop/replay/pause actions per flow
- Session save/load (JSON) with full manipulation history
- HAR 1.2 export for browser DevTools import
- Intercept rules display with pattern matching
- `ManipulationRecord` audit trail for every edit
- `InterceptSession` type for saveable sessions with flow actions
- Edit modal for in-place header/body/path modification with diff preview
- Performance mode for large sessions (>5000 flows)
- Virtual scrolling for efficient flow list rendering

**Phase 3 (Advanced Protocols & Enhanced Rule Engine, complete)**:
- WebSocket interception via `tokio-tungstenite` with full message capture (text, binary, close, ping, pong)
- HTTP/2 stream tracking via `h2` with multiplexed stream state, priority, and window updates
- gRPC call interception with method type detection (unary, server/client/bidirectional streaming)
- Enhanced rule engine: AND/OR/NOT condition combinators, regex, body size thresholds, protocol-specific matching
- Rule actions: Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag
- Rule persistence: JSON file save/load with `save_to_file()`/`load_from_file()`
- Cross-loadout correlation hooks (jwt-to-db, auth, mobile)
- Protocol detail panes in TUI (WebSocket message stream, HTTP/2 stream list, gRPC call inspector)
- Legacy/Enhanced rule view toggle in TUI Rules pane

**Phase 4 (Pipeline, MCP, Evidence, Performance, complete)**:
- Pipeline profile: `ScanProfile::WebProxy` / `Stage::WebProxy` for automated proxy assessments
- MCP proxy surface: 12 tools via `web-proxy-mcp` marker feature (list flows, inspect, edit, rules, session, HAR, evidence bundle, flow actions)
- Evidence bundle v2: `EvidenceBundle` / `BundleManifest` in `proxy/intercept/bundle.rs` with gzip compression and multi-loadout correlation
- Performance: `FlowBuffer` (capacity-capped) and `ProxyMetrics` (telemetry snapshot) in `proxy/intercept/types.rs`
- Real WebSocket backend via `tokio-tungstenite`
- Real HTTP/2 backend via `h2`
- Extended bridge categories: `proxy-websocket-session`, `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`

## Policy Note

**Standalone defense-lab surface** (same pattern as wireless, mobile, and auth-test):

- `operation: "proxy-intercept"`, `mode: DefenseLab`, `risk: TrafficInterception` (real) / `SafeActive` (dry-run), `required_features: ["web-proxy"]`.
- Non-dry-run requires explicit `--allow-web-proxy` (audited; same pattern as `wireless deauth --allow-active-wireless`).
- `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate via `CommandContext::evaluate_and_enforce_operation()`.
- MCP/agent exposure is intentionally absent (standalone defense-lab; no `SecurityTool` registration).
- Always produces policy decision + actions audit even in dry-run.

See `config/policy_decision.rs`, `commands/handlers/web_proxy.rs`, and `proxy/intercept/mod.rs`.

## References

- Source: `crates/eggsec/src/proxy/intercept/` (types, cert, rules, interceptor, bridge, mod)
- CLI: `crates/eggsec/src/cli/web_proxy.rs`
- Handler/policy: `crates/eggsec/src/commands/handlers/web_proxy.rs`
- Output conversion: `crates/eggsec/src/commands/handlers/report.rs` (auto-bridge)
- Architecture: `architecture/proxy.md`
