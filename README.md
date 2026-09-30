# Eggsec - Rust Security Assessment Engine

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing: reconnaissance, port scanning, web fuzzing, WAF evaluation, load testing, and repeatable pipeline assessments with structured JSON/SARIF/JUnit/HTML/CSV output.

For authorized testing of systems you own or have explicit written permission to test. See [Safety](#safety) below.

## Quickstart

```bash
git clone https://github.com/eggstack/eggsec.git
cd eggsec
cargo build --release -p eggsec-cli

# Generate and validate config
./target/release/eggsec --generate-config > eggsec.toml
./target/release/eggsec config validate --config eggsec.toml

# Preview what a scan would do (dry-run, no traffic sent)
./target/release/eggsec plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

# Run a scoped scan against localhost (loopback targets need explicit opt-in)
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 ./target/release/eggsec scan 127.0.0.1 \
  --profile quick --scope examples/scope-localhost.toml --allow-private-resolution --json
```

Example output (your open ports will differ):

```json
{"target": "127.0.0.1",
 "stage_results": [{"stage": "PortScan", "success": true}, {"stage": "Fingerprint", "success": true}],
 "open_ports": [{"port": 22, "service": "SSH"}, {"port": 80, "service": "HTTP"}]}
```

## Common commands

```bash
eggsec scan example.com --profile quick        # port scan + fingerprinting
eggsec scan example.com --profile web          # endpoint discovery + fuzzing
eggsec scan example.com --profile full         # all stages including load testing
eggsec fuzz https://example.com/search?q=test -t xss   # fuzz with security payloads
eggsec recon example.com                       # DNS, WHOIS, subdomains, tech detection
eggsec waf https://example.com                 # WAF detection (34 products) + evasion resistance
eggsec doctor                                  # hermetic dependency/platform check
```

Full command reference: [`docs/USAGE.md`](docs/USAGE.md). Pipeline profiles: [`docs/PIPELINE.md`](docs/PIPELINE.md).

## Safety

Every scan runs through `EnforcementContext::evaluate()` before dispatch. Scope files restrict targets; execution profiles separate manual operator discretion from hard enforcement in automated modes:

```bash
# Manual strict (hard enforcement)
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# MCP/Agent strict (override flags ignored)
eggsec codegg-mcp --stdio --scope scope.toml
```

Details: [`docs/SAFETY.md`](docs/SAFETY.md) (authorization, risk tiers, scope rules) and [`docs/ENFORCEMENT_MODES.md`](docs/ENFORCEMENT_MODES.md).

## Feature-gated builds

The default build covers scanning, fuzzing, recon, WAF, pipelines, and reporting. These need explicit features (see [`docs/BUILD.md`](docs/BUILD.md) and [`docs/FEATURE_MATRIX.md`](docs/FEATURE_MATRIX.md)):

```bash
cargo build --release -p eggsec-cli --features daemon-client  # eggsec daemon/session/task
cargo build --release -p eggsec-cli --features rest-api       # serve, mcp-serve, codegg-mcp, agent
cargo build --release -p eggsec-cli --features nse            # Nmap NSE script support
```

NSE compatibility notes: [`docs/NSE_COMPATIBILITY.md`](docs/NSE_COMPATIBILITY.md). Docker lab targets: [`DOCKER.md`](DOCKER.md).

## Python bindings

Pre-1.0 release candidate (22 stable operations, not yet on PyPI):

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")
```

Full docs: [`docs/python/quickstart.md`](docs/python/quickstart.md) and [`docs/python/api-reference.md`](docs/python/api-reference.md).

## Documentation

| Document | Contents |
|----------|----------|
| [Usage Guide](docs/USAGE.md) | Command examples and flags |
| [Safety](docs/SAFETY.md) | Authorization, risk tiers, scope rules |
| [Capability Matrix](docs/CAPABILITY_MATRIX.md) | Operations, risk tiers, feature gates |
| [Architecture](docs/ARCHITECTURE.md) | Crate ownership, enforcement model |
| [Verification](docs/VERIFICATION.md) | Mandatory vs optional CI checks |
| [Extending Eggsec](docs/EXTENSIBILITY.md) | Adding operations, domains, commands |
| [Releasing](docs/RELEASING.md) | Manual release procedure |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Run `make check` before submitting Rust changes (fmt, no-default checks, dependency policy, clippy, tests, architecture guards); Python-facing changes also require `make check-python`. Contract: [`docs/VERIFICATION.md`](docs/VERIFICATION.md).

## License

MIT. See [LICENSE](LICENSE).
