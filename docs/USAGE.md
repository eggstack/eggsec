# Eggsec Usage Guide

This guide provides detailed examples for common security testing scenarios with Eggsec.

## Build Features

Some features require specific Cargo build flags:

| Feature Flag | Required For |
|--------------|--------------|
| `--features stress-testing` | `stress`, `proxy`, `icmp`, `traceroute` |
| `--features packet-inspection` | `packet capture`, `packet send` (live) |
| `--features nse` | NSE script execution |
| `--features full` | All features |

```bash
# Full build (recommended for pentesting)
cargo build --release --features full
```

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
eggsec fuzz https://example.com -t xss --mutate -m 10

# Test for stored XSS (requires session handling)
eggsec fuzz https://example.com/comment -t xss --session
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
eggsec o-auth https://oauth.example.com/authorize --redirect-test
eggsec o-auth https://oauth.example.com/authorize --scope-test
eggsec o-auth https://oauth.example.com/authorize --state-test
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
# JSON fuzzing
eggsec fuzz https://example.com/api -t json --grammar-fuzz --grammar-type json

# GraphQL fuzzing
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
eggsec fuzz https://example.com -t sqli,xss --sarif -o results.sarif
```

### JUnit XML (CI Test Reports)

```bash
eggsec fuzz https://example.com -t all --junit -o results.xml
```

### GitHub Actions Example

```yaml
- name: Security Scan
  run: |
    eggsec fuzz ${{ secrets.TARGET_URL }} -t sqli,xss,ssrf \
      --sarif -o results.sarif \
      --rate-limit 10

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v2
  with:
    sarif_file: results.sarif
```

### GitLab CI Example

```yaml
security_scan:
  script:
    - eggsec fuzz $TARGET_URL -t sqli,xss --junit -o gl-sast-report.xml
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
> cargo build --release --features stress-testing
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
eggsec proxy add --file proxies.txt
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
> cargo build --release --features packet-inspection
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

# Custom payload
eggsec packet send example.com:8080 --payload "GET / HTTP/1.1\r\n\r\n"
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
eggsec traceroute 192.168.1.1 --max-hops 30 --probes 5
```

## Report Management

### Convert Reports

Convert scan results between formats. The converter accepts canonical `ScanReportData` JSON. It also accepts native JSON output from standalone defense-lab commands (`eggsec wireless` and `eggsec mobile`, when the corresponding feature is enabled) via an automatic bridge to `ScanReportData` — so you can directly pipe their `--json` outputs without manual conversion.

**Output Models (standalone defense-lab surfaces vs. pipeline)**

- **Pipeline scans** (`eggsec scan <target> --profile <p>` and most other assessment commands): always produce a full `ScanReportData` (unified findings + metadata). This is loadable via `load_scan_report`, diffable, and exportable to every format (JSON, SARIF, JUnit, HTML, Markdown, CSV, etc.) through the `eggsec-output` converters.
- **Wireless / Mobile** (standalone defense-lab CLIs under their feature flags): emit their native local types directly (`WirelessScanResult` or `MobileScanReport`) for human-readable output, `--json`, and file writes. They also provide an *optional* `to_scan_report_data()` bridge (plus an auto-bridge inside `report convert`) so native `--json` can flow into the unified SARIF/JUnit/HTML/etc. consumers when desired. Use native shapes for lab-specific workflows and repeated-scan summaries; use the bridge (or `report convert` on native JSON) for reporting unification. Categories in bridged output are `wireless-*` or `mobile-{android,ios}-*`. See docs/WIRELESS.md and docs/MOBILE.md ("Integration with Reporting Pipeline" sections) and the per-module architecture docs. MCP/agent tool exposure is intentionally absent for wireless (design decision; not a SecurityTool; see architecture/wireless.md MCP/Agentic section and plans/wireless-tui-mcp-agentic-handoff-plan.md resolution note). Active wireless (Phase 1+ under `wireless-advanced`, per `plans/wireless-active-attacks-loadout-design-plan.md`) will emit native results + extend the optional bridge with `wireless-active-*` categories while preserving the standalone defense-lab model and MCP-absent design. See docs/WIRELESS.md Integration section.
- **`auth-test`** (standalone defense-lab CLI): intentionally produces and emits only local `AuthTestReport` / `AuthFinding` types (direct text or `--json` from the handler). There is **no** `to_scan_report_data` bridge, no `FindingData` / `ScanReportData` conversion, and no SARIF/JUnit/etc. path. It is deliberately kept outside the unified reporting system to preserve its narrow "credential control validation in authorized labs" purpose. Distinct from the pipeline `ScanProfile::Auth` (which does produce `ScanReportData`). See docs/AUTH_LAB.md ("Output Model (Local Findings Only)" section) and architecture/auth.md.

The three models are summarized here for discoverability; the detailed rationale and examples live in the linked per-module docs.

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
eggsec wireless wlan0 --json -o wireless.json
eggsec report convert wireless.json -f sarif -o wireless.sarif
eggsec report convert wireless.json -f junit -o wireless.xml

# Mobile (native --json; auto-bridged)
eggsec mobile app.apk --json -o mobile.json
eggsec report convert mobile.json -f html -o mobile.html
eggsec report convert mobile.json -f markdown -o mobile.md
```

See the "Output Models" block above, plus `docs/WIRELESS.md` (Integration with Reporting Pipeline) and `docs/MOBILE.md` (same) for when to use the native types vs. the optional bridge, and for notes on rogue-in-bridge and category naming. `auth-test` has no bridge (see its section above and docs/AUTH_LAB.md). See architecture/wireless.md (MCP/Agentic section) for why MCP/agent tool exposure is intentionally absent for wireless (and the handoff plan resolution). Active wireless future per `plans/wireless-active-attacks-loadout-design-plan.md`.

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

# Start with TLS
eggsec remote start --port 9000 --tls-cert cert.p12 --tls-password password
```

### Execute Remote Commands

```bash
# Execute on single target
eggsec exec --target 192.168.1.1:7890 --auth your-psk "scan-ports example.com -p 1-1000"

# Execute on multiple targets
eggsec exec --targets targets.txt --auth your-psk "recon example.com"

# With TLS
eggsec exec --target host:7890 --tls-cert cert.p12 "fuzz https://example.com -t xss"
```
