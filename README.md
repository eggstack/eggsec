# Eggsec - Rust Security Assessment Engine

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing, local lab validation, WAF regression, CI security checks, and agent-readable security workflows.

## What Eggsec is

Eggsec is a command-line security assessment tool designed for security professionals, developers, and defensive teams who need to:

- **Discover attack surfaces** - Reconnaissance, subdomain enumeration, technology detection
- **Assess web application security** - Find vulnerabilities like SQL injection, XSS, SSRF, and more
- **Test infrastructure** - Scan ports, fingerprint services, discover endpoints
- **Evaluate defenses** - Test WAF detection and evasion-resistance
- **Load test** - Measure application performance under controlled load
- **Repeat assessments** - Pipeline scans with customizable profiles for regression workflows

### Why Eggsec?

| Capability | Description |
|------------|-------------|
| **Scoped Repeatable Testing** | Run the same assessment profiles repeatedly for regression validation |
| **Rust-Native Primitives** | High-performance async I/O, no external runtime dependencies |
| **Structured Outputs** | JSON, SARIF, JUnit, HTML, CSV for humans, CI, and agents |
| **WAF and Defense Validation** | Detection of 34 WAF products with evasion-resistance testing |
| **Local Lab/Regression Workflows** | Repeatable profiles against local test environments |
| **Optional NSE Compatibility** | Curated Nmap NSE script support as an optional layer |

### Core Capabilities

| Category | Capabilities |
|----------|-------------|
| **Reconnaissance** | DNS enumeration, subdomain discovery, WHOIS, tech stack detection, CVE mapping, cloud asset discovery, CORS analysis |
| **Web Security** | SQLi, XSS, SSRF, Path Traversal, ReDoS, Header Injection, SSTI, IDOR testing |
| **API Security** | GraphQL introspection/injection, JWT analysis, OAuth/OIDC testing, gRPC fuzzing |
| **Scanning** | Port scanning, service fingerprinting (42+ protocols), endpoint discovery |
| **WAF** | Detection of 34 WAF products, header manipulation, HTTP smuggling, evasion-resistance testing |
| **Load Testing** | High-concurrency HTTP testing with detailed metrics |
| **Controlled Stress** | SYN, UDP, HTTP, TCP, ICMP flood testing (requires `--features stress-testing`) |
| **Auth Control Validation** | Brute-force, credential stuffing, lockout/MFA/rate-limit/timing testing via `eggsec auth-test` (standalone defense-lab CLI; runtime policy gate only via `CredentialTesting` + `allow_credential_testing`; local `AuthTestReport`/`AuthFinding` only; for validating auth controls in authorized labs, not credential attacks; see docs/AUTH_LAB.md + architecture/auth.md) |
| **Proxy Management** | SOCKS4, SOCKS5, HTTP, HTTPS, Tor proxy pool with health checking |
| **Cluster Mode** | Distributed scanning with worker/coordinator architecture |
| **Repeatable Profiles** | 16 pipeline profiles, session resumption, multiple output formats |

## What Eggsec is not

Eggsec is not an exploitation framework, botnet component, credential attack platform, or tool for unscoped internet scanning. The `auth-test` command exists for defense validation of authentication controls (lockout policies, MFA enforcement, rate limiting, etc.) under strict scope/policy gating — it is not a credential attack platform (see architecture/auth.md for adopted model details: runtime policy gate, local findings only, standalone CLI distinct from pipeline `ScanProfile::Auth`). Some modules can generate aggressive traffic or security-test payloads, so advanced capabilities are feature-gated and intended for systems you own, operate, or have explicit authorization to test.

## Why Low-Level Features Exist

Eggsec includes stress testing, raw packet inspection, proxy management, and distributed scanning capabilities. These tools exist to validate the resilience of systems you own or are explicitly authorized to test — such as Synvoid, a distributed WAF platform.

These capabilities are framed as **defense-lab** and **hazardous-lab** workflows with:
- Mandatory scope files restricting targets to localhost or private lab networks
- Finite execution budgets (duration, request count, packet count)
- Policy decision records for auditability
- Clear CLI help text indicating the operating mode and required features

They are **not** generic offensive automation.

## Safety Model

Eggsec enforces a defense-in-depth safety model built around scope control, configuration defaults, and feature gating.

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

**Configuration defaults** keep aggressive capabilities disabled until you opt in. Rate limits, concurrency caps, and timeouts are configurable per profile. Dry-run planning (`eggsec plan`) previews what a scan will do without sending traffic.

**Feature gating** ensures intrusive modules (stress testing, raw packet crafting, headless browser, NSE, database storage, container scanning, and more) require explicit build flags and cannot be invoked accidentally.

**Execution profiles** separate manual CLI/TUI operator-directed discretion from hard enforcement in strict and automated modes:
- **Manual CLI/TUI (default)**: `ManualPermissive` — operator-directed: warnings for safe scope ambiguity/missing scope; `RequireConfirmation` (with confirm/override) for discretion cases (explicit allowlist miss with positive rules, exclusions, high-risk, non-baseline capabilities, private resolution, cross-host redirect, target expansion). `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. are required for their classes. Override flags honored and audited only here; strict profiles/MCP/agent never honor overrides.
- **Manual strict**: `--strict-scope` uses `ManualGuarded` — hard enforcement (no discretion path).
- **MCP server**: always `McpStrict` (via `EnforcementContext`); explicit scope manifest (`LoadedScope::is_explicit_manifest()`) required for networked operations; warnings and `RequireConfirmation` treated as denials. Manual override flags ignored.
- **Agent**: `AgentStrict`; explicit scope manifest required; per-scan enforcement re-evaluated immediately before dispatch (in addition to startup gating); override flags ignored.
- **CI**: `CiStrict` — hard enforcement; override flags ignored.

`EnforcementContext::evaluate()` is the mandatory central boundary for all paths (CLI, TUI, MCP, agent, CI): performs LoadedScope provenance checks (strict/automated profiles require explicit manifest for networked ops), applies DenialClass downgrade (ManualPermissive only for safe ScopeMissing/TargetOutOfScope when no positive rules), positive capability allow for strict profiles, `RequireConfirmation` for manual discretion cases, and full risk/feature/policy enforcement. `DenialClass` drives `ManualPermissive` downgrade logic for safe scope-selection misses only (never for explicit exclusions, feature/risk/capability/hazard denials, or when positive scope rules were declared). Strict profiles, higher-risk operations, and automated paths never downgrade; `RequireConfirmation` is treated as hard `Deny` outside ManualPermissive. MCP production uses `McpServer::with_enforcement`; legacy `policy_decision_for_mcp_call` / direct `evaluate_operation_policy` deprecated for denial paths.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution. Baseline strict-automated capabilities are `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`; non-baseline require explicit `allowed_capabilities`. Manual override flags have no effect for MCP, agent, CI, or strict manual profiles.

MCP and autonomous agent paths are always strict and cannot be downgraded or overridden by model-supplied flags.

```bash
# Manual permissive (default: operator-directed; warn + confirm/override for discretion)
eggsec scan example.com --profile quick

# Manual permissive with override (audited; --yes narrow, use dedicated for private/redirect)
eggsec waf-stress https://lab.example --allow-high-risk --manual-override-reason "authorized Synvoid regression"
eggsec scan 10.0.0.5 --allow-private-resolution --manual-override-reason "lab private target"

# Manual strict (hard enforcement)
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# MCP strict (hard enforcement; override flags ignored)
eggsec codegg-mcp --stdio --scope scope.toml

# Agent strict (hard enforcement; override flags ignored)
eggsec agent run --scope scope.toml --portfolio portfolio.json
```

See [docs/SAFETY.md](docs/SAFETY.md) for full details on authorization, risk tiers, and scope rule evaluation.

## Quick Start

### Workspace Layout

Eggsec is organized as a Cargo workspace with eight crates:

| Crate | Purpose |
|-------|---------|
| `eggsec-core` | Dependency-light types, constants, shared primitives |
| `eggsec-tool-core` | Core data types for the tool abstraction layer (requests, responses, findings, errors) |
| `eggsec` | Assessment engine library (no binary) |
| `eggsec-nse` | Optional Nmap NSE compatibility runtime |
| `eggsec-tui` | Terminal UI adapter (`ratatui`/`crossterm`, packaged themes, non-blocking background loading). 10-phase architecture/usability pass (2026-06-11) added UiAction layer, OverlayController, TabSpec registry, delegated descriptors, manual-mode preflight/status indicators (enforcement posture, scope provenance, risk, "will confirm?"), global task strip, action-complete palette, copy-CLI equivalent, small-terminal degraded layouts + "too small" fallback, and semantic styling tokens for risk/policy/task/scope — all while preserving the adapter boundary and strict enforcement for agent/MCP paths. |
| `eggsec-cli` | CLI binary entry point |
| `eggsec-output` | Report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown) |
| `eggsec-agent` | Agent coordination primitives (registry, scheduler, lifecycle, communication) |

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install libpcap-dev libssl-dev wireless-tools

# Fedora/RHEL
sudo dnf install libpcap-devel openssl-devel wireless-tools
```

### Build and Run

```bash
# Clone and build
git clone https://github.com/eggstack/eggsec.git
cd eggsec
cargo build --release -p eggsec-cli

# Generate a config file
./target/release/eggsec --generate-config > eggsec.toml

# Validate your config
./target/release/eggsec config validate --config eggsec.toml

# Plan a scan (dry-run, no traffic sent)
./target/release/eggsec plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

# Run a scoped scan against localhost
./target/release/eggsec scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

### Installing to PATH

```bash
# Install to ~/.cargo/bin/eggsec
cargo install --path crates/eggsec-cli

# With all features
cargo install --path crates/eggsec-cli --features full

# Verify
eggsec --version
```

## Pipeline Profiles

Eggsec includes 16 built-in profiles that chain multiple security tests together. Choose the profile that matches your assessment goals.

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
| **auth** | JWT, OAuth, IDOR focused (pipeline: PortScan+Fingerprint+EndpointScan+Fuzz; distinct from `auth-test` credential/brute/MFA control validation) |
| **defense-lab** | Local lab regression testing |
| **synvoid-local** | Local SYN scan testing |
| **waf-regression** | WAF regression testing |
| **protocol-edge** | Protocol edge case testing (requires `packet-inspection`) |
| **nse-safe** | Safe NSE script execution (requires `nse`) |

Defense-lab profiles require private/localhost targets and enforce conservative budgets. Use `eggsec policy-explain` to inspect what a profile would do before running it.

```bash
# Quick scan - port scan + fingerprinting
./eggsec scan example.com --profile quick

# Web assessment - endpoint discovery + vulnerability fuzzing
./eggsec scan example.com --profile web

# Full assessment - all stages including load testing
./eggsec scan example.com --profile full

# API-focused - GraphQL/JWT/OAuth testing
./eggsec scan example.com --profile api
```

## Core Workflows

- **Scoped web assessment** - Port scanning, service fingerprinting, endpoint discovery, and vulnerability fuzzing against authorized targets
- **WAF/defense validation in lab** - Detect 34 WAF products, test evasion resistance, run regression suites against local WAF instances
- **CI regression checks** - Structured output (SARIF, JUnit, JSON) for integration into GitHub Actions, GitLab CI, and other pipelines
- **Agent/MCP integration** - Autonomous security agent with skills, portfolio management, and structured findings for AI-driven workflows
- **Optional NSE compatibility** - Curated Nmap NSE script support as an optional build layer

## Quick Command Reference

```bash
# Load testing
./eggsec load https://example.com -n 1000 -c 50

# Port scanning
./eggsec scan-ports example.com -p 1-1000 -c 100

# Endpoint discovery
./eggsec scan-endpoints https://example.com

# Vulnerability fuzzing
./eggsec fuzz https://example.com/api -t sqli,xss

# GraphQL security testing
./eggsec graphql https://api.example.com/graphql

# WAF detection and bypass testing
./eggsec waf https://example.com --bypass

# Reconnaissance
./eggsec recon example.com

# Wireless (standalone-complete passive, lab-only; requires --features wireless + root/CAP_NET_ADMIN + wireless-tools/iwlist)
./eggsec wireless wlan0 --repeat 3

# Wireless with known-good baseline (suppress rogue heuristic for lab APs), dry-run, or full details; human output summarizes rogue candidates by default
# Add --detect_suspicious for the full findings list
./eggsec wireless wlan0 --repeat 5 --known-good ./lab-aps.txt
./eggsec wireless wlan0 --dry-run --json
./eggsec wireless wlan0 --detect_suspicious --repeat 3

# Resume a previous scan
./eggsec resume session.json
```

Human-readable wireless output summarizes rogue candidates by default; add `--detect_suspicious` when you want the full findings list.

Run `eggsec --help` or `eggsec <command> --help` for the full command reference with all options.

### Lab Defense Commands

| Command | Mode | Description |
|---------|------|-------------|
| `eggsec policy-explain` | - | Explain policy decisions for a target/profile |
| `eggsec scope-explain` | - | Explain scope matching for a target |
| `eggsec scan --profile defense-lab` | defense-lab | Comprehensive local defense validation |
| `eggsec scan --profile waf-regression` | defense-lab | WAF payload regression |
| `eggsec scan --profile synvoid-local` | defense-lab | Synvoid-specific local validation |
| `eggsec scan --profile protocol-edge` | defense-lab | Malformed protocol edge testing |
| `eggsec auth-test <target>` | defense-lab | High-risk credential control validation (brute-force, stuffing, lockout, MFA, rate-limit, timing; policy-gated via `CredentialTesting` risk + `allow_credential_testing`). Standalone CLI; local `Auth*` types only (no `ScanReportData` conversion). See `docs/AUTH_LAB.md` + architecture/auth.md. |
| `eggsec wireless <iface>` | defense-lab (passive) | Standalone-complete passive WiFi recon (iwlist): Open/WEP/WPA/WPA2/WPA3/Enterprise + WPS/hidden/transition/weak-signal detection, vuln findings, rogue/Evil-Twin heuristic (passive; security-diff elevates to Medium). Supports `--repeat` (diffs + temporal summary), `--known-good` allowlist (suppresses rogue for lab baselines), `--dry-run` (plan/CI, valid JSON), `--detect_suspicious` (full rogue details; summarized by default in human output). Requires `--features wireless` + root/CAP_NET_ADMIN + wireless-tools/iwlist. See docs/WIRELESS.md. |

## Build Features

| Feature | Description | Status |
|---------|-------------|--------|
| `stress-testing` | SYN/UDP/ICMP floods, proxy management, IP spoofing | Lab-only |
| `packet-inspection` | Live packet capture, traceroute | Experimental |
| `nse` | Nmap NSE script compatibility | Experimental |
| `nse-ssh2` | NSE with SSH2/libssh2 support | Experimental |
| `nse-sandbox` | Restrict dangerous Lua operations | Experimental |
| `api-schema` | OpenAPI v3 schema-based fuzzing | Stable |
| `sbom` | SBOM generation (CycloneDX, SPDX) | Stable |
| `rest-api` | REST API server for agent integration | Experimental |
| `grpc-api` | gRPC API server | Experimental |
| `ws-api` | WebSocket pub/sub | Experimental |
| `ai-integration` | AI planner, script generation, autonomous agent | Experimental |
| `websocket` | WebSocket security testing | Stable |
| `headless-browser` | DOM XSS and SPA crawling | Stable |
| `database` | SQLx-based PostgreSQL persistence | Stable |
| `container` | Kubernetes/Docker security scanning | Stable |
| `cloud` | AWS/GCP/Azure asset discovery | Stable |
| `git-secrets` | Git secrets scanning | Stable |
| `wireless` | WiFi scanning (standalone-complete passive recon + security analysis; summary-by-default rogue heuristic; --repeat, --known-good, --dry-run, --detect_suspicious) | Stable |
| `pdf` | PDF report generation | Stable |
| `advanced-hunting` | Advanced threat hunting | Stable |
| `compliance` | Compliance scanning (OWASP, PCI, HIPAA, SOC2) | Stable |
| `external-integrations` | Jira, GitHub, GitLab connectors | Stable |
| `finding-workflow` | Finding lifecycle management | Stable |
| `vuln-management` | Vulnerability triage and CVSS scoring | Stable |
| `full` | All features combined (excludes `grpc-api`, `ws-api`, `pdf`) | - |

### Build Examples

```bash
# Default build - load testing, scanning, fuzzing, WAF testing
cargo build --release -p eggsec-cli

# With stress testing (controlled flood testing, proxy pool)
cargo build --release -p eggsec-cli --features stress-testing

# With packet inspection (live capture)
cargo build --release -p eggsec-cli --features packet-inspection

# With NSE support
cargo build --release -p eggsec-cli --features nse

# Full build - all features
cargo build --release -p eggsec-cli --features full
```

## System Dependencies

| Feature | Required Packages | Install (Ubuntu/Debian) |
|---------|-------------------|--------------------------|
| `packet-inspection` | `libpcap-dev` | `sudo apt-get install libpcap-dev` |
| `wireless` | `wireless-tools` | `sudo apt-get install wireless-tools` (provides `iwlist` scanner) |
| `nse` | `libssl-dev` | `sudo apt-get install libssl-dev` |

## Output Formats

| Format | Use Case |
|--------|----------|
| Pretty | Human-readable terminal output (default) |
| JSON | Machine parsing, automation |
| Compact | Condensed terminal output |
| HTML | Human-readable reports |
| CSV | Spreadsheet analysis |
| SARIF | CI/CD security scanning (GitHub, GitLab) |
| JUnit XML | Test integration (CI pipelines) |
| Markdown | Documentation, GitHub issues |

## Defense-Lab Mode

Eggsec can run local, repeatable profiles against defensive systems for regression testing.

- **Repeatable adversarial traffic** - Run the same probe suite multiple times to measure changes in WAF or protocol behavior
- **Structured observations and baseline diffs** - Compare current results against a saved baseline to identify regressions or improvements
- **WAF regression testing** - Validate that WAF rules continue to catch known evasion patterns after updates

```bash
# Run a defense-lab profile against a local instance
./eggsec scan localhost:8080 --profile defense-lab --json -o baseline.json

# Run WAF regression testing
./eggsec scan localhost:8080 --profile waf-regression --json
```

## Relationship to Nmap/NSE

Eggsec borrows proven scanning concepts from Nmap but is not a drop-in replacement.

- **NSE is an optional compatibility layer.** Build with `--features nse` to enable curated Nmap NSE script support.
- **No full Nmap parity.** Eggsec does not aim to replicate all Nmap behavior. The goal is broad practical compatibility for useful script categories.
- **NSE is a protocol-testing knowledge source.** Selected behaviors may be promoted into Rust-native probes over time for repeatability, performance, and safety.

## Agent and Orchestration

Eggsec includes a security agent for continuous monitoring and scheduled assessments. The agent maintains longitudinal memory of scan results, routes alerts to configured channels, and uses AI-powered skills for intelligent security testing.

The agent always requires an explicit scope manifest and uses `AgentStrict` execution profile. Networked operations are rejected without a valid scope file.

```bash
# Build with agent support
cargo build --release --features rest-api

# Run the agent with explicit scope
./eggsec agent run --scope scope.toml --portfolio /path/to/portfolio.json
```

See [docs/AGENT.md](docs/AGENT.md) for full documentation.

## Docker Usage

```bash
# Start test environment with vulnerable targets
docker-compose --profile testing up -d dvwa

# Run scans against containerized target
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/login -t xss
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
Packet capture requires root/sudo privileges. Run with `sudo eggsec packet capture -i eth0`.

**Panic: "command X alias X is duplicated"**
Update to the latest version from the repository.

**Target rejected by scope file**
Ensure your target matches an `allowed_targets` pattern or CIDR range in your scope TOML file. Use `eggsec plan` to preview what targets will be accepted.

**Build fails with missing system packages**
Install the required system dependencies for your platform. See the System Dependencies section above.

**High memory usage during large scans**
Reduce concurrency with `--concurrency 10` or use a more targeted port range with `-p`.

**`cargo install` fails with "found a virtual manifest"**
Run `cargo install --path crates/eggsec-cli` instead of bare `cargo install`. The workspace root is a virtual manifest; the binary crate is in `crates/eggsec-cli`.

## Responsible Use

Eggsec is designed for authorized security testing only. Use it against systems you own, operate, or have explicit written authorization to test. Always define scope files, use rate limits, and prefer local lab environments for development and regression testing.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
