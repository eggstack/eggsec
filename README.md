# Slapper - Security Testing Toolkit

A high-performance, extensible security testing toolkit written in Rust. Slapper provides comprehensive assessment capabilities including reconnaissance, port scanning, endpoint discovery, service fingerprinting, WAF detection/bypass, security fuzzing, and load testing.
Developed alongside and tested against MaluWAF.

## What is Slapper?

Slapper is a command-line security testing tool designed for security professionals, penetration testers, and developers who need to:

- **Discover attack surfaces** - Reconnaissance, subdomain enumeration, technology detection
- **Assess web application security** - Find vulnerabilities like SQL injection, XSS, SSRF, and more
- **Test infrastructure** - Scan ports, fingerprint services, discover endpoints
- **Evaluate defenses** - Test WAF detection and bypass capabilities
- **Load test** - Measure application performance under stress
- **Automate assessments** - Pipeline scans with customizable profiles

## Why Slapper?

Slapper excels in areas that complement your existing toolkit:

| Capability | Description |
|------------|-------------|
| **High Performance** | Built in Rust with async I/O for rapid scanning and fuzzing |
| **Modern API Testing** | Specialized support for GraphQL, JWT, OAuth/OIDC, gRPC, and WebSocket security |
| **WAF Evaluation** | Detection of 30+ WAF products with multiple bypass techniques |
| **CVE Prioritization** | Map discovered technologies to known vulnerabilities |
| **CI/CD Integration** | SARIF and JUnit XML output for automated pipelines |
| **Interactive TUI** | Real-time progress monitoring with terminal UI |

## Core Features

| Category | Capabilities |
|----------|-------------|
| **Reconnaissance** | DNS enumeration, subdomain discovery, WHOIS, tech stack detection, CVE mapping, cloud asset discovery, CORS analysis |
| **Web Security** | SQLi, XSS, SSRF, Path Traversal, ReDoS, Header Injection, SSTI, IDOR testing |
| **API Security** | GraphQL introspection/injection, JWT analysis, OAuth/OIDC testing, gRPC fuzzing |
| **Scanning** | Port scanning, service fingerprinting (20+ protocols), endpoint discovery |
| **WAF** | Detection of 26 WAF products, header manipulation, HTTP smuggling, evasion techniques |
| **Load Testing** | High-concurrency HTTP testing with detailed metrics |
| **Stress Testing** | SYN, UDP, HTTP, TCP, ICMP flood testing (requires `--features stress-testing`) |
| **Proxy Management** | SOCKS4, SOCKS5, HTTP, HTTPS, Tor proxy pool with health checking |
| **Cluster Mode** | Distributed scanning with worker/coordinator architecture |
| **Notifications** | Slack, Discord, Teams, and custom webhook integrations |
| **Automation** | 11 pipeline profiles, session resumption, multiple output formats |

## System Dependencies

Some features require system-level packages to be installed via your package manager:

| Feature | Required Packages | Package Manager Commands |
|---------|-------------------|--------------------------|
| `ruby-plugins` | `ruby-dev`, `clang` | `sudo apt-get install ruby-dev clang` (Ubuntu/Debian) |
| `packet-inspection` | `libpcap-dev` (optional, for full packet capture) | `sudo apt-get install libpcap-dev` (Ubuntu/Debian) |
| `wireless` | `libusb-1.0-0-dev` (optional, for wireless testing) | `sudo apt-get install libusb-1.0-0-dev` (Ubuntu/Debian) |
| `nse` | `libssl-dev` (for NSE script compatibility) | `sudo apt-get install libssl-dev` (Ubuntu/Debian) |

**Ubuntu/Debian:**
```bash
# For Ruby plugin support
sudo apt-get install ruby-dev clang

# For packet inspection (optional)
sudo apt-get install libpcap-dev

# For wireless testing (optional)
sudo apt-get install libusb-1.0-0-dev
```

**Fedora/RHEL:**
```bash
# For Ruby plugin support
sudo dnf install ruby-devel clang

# For packet inspection (optional)
sudo dnf install libpcap-devel

# For wireless testing (optional)
sudo dnf install libusb1-devel
```

**macOS:**
```bash
# For Ruby plugin support
xcode-select --install  # Installs clang
brew install ruby       # Installs ruby with development headers
```

## Build Features

Slapper uses Cargo feature flags to enable optional capabilities. Some commands require specific build configurations:

| Feature | Description | Required For |
|---------|-------------|--------------|
| `stress-testing` | SYN/UDP/ICMP floods, proxy management | `stress`, `proxy`, `icmp`, `traceroute` commands |
| `packet-inspection` | Live packet capture, traceroute | `packet capture`, `packet send` (live) |
| `python-plugins` | Python plugin support | Python-based security plugins |
| `ruby-plugins` | Ruby plugin support + Metasploit RPC | Ruby plugins, Metasploit integration |
| `api-schema` | OpenAPI v3 schema-based fuzzing | Type-aware API fuzzing from OpenAPI specs |
| `sbom` | SBOM generation and analysis | Software bill of materials (`cyclonedx-bom`, `spdx`) |
| `full` | All features combined | All commands available |

### Build Examples

```bash
# Default build - load testing, scanning, fuzzing, WAF testing
cargo build --release

# With stress testing (DoS tools, proxy pool)
cargo build --release --features stress-testing

# With packet inspection (live capture)
cargo build --release --features packet-inspection

# Full build - all features
cargo build --release --features full

# With plugin support
cargo build --release --features python-plugins
cargo build --release --features ruby-plugins
cargo build --release --features all-plugins
```

## Quick Start

### Prerequisites

Before building, ensure you have Rust installed. For features that require system dependencies, install the necessary packages:

**For Ruby plugin support (recommended):**
```bash
# Ubuntu/Debian
sudo apt-get install ruby-dev clang

# Fedora/RHEL
sudo dnf install ruby-devel clang

# macOS
xcode-select --install
brew install ruby
```

**For full build with all features:**
```bash
# Ubuntu/Debian
sudo apt-get install ruby-dev clang libpcap-dev libssl-dev libusb-1.0-0-dev

# Fedora/RHEL
sudo dnf install ruby-devel clang libpcap-devel openssl-devel libusb1-devel
```

### Installation

```bash
# Clone and build (default features)
git clone https://github.com/slapper-tool/slapper.git
cd slapper
cargo build --release

# The binary will be at ./target/release/slapper

# Full build with all features (recommended for pentesting)
cargo build --release --features full

# Build with stress testing (DoS tools, proxy pool)
cargo build --release --features stress-testing
```

### Basic Usage

```bash
# Load test a URL
./slapper load https://example.com -n 1000 -c 50

# Scan ports
./slapper scan-ports example.com -p 1-1000 -c 100

# Discover endpoints
./slapper scan-endpoints https://example.com

# Fuzz for vulnerabilities
./slapper fuzz https://example.com/api -t sqli,xss

# GraphQL security testing
./slapper graphql https://api.example.com/graphql

# OAuth/OIDC security testing
./slapper oauth https://oauth.example.com/authorize

# Full security assessment
./slapper scan example.com --profile full

# Reconnaissance
./slapper recon example.com
```

### Advanced Features

```bash
# Stress testing (requires stress-testing feature)
./slapper stress example.com --type http -r 1000 -d 60
./slapper stress example.com --type syn -r 5000 -d 30

# Proxy management
./slapper proxy add --file proxies.txt
./slapper proxy list --healthy

# Distributed cluster
./slapper cluster coordinator --port 9000
./slapper cluster worker --coordinator localhost:9000 --workers 4

# Notifications
./slapper notify test --slack https://hooks.slack.com/services/XXX
./slapper notify send "Vulnerability found" --discord https://discord.com/api/webhooks/XXX
```

## Command Reference

### Load Testing

HTTP load testing measures server performance under concurrent requests. Useful for capacity planning, finding bottlenecks, and testing resilience.

```bash
# Basic load test - sends 1000 requests with 50 concurrent connections
./slapper load https://example.com -n 1000 -c 50

# With POST data - sends JSON body
./slapper load https://api.example.com/endpoint -n 500 -c 20 -m POST -d '{"key": "value"}'

# With proxy - routes through HTTP proxy
./slapper load https://example.com -n 200 -c 10 --proxy http://127.0.0.1:8080

# JSON output - machine-readable results
./slapper load https://example.com -n 100 --json
```

### Port Scanning

TCP port scanning discovers open services on target hosts. Supports concurrent scanning for speed and configurable timeouts for reliability.

```bash
# Scan common ports (1-1000)
./slapper scan-ports example.com -p 1-1000

# Specific ports - scan enumerated list
./slapper scan-ports 192.168.1.1 -p 22,80,443,8080

# High concurrency - faster scan with more parallel connections
./slapper scan-ports example.com -p 1-1024 -c 50
```

### Service Fingerprinting

Identifies running services by grabbing banners and matching against known fingerprints. Detects service type, version, and sometimes configuration.

```bash
# Fingerprint specific ports
./slapper fingerprint example.com -p 80,443,22,21,25,3306,5432

# Full port range with service detection
./slapper fingerprint example.com -p 1-1000
```

### Endpoint Discovery

Directory and endpoint brute-forcing discovers hidden paths, administrative interfaces, sensitive files, and API endpoints using wordlist-based scanning.

```bash
# Basic scan - uses default wordlist
./slapper scan-endpoints https://example.com

# Custom wordlist - specify your own wordlist
./slapper scan-endpoints https://example.com -w wordlist.txt -c 20
```

### Security Fuzzing

Fuzz testing injects payloads into parameters to discover vulnerabilities. Slapper supports 20+ payload types targeting common vulnerabilities.

```bash
# SQL Injection - tests for SQL injection vulnerabilities
./slapper fuzz https://example.com/api?id=1 -t sqli

# XSS (Cross-Site Scripting) - tests for reflected/stored XSS
./slapper fuzz https://example.com/search?q=test -t xss

# SSRF (Server-Side Request Forgery) - tests for internal service access
./slapper fuzz https://example.com/url?url=https://internal -t ssrf

# Path Traversal - tests for directory traversal (LFI/FI)
./slapper fuzz https://example.com/file?path=/etc/passwd -t traversal

# Open Redirect - tests for redirect injection
./slapper fuzz https://example.com/redirect?url=https://evil.com -t redirect

# ReDoS (Regular Expression DoS) - tests for catastrophic backtracking
./slapper fuzz https://example.com/search?q=test -t redos

# All payload types
./slapper fuzz https://example.com -t all

# Multiple types at once
./slapper fuzz https://example.com -t sqli,xss,ssrf -c 20

# JWT testing - tests for JWT vulnerabilities (weak algo, none alg, key confusion)
./slapper fuzz https://api.example.com -t jwt

# IDOR (Insecure Direct Object Reference) - tests for authorization bypass
./slapper fuzz https://example.com/api/user/1 -t idor

# SSTI (Server-Side Template Injection) - tests for template injection
./slapper fuzz https://example.com/template?name=test -t ssti

# XXE (XML External Entity) - tests for XML injection
./slapper fuzz https://example.com/api/xml -t xxe

# LDAP Injection - tests for LDAP injection
./slapper fuzz https://example.com/login?user=admin -t ldap

# Command Injection - tests for OS command execution
./slapper fuzz https://example.com/ping?host=127.0.0.1 -t cmd

# Deserialization - tests for unsafe deserialization
./slapper fuzz https://example.com/api/deserialize -t deser

# Host Header Injection - tests for host header manipulation
./slapper fuzz https://example.com -t host

# Cache Poisoning - tests for HTTP cache manipulation
./slapper fuzz https://example.com -t cache

# CSV Injection - tests for formula injection in CSV exports
./slapper fuzz https://example.com/export -t csv

# SOAP Injection - tests for SOAP XML injection
./slapper fuzz https://example.com/soap -t soap

# HTTP Header Injection - tests for response splitting
./slapper fuzz https://example.com -t headers

# Compression Bomb - tests for zip bomb decompression
./slapper fuzz https://example.com/upload -t compression
```

#### Fuzzing Modes

```bash
# Sequential mode (default) - one request at a time
./slapper fuzz https://example.com -t sqli --mode sequential

# Burst mode - concurrent requests for speed
./slapper fuzz https://example.com -t sqli --mode burst -c 50

# Adaptive mode - auto-adjusts rate based on server responses
./slapper fuzz https://example.com -t sqli --mode adaptive
```

#### Advanced Fuzzing Options

```bash
# Mutation fuzzing - mutates existing inputs to find edge cases
./slapper fuzz https://example.com -t xss --mutate -m 5

# Grammar-based fuzzing (generative) - generates inputs based on grammar
./slapper fuzz https://example.com/api -t json --grammar-fuzz --grammar-type json

# Adaptive rate limiting - auto-adjusts to server responses
./slapper fuzz https://example.com -t sqli --adaptive-rate

# HTTP session handling - maintains cookies across requests
./slapper fuzz https://example.com -t xss --session

# Response diffing - compares responses to detect anomalies
./slapper fuzz https://example.com -t all --diffing --capture-baseline

# Enhanced ReDoS detection - executes regexes to find catastrophic backtracking
./slapper fuzz https://example.com -t redos --enhanced-redos

# WAF fingerprinting - identifies specific WAF products
./slapper fuzz https://example.com -t all --waf-fingerprint

# Request chaining - chains multiple requests for multi-step exploitation
./slapper fuzz https://example.com -t ssrf --chaining --chain-file chain.yaml

# Target-specific payloads - uses payloads tailored to specific technologies
./slapper fuzz https://example.com -t sqli --target php

# Combined: adaptive + session + diffing + waf detection
./slapper fuzz https://example.com -t all --adaptive-rate --session --diffing --waf-fingerprint
```

#### Payload Type Reference

| Type | Alias | Tests For |
|------|-------|-----------|
| `sqli` | sql | SQL Injection |
| `xss` | - | Cross-Site Scripting |
| `traversal` | lfi, path | Path Traversal / Local File Inclusion |
| `ssrf` | - | Server-Side Request Forgery |
| `redirect` | open-redirect | Open Redirect |
| `redos` | regex | Regular Expression DoS |
| `headers` | - | HTTP Header Injection |
| `compression` | gzip | Compression Bomb |
| `graphql` | - | GraphQL security issues |
| `oauth` | - | OAuth/OIDC vulnerabilities |
| `jwt` | - | JWT vulnerabilities |
| `idor` | - | Insecure Direct Object Reference |
| `ssti` | - | Server-Side Template Injection |
| `xxe` | - | XML External Entity |
| `ldap` | - | LDAP Injection |
| `cmd` | - | Command Injection |
| `deser` | - | Deserialization vulnerabilities |
| `host` | - | Host Header Injection |
| `cache` | - | Cache Poisoning |
| `csv` | - | CSV Injection |
| `soap` | - | SOAP Injection |

### GraphQL Security

GraphQL endpoints have unique security considerations. This command tests for GraphQL-specific vulnerabilities including introspection leakage, query injection, and DoS vectors.

```bash
# Basic GraphQL scan (runs all tests)
./slapper graphql https://api.example.com/graphql

# Introspection tests - queries schema without authentication
./slapper graphql https://api.example.com/graphql --introspection

# Query injection - tests for injection via query parameters
./slapper graphql https://api.example.com/graphql --inject

# Depth limit bypass - tests if nested queries are properly limited
./slapper graphql https://api.example.com/graphql --depth-bypass

# Alias overload DoS - tests if aliases can cause denial of service
./slapper graphql https://api.example.com/graphql --alias-overload
```

### OAuth/OIDC Security

Tests OAuth 2.0 and OpenID Connect implementations for common misconfigurations including redirect URI validation, scope escalation, and state parameter handling.

```bash
# Redirect URI validation - tests for redirect URI bypass
./slapper o-auth https://oauth.example.com --redirect-test

# Scope escalation - tests if scope can be expanded
./slapper o-auth https://oauth.example.com --scope-test

# State parameter tests - checks for CSRF via state parameter
./slapper o-auth https://oauth.example.com --state-test
```

### WAF Testing

Web Application Firewall detection and bypass testing. Identifies 26 WAF products and attempts bypass techniques including header manipulation, HTTP smuggling, and evasion.

Supported WAFs: Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva, Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler, ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong, Varnish, Radware, Signal Sciences, Wallarm, Reblaze.

```bash
# Detect WAF - identifies WAF products
./slapper waf https://example.com

# Detect and bypass - tries multiple bypass techniques
./slapper waf https://example.com --bypass

# WAF-specific bypass - targets specific WAF products
./slapper waf https://example.com --profile cloudflare --bypass
```

### WAF Stress Testing

Comprehensive WAF stress testing with multiple attack vectors to evaluate WAF rule effectiveness and detection capabilities.

```bash
# Full stress test
./slapper waf-stress https://example.com

# Targeted stress testing
./slapper waf-stress https://example.com --profile owasp
```

### Packet Tools

Slapper includes packet manipulation tools for network analysis, reconnaissance, and crafting custom packets. These require root/sudo privileges for live capture.

```bash
# List available network interfaces
sudo slapper packet interfaces

# Capture packets from an interface (requires root)
sudo slapper packet capture -i eth0

# Capture with filter and limit
sudo slapper packet capture -i eth0 --filter tcp --max 100

# Hexdump a pcap file
slapper packet dump capture.pcap

# Hexdump raw packet data
slapper packet dump --hex "45 00 00 3c 1c 46 40 00 40 06 b1 e6 ac 10 0a 0a ac 10 0a 01"

# Traceroute to target
slapper packet traceroute example.com

# Send custom TCP packet
slapper packet send --tcp --dst example.com:80 --flags SYN

# Send custom UDP packet
slapper packet send --udp --dst 192.168.1.1:53 --data "hello"
```

**Packet Capture** - Captures network packets from a specified interface using libpcap. Useful for analyzing network traffic during tests or inspecting responses.

**Packet Send** - Crafts and sends custom packets with specified protocols (TCP, UDP, ICMP), flags, and payloads. Essential for firewall testing and network discovery.

**Packet Dump** - Displays packet data in hexdump format. Supports reading from pcap files or raw hex data.

**Traceroute** - Traces the network path to a target host, showing each hop. Helps understand network topology and identify firewalls.

### Resume Command

Resume a previous scan from a saved session file. This is useful for long-running scans that were interrupted or to continue analysis.

```bash
# Resume a previous scan
./slapper resume session.json

# Resume with new output file
./slapper resume session.json -o results.json
```

### Pipeline Scans

Pipeline scans chain multiple security tests together in a single command. Choose the appropriate profile based on your assessment goals.

| Profile | Use Case |
|---------|----------|
| **quick** | Fast port scan and service fingerprinting |
| **endpoint** | Quick + directory/endpoint discovery |
| **web** | Endpoint + web vulnerability fuzzing |
| **waf** | Endpoint + WAF detection and bypass |
| **full** | All stages including load testing |
| **api** | GraphQL, JWT, OAuth focused |
| **recon** | Intelligence-led with tech detection and CVE mapping |
| **stealth** | Evasion mode with randomized delays and header rotation |
| **deep** | Mutation fuzzing enabled for thorough testing |
| **vuln** | CVE-prioritized based on detected technologies |
| **auth** | JWT, OAuth, IDOR focused |

```bash
# Quick - port scan + fingerprinting
./slapper scan example.com --profile quick

# Endpoint - quick + endpoint discovery
./slapper scan example.com --profile endpoint

# Web - endpoint + web fuzzing
./slapper scan example.com --profile web

# WAF - endpoint + WAF detection and bypass
./slapper scan example.com --profile waf

# Full - all stages including load testing
./slapper scan example.com --profile full

# API - GraphQL/JWT/OAuth focused
./slapper scan example.com --profile api

# Recon - intelligence-led with tech detection
./slapper scan example.com --profile recon

# Stealth - evasion mode (randomized delays, header rotation)
./slapper scan example.com --profile stealth

# Deep - mutation fuzzing enabled
./slapper scan example.com --profile deep

# Vuln - CVE-prioritized based on detected tech
./slapper scan example.com --profile vuln

# Auth - JWT/OAuth/IDOR focused
./slapper scan example.com --profile auth
```

### Reconnaissance

Passive reconnaissance gathers intelligence about targets without direct interaction. Collects DNS records, technology stack, subdomains, SSL info, wayback data, CORS policies, and CVE mappings.

```bash
# Full reconnaissance - all available checks
./slapper recon example.com

# Skip certain checks - disable specific modules
./slapper recon example.com --no-tech --no-whois

# Concurrency control - adjust parallel requests
./slapper recon example.com --concurrency 20
```

## Plugin Support

Slapper supports extending functionality through plugins in Python and Ruby.

### Python Plugins

Build with Python support:
```bash
cargo build --release --features python-plugins
```

Create a Python plugin:
```python
# ~/.config/slapper/plugins/my_plugin.py
from typing import Dict, List, Optional
from dataclasses import dataclass
import json

@dataclass
class Finding:
    severity: str
    finding_type: str
    description: str
    location: str
    evidence: Optional[str] = None

class MyPlugin:
    @property
    def name(self) -> str:
        return "my_plugin"

    @property
    def version(self) -> str:
        return "1.0.0"

    def run(self, target: str, config: Dict) -> Dict:
        findings = []

        # Your scanning logic here
        # Make HTTP requests, analyze responses, etc.

        return {
            "target": target,
            "findings": findings,
            "success": True
        }

# Register plugin
PLUGINS = [MyPlugin]
```

### Ruby Plugins

Build with Ruby support:
```bash
cargo build --release --features ruby-plugins
```

Create a Ruby plugin:
```ruby
# ~/.config/slapper/plugins/my_plugin.rb
module Slapper
  class Plugin
    NAME = "my_plugin"
    VERSION = "1.0.0"

    def run(target, config = {})
      # Your scanning logic here
      # Use the Ruby API below

      Slapper::Report.success("My Plugin", "Scan completed")

      { success: true, findings: [] }
    end
  end
end
```

#### Ruby API Reference

```ruby
# HTTP requests
Slapper::HTTP.get(url)
Slapper::HTTP.post(url, body)
Slapper::HTTP.put(url, body)
Slapper::HTTP.delete(url)
Slapper::HTTP.request(method, url)

# Scanning
Slapper::Scanner.tcp_connect(host, port)
Slapper::Scanner.scan_port(host, port)
Slapper::Scanner.grab_banner(host, port)

# Fuzzing
Slapper::Fuzzer.fuzz_param(url, param, payloads, options)
Slapper::Fuzzer.fuzz_header(url, header, payloads, options)
Slapper::Fuzzer.fuzz_cookie(url, cookie_name, payloads, options)
Slapper::Fuzzer.fuzz_path(url, paths)

# Reporting
Slapper::Report.finding(severity, type, description, location)
Slapper::Report.vulnerability(severity, type, description, location, cve)
Slapper::Report.info(title, message)
Slapper::Report.success(title, message)
Slapper::Report.warning(title, message)
Slapper::Report.error(title, message)
```

## Autonomous Agent

Slapper includes an autonomous security agent for continuous monitoring and scheduled security assessments. The agent maintains longitudinal memory of scan results, routes alerts to configured channels, and uses AI-powered skills for intelligent security testing.

### Build Requirements

```bash
# Agent requires rest-api feature
cargo build --release --features rest-api

# With AI integration (recommended)
cargo build --release --features "rest-api ai-integration"
```

### Quick Start

```bash
# Run the agent (continuous monitoring)
./slapper agent run --portfolio /path/to/portfolio.json

# Run once (single assessment)
./slapper agent run --once

# With AI analysis
./slapper agent run --with-ai --ai-config /path/to/ai.toml
```

### Target Management

```bash
# List configured targets
./slapper agent targets list

# Add a target with scheduled scan
./slapper agent targets add example-com \
  --target https://example.com \
  --schedule "0 0 * * *"

# Remove a target
./slapper agent targets remove example-com

# Enable/disable targets
./slapper agent targets enable example-com
./slapper agent targets disable example-com
```

### Skills

Skills are YAML+Markdown files that guide the agent's behavior. See `slapper_skills/` for available skills.

```bash
# List available skills
./slapper agent skills list

# Load custom skills
./slapper agent skills load /path/to/skills/

# Show skill details
./slapper agent skills show sql_injection_fuzzing
```

### Configuration

Create a portfolio file (`portfolio.json`):

```json
{
  "version": "1.0",
  "targets": {
    "example-com": {
      "target": "https://example.com",
      "target_type": "url",
      "priority": "high",
      "schedule": "0 0 * * *",
      "alert_channels": ["webhook"],
      "enabled": true
    }
  }
}
```

### Alert Configuration

Configure webhooks in `config.toml`:

```toml
[agent]
memory_dir = "~/.config/slapper/memory"
poll_interval_secs = 60

[[agent.alert_channels]]
type = "webhook"
url = "https://hooks.example.com/security"
secret = "your-hmac-secret"
```

For detailed documentation, see [docs/AGENT.md](docs/AGENT.md).

## Docker Usage

```bash
# Start test environment with vulnerable targets
docker-compose --profile testing up -d dvwa

# Run scans against containerized target
docker-compose --profile testing run --rm slapper fuzz http://dvwa.target.local/login -t xss

# Full environment with Elasticsearch storage
docker-compose --profile full up -d
```

See [DOCKER_COMPOSE.md](DOCKER_COMPOSE.md) for detailed Docker setup.

## Configuration

### Scope Configuration

Scope files restrict testing to authorized targets only. Enable `require_explicit_scope` to enforce scope checking - any target not in the allowed list will be rejected.

Create `scope.toml` to define allowed targets:

```toml
# When true, only explicitly allowed targets can be scanned
require_explicit_scope = true

# Wildcard patterns for allowed domains
[[allowed_targets]]
pattern = "*.example.com"
description = "Production environment"

# CIDR ranges for internal networks
[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"

# Exclude specific hosts
[[excluded_targets]]
pattern = "admin.example.com"
description = "Admin panel - excluded"
```

### Custom Payloads

Add your own payloads for specialized testing. Place custom payloads in `~/.config/slapper/payloads/`:

```toml
[[payloads]]
name = "custom_sqli"
payload_type = "sqli"
payload = "' UNION SELECT username,password FROM users--"
description = "Custom UNION-based SQL injection"
severity = "critical"
tags = ["sqli", "union", "custom"]
```

### Main Configuration

Generate a default configuration file:

```bash
slapper --generate-config
```

Key configuration sections:
- `http` - Timeout, retries, TLS, proxy settings
- `scan` - Concurrency, rate limiting, stealth mode
- `output` - Default format, report settings
- `recon` - API keys for geolocation (MaxMind, ipapi)

## Output Formats

Slapper supports multiple output formats for different use cases:

| Format | Use Case |
|--------|-----------|
| JSON | Machine parsing, automation |
| HTML | Human-readable reports |
| CSV | Spreadsheet analysis |
| SARIF | CI/CD security scanning (GitHub, GitLab) |
| JUnit XML | Test integration (CI pipelines) |

```bash
# JSON output
./slapper scan example.com --json -o results.json

# HTML report
./slapper scan example.com --format html -o report.html

# CSV export
./slapper scan example.com --format csv -o results.csv

# SARIF (for CI/CD)
./slapper fuzz https://example.com --sarif -o results.sarif

# JUnit XML (for test integration)
./slapper fuzz https://example.com --junit -o results.xml
```

## Global Options

```bash
slapper --help                           # Show help
slapper --version                         # Show version
slapper --generate-config                 # Generate default config
slapper --generate-shell-completion bash # Generate bash completions

# Common flags
slapper --json                            # JSON output
slapper --config /path/to/config.toml     # Custom config
slapper --scope /path/to/scope.toml       # Scope file
```

## Documentation

- [Safety and Scope Enforcement](docs/SAFETY.md) - Authorization, risk tiers, scope rules
- [Canonical Findings Schema](docs/FINDINGS_SCHEMA.md) - Finding structure, fingerprinting, redaction
- [Auth Context Configuration](docs/AUTH_CONTEXT.md) - Multi-role testing, env interpolation
- [Baselines and Differential Scans](docs/BASELINES_AND_DIFFS.md) - Comparing scan results over time
- [API Testing with OpenAPI Schemas](docs/API_TESTING.md) - Schema import, fuzz target generation
- [Agent Documentation](docs/AGENT.md) - Autonomous agent setup and usage
- [Plugin Development](docs/PLUGIN_DEVELOPMENT.md) - Python and Ruby plugin authoring
- [Capabilities](docs/CAPABILITIES.md) - Feature matrix and capabilities overview

## Security Considerations

- **Always ensure you have explicit permission** to test targets
- Use the scope file to restrict testing to authorized systems
- Use rate limiting to avoid overwhelming targets: `--rate-limit 10`
- Consider stealth mode for evasive testing: `--stealth`

## Troubleshooting

### Build Issues

**Error: `ruby.h` file not found**
- **Cause:** Ruby development headers not installed
- **Fix:** Install `ruby-dev` (Ubuntu/Debian) or `ruby-devel` (Fedora/RHEL)

**Error: `stdarg.h` file not found or clang not found**
- **Cause:** `clang` compiler not installed (required for Ruby FFI bindings)
- **Fix:** Install `clang` via package manager

**Error: `regex` crate not found during build**
- **Cause:** This should not happen with the current codebase
- **Fix:** Ensure you're using the latest version from the repository

### Runtime Issues

**Panic: "command X alias X is duplicated"**
- **Cause:** Duplicate command alias in CLI configuration (fixed in current version)
- **Fix:** Update to the latest version from the repository

**Permission denied when running packet capture**
- **Cause:** Packet capture requires root/sudo privileges
- **Fix:** Run with `sudo slapper packet capture -i eth0`

## License

Licensed under either Apache License 2.0 or MIT license at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
