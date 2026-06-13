# Eggsec Web Proxy Loadout — Release Notes

## Version

Available in Eggsec with `--features web-proxy` (or `--features full`).

## What's New

### Interactive MITM Web Proxy

Eggsec now includes a fully interactive man-in-the-middle proxy for intercepting and inspecting HTTP/HTTPS traffic in authorized lab environments.

**Key capabilities:**
- Dynamic TLS certificate generation with per-host caching
- Request/response interception, inspection, and modification
- WebSocket, HTTP/2, and gRPC protocol support
- Complex rule engine with AND/OR/NOT conditions
- Evidence bundle export with HMAC-SHA256 integrity signing
- Cross-loadout correlation (links proxy findings with database, auth, mobile, and wireless tests)
- Attack narrative generation from session reports
- 12 MCP tools for agent and automation integration

### Safety Model

The proxy operates under the same defense-lab safety model as other Eggsec standalone surfaces:

- **Dry-run always safe**: Produces a complete report with synthetic flows, zero network activity
- **Real interception gated**: Requires `--allow-web-proxy` + policy confirmation
- **Private IP blocking**: Connections to RFC 1918, loopback, multicast, and broadcast addresses are rejected
- **CRLF injection prevention**: Header values are validated against injection attacks
- **Budget enforcement**: Configurable limits on flows, bytes, duration, and concurrency
- **Audit trail**: Every manipulation is recorded with before/after values, timestamps, and reasons

### TUI Integration

The `Tab::Intercept` TUI tab provides:

- Live flow list with virtual scrolling for high-volume sessions
- Header/body/manipulation/rules/timeline detail panes
- Search and filter (`/` key) for finding specific flows
- Session save/load and HAR 1.2 export
- Edit modal for in-place request/response modification
- Performance mode for sessions with >5000 flows

### Pipeline Integration

- `ScanProfile::WebProxy` / `Stage::WebProxy` for automated assessments
- `eggsec scan --profile web-proxy` for pipeline-based proxy testing
- Auto-bridge in `report convert` for unified reporting

### MCP Proxy Surface

12 MCP tools (via `web-proxy-mcp` feature) for agent integration:

| Tool | Description |
|------|-------------|
| `proxy-start` | Start the intercepting proxy |
| `proxy-stop` | Stop and finalize session |
| `proxy-status` | Session status and budget usage |
| `proxy-list-flows` | List intercepted flows |
| `proxy-inspect-flow` | Full flow detail |
| `proxy-forward-flow` | Forward a paused flow |
| `proxy-drop-flow` | Drop a paused flow |
| `proxy-replay-flow` | Replay a flow |
| `proxy-add-rule` | Add an intercept rule |
| `proxy-list-rules` | List configured rules |
| `proxy-remove-rule` | Remove a rule |
| `proxy-export-session` | Export as JSON or HAR |

## Quick Start

```bash
# Dry-run (safe, no network activity)
eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json -o report.json

# Interactive TUI
# Launch eggsec-tui, navigate to Intercept tab

# Convert to SARIF
eggsec report convert report.json -f sarif -o report.sarif
```

## Browser/OS Trust Store

For HTTPS interception, install the generated CA certificate:

- **Firefox**: Settings → Privacy & Security → Certificates → Import CA PEM
- **Chrome**: Settings → Privacy and Security → Security → Manage certificates → Import
- **macOS**: `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ca-cert.pem`
- **Windows**: `certutil -addstore -f "Root" ca-cert.pem`
- **Linux**: `sudo cp ca-cert.pem /usr/local/share/ca-certificates/eggsec-proxy.crt && sudo update-ca-certificates`

## Limitations

- Certificate pinning bypass requires additional instrumentation
- mTLS endpoints require client certificate configuration to the proxy
- Transparent proxy mode (iptables redirect) is not supported
- Binary protobuf editing is best-effort for common schemas
- Streaming body capture logs complete bodies only

## Security

See `docs/internal/WEB_PROXY_SECURITY_AUDIT.md` for the code-level security audit.

Key security controls:
- CRLF injection prevention in header manipulation
- Private IP blocking for LAN protection
- HMAC-SHA256 bundle integrity verification
- Policy enforcement via `EnforcementContext::evaluate()`
- Thread-safe certificate caching with `parking_lot::RwLock`
