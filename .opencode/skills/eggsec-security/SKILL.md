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
- 28 tabs covering all security testing functions
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

Passive information gathering about targets:

```bash
# DNS enumeration
eggsec recon dns --target example.com

# Subdomain discovery
eggsec recon subdomains --target example.com

# SSL/TLS analysis
eggsec recon ssl --target https://example.com

# WHOIS lookup
eggsec recon whois --target example.com

# Technology detection
eggsec recon tech --target https://example.com

# Full recon suite
eggsec recon --target example.com --all
```

### 2. Port Scanning

Network discovery and service fingerprinting:

```bash
# Basic port scan
eggsec scan ports --target 192.168.1.1

# Full port range scan
eggsec scan ports --target 192.168.1.1 --ports 1-65535

# Service version detection
eggsec scan ports --target 192.168.1.1 --service-detect

# With IP spoofing (feature-gated)
eggsec scan ports --target 192.168.1.1 --spoof-ip 10.0.0.1
```

### 3. Endpoint Discovery

Finding web application endpoints:

```bash
# Discover endpoints
eggsec scan endpoints --target https://example.com

# With custom wordlist
eggsec scan endpoints --target https://example.com --wordlist paths.txt
```

### 4. Fuzzing

Security payload testing across 30 payload types:

```bash
# SQL injection fuzzing
eggsec fuzz --target https://example.com/api --type sqli

# XSS fuzzing
eggsec fuzz --target https://example.com/search --type xss

# Path traversal
eggsec fuzz --target https://example.com/file --type traversal

# SSRF testing
eggsec fuzz --target https://example.com/fetch --type ssrf

# All payload types
eggsec fuzz --target https://example.com --type all
```

Available payload types: sqli, xss, traversal, ssrf, redirect, redos, headers, compression, graphql, oauth, jwt, idor, ssti, grpc, xxe, ldap, cmd, deser, host, cache, csv, soap, websocket, nosql, xpath, expression, prototype, race, massassign, oast

Note: `traversal` is the correct type name for path traversal (not `path-traversal`).

### 5. WAF Detection & Bypass

```bash
# Detect WAF
eggsec waf detect --target https://example.com

# Attempt bypass
eggsec waf bypass --target https://example.com --waf cloudflare
```

### 6. Load Testing

```bash
# HTTP load test
eggsec loadtest --target https://example.com --requests 10000 --concurrency 100

# Rate limit testing
eggsec loadtest --target https://example.com/api --rate-limit-test
```

### 7. Pipeline Mode

Run comprehensive security assessments:

```bash
# Full pipeline: recon -> scan -> fuzz -> report
eggsec pipeline --target https://example.com --output report.json

# With scope rules
eggsec pipeline --target https://example.com --scope "*.example.com" --output report.json
```

## Configuration

Configuration uses TOML format. Default location: `~/.config/eggsec/config.toml`

```toml
[target]
hosts = ["example.com"]

[scan]
timeout = 30
concurrency = 100

[fuzz]
rate_limit = 100
payload_count = 1000

[output]
format = "json"
path = "./reports"

[ai]
provider = "openai"
model = "gpt-4"
base_url = "https://api.openai.com/v1"
# api_key = "sk-..."  # Use SensitiveString, zeroized on drop
max_tokens = 4096
temperature = 0.7
```

## Output Formats

Eggsec supports multiple report formats:

```bash
# JSON report
eggsec pipeline --target https://example.com --format json --output report.json

# HTML report
eggsec pipeline --target https://example.com --format html --output report.html

# SARIF (for GitHub code scanning)
eggsec pipeline --target https://example.com --format sarif --output results.sarif

# JUnit XML (for CI/CD)
eggsec pipeline --target https://example.com --format junit --output results.xml
```

## Severity Levels

Findings are rated using the canonical Severity enum:

- **CRITICAL** - Immediate exploitation possible (as_int: 5)
- **HIGH** - Significant security impact (as_int: 4)
- **MEDIUM** - Moderate risk (as_int: 3)
- **LOW** - Minor security concern (as_int: 2)
- **INFO** - Informational finding (as_int: 1)

## API Server

When built with `--features rest-api`, Eggsec exposes a REST API:

```bash
# Start API server
eggsec api --port 8080

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

Always respect scope boundaries:

```bash
# Single target
eggsec scan --target example.com

# Wildcard scope (includes apex domain)
eggsec scan --target "*.example.com"

# Multiple targets
eggsec scan --target example.com --target api.example.com

# IP scope
eggsec scan --target 192.168.1.0/24
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
| `ai-integration` | AI/LLM integration for analysis |
| `stress-testing` | ICMP probing, IP spoofing, raw sockets |
| `packet-inspection` | Packet capture features |
| `nse` | Nmap NSE script support |
| `nse-sandbox` | Sandboxed NSE execution |
| `full` | All features (except grpc-api) |

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
