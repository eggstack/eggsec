# Eggsec - Rust Security Assessment Engine

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing, local lab validation, WAF regression, CI security checks, and agent-readable security workflows.

## What Eggsec Is

A command-line security assessment tool designed for security professionals, developers, and defensive teams:

- **Discover attack surfaces** — Reconnaissance, subdomain enumeration, technology detection
- **Assess web application security** — Find vulnerabilities like SQL injection, XSS, SSRF, and more
- **Test infrastructure** — Scan ports, fingerprint services, discover endpoints
- **Evaluate defenses** — Test WAF detection and evasion-resistance
- **Load test** — Measure application performance under controlled load
- **Repeat assessments** — Pipeline scans with customizable profiles for regression workflows

| Capability | Description |
|------------|-------------|
| **Scoped Repeatable Testing** | Run the same assessment profiles repeatedly for regression validation |
| **Rust-Native Primitives** | High-performance async I/O, no external runtime dependencies |
| **Structured Outputs** | JSON, SARIF, JUnit, HTML, CSV for humans, CI, and agents |
| **WAF and Defense Validation** | Detection of 34 WAF products with evasion-resistance testing |
| **Local Lab/Regression Workflows** | Repeatable profiles against local test environments |
| **Optional NSE Compatibility** | Curated Nmap NSE script support as an optional build layer |

For the full capability matrix with risk tiers, feature gates, surface exposure, and scope requirements, see [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md).

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for workspace crate ownership, enforcement model, and execution flows. See [`docs/COMMAND_REGISTRY.md`](docs/COMMAND_REGISTRY.md) for the command registry and dispatch architecture. See [`docs/ARCHITECTURE_INVARIANTS.md`](docs/ARCHITECTURE_INVARIANTS.md) for the 41 normative invariants.

## Safety Model

Eggsec enforces defense-in-depth safety: scope files restrict targets, configuration defaults keep aggressive capabilities disabled until opted in, and feature gating prevents accidental invocation of intrusive modules.

**Scope files** restrict every scan to explicitly authorized targets. When `require_explicit_scope = true`, any target not in the allowed list is rejected before a single packet is sent.

```toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "*.lab.internal"
description = "Lab environment"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"
```

**Execution profiles** separate manual operator-directed discretion from hard enforcement in automated modes. `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate for all surfaces (CLI, TUI, REST, MCP, agent, CI).

```bash
# Manual permissive (default: operator-directed)
eggsec scan example.com --profile quick

# Manual strict (hard enforcement)
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# MCP/Agent strict (hard enforcement; override flags ignored)
eggsec codegg-mcp --stdio --scope scope.toml
```

See [docs/SAFETY.md](docs/SAFETY.md) for authorization, risk tiers, and scope rules. See [docs/ENFORCEMENT_MODES.md](docs/ENFORCEMENT_MODES.md) for the dual-mode enforcement contract.

## Quick Start

### Workspace Layout

| Crate | Purpose |
|-------|---------|
| `eggsec-core` | Dependency-light types, constants, shared primitives |
| `eggsec-tool-core` | Core data types for the tool abstraction layer |
| `eggsec` | Assessment engine library (no binary) |
| `eggsec-nse` | Optional Nmap NSE compatibility runtime |
| `eggsec-tui` | Terminal UI adapter (`ratatui`/`crossterm`) |
| `eggsec-cli` | CLI binary entry point |
| `eggsec-output` | Report formatting (JSON, CSV, HTML, SARIF, JUnit, Markdown) |
| `eggsec-agent` | Agent coordination primitives |
| `eggsec-db-lab` | Database pentesting domain crate |
| `eggsec-web-proxy` | Web proxy and MITM interception domain crate |
| `eggsec-mobile-lab` | Mobile app security analysis domain crate |
| `eggsec-daemon` | Long-running daemon host for persistent sessions |
| `eggsec-runtime` | Frontend-neutral runtime with task lifecycle management |
| `eggsec-ui-model` | Frontend-neutral view DTOs |
| `eggsec-python` | Python bindings (PyO3/maturin) |

### Build and Run

```bash
git clone https://github.com/eggstack/eggsec.git
cd eggsec
cargo build --release -p eggsec-cli

# Generate and validate config
./target/release/eggsec --generate-config > eggsec.toml
./target/release/eggsec config validate --config eggsec.toml

# Plan a scan (dry-run, no traffic sent)
./target/release/eggsec plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

# Run a scoped scan against localhost
./target/release/eggsec scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

See [docs/BUILD.md](docs/BUILD.md) for system dependencies, feature flags, and build examples.

## Pipeline Profiles

Eggsec includes 18 built-in profiles that chain multiple security tests together. Common profiles: `quick` (port scan + fingerprinting), `web` (endpoint discovery + fuzzing), `full` (all stages), `api` (GraphQL/JWT/OAuth), `defense-lab` (local regression). See [docs/PIPELINE.md](docs/PIPELINE.md) for the full profile reference and command examples.

```bash
eggsec scan example.com --profile quick    # port scan + fingerprinting
eggsec scan example.com --profile web      # endpoint discovery + fuzzing
eggsec scan example.com --profile full     # all stages including load testing
eggsec scan localhost:8080 --profile defense-lab --json
```

## Python Bindings

Eggsec provides Python bindings via [PyO3](https://pyo3.rs) and [maturin](https://github.com/PyO3/maturin). **Status: pre-1.0 release candidate — 22 stable operations, not yet published to PyPI.** Windows is outside the primary support scope.

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")
```

See [`docs/python/`](docs/python/) for the full documentation: [Quick Start](docs/python/quickstart.md), [API Reference](docs/python/api-reference.md), [Scope & Safety](docs/python/scope-and-safety.md), [Domain Maturity](docs/python/domain-maturity.md).

## Daemon

The `eggsec-daemon` crate provides optional durable session state backed by SQLite. Session snapshots persist across restarts. See [docs/DAEMON.md](docs/DAEMON.md) for transport configuration, schema, and CLI commands.

```bash
eggsec daemon start
eggsec daemon history
eggsec daemon show <session-id>
```

## Agent and Orchestration

Eggsec includes a security agent for continuous monitoring and scheduled assessments. The agent always requires an explicit scope manifest and uses `AgentStrict` execution profile.

```bash
cargo build --release --features rest-api
./eggsec agent run --scope scope.toml --portfolio /path/to/portfolio.json
```

See [docs/AGENT.md](docs/AGENT.md) for full documentation.

## Nmap/NSE Compatibility

Eggsec includes optional Nmap NSE script support as a build layer (`--features nse`). It is not a full Nmap replacement — the goal is selective practical NSE compatibility for useful script categories. Selected behaviors may be promoted into Rust-native probes over time for repeatability, performance, and safety. See [docs/NSE_COMPATIBILITY.md](docs/NSE_COMPATIBILITY.md).

## Docker

```bash
docker-compose --profile testing up -d dvwa
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/login -t xss
```

See [docker-compose.yml](docker-compose.yml) for configuration.

## Documentation

| Document | Description |
|----------|-------------|
| [Safety and Scope Enforcement](docs/SAFETY.md) | Authorization, risk tiers, scope rules |
| [Enforcement Modes](docs/ENFORCEMENT_MODES.md) | Dual-mode enforcement contract |
| [Capability Matrix](docs/CAPABILITY_MATRIX.md) | Operation/risk/feature/exposure matrix |
| [Feature Matrix](docs/FEATURE_MATRIX.md) | Feature inventory and classification |
| [Build Features](docs/BUILD.md) | System dependencies, feature flags, build examples |
| [Pipeline Profiles](docs/PIPELINE.md) | Profile reference, command examples, defense-lab mode |
| [Daemon Persistence](docs/DAEMON.md) | Session persistence, transport, CLI commands |
| [Command Registry](docs/COMMAND_REGISTRY.md) | Command registry and dispatch |
| [Architecture](docs/ARCHITECTURE.md) | Workspace overview, enforcement model |
| [Verification Contract](docs/VERIFICATION.md) | Mandatory vs optional CI checks, merge vs release readiness |
| [Releasing](docs/RELEASING.md) | Manual maintainer-controlled release procedure |
| [Extending Eggsec](docs/EXTENSIBILITY.md) | Adding operations, domains, commands, tools |

Additional docs: [Web Proxy](docs/WEB_PROXY.md), [Database Pentesting](docs/DATABASE_PENTEST.md), [Wireless Testing](docs/WIRELESS.md), [Mobile Analysis](docs/MOBILE.md), [Auth Testing](docs/AUTH_LAB.md), [Agent](docs/AGENT.md), [Usage Guide](docs/USAGE.md), [Findings Schema](docs/FINDINGS_SCHEMA.md).

## Responsible Use

Eggsec is designed for authorized security testing of systems you own, operate, or have explicit written authorization to test. Always define scope files, use rate limits, and prefer local lab environments for development and regression testing.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines. For the full verification contract (mandatory vs optional CI checks, merge vs release readiness), see [docs/VERIFICATION.md](docs/VERIFICATION.md). For adding new operations, domains, commands, tools, TUI actions, report outputs, or features, start with the [Extensibility Guide](docs/EXTENSIBILITY.md).
