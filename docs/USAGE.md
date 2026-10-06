# Eggsec Usage Guide

This guide provides detailed examples for common security testing scenarios with Eggsec.

## Build Features

Some features require specific Cargo build flags:

| Feature Flag | Required For |
|--------------|--------------|
| `--features daemon-client` | `daemon`, `session`, `task` |
| `--features rest-api` | `serve`, `mcp-serve`, `codegg-mcp`, `agent` |
| `--features stress-testing` | `stress`, `proxy`, `icmp`, `traceroute` |
| `--features packet-inspection` | `packet capture`, `packet send` (live) |
| `--features nse` | `nse` script execution |
| `--features mobile` / `wireless` | `mobile`, `wireless` (defense-lab) |
| `--features mobile-dynamic` | Dynamic mobile analysis (`mobile-dynamic`, Frida/traffic) |
| `--features web-proxy` / `db-pentest` | `proxy-intercept`, `db` (defense-lab) |
| `--features full` | Curated superset, including `evasion`/`postex`/`c2` which have no standalone `eggsec-cli` feature flag |

The workspace root is a virtual manifest, so builds must name the package:

```bash
# Full build (recommended for pentesting)
cargo build --release -p eggsec-cli --features full
```

See [`BUILD.md`](BUILD.md) for the full feature/system-dependency list.

## Local Lab Targets

Loopback and private targets are blocked by default at two layers. For lab scans against `127.0.0.1`/localhost, opt in explicitly:

```bash
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 eggsec scan 127.0.0.1 --profile quick \
  --scope examples/scope-localhost.toml --allow-private-resolution
```

- `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` opts in to loopback resolution in the probe path.
- `--allow-private-resolution` is the manual-only CLI override for private/loopback targets (audited; see [SAFETY.md](SAFETY.md)).
- Prefer `eggsec plan --scope <file> --target <url>` first: it previews the execution plan without sending traffic.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Port Scanning](#port-scanning)
- [Web Application Testing](#web-application-testing)
- [API Security Testing](#api-security-testing)
- [Advanced Fuzzing](#advanced-fuzzing)
- [CI/CD Integration](#cicd-integration)

## Quick Reference

| Task | Command |
|------|---------|
| Quick port scan | `eggsec scan-ports target.com -p 1-1000` |
| Find hidden paths | `eggsec scan-endpoints https://target.com` |
| Test for SQLi | `eggsec fuzz https://target.com/api?id=1 -t sqli` |
| Test for XSS | `eggsec fuzz https://target.com/search?q=test -t xss` |
| Full web scan | `eggsec scan target.com --profile web` |
| Load test | `eggsec load https://target.com -n 1000 -c 50` |
| Reconnaissance | `eggsec recon target.com` |

## Port Scanning

### Basic Port Scan

Scan the most common 1000 ports:

```bash
eggsec scan-ports example.com
```

### Specific Port Range

Scan a specific range:

```bash
# Scan ports 1-10000
eggsec scan-ports example.com -p 1-10000

# Scan specific ports
eggsec scan-ports example.com -p 22,80,443,3306,5432,6379

# Scan common web ports
eggsec scan-ports example.com -p 80,443,8080,8443,8888
```

### High-Speed Scan

For faster scanning on reliable networks:

```bash
eggsec scan-ports example.com -p 1-65535 -c 200 --timeout 1
```

### Service Fingerprinting

After finding open ports, identify services:

```bash
# Fingerprint discovered services
eggsec fingerprint example.com -p 22,80,443,3306

# Full fingerprint scan
eggsec fingerprint example.com -p 1-1000
```

## Web Application Testing

### Discovery

Find hidden directories and files:

```bash
# Basic endpoint discovery
eggsec scan-endpoints https://example.com

# With custom wordlist
eggsec scan-endpoints https://example.com -w /path/to/wordlist.txt

# Faster discovery
eggsec scan-endpoints https://example.com -c 50
```

### Vulnerability Scanning

#### SQL Injection

```bash
# Test a single parameter
eggsec fuzz "https://example.com/api/user?id=1" -t sqli

# Test multiple parameters
eggsec fuzz "https://example.com/search?q=test&category=1" -t sqli -p q,category

# With specific concurrency
eggsec fuzz "https://example.com/login" -t sqli -c 20 --method POST
```

#### Cross-Site Scripting (XSS)

```bash
# Basic XSS test
eggsec fuzz "https://example.com/search?q=test" -t xss

# Test all inputs with mutation
eggsec fuzz https://example.com -t xss --mutate --mutation-count 10

# Test for stored XSS (requires session handling)
eggsec fuzz https://example.com/comment -t xss --http-session
```

#### Server-Side Request Forgery (SSRF)

```bash
# Test URL parameter
eggsec fuzz "https://example.com/url?url=http://example.com" -t ssrf

# Test for cloud metadata
eggsec fuzz "https://example.com/fetch?url=TEST" -t ssrf
```

#### All Common Vulnerabilities

```bash
# Full web vulnerability scan
eggsec fuzz https://example.com -t all

# With enhanced detection
eggsec fuzz https://example.com -t all --enhanced-redos --diffing --capture-baseline
```

## API Security Testing

### GraphQL

```bash
# Full GraphQL security test
eggsec graphql https://api.example.com/graphql

# Test introspection
eggsec graphql https://api.example.com/graphql --introspection

# Test for injection
eggsec graphql https://api.example.com/graphql --inject

# Test depth limits
eggsec graphql https://api.example.com/graphql --depth-bypass

# Test alias overload DoS
eggsec graphql https://api.example.com/graphql --alias-overload
```

### JWT Testing

```bash
# Test JWT vulnerabilities
eggsec fuzz https://api.example.com/auth -t jwt
```

### OAuth/OIDC

```bash
# Test OAuth security
eggsec oauth https://oauth.example.com/authorize --redirect-test
eggsec oauth https://oauth.example.com/authorize --scope-test
eggsec oauth https://oauth.example.com/authorize --state-test
```

## Advanced Fuzzing

### Adaptive Rate Limiting

Automatically adjusts request rate based on server responses:

```bash
eggsec fuzz https://example.com -t sqli --adaptive-rate
```

### Request Chaining

Chain multiple requests for complex attacks:

```bash
eggsec fuzz https://example.com -t ssrf --chaining --chain-file examples/chain.yaml
```

### Grammar-Based Fuzzing

Generate inputs based on grammar:

```bash
# JSON grammar fuzzing (-t must be a real payload type; -t json is not one)
eggsec fuzz https://example.com/api -t sqli --grammar-fuzz --grammar-type json

# GraphQL grammar fuzzing
eggsec fuzz https://example.com/graphql -t graphql --grammar-fuzz --grammar-type graphql
```

### Target-Specific Payloads

Use payloads tailored to specific technologies:

```bash
# PHP-specific payloads
eggsec fuzz https://example.com -t sqli --target php

# Apache-specific
eggsec fuzz https://example.com -t xss --target apache

# Nginx-specific
eggsec fuzz https://example.com -t xss --target nginx
```

## CI/CD Integration

### SARIF Output (GitHub Advanced Security)

```bash
eggsec fuzz https://example.com -t sqli,xss --format sarif -o results.sarif
```

### JUnit XML (CI Test Reports)

```bash
eggsec fuzz https://example.com -t all --format junit -o results.xml
```

### GitHub Actions Example

```yaml
- name: Security Scan
  run: |
    eggsec fuzz ${{ secrets.TARGET_URL }} -t sqli,xss,ssrf \
      --format sarif -o results.sarif \
      --rate-limit 10

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

### GitLab CI Example

```yaml
security_scan:
  script:
    - eggsec fuzz $TARGET_URL -t sqli,xss --format junit -o gl-sast-report.xml
  artifacts:
    reports:
      sast: gl-sast-report.xml
```

## Load Testing

### Basic Load Test

```bash
eggsec load https://example.com -n 1000 -c 50
```

### POST Request Load Test

```bash
eggsec load https://api.example.com/endpoint \
  -n 500 -c 20 \
  -m POST \
  -d '{"username":"test","password":"test"}'
```

### With Authentication

```bash
eggsec load https://example.com/api \
  -n 100 -c 10 \
  --bearer "your-token"
```

## Reconnaissance

### Full Recon

```bash
eggsec recon example.com
```

### Targeted Recon

```bash
# Skip certain checks
eggsec recon example.com --no-tech --no-whois

# Just tech detection
eggsec recon example.com --no-dns --no-whois --no-subdomains
```

## Using Scope Files

Create a scope file to ensure you only test authorized targets:

```bash
# scope.toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "*.example.com"

[[allowed_targets]]
cidr = "10.0.0.0/8"

[[excluded_targets]]
pattern = "admin.example.com"
```

Use the scope file:

```bash
eggsec scan example.com --scope scope.toml
```

## Rate Limiting and Stealth

Avoid detection and respect target resources:

```bash
# Rate limit to 10 requests/second
eggsec fuzz https://example.com -t all --rate-limit 10

# Add random jitter
eggsec fuzz https://example.com -t all --jitter 100-500

# Full stealth mode
eggsec scan example.com --profile stealth
```

## Output Formats

```bash
# Pretty output (default)
eggsec scan example.com

# JSON
eggsec scan example.com --json

# HTML report
eggsec scan example.com --format html -o report.html

# CSV
eggsec scan example.com --format csv -o results.csv
```

## Stress Testing

> **Warning**: Only use stress testing on systems you own or have explicit written permission to test.
> 
> **Note**: Requires building with `--features stress-testing`:
> ```bash
> cargo build --release -p eggsec-cli --features stress-testing
> ```

### HTTP Stress Test

```bash
eggsec stress example.com --type http -r 1000 -d 60
```

### SYN Flood

```bash
eggsec stress example.com --type syn -r 5000 -d 30
```

### UDP Flood

```bash
eggsec stress 192.168.1.1:80 --type udp -r 10000 -d 120 --payload-size 512
```

### With Proxy Pool

```bash
eggsec stress example.com --type http -r 1000 -d 60 --use-proxies --proxy-file proxies.txt
```

## Proxy Management

> **Note**: Requires building with `--features stress-testing`

### Add Proxies

```bash
eggsec proxy add proxies.txt
```

Proxy file format (one per line):
```
http://127.0.0.1:8080
socks5://user:pass@proxy.example.com:1080
https://proxy2.example.com:443
```

### List Proxies

```bash
# List all proxies
eggsec proxy list

# Show only healthy proxies
eggsec proxy list --healthy

# Verbose output
eggsec proxy list --verbose
```

### Test Proxies

```bash
# Test a single proxy
eggsec proxy test http://127.0.0.1:8080 --test-url https://example.com
```

### Health Check

```bash
eggsec proxy health-check --test-url https://google.com --timeout 10
```

## Distributed Cluster Mode

### Start Coordinator

```bash
eggsec cluster coordinator --port 9000
```

### Start Workers

```bash
eggsec cluster worker --coordinator localhost:9000 --workers 4
```

### Check Cluster Status

```bash
# Local status
eggsec cluster status

# Remote status
eggsec cluster status --coordinator localhost:9000
```

## Notifications

### Test Webhooks

```bash
# Test Slack webhook
eggsec notify test --slack https://hooks.slack.com/services/XXX/YYY/ZZZ

# Test Discord webhook
eggsec notify test --discord https://discord.com/api/webhooks/XXX/YYY

# Test Teams webhook
eggsec notify test --teams https://example.webhook.office.com/XXX

# Test custom webhook
eggsec notify test --webhook https://example.com/hook --secret mysecret
```

### Send Notifications

```bash
# Send to Slack
eggsec notify send "Vulnerability found: SQL Injection" --slack https://hooks.slack.com/services/XXX --severity critical --target example.com

# Send to multiple channels
eggsec notify send "Scan complete" --slack <url> --discord <url> --target example.com
```

### Configuration File

Add webhooks to your config file:

```toml
[[notifications.webhooks]]
name = "slack-alerts"
url = "https://hooks.slack.com/services/XXX"
secret = "your-secret"
events = ["ScanComplete", "Finding"]

[notifications]
slack_webhook = "https://hooks.slack.com/services/XXX"
notify_on_complete = true
notify_on_findings = true
```

## Packet Inspection

> **Note**: Requires building with `--features packet-inspection` for live capture.
> ```bash
> cargo build --release -p eggsec-cli --features packet-inspection
> ```

### Capture Packets

```bash
# Capture from interface (requires root)
eggsec packet capture -i eth0 --max 100

# With BPF filter
eggsec packet capture -i eth0 --filter "tcp port 80" --max 50
```

### Traceroute

```bash
# UDP traceroute (default)
eggsec packet traceroute example.com

# ICMP traceroute (requires root)
eggsec packet traceroute example.com --icmp
```

### Packet Crafting

```bash
# Send TCP SYN packet
eggsec packet send 192.168.1.1 --dst-port 80 --flags SYN

# Send ICMP ping
eggsec packet send 8.8.8.8 --icmp

# Custom payload (hex-encoded)
eggsec packet send example.com:8080 --payload "474554202f20485454502f312e310d0a0d0a"
```

## ICMP Probes

> **Note**: Requires building with `--features stress-testing`

```bash
# Basic ping
eggsec icmp 8.8.8.8

# Multiple probes
eggsec icmp example.com -c 10

# With timeout
eggsec icmp 192.168.1.1 --timeout 5 --json

# Traceroute (UDP mode, default)
eggsec traceroute 8.8.8.8

# Traceroute with ICMP
eggsec traceroute example.com --icmp

# Traceroute with custom settings
eggsec traceroute 192.168.1.1 --max-hops 30 --timeout 5
```

## Report Management

### Convert Reports

Convert scan results between formats. The converter accepts canonical `ScanReportData` JSON. It also accepts native JSON output from standalone defense-lab commands (when the corresponding feature is enabled) via an automatic bridge to `ScanReportData` — so you can pipe their `--json` output without manual conversion.

**Output models (standalone defense-lab surfaces vs. pipeline)**

- **Pipeline scans** (`eggsec scan <target> --profile <p>` and most other assessment commands): produce a full `ScanReportData` (unified findings + metadata). This is loadable via `load_scan_report`, diffable, and exportable to every format (JSON, SARIF, JUnit, HTML, Markdown, CSV) through the `eggsec-output` converters.
- **Standalone defense-lab CLIs** — `wireless`, `mobile`, `db`, `proxy-intercept`, `evasion`, `c2` — emit their own local report types for human-readable output, `--json`, and file writes. Each ships an *optional* `to_scan_report_data()` bridge (plus an auto-bridge inside `report convert`) so native `--json` can flow into the unified SARIF/JUnit/HTML consumers. Use the native shapes for lab-specific workflows; use the bridge (or `report convert` on native JSON) for reporting unification. Bridged findings carry domain-prefixed categories such as `wireless-*`, `mobile-android-*` / `mobile-ios-*`, `db-postgres-*`, `proxy-intercept-flow`, `evasion-*`, and `c2-*`. See the per-module docs for exact category names and bridge caveats.
- **`auth-test`** intentionally emits only local `AuthTestReport` / `AuthFinding` types. There is **no** `to_scan_report_data` bridge and no SARIF/JUnit/etc. path. It is deliberately kept outside the unified reporting system. Distinct from the pipeline `ScanProfile::Auth`, which does produce `ScanReportData`. See [`AUTH_LAB.md`](AUTH_LAB.md) and `architecture/auth.md`.

```bash
# Convert canonical or bridged JSON to HTML
eggsec report convert input.json -f html -o report.html

# Convert JSON to CSV
eggsec report convert results.json -f csv -o results.csv

# Convert to SARIF (for CI/CD)
eggsec report convert scan.json -f sarif -o results.sarif

# Convert to JUnit XML (for test integration)
eggsec report convert scan.json -f junit -o results.xml

# Convert to Markdown
eggsec report convert scan.json -f markdown -o report.md

# Wireless (native --json from defense-lab command; auto-bridged)
eggsec wireless wlan0 scan --json -o wireless.json
eggsec report convert wireless.json -f sarif -o wireless.sarif
eggsec report convert wireless.json -f junit -o wireless.xml

# Mobile (native --json; auto-bridged)
eggsec mobile app.apk --json -o mobile.json
eggsec report convert mobile.json -f html -o mobile.html
eggsec report convert mobile.json -f markdown -o mobile.md
```

Per-module detail lives in [`WIRELESS.md`](WIRELESS.md), [`MOBILE.md`](MOBILE.md), [`DATABASE_PENTEST.md`](DATABASE_PENTEST.md), [`WEB_PROXY.md`](WEB_PROXY.md), and the matching `architecture/*.md` files.

### Trend Analysis

Compare scan results over time:

```bash
eggsec report trend before.json after.json -o trends.json
```

### Scheduled Scans

Manage scheduled scans:

```bash
# List scheduled scans
eggsec report schedule list

# Add scheduled scan (cron expression)
eggsec report schedule add "0 */6 * * *" example.com --scan-type web

# Generate crontab entry
eggsec report schedule cron

# Remove scheduled scan
eggsec report schedule remove <id>
```

## Remote Execution

Eggsec supports remote execution via a listener/agent architecture for distributed command execution.

### Generate Authentication Key

```bash
# Generate a pre-shared key for authentication
eggsec remote generate-key
```

### Generate TLS Certificate

```bash
# Generate instructions for TLS cert
eggsec remote cert --openssl
```

### Start Remote Listener

```bash
# Start on default port (7890)
eggsec remote start

# Start with custom port and PSK
eggsec remote start --port 9000 --auth your-psk

# Start with TLS (PEM cert + key)
eggsec remote start --port 9000 --tls-cert cert.pem --tls-key key.pem
```

### Execute Remote Commands

```bash
# Execute on single target
eggsec exec --target 192.168.1.1:7890 --auth your-psk "scan-ports example.com -p 1-1000"

# Execute on multiple targets
eggsec exec --targets targets.txt --auth your-psk "recon example.com"

# With TLS
eggsec exec --target host:7890 --tls-cert cert.pem "fuzz https://example.com -t xss"
```

## TUI Appearance (Themes)

The interactive TUI supports 50 packaged Halloy-format themes plus three
built-in fallback themes (`cyber-red`, `dark`, `light`).

### Changing Themes

| Method | How |
|--------|-----|
| Cycle through all installed themes (alphabetical, wraps) | `Ctrl+T` |
| Pick a specific theme from a dropdown | Open `Settings` tab → `Theme` section |
| Run a palette command | `Ctrl+P` → type `theme` → Enter |

Each theme switch shows a `Theme: <Display Name>` notification so you know
the new theme was applied.

### Adding Custom Themes

Drop a Halloy-format `.toml` file into:

```
~/.config/eggsec/themes/<name>.toml
```

The TUI loads these on startup. Bundled themes that are missing from the
user's directory are auto-installed on first run (idempotently - existing
files are never overwritten). `cyber-red` is always available as a fallback
independent of file system access.

See `crates/eggsec-tui/src/theme/loader.rs` for the parsed TOML schema
(matches Halloy's `themes` format: `[general]`, `[palette]`, `[buffer]`,
`[buttons]`).

### Theme Persistence

The selected theme name is saved with the session
(`~/.local/share/eggsec/sessions/`) and restored on next launch. If the
saved theme was a packaged one that has not yet been installed when the
session starts, the restore is deferred and applied once the background
loader reports back (handled by `ThemeLoadState`).

### Built-in vs Custom

- **Built-in trio** (`cyber-red`, `dark`, `light`): compiled into the binary
  via `theme/builtin.rs`. Always available; `cyber-red` is the default
  fallback.
- **Packaged themes** (50 Halloy themes): compiled into the binary as an
  LZMA-compressed blob (`theme/packaged.rs`, regenerated by
  `scripts/package_themes.py`). Installed to `~/.config/eggsec/themes/` on
  first run.
- **User themes**: any `.toml` files you place in
  `~/.config/eggsec/themes/`. Loaded alongside packaged themes.

If a theme's `.toml` fails to load, the failure is logged and surfaced via
a `Warning` notification. The selector still shows the theme as a
placeholder marked `[! <name>] (not installed)` so you can see what was
selected even when it's not actually applied.

## TUI Quick Reference

| Action | Key |
|--------|-----|
| Open the TUI | Run `eggsec` with no subcommand (in a terminal) |
| Quit (no active task) | `q` |
| Interrupt running task | `Ctrl+C` |
| Command palette | `Ctrl+P` |
| Quick tab switch (fuzzy search) | `Ctrl+X` |
| Global search | `Ctrl+F` |
| Cycle theme | `Ctrl+T` |
| Toggle help overlay | `Space` |
| Bookmark current tab | `Ctrl+B` |
| Export results | `Shift+E` |
| Pause/resume task updates | `Ctrl+Z` / `Ctrl+Y` |
| Jump to tab by number | `1`..`9`, `0` for tab 10 |
| Save settings (in Settings tab) | `s` |

