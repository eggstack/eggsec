---
name: eggsec-security
description: "Security testing skill for using Eggsec effectively - use when learning Eggsec capabilities, TUI navigation, CLI commands, configuration, output formats, scope rules, or feature flags."
---

# Eggsec Security Testing Skill

## Overview

Eggsec is a Rust-based security testing toolkit for penetration testing, vulnerability scanning, and security assessment. This skill teaches agents how to use Eggsec effectively for security testing workflows.

## TUI Mode

Eggsec features an interactive terminal UI (TUI) built with ratatui:

```bash
# Start TUI mode (default)
eggsec

# Navigate tabs with n/p or arrow keys
# Press Enter to start operations
# Use / to search, Ctrl+P for command palette
```

Key TUI features:
- 33 tabs covering all security testing functions
- Real-time progress monitoring with spinners
- Session persistence (resume previous scans)
- Bookmark favorite tabs with Ctrl+B
- Command palette (Ctrl+P) for quick navigation
- Help overlay (Space) with tab-specific commands

Tab navigation:
- `n` / `p` - Next/previous tab
- `1-9` / `0` - Jump to tab 1-10
- `h` / `l` - Within-tab left/right movement
- `gg` / `G` - Go to top/bottom

## Core Capabilities

### 1. Reconnaissance

Passive information gathering about targets (single `recon` command; skip stages with `--no-*` flags):

```bash
# Full recon suite (13 parallel tasks)
eggsec recon example.com

# Skip specific stages
eggsec recon https://example.com --no-tech --no-geo --no-wayback

# Other useful skips: --no-dns, --no-subdomains, --no-ssl,
# --no-whois, --no-js, --no-content, --no-cloud, --no-cors,
# --no-threat, --no-cve, --no-dns-records
```

### 2. Port Scanning

Network discovery and service fingerprinting:

```bash
# Basic port scan (default range 1-1024)
eggsec scan-ports 192.168.1.1

# Custom port range
eggsec scan-ports 192.168.1.1 -p 1-65535

# With IP spoofing (stress-testing feature, Unix only)
eggsec scan-ports 192.168.1.1 --source-ip 10.0.0.1
```

### 3. Endpoint Discovery

Finding web application endpoints:

```bash
# Discover endpoints (347 built-in paths)
eggsec scan-endpoints https://example.com

# With custom wordlist
eggsec scan-endpoints https://example.com --wordlist paths.txt
```

### 4. Fuzzing

Security payload testing across 40 payload types (URL is positional; `-t` selects types):

```bash
# SQL injection fuzzing
eggsec fuzz "https://example.com/api?id=1" -t sqli

# XSS fuzzing
eggsec fuzz "https://example.com/search?q=test" -t xss

# Multiple types
eggsec fuzz "https://example.com/fetch?url=" -t ssrf,redirect

# All payload types (default)
eggsec fuzz "https://example.com/page?x=1"
```

All 40 payload types: sqli, xss, traversal, ssrf, redirect, redos, headers, compression, graphql, oauth, jwt, idor, ssti, grpc, xxe, ldap, cmd, deser, host, cache, csv, soap, websocket, nosql, xpath, expression, prototype, race, massassign, oast, saml, htmlinject, cssinject, ssi, domclobber, xslt, viewstate, depconfusion, xsleak, latex

Note: `traversal` is the correct type name for path traversal (not `path-traversal`). The fuzz URL must contain the parameter(s) to test.

### 5. WAF Detection & Bypass

```bash
# Detect WAF only
eggsec waf https://example.com -d

# Detection + evasion-resistance testing
eggsec waf https://example.com -b
```

### 6. Load Testing

```bash
# HTTP load test (-n requests, -c concurrency)
eggsec load https://example.com -n 10000 -c 100

# Rate limiting
eggsec load https://example.com/api -n 5000 --rate-limit 50
```

### 7. Pipeline Mode

Run comprehensive security assessments with the `scan` command and a profile:

```bash
# Quick profile: port scan + fingerprint
eggsec scan example.com --profile quick

# Full pipeline with output
eggsec scan example.com --profile full --json -o report.json

# With explicit scope file + hard enforcement
eggsec scan example.com --profile quick --scope scope.toml --strict-scope
```

## Configuration

Configuration uses TOML format. Discovery order: `./eggsec.toml`, `./.eggsec/eggsec.toml`, `./config/eggsec.toml`, then `~/.config/eggsec/eggsec.toml` (explicit `-c` path wins).

```toml
[http]
timeout_secs = 30
max_retries = 2
verify_tls = true

[scan]
default_concurrency = 100
rate_limit_per_second = 100

[output]
format = "json"        # pretty | json | compact | html | sarif | junit | csv | markdown
color = true

[ai]
provider = "openai"
model = "gpt-4"
base_url = "https://api.openai.com/v1"
# api_key = "sk-..."  # Use SensitiveString, zeroized on drop
max_tokens = 4096
temperature = 0.7

[remote]
# psk = "..."         # SensitiveString; required for distributed workers
default_port = 7890
```

Generate a fully commented template with `eggsec --generate-config > eggsec.toml`.

## Output Formats

Eggsec supports multiple report formats (`--format`, `-o/--output`):

```bash
# Pretty (default, human-readable)
eggsec scan example.com --profile quick

# JSON report
eggsec scan example.com --profile quick --json -o report.json

# HTML report
eggsec scan example.com --profile quick --format html -o report.html

# SARIF (for GitHub code scanning)
eggsec scan example.com --profile quick --format sarif -o results.sarif

# JUnit XML (for CI/CD)
eggsec scan example.com --profile quick --format junit -o results.xml
```

## Severity Levels

Findings are rated using the canonical Severity enum (`eggsec-core::types`):

- **CRITICAL** - Immediate exploitation possible (as_int: 4)
- **HIGH** - Significant security impact (as_int: 3)
- **MEDIUM** - Moderate risk (as_int: 2)
- **LOW** - Minor security concern (as_int: 1)
- **INFO** - Informational finding, the default (as_int: 0)

## API Server

When built with `--features rest-api`, Eggsec exposes a REST API (command: `serve`; MCP server: `mcp-serve`):

```bash
# Start API server
eggsec serve --port 8080 --bind 127.0.0.1

# Start MCP server
eggsec mcp-serve --port 8081
```

# Available endpoints:
# GET  /health
# GET  /openapi.json
# GET  /api/v1/tools
# GET  /api/v1/tools/:tool_id
# POST /api/v1/tools/:tool_id/execute
# GET  /v1/models
# GET  /v1/models/:model_id
# POST /v1/chat/completions
```

### OpenAI-Compatible API

The `/v1/models` endpoint returns available Eggsec "models" (tool categories):
- `eggsec-recon` - Reconnaissance capabilities
- `eggsec-fuzzer` - Fuzzing engine
- `eggsec-waf` - WAF detection and bypass
- `eggsec-scanner` - Port and endpoint scanning
- `eggsec-loadtest` - Load testing
- `eggsec-pipeline` - Full security pipeline

### Agent Management (with rest-api feature)

```
POST   /api/v1/agents              - Register agent
GET    /api/v1/agents              - List agents
GET    /api/v1/agents/:id          - Get agent
DELETE /api/v1/agents/:id          - Unregister agent
POST   /api/v1/agents/:id/heartbeat - Update heartbeat
POST   /api/v1/tasks               - Create task
GET    /api/v1/tasks               - List tasks
GET    /api/v1/tasks/:id           - Get task status
POST   /api/v1/tasks/:id/cancel    - Cancel task
```

### AI Integration (with ai-integration feature)

```
POST /api/v1/ai/analyze         - Analyze findings with AI
POST /api/v1/ai/suggest-payloads - Get AI payload suggestions
POST /api/v1/ai/waf-bypass      - Get WAF bypass suggestions
POST /api/v1/ai/scan-strategy   - Get adaptive scan strategy
GET  /api/v1/ai/circuit-breaker - Check AI circuit breaker state
POST /api/v1/ai/validate-config - Validate AI configuration
```

## Scope Rules

Always respect scope boundaries (scope file + CLI target):

```bash
# Single target with explicit scope
eggsec scan example.com --scope scope.toml

# Wildcard scope pattern inside scope.toml (includes apex domain)
# allowed_targets = [{ pattern = "*.example.com" }]

# CIDR scope inside scope.toml
# allowed_targets = [{ cidr = "192.168.1.0/24" }]

# Hard enforcement (reject instead of warn)
eggsec scan example.com --scope scope.toml --strict-scope
```

## Best Practices

1. **Always define scope** - Never scan without explicit authorization
2. **Start passive** - Begin with recon before active testing
3. **Rate limit responsibly** - Use `--rate-limit` to avoid DoS
4. **Save reports** - Use `--output` to persist findings
5. **Use CI/CD mode** - `eggsec ci --fail-on high` for automated pipelines
6. **Respect rate limits** - Configure appropriate delays between requests
7. **Check WAF first** - Detect WAF before fuzzing to adjust payloads

## Feature Flags

| Feature | Description |
|---------|-------------|
| `rest-api` | REST API server with OpenAI compatibility |
| `grpc-api` | gRPC API server for external tool integration |
| `ai-integration` | AI/LLM integration for analysis |
| `stress-testing` | ICMP probing, IP spoofing, raw sockets |
| `packet-inspection` | Packet capture features |
| `nse` | Nmap NSE script support |
| `nse-ssh2` | NSE with SSH2/libssh2 support |
| `nse-sandbox` | Sandboxed NSE execution |
| `websocket` | WebSocket security testing |
| `headless-browser` | DOM XSS and SPA crawling |
| `database` | SQLx-based persistence |
| `container` | Kubernetes/Docker scanning |
| `sbom` | SBOM generation |
| `advanced-hunting` | Advanced threat hunting |
| `compliance` | Compliance scanning (OWASP, PCI, HIPAA, SOC2) |
| `external-integrations` | Jira, GitHub, GitLab connectors |
| `finding-workflow` | Finding lifecycle management |
| `vuln-management` | Vulnerability triage and CVSS scoring |
| `cloud` | AWS/GCP/Azure asset discovery |
| `git-secrets` | Git secrets scanning |
| `wireless` | Passive WiFi scanning and security analysis |
| `wireless-advanced` | Active wireless attacks (deauth/disassoc, lab-only) |
| `mobile` | Mobile app static analysis (APK/IPA) |
| `mobile-dynamic` | Mobile dynamic testing (ADB + Frida) |
| `db-pentest` | Database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis) |
| `web-proxy` | Interactive web proxy (HTTP/HTTPS/WebSocket/HTTP2/gRPC) |
| `pdf` | PDF report generation |
| `api-schema` | OpenAPI v3 schema-based fuzzing (marker-only) |
| `full` | Most non-default features combined (excludes `grpc-api`, `ws-api`, `pdf`, `nse-ssh2`, `nse-sandbox`, `db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`, `c2-mcp`, `web-proxy-mcp`, `transparent-proxy`, `dynamic-plugins`, `api-schema`, `git-secrets`, `cloud`, `insecure-tls`, `tool-api`) |

## Error Handling

Eggsec uses `EggsecError` with 23 variants. Common errors:
- `Config` - Configuration issues
- `InvalidTarget` - Target validation failed
- `Network` - Network connectivity issues
- `Timeout` - Request timed out

## Security Notes

- Credentials use `SensitiveString` (zeroized on drop, constant-time comparison)
- API keys are never logged in plaintext
- Circuit breaker pattern prevents API abuse
- Rate limiting built into all scanning operations
