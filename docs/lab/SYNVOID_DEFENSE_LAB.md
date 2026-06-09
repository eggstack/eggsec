# Synvoid Defense Lab

## Purpose

Eggsec's defense-lab and hazardous-lab modes exist to validate the security and resilience of systems you own or are explicitly authorized to test. The primary use case is hardening Synvoid — a distributed WAF and network security platform — against abuse, evasion, and resource exhaustion.

These tools are **not** generic offensive automation. They are scoped, budgeted, policy-gated workflows designed for repeatable defensive validation.

## Threat Classes Modeled

The defense-lab profiles target these threat classes:

- **Request floods**: HTTP, SYN, UDP, ICMP flood patterns at controlled rates
- **Malformed headers**: HTTP request smuggling, header injection, CRLF injection
- **Protocol edge cases**: TCP/TLS/HTTP ambiguity, incomplete requests, oversized headers
- **WAF evasion attempts**: Payload encoding, chunked transfer, case manipulation, polyglot payloads
- **Rate-limit/tarpit behavior**: Validation that rate limits and tarpits trigger correctly
- **Load-bearing validation**: Confirm services remain responsive under sustained load

## Required Environment

- **Local or private lab only**: All defense-lab profiles require localhost or private CIDR targets
- **Explicit scope file**: Use `--scope` with a TOML scope file restricting targets
- **Conservative budgets**: Presets enforce max duration, request count, and concurrency limits
- **Feature gates**: Stress testing and packet inspection require build features

## Three Operating Modes

| Mode | Risk Level | Use Case |
|------|-----------|----------|
| `standard-assessment` | Passive to SafeActive | Ordinary scoped scanning and fuzzing |
| `defense-lab` | Up to Intrusive | Local WAF regression, Synvoid validation, protocol edge testing |
| `hazardous-lab` | Up to AgentAutonomous | Raw packets, flood stress, proxy rotation, distributed stress |

## Example Scope Files

### scope-localhost.toml

```toml
[[allowed_targets]]
pattern = "127.0.0.1"
description = "Localhost"

[[allowed_targets]]
pattern = "::1"
description = "IPv6 localhost"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Private lab range"

require_explicit_scope = true
```

### scope-synvoid-lab.toml

```toml
[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Synvoid lab network"

[[allowed_targets]]
cidr = "172.16.0.0/12"
description = "Container lab network"

[[allowed_targets]]
pattern = "localhost"
description = "Local development"

[[excluded_targets]]
cidr = "10.0.1.0/24"
description = "Management network - do not test"

require_explicit_scope = true
max_requests_per_second = 100
```

## Example Profiles

| Profile | Mode | Max Risk | Stages | Use Case |
|---------|------|----------|--------|----------|
| `defense-lab` | defense-lab | Intrusive | PortScan, Fingerprint, Endpoint, Waf, Fuzz | Comprehensive defense validation |
| `synvoid-local` | defense-lab | Intrusive | PortScan, Fingerprint, Endpoint, Waf | Local Synvoid WAF validation |
| `waf-regression` | defense-lab | Intrusive | PortScan, Fingerprint, Waf | WAF payload regression |
| `protocol-edge` | defense-lab | SafeActive | PortScan, Fingerprint | Malformed protocol testing |
| `nse-safe` | defense-lab | SafeActive | PortScan, Fingerprint, Endpoint | Sandboxed NSE scripts |

## Example WAF Regression Run

```bash
# 1. Inspect what would happen
eggsec policy-explain \
  --target http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml

# 2. Run WAF regression
eggsec scan http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json -o reports/waf-regression.json

# 3. View the report
eggsec report --input reports/waf-regression.json
```

## Example Synvoid Local Run

```bash
eggsec scan http://127.0.0.1:8080 \
  --profile synvoid-local \
  --scope examples/scope-localhost.toml \
  --json -o reports/synvoid-local.json
```

## Example Defense Lab Full Run

```bash
eggsec scan http://127.0.0.1:8080 \
  --profile defense-lab \
  --scope examples/scope-localhost.toml \
  --json -o reports/defense-lab.json
```

## Expected Outputs

Reports include:
- **Policy summary**: Which mode, risk level, and intended use were evaluated
- **Scope summary**: Which targets were allowed/excluded
- **Budget summary**: Duration, request count, and termination reason
- **Findings**: Security issues discovered during the run
- **WAF behavior matrix** (for waf-regression): Per-payload-family blocked/allowed/challenged counts
- **Baseline diff**: Changes relative to a previous run (when using `eggsec report diff`)

## Safety Constraints

1. Defense-lab profiles **require** localhost or private CIDR targets
2. All operations go through scope validation before execution
3. Budgets enforce finite duration and request/packet limits
4. Policy decisions are recorded with unique IDs for auditability
5. High-risk operations require explicit policy enablement in config

## Non-Goals

- This is **not** an exploitation framework
- These tools are **not** for unauthorized testing of third-party systems
- No credential brute-forcing, no botnet simulation, no real DDoS

## See Also

- `docs/SAFETY.md` — Scope enforcement and operation risk tiers
- `docs/BASELINES_AND_DIFFS.md` — Baseline comparison and diff workflow
- `docs/FINDINGS_SCHEMA.md` — Finding structure and fingerprinting
- `architecture/defense_lab.md` — Architecture details for the defense-lab system
