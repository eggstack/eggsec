# Slapper - Rust Security Assessment Engine

Slapper is a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing, local lab validation, WAF regression, CI security checks, and agent-readable security workflows.

## What Slapper is

Slapper is a command-line security assessment tool designed for security professionals, developers, and defensive teams who need to:

- **Discover attack surfaces** - Reconnaissance, subdomain enumeration, technology detection
- **Assess web application security** - Find vulnerabilities like SQL injection, XSS, SSRF, and more
- **Test infrastructure** - Scan ports, fingerprint services, discover endpoints
- **Evaluate defenses** - Test WAF detection and evasion-resistance
- **Load test** - Measure application performance under controlled load
- **Repeat assessments** - Pipeline scans with customizable profiles for regression workflows

### Why Slapper?

| Capability | Description |
|------------|-------------|
| **Scoped Repeatable Testing** | Run the same assessment profiles repeatedly for regression validation |
| **Rust-Native Primitives** | High-performance async I/O, no external runtime dependencies |
| **Structured Outputs** | JSON, SARIF, JUnit, HTML, CSV for humans, CI, and agents |
| **WAF and Defense Validation** | Detection of 26+ WAF products with evasion-resistance testing |
| **Local Lab/Regression Workflows** | Repeatable profiles against local test environments |
| **Optional NSE Compatibility** | Curated Nmap NSE script support as an optional layer |

### Core Capabilities

| Category | Capabilities |
|----------|-------------|
| **Reconnaissance** | DNS enumeration, subdomain discovery, WHOIS, tech stack detection, CVE mapping, cloud asset discovery, CORS analysis |
| **Web Security** | SQLi, XSS, SSRF, Path Traversal, ReDoS, Header Injection, SSTI, IDOR testing |
| **API Security** | GraphQL introspection/injection, JWT analysis, OAuth/OIDC testing, gRPC fuzzing |
| **Scanning** | Port scanning, service fingerprinting (20+ protocols), endpoint discovery |
| **WAF** | Detection of 26 WAF products, header manipulation, HTTP smuggling, evasion-resistance testing |
| **Load Testing** | High-concurrency HTTP testing with detailed metrics |
| **Controlled Stress** | SYN, UDP, HTTP, TCP, ICMP flood testing (requires `--features stress-testing`) |
| **Proxy Management** | SOCKS4, SOCKS5, HTTP, HTTPS, Tor proxy pool with health checking |
| **Cluster Mode** | Distributed scanning with worker/coordinator architecture |
| **Repeatable Profiles** | 11 pipeline profiles, session resumption, multiple output formats |

## What Slapper is not

Slapper is not an exploitation framework, botnet component, credential attack platform, or tool for unscoped internet scanning. Some modules can generate aggressive traffic or security-test payloads, so advanced capabilities are feature-gated and intended for systems you own, operate, or have explicit authorization to test.

## Safety Model

Slapper enforces a defense-in-depth safety model built around scope control, configuration defaults, and feature gating.

**Scope files** restrict every scan to explicitly authorized targets. Define allowed domains, CIDR ranges, and exclusions in a TOML file. When `require_explicit_scope = true`, any target not in the allowed list is rejected before a single packet is sent.

```toml
# scope.toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "*.lab.internal"
description = "Lab environment"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"

[[excluded_targets]]
pattern = "admin.lab.internal"
description = "Admin panel - excluded"
```

**Configuration defaults** keep aggressive capabilities disabled until you opt in. Rate limits, concurrency caps, and timeouts are configurable per profile. Dry-run planning (`slapper plan`) previews what a scan will do without sending traffic.

**Feature gating** ensures intrusive modules (stress testing, raw packet crafting, IP spoofing) require explicit build flags like `--features stress-testing` and cannot be invoked accidentally.

See [docs/SAFETY.md](docs/SAFETY.md) for full details on authorization, risk tiers, and scope rule evaluation.

## Quick Start

### Workspace Layout

Slapper is organized as a Cargo workspace with seven crates:

| Crate | Purpose |
|-------|---------|
| `slapper-core` | Dependency-light types, constants, shared primitives |
| `slapper-tool-core` | Core data types for the tool abstraction layer (requests, responses, findings, errors) |
| `slapper` | Assessment engine library (no binary) |
| `slapper-nse` | Optional Nmap NSE compatibility runtime |
| `slapper-tui` | Terminal UI adapter (`ratatui`/`crossterm`) |
| `slapper-cli` | CLI binary entry point |
| `slapper-output` | Report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown) |

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install libpcap-dev libssl-dev libusb-1.0-0-dev

# Fedora/RHEL
sudo dnf install libpcap-devel openssl-devel libusb1-devel
```

### Build and Run

```bash
# Clone and build
git clone https://github.com/dbowm91/slapper.git
cd slapper
cargo build --release -p slapper-cli

# Generate a config file
./target/release/slapper --generate-config > slapper.toml

# Validate your config
./target/release/slapper config validate --config slapper.toml

# Plan a scan (dry-run, no traffic sent)
./target/release/slapper plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

# Run a scoped scan against localhost
./target/release/slapper scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

## Pipeline Profiles

Slapper includes 11 built-in profiles that chain multiple security tests together. Choose the profile that matches your assessment goals.

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
# Quick scan - port scan + fingerprinting
./slapper scan example.com --profile quick

# Web assessment - endpoint discovery + vulnerability fuzzing
./slapper scan example.com --profile web

# Full assessment - all stages including load testing
./slapper scan example.com --profile full

# API-focused - GraphQL/JWT/OAuth testing
./slapper scan example.com --profile api
```

## Core Workflows

- **Scoped web assessment** - Port scanning, service fingerprinting, endpoint discovery, and vulnerability fuzzing against authorized targets
- **WAF/defense validation in lab** - Detect 26+ WAF products, test evasion resistance, run regression suites against local WAF instances
- **CI regression checks** - Structured output (SARIF, JUnit, JSON) for integration into GitHub Actions, GitLab CI, and other pipelines
- **Agent/MCP integration** - Autonomous security agent with skills, portfolio management, and structured findings for AI-driven workflows
- **Optional NSE compatibility** - Curated Nmap NSE script support as an optional build layer

## Quick Command Reference

```bash
# Load testing
./slapper load https://example.com -n 1000 -c 50

# Port scanning
./slapper scan-ports example.com -p 1-1000 -c 100

# Endpoint discovery
./slapper scan-endpoints https://example.com

# Vulnerability fuzzing
./slapper fuzz https://example.com/api -t sqli,xss

# GraphQL security testing
./slapper graphql https://api.example.com/graphql

# WAF detection and bypass testing
./slapper waf https://example.com --bypass

# Reconnaissance
./slapper recon example.com

# Resume a previous scan
./slapper resume session.json
```

For the full command reference with all options, see [docs/cli.md](docs/cli.md).

## Build Features

| Feature | Description | Status |
|---------|-------------|--------|
| `stress-testing` | SYN/UDP/ICMP floods, proxy management, IP spoofing | Lab-only |
| `packet-inspection` | Live packet capture, traceroute | Experimental |
| `nse` | Nmap NSE script compatibility | Experimental |
| `api-schema` | OpenAPI v3 schema-based fuzzing | Stable |
| `sbom` | SBOM generation (CycloneDX, SPDX) | Stable |
| `rest-api` | REST API server for agent integration | Experimental |
| `ai-integration` | AI planner, script generation, autonomous agent | Experimental |
| `ws-api` | WebSocket pub/sub | Experimental |
| `full` | All features combined | - |

### Build Examples

```bash
# Default build - load testing, scanning, fuzzing, WAF testing
cargo build --release -p slapper-cli

# With stress testing (controlled flood testing, proxy pool)
cargo build --release -p slapper-cli --features stress-testing

# With packet inspection (live capture)
cargo build --release -p slapper-cli --features packet-inspection

# With NSE support
cargo build --release -p slapper-cli --features nse

# Full build - all features
cargo build --release -p slapper-cli --features full
```

## System Dependencies

| Feature | Required Packages | Install (Ubuntu/Debian) |
|---------|-------------------|--------------------------|
| `packet-inspection` | `libpcap-dev` | `sudo apt-get install libpcap-dev` |
| `wireless` | `libusb-1.0-0-dev` | `sudo apt-get install libusb-1.0-0-dev` |
| `nse` | `libssl-dev` | `sudo apt-get install libssl-dev` |

## Output Formats

| Format | Use Case |
|--------|----------|
| JSON | Machine parsing, automation |
| HTML | Human-readable reports |
| CSV | Spreadsheet analysis |
| SARIF | CI/CD security scanning (GitHub, GitLab) |
| JUnit XML | Test integration (CI pipelines) |

## Defense-Lab Mode

Slapper can run local, repeatable profiles against defensive systems for regression testing.

- **Repeatable adversarial traffic** - Run the same probe suite multiple times to measure changes in WAF or protocol behavior
- **Structured observations and baseline diffs** - Compare current results against a saved baseline to identify regressions or improvements
- **WAF regression testing** - Validate that WAF rules continue to catch known evasion patterns after updates

```bash
# Run a profile against a local instance
./slapper scan localhost:8080 --profile waf --json -o baseline.json

# Later, compare against baseline
./slapper diff baseline.json current.json
```

## Relationship to Nmap/NSE

Slapper borrows proven scanning concepts from Nmap but is not a drop-in replacement.

- **NSE is an optional compatibility layer.** Build with `--features nse` to enable curated Nmap NSE script support.
- **No full Nmap parity.** Slapper does not aim to replicate all Nmap behavior. The goal is broad practical compatibility for useful script categories.
- **NSE is a protocol-testing knowledge source.** Selected behaviors may be promoted into Rust-native probes over time for repeatability, performance, and safety.

## Agent and Orchestration

Slapper includes a security agent for continuous monitoring and scheduled assessments. The agent maintains longitudinal memory of scan results, routes alerts to configured channels, and uses AI-powered skills for intelligent security testing.

```bash
# Build with agent support
cargo build --release --features rest-api

# Run the agent
./slapper agent run --portfolio /path/to/portfolio.json
```

See [docs/AGENT.md](docs/AGENT.md) for full documentation.

## Docker Usage

```bash
# Start test environment with vulnerable targets
docker-compose --profile testing up -d dvwa

# Run scans against containerized target
docker-compose --profile testing run --rm slapper fuzz http://dvwa.target.local/login -t xss
```

See [DOCKER_COMPOSE.md](DOCKER_COMPOSE.md) for detailed Docker setup.

## Documentation

- [Safety and Scope Enforcement](docs/SAFETY.md) - Authorization, risk tiers, scope rules
- [Canonical Findings Schema](docs/FINDINGS_SCHEMA.md) - Finding structure, fingerprinting, redaction
- [Auth Context Configuration](docs/AUTH_CONTEXT.md) - Multi-role testing, env interpolation
- [Baselines and Differential Scans](docs/BASELINES_AND_DIFFS.md) - Comparing scan results over time
- [API Testing with OpenAPI Schemas](docs/API_TESTING.md) - Schema import, fuzz target generation
- [Agent Documentation](docs/AGENT.md) - Autonomous agent setup and usage
- [Capabilities](docs/CAPABILITIES.md) - Feature matrix and capabilities overview

## Security Considerations

- **Always ensure you have explicit permission** to test targets
- Use the scope file to restrict testing to authorized systems
- Use rate limiting to avoid overwhelming targets: `--rate-limit 10`
- Consider stealth mode for evasive testing: `--stealth`

## Troubleshooting

**Permission denied when running packet capture**
Packet capture requires root/sudo privileges. Run with `sudo slapper packet capture -i eth0`.

**Panic: "command X alias X is duplicated"**
Update to the latest version from the repository.

**Target rejected by scope file**
Ensure your target matches an `allowed_targets` pattern or CIDR range in your scope TOML file. Use `slapper plan` to preview what targets will be accepted.

**Build fails with missing system packages**
Install the required system dependencies for your platform. See the System Dependencies section above.

**High memory usage during large scans**
Reduce concurrency with `--concurrency 10` or use a more targeted port range with `-p`.

## Responsible Use

Slapper is designed for authorized security testing only. Use it against systems you own, operate, or have explicit written authorization to test. Always define scope files, use rate limits, and prefer local lab environments for development and regression testing.

## License

Licensed under either Apache License 2.0 or MIT license at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
