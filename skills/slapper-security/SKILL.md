# Slapper Security Testing Skill

## Overview

Slapper is a Rust-based security testing toolkit for penetration testing, vulnerability scanning, and security assessment. This skill teaches agents how to use Slapper effectively for security testing workflows.

## Core Capabilities

### 1. Reconnaissance

Passive information gathering about targets:

```bash
# DNS enumeration
slapper recon dns --target example.com

# Subdomain discovery
slapper recon subdomains --target example.com

# SSL/TLS analysis
slapper recon ssl --target https://example.com

# WHOIS lookup
slapper recon whois --target example.com

# Technology detection
slapper recon tech --target https://example.com

# Full recon suite
slapper recon --target example.com --all
```

### 2. Port Scanning

Network discovery and service fingerprinting:

```bash
# Basic port scan
slapper scan ports --target 192.168.1.1

# Full port range scan
slapper scan ports --target 192.168.1.1 --ports 1-65535

# Service version detection
slapper scan ports --target 192.168.1.1 --service-detect

# With IP spoofing (feature-gated)
slapper scan ports --target 192.168.1.1 --spoof-ip 10.0.0.1
```

### 3. Endpoint Discovery

Finding web application endpoints:

```bash
# Discover endpoints
slapper scan endpoints --target https://example.com

# With custom wordlist
slapper scan endpoints --target https://example.com --wordlist paths.txt
```

### 4. Fuzzing

Security payload testing across 23+ payload types:

```bash
# SQL injection fuzzing
slapper fuzz --target https://example.com/api --type sqli

# XSS fuzzing
slapper fuzz --target https://example.com/search --type xss

# Path traversal
slapper fuzz --target https://example.com/file --type path-traversal

# SSRF testing
slapper fuzz --target https://example.com/fetch --type ssrf

# All payload types
slapper fuzz --target https://example.com --type all
```

Available payload types: sqli, xss, ssrf, path-traversal, command-injection, ldap-injection, xpath-injection, ssti, xxe, file-inclusion, header-injection, open-redirect, crlf, graphql, soap, xml-rpc, rce, lfi, rfi, idor, broken-auth, sensitive-data, custom

### 5. WAF Detection & Bypass

```bash
# Detect WAF
slapper waf detect --target https://example.com

# Attempt bypass
slapper waf bypass --target https://example.com --waf cloudflare
```

### 6. Load Testing

```bash
# HTTP load test
slapper loadtest --target https://example.com --requests 10000 --concurrency 100

# Rate limit testing
slapper loadtest --target https://example.com/api --rate-limit-test
```

### 7. Pipeline Mode

Run comprehensive security assessments:

```bash
# Full pipeline: recon -> scan -> fuzz -> report
slapper pipeline --target https://example.com --output report.json

# With scope rules
slapper pipeline --target https://example.com --scope "*.example.com" --output report.json
```

## Configuration

Configuration uses TOML format. Default location: `~/.config/slapper/config.toml`

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

Slapper supports multiple report formats:

```bash
# JSON report
slapper pipeline --target https://example.com --format json --output report.json

# HTML report
slapper pipeline --target https://example.com --format html --output report.html

# SARIF (for GitHub code scanning)
slapper pipeline --target https://example.com --format sarif --output results.sarif

# JUnit XML (for CI/CD)
slapper pipeline --target https://example.com --format junit --output results.xml
```

## Severity Levels

Findings are rated using the canonical Severity enum:

- **CRITICAL** - Immediate exploitation possible (as_int: 5)
- **HIGH** - Significant security impact (as_int: 4)
- **MEDIUM** - Moderate risk (as_int: 3)
- **LOW** - Minor security concern (as_int: 2)
- **INFO** - Informational finding (as_int: 1)

## API Server

When built with `--features rest-api`, Slapper exposes a REST API:

```bash
# Start API server
slapper api --port 8080

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

The `/v1/models` endpoint returns available Slapper "models" (tool categories):
- `slapper-recon` - Reconnaissance capabilities
- `slapper-fuzzer` - Fuzzing engine
- `slapper-waf` - WAF detection and bypass
- `slapper-scanner` - Port and endpoint scanning
- `slapper-loadtest` - Load testing
- `slapper-pipeline` - Full security pipeline

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
slapper scan --target example.com

# Wildcard scope (includes apex domain)
slapper scan --target "*.example.com"

# Multiple targets
slapper scan --target example.com --target api.example.com

# IP scope
slapper scan --target 192.168.1.0/24
```

## Best Practices

1. **Always define scope** - Never scan without explicit authorization
2. **Start passive** - Begin with recon before active testing
3. **Rate limit responsibly** - Use `--rate-limit` to avoid DoS
4. **Save reports** - Use `--output` to persist findings
5. **Use CI/CD mode** - `slapper ci --fail-on high` for automated pipelines
6. **Respect rate limits** - Configure appropriate delays between requests
7. **Check WAF first** - Detect WAF before fuzzing to adjust payloads

## Feature Flags

| Feature | Description |
|---------|-------------|
| `rest-api` | REST API server with OpenAI compatibility |
| `ai-integration` | AI/LLM integration for analysis |
| `stress-testing` | ICMP probing, IP spoofing, raw sockets |
| `packet-inspection` | Packet capture features |
| `python-plugins` | Python plugin support |
| `ruby-plugins` | Ruby plugin support |
| `nse` | Nmap NSE script support |
| `nse-sandbox` | Sandboxed NSE execution |
| `full` | All features (except grpc-api) |

## Error Handling

Slapper uses `SlapperError` with 23 variants. Common errors:
- `Config` - Configuration issues
- `InvalidTarget` - Target validation failed
- `Network` - Network connectivity issues
- `Timeout` - Request timed out

## Security Notes

- Credentials use `SensitiveString` (zeroized on drop, constant-time comparison)
- API keys are never logged in plaintext
- Circuit breaker pattern prevents API abuse
- Rate limiting built into all scanning operations
