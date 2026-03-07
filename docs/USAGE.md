# Slapper Usage Guide

This guide provides detailed examples for common security testing scenarios with Slapper.

## Build Features

Some features require specific Cargo build flags:

| Feature Flag | Required For |
|--------------|--------------|
| `--features stress-testing` | `stress`, `proxy`, `icmp`, `traceroute` |
| `--features packet-inspection` | `packet capture`, `packet send` (live) |
| `--features python-plugins` | Python plugin support |
| `--features ruby-plugins` | Ruby plugins, Metasploit integration |
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
| Quick port scan | `slapper scan-ports target.com -p 1-1000` |
| Find hidden paths | `slapper scan-endpoints https://target.com` |
| Test for SQLi | `slapper fuzz https://target.com/api?id=1 -t sqli` |
| Test for XSS | `slapper fuzz https://target.com/search?q=test -t xss` |
| Full web scan | `slapper scan target.com --profile web` |
| Load test | `slapper load https://target.com -n 1000 -c 50` |
| Reconnaissance | `slapper recon target.com` |

## Port Scanning

### Basic Port Scan

Scan the most common 1000 ports:

```bash
slapper scan-ports example.com
```

### Specific Port Range

Scan a specific range:

```bash
# Scan ports 1-10000
slapper scan-ports example.com -p 1-10000

# Scan specific ports
slapper scan-ports example.com -p 22,80,443,3306,5432,6379

# Scan common web ports
slapper scan-ports example.com -p 80,443,8080,8443,8888
```

### High-Speed Scan

For faster scanning on reliable networks:

```bash
slapper scan-ports example.com -p 1-65535 -c 200 --timeout 1
```

### Service Fingerprinting

After finding open ports, identify services:

```bash
# Fingerprint discovered services
slapper fingerprint example.com -p 22,80,443,3306

# Full fingerprint scan
slapper fingerprint example.com -p 1-1000
```

## Web Application Testing

### Discovery

Find hidden directories and files:

```bash
# Basic endpoint discovery
slapper scan-endpoints https://example.com

# With custom wordlist
slapper scan-endpoints https://example.com -w /path/to/wordlist.txt

# Faster discovery
slapper scan-endpoints https://example.com -c 50
```

### Vulnerability Scanning

#### SQL Injection

```bash
# Test a single parameter
slapper fuzz "https://example.com/api/user?id=1" -t sqli

# Test multiple parameters
slapper fuzz "https://example.com/search?q=test&category=1" -t sqli -p q,category

# With specific concurrency
slapper fuzz "https://example.com/login" -t sqli -c 20 --method POST
```

#### Cross-Site Scripting (XSS)

```bash
# Basic XSS test
slapper fuzz "https://example.com/search?q=test" -t xss

# Test all inputs with mutation
slapper fuzz https://example.com -t xss --mutate -m 10

# Test for stored XSS (requires session handling)
slapper fuzz https://example.com/comment -t xss --session
```

#### Server-Side Request Forgery (SSRF)

```bash
# Test URL parameter
slapper fuzz "https://example.com/url?url=http://example.com" -t ssrf

# Test for cloud metadata
slapper fuzz "https://example.com/fetch?url=TEST" -t ssrf
```

#### All Common Vulnerabilities

```bash
# Full web vulnerability scan
slapper fuzz https://example.com -t all

# With enhanced detection
slapper fuzz https://example.com -t all --enhanced-redos --diffing --capture-baseline
```

## API Security Testing

### GraphQL

```bash
# Full GraphQL security test
slapper graphql https://api.example.com/graphql

# Test introspection
slapper graphql https://api.example.com/graphql --introspection

# Test for injection
slapper graphql https://api.example.com/graphql --inject

# Test depth limits
slapper graphql https://api.example.com/graphql --depth-bypass

# Test alias overload DoS
slapper graphql https://api.example.com/graphql --alias-overload
```

### JWT Testing

```bash
# Test JWT vulnerabilities
slapper fuzz https://api.example.com/auth -t jwt
```

### OAuth/OIDC

```bash
# Test OAuth security
slapper o-auth https://oauth.example.com/authorize --redirect-test
slapper o-auth https://oauth.example.com/authorize --scope-test
slapper o-auth https://oauth.example.com/authorize --state-test
```

## Advanced Fuzzing

### Adaptive Rate Limiting

Automatically adjusts request rate based on server responses:

```bash
slapper fuzz https://example.com -t sqli --adaptive-rate
```

### Request Chaining

Chain multiple requests for complex attacks:

```bash
slapper fuzz https://example.com -t ssrf --chaining --chain-file examples/chain.yaml
```

### Grammar-Based Fuzzing

Generate inputs based on grammar:

```bash
# JSON fuzzing
slapper fuzz https://example.com/api -t json --grammar-fuzz --grammar-type json

# GraphQL fuzzing
slapper fuzz https://example.com/graphql -t graphql --grammar-fuzz --grammar-type graphql
```

### Target-Specific Payloads

Use payloads tailored to specific technologies:

```bash
# PHP-specific payloads
slapper fuzz https://example.com -t sqli --target php

# Apache-specific
slapper fuzz https://example.com -t xss --target apache

# Nginx-specific
slapper fuzz https://example.com -t xss --target nginx
```

## CI/CD Integration

### SARIF Output (GitHub Advanced Security)

```bash
slapper fuzz https://example.com -t sqli,xss --sarif -o results.sarif
```

### JUnit XML (CI Test Reports)

```bash
slapper fuzz https://example.com -t all --junit -o results.xml
```

### GitHub Actions Example

```yaml
- name: Security Scan
  run: |
    slapper fuzz ${{ secrets.TARGET_URL }} -t sqli,xss,ssrf \
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
    - slapper fuzz $TARGET_URL -t sqli,xss --junit -o gl-sast-report.xml
  artifacts:
    reports:
      sast: gl-sast-report.xml
```

## Load Testing

### Basic Load Test

```bash
slapper load https://example.com -n 1000 -c 50
```

### POST Request Load Test

```bash
slapper load https://api.example.com/endpoint \
  -n 500 -c 20 \
  -m POST \
  -d '{"username":"test","password":"test"}'
```

### With Authentication

```bash
slapper load https://example.com/api \
  -n 100 -c 10 \
  --bearer "your-token"
```

## Reconnaissance

### Full Recon

```bash
slapper recon example.com
```

### Targeted Recon

```bash
# Skip certain checks
slapper recon example.com --no-tech --no-whois

# Just tech detection
slapper recon example.com --no-dns --no-whois --no-subdomains
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
slapper scan example.com --scope scope.toml
```

## Rate Limiting and Stealth

Avoid detection and respect target resources:

```bash
# Rate limit to 10 requests/second
slapper fuzz https://example.com -t all --rate-limit 10

# Add random jitter
slapper fuzz https://example.com -t all --jitter 100-500

# Full stealth mode
slapper scan example.com --profile stealth
```

## Output Formats

```bash
# Pretty output (default)
slapper scan example.com

# JSON
slapper scan example.com --json

# HTML report
slapper scan example.com --format html -o report.html

# CSV
slapper scan example.com --format csv -o results.csv
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
slapper stress example.com --type http -r 1000 -d 60
```

### SYN Flood

```bash
slapper stress example.com --type syn -r 5000 -d 30
```

### UDP Flood

```bash
slapper stress 192.168.1.1:80 --type udp -r 10000 -d 120 --payload-size 512
```

### With Proxy Pool

```bash
slapper stress example.com --type http -r 1000 -d 60 --use-proxies --proxy-file proxies.txt
```

## Proxy Management

> **Note**: Requires building with `--features stress-testing`

### Add Proxies

```bash
slapper proxy add --file proxies.txt
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
slapper proxy list

# Show only healthy proxies
slapper proxy list --healthy

# Verbose output
slapper proxy list --verbose
```

### Test Proxies

```bash
# Test a single proxy
slapper proxy test http://127.0.0.1:8080 --test-url https://example.com
```

### Health Check

```bash
slapper proxy health-check --test-url https://google.com --timeout 10
```

## Distributed Cluster Mode

### Start Coordinator

```bash
slapper cluster coordinator --port 9000
```

### Start Workers

```bash
slapper cluster worker --coordinator localhost:9000 --workers 4
```

### Check Cluster Status

```bash
# Local status
slapper cluster status

# Remote status
slapper cluster status --coordinator localhost:9000
```

## Notifications

### Test Webhooks

```bash
# Test Slack webhook
slapper notify test --slack https://hooks.slack.com/services/XXX/YYY/ZZZ

# Test Discord webhook
slapper notify test --discord https://discord.com/api/webhooks/XXX/YYY

# Test Teams webhook
slapper notify test --teams https://example.webhook.office.com/XXX

# Test custom webhook
slapper notify test --webhook https://example.com/hook --secret mysecret
```

### Send Notifications

```bash
# Send to Slack
slapper notify send "Vulnerability found: SQL Injection" --slack https://hooks.slack.com/services/XXX --severity critical --target example.com

# Send to multiple channels
slapper notify send "Scan complete" --slack <url> --discord <url> --target example.com
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
slapper packet capture -i eth0 --max 100

# With BPF filter
slapper packet capture -i eth0 --filter "tcp port 80" --max 50
```

### Traceroute

```bash
# UDP traceroute (default)
slapper packet traceroute example.com

# ICMP traceroute (requires root)
slapper packet traceroute example.com --icmp
```

### Packet Crafting

```bash
# Send TCP SYN packet
slapper packet send 192.168.1.1 --dst-port 80 --flags SYN

# Send ICMP ping
slapper packet send 8.8.8.8 --icmp

# Custom payload
slapper packet send example.com:8080 --payload "GET / HTTP/1.1\r\n\r\n"
```

## ICMP Probes

> **Note**: Requires building with `--features stress-testing`

```bash
# Basic ping
slapper icmp 8.8.8.8

# Multiple probes
slapper icmp example.com -c 10

# With timeout
slapper icmp 192.168.1.1 --timeout 5 --json

# Traceroute (UDP mode, default)
slapper traceroute 8.8.8.8

# Traceroute with ICMP
slapper traceroute example.com --icmp

# Traceroute with custom settings
slapper traceroute 192.168.1.1 --max-hops 30 --probes 5
```

## Report Management

### Convert Reports

Convert scan results between formats:

```bash
# Convert JSON to HTML
slapper report convert input.json -f html -o report.html

# Convert JSON to CSV
slapper report convert results.json -f csv -o results.csv

# Convert to SARIF (for CI/CD)
slapper report convert scan.json -f sarif -o results.sarif

# Convert to JUnit XML (for test integration)
slapper report convert scan.json -f junit -o results.xml

# Convert to Markdown
slapper report convert scan.json -f markdown -o report.md
```

### Trend Analysis

Compare scan results over time:

```bash
slapper report trend before.json after.json -o trends.json
```

### Scheduled Scans

Manage scheduled scans:

```bash
# List scheduled scans
slapper report schedule list

# Add scheduled scan (cron expression)
slapper report schedule add "0 */6 * * *" example.com --scan-type web

# Generate crontab entry
slapper report schedule cron

# Remove scheduled scan
slapper report schedule remove <id>
```

## Plugin Management

> **Note**: Requires building with `--features python-plugins` or `--features ruby-plugins`

### List Plugins

```bash
slapper plugin list
slapper plugin list --verbose
```

### Run a Plugin

```bash
slapper plugin run my_plugin https://example.com
slapper plugin run my_plugin https://example.com -o results.json
```

## Remote Execution

Slapper supports remote execution via a listener/agent architecture for distributed command execution.

### Generate Authentication Key

```bash
# Generate a pre-shared key for authentication
slapper remote generate-key
```

### Generate TLS Certificate

```bash
# Generate instructions for TLS cert
slapper remote cert --openssl
```

### Start Remote Listener

```bash
# Start on default port (7890)
slapper remote start

# Start with custom port and PSK
slapper remote start --port 9000 --auth your-psk

# Start with TLS
slapper remote start --port 9000 --tls-cert cert.p12 --tls-password password
```

### Execute Remote Commands

```bash
# Execute on single target
slapper exec --target 192.168.1.1:7890 --auth your-psk "scan-ports example.com -p 1-1000"

# Execute on multiple targets
slapper exec --targets targets.txt --auth your-psk "recon example.com"

# With TLS
slapper exec --target host:7890 --tls-cert cert.p12 "fuzz https://example.com -t xss"
```
