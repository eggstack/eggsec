# Pipeline Profiles

Eggsec includes 18 built-in profiles that chain multiple security tests together. Choose the profile that matches your assessment goals.

## Profile Reference

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
| **db-regression** | Database security regression (requires `db-pentest`) |
| **web-proxy** | Web proxy intercept pipeline (requires `web-proxy`) |

Defense-lab profiles require private/localhost targets and enforce conservative budgets. Use `eggsec policy-explain` to inspect what a profile would do before running it.

## Usage Examples

```bash
# Quick scan - port scan + fingerprinting
eggsec scan example.com --profile quick

# Web assessment - endpoint discovery + vulnerability fuzzing
eggsec scan example.com --profile web

# Full assessment - all stages including load testing
eggsec scan example.com --profile full

# API-focused - GraphQL/JWT/OAuth testing
eggsec scan example.com --profile api

# Run a defense-lab profile against a local instance
eggsec scan localhost:8080 --profile defense-lab --json -o baseline.json

# Run WAF regression testing
eggsec scan localhost:8080 --profile waf-regression --json
```

## Defense-Lab Mode

Eggsec can run local, repeatable profiles against defensive systems for regression testing:

- **Repeatable adversarial traffic** — Run the same probe suite multiple times to measure changes in WAF or protocol behavior
- **Structured observations and baseline diffs** — Compare current results against a saved baseline to identify regressions or improvements
- **WAF regression testing** — Validate that WAF rules continue to catch known evasion patterns after updates

## Lab Defense Commands

| Command | Mode | Description |
|---------|------|-------------|
| `eggsec policy-explain` | — | Explain policy decisions for a target/profile |
| `eggsec scope-explain` | — | Explain scope matching for a target |
| `eggsec preflight <operation>` | — | Preview enforcement decision for an operation without executing (shows scope, risk, confirmation requirements, suggested CLI flags) |
| `eggsec scan --profile defense-lab` | defense-lab | Comprehensive local defense validation |
| `eggsec scan --profile waf-regression` | defense-lab | WAF payload regression |
| `eggsec scan --profile synvoid-local` | defense-lab | Synvoid-specific local validation |
| `eggsec scan --profile protocol-edge` | defense-lab | Malformed protocol edge testing |
| `eggsec auth-test <target>` | defense-lab | High-risk credential control validation (brute-force, stuffing, lockout, MFA, rate-limit, timing; policy-gated). Standalone defense-lab CLI (distinct from pipeline `ScanProfile::Auth`). See [AUTH_LAB.md](AUTH_LAB.md). |
| `eggsec proxy-intercept` | defense-lab | Interactive web proxy for HTTP/HTTPS traffic interception. See [WEB_PROXY.md](WEB_PROXY.md). |
| `eggsec wireless <iface>` | defense-lab (passive) | Passive WiFi recon and security analysis. See [WIRELESS.md](WIRELESS.md). |
| `eggsec mobile <path>` | defense-lab (static) | APK/IPA static analysis. See [MOBILE.md](MOBILE.md). |
| `eggsec evasion` | defense-lab | Evasion technique detection (MITRE ATT&CK mapped). |
| `eggsec postex` | defense-lab | Post-exploitation simulation for purple teaming. |
| `eggsec c2` | defense-lab | C2 simulation for purple teaming. |

## Core Workflows

- **Scoped web assessment** — Port scanning, service fingerprinting, endpoint discovery, and vulnerability fuzzing against authorized targets
- **WAF/defense validation in lab** — Detect 34 WAF products, test evasion resistance, run regression suites against local WAF instances
- **CI regression checks** — Structured output (SARIF, JUnit, JSON) for integration into GitHub Actions, GitLab CI, and other pipelines
- **Agent/MCP integration** — Autonomous security agent with skills, portfolio management, and structured findings for AI-driven workflows
- **Optional NSE compatibility** — Curated Nmap NSE script support as an optional build layer

## Quick Command Reference

```bash
# Load testing
eggsec load https://example.com -n 1000 -c 50

# Port scanning
eggsec scan-ports example.com -p 1-1000 -c 100

# Endpoint discovery
eggsec scan-endpoints https://example.com

# Vulnerability fuzzing
eggsec fuzz https://example.com/api -t sqli,xss

# GraphQL security testing
eggsec graphql https://api.example.com/graphql

# WAF detection and bypass testing
eggsec waf https://example.com --bypass

# Reconnaissance
eggsec recon example.com

# Preview enforcement decision for an operation (dry-run policy check)
eggsec preflight scan-ports --target 192.168.1.1
eggsec preflight fuzz --target https://example.com/api --json

# Resume a previous scan
eggsec resume session.json
```

Run `eggsec --help` or `eggsec <command> --help` for the full command reference with all options.

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
