# Web Proxy Examples

Ready-to-run examples for the Eggsec interactive web proxy loadout.

## Quick Start

```bash
# Dry-run: complete report, no network activity
eggsec proxy-intercept --dry-run --json -o report.json

# Dry-run with custom scope
eggsec proxy-intercept --dry-run --json --scope examples/lab-proxy-scope.toml -o report.json

# Interactive TUI: launch eggsec-tui, navigate to Intercept tab, press Enter
```

## Examples

| File | Description |
|------|-------------|
| `intercept-rules.yaml` | Example intercept rules (block, allow, modify) |
| `web-proxy-config.toml` | Example `eggsec.toml` section for web-proxy settings |
| `scope.toml` | Example scope restricting proxy targets to lab hosts |

## Prerequisites

1. Build with `web-proxy` feature:
   ```bash
   cargo build --release -p eggsec-cli --features web-proxy
   ```

2. For HTTPS interception, install the generated CA certificate into your browser/OS trust store. See `docs/WEB_PROXY.md` for platform-specific instructions.

3. Configure your browser or application to use the proxy:
   - HTTP proxy: `http://127.0.0.1:8080`
   - SOCKS5 proxy: `socks5://127.0.0.1:8080` (if supported)

## Scenarios

### Scenario 1: Lab Assessment (Dry-Run)

```bash
# Generate a dry-run report with synthetic flows
eggsec proxy-intercept --listen 127.0.0.1:8080 --dry-run --json -o lab-report.json

# Convert to SARIF for CI/CD integration
eggsec report convert lab-report.json -f sarif -o lab-report.sarif

# Convert to HTML for human review
eggsec report convert lab-report.json -f html -o lab-report.html
```

### Scenario 2: Live Traffic Interception

```bash
# Start the proxy server (requires --allow-web-proxy)
eggsec proxy-intercept \
  --listen 127.0.0.1:8080 \
  --allow-web-proxy \
  --manual-override-reason "Authorized lab testing" \
  --scope examples/scope.toml \
  --max-flows 500 \
  --max-duration 120 \
  --json -o live-report.json

# Apply intercept rules at runtime
eggsec proxy-intercept \
  --listen 127.0.0.1:8080 \
  --allow-web-proxy \
  --manual-override-reason "Lab testing" \
  --intercept-rule "*:*:monitor" \
  --intercept-rule "evil.com:*:block" \
  --intercept-rule "example.com:/api/*:intercept" \
  --json -o rule-report.json
```

### Scenario 3: Interactive TUI Session

```bash
# Launch TUI and navigate to Intercept tab
eggsec-tui

# In the Intercept tab:
# 1. Configure listen address and dry-run toggle
# 2. Press Enter to start the session
# 3. Use arrow keys to navigate flows
# 4. Tab to cycle between Flow List, Detail View, and Action Bar
# 5. Use 'e' to open the edit modal for header/body modification
# 6. Save session or export HAR from the Action Bar
```

### Scenario 4: Pipeline Integration

```bash
# Run the web-proxy pipeline profile
eggsec scan 127.0.0.1 --profile web-proxy --scope examples/scope.toml
```

### Scenario 5: MCP Agent Integration

```bash
# Build with MCP tools
cargo build --release -p eggsec-cli --features web-proxy-mcp

# The 12 MCP tools are available for agent automation:
# - proxy_list_flows, proxy_inspect_flow, proxy_edit_request/response
# - proxy_manage_rules, proxy_session_save/load, proxy_har_export
# - proxy_evidence_bundle, proxy_forward/drop/replay_flow
```

## Intercept Rules Reference

Rules use the format `host:path:action`:

| Action | Description |
|--------|-------------|
| `allow` | Pass traffic through without interception |
| `block` | Block the connection entirely |
| `intercept` | Capture and log the full request/response |
| `monitor` | Log the flow without modifying it |
| `modify` | Apply header/body modifications (use TUI for editing) |

### Pattern Matching

- `*` — Match all hosts or paths
- `*.example.com` — Match any subdomain of example.com
- `/api/*` — Match any path under /api/
- `example.com:/api/v1/users` — Match exact host and path

### Priority

Rules are evaluated in priority order (higher first). First match wins. Default action when no rule matches is `Allow`.

## Resources

- [docs/WEB_PROXY.md](../../docs/WEB_PROXY.md) — Complete documentation
- [architecture/web_proxy.md](../../architecture/web_proxy.md) — Architecture reference
- [lab-proxy-scope.toml](../lab-proxy-scope.toml) — Example lab scope
