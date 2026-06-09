# Scope Model

Eggsec uses a scope file to constrain all target-bearing operations to authorized systems. Scope enforcement prevents accidental testing of out-of-scope infrastructure.

## Scope File Format

Scope files use TOML (or YAML with `.yml`/`.yaml` extension). See `examples/configs/scope.toml` for a full annotated example.

```toml
require_explicit_scope = true
max_requests_per_second = 100

[[allowed_targets]]
pattern = "*.example.com"
description = "Production web applications"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"

[[excluded_targets]]
pattern = "admin.example.com"
description = "Admin panel - excluded by policy"

excluded_ports = [22, 3389]
```

## Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `require_explicit_scope` | bool | No | When `true`, targets must match an `allowed_targets` rule. When `false` and no rules exist, all non-private targets are allowed. |
| `max_requests_per_second` | int | No | Rate limit (1..=10000). Null means no limit. |
| `allowed_targets` | list | No | Rules defining permitted targets. Empty list + `require_explicit_scope = true` = deny all. |
| `excluded_targets` | list | No | Rules that override `allowed_targets`. Exclusion always wins. |
| `allowed_ports` | list | No | Restrict scanning to specific ports. Null means all non-excluded ports. |
| `excluded_ports` | list | No | Ports always blocked regardless of `allowed_ports`. |

## Allowed Targets

Each `[[allowed_targets]]` rule has:

- **`pattern`** (string) - Hostname or wildcard. Supports:
  - Exact match: `"example.com"` matches only `example.com`
  - Wildcard: `"*.example.com"` matches `sub.example.com` and `example.com`
  - Glob-all: `"*"` matches any hostname (use cautiously)
  - CIDR-in-pattern: `"10.0.0.0/8"` matches any IP in that range
- **`cidr`** (string, optional) - Explicit CIDR notation. Same behavior as CIDR-in-pattern but separated for clarity.
- **`description`** (string, optional) - Human-readable note.

## Excluded Targets

Excluded rules are evaluated **before** allowed rules. If a target matches any exclusion, it is rejected immediately regardless of allowed rules.

## Port Restrictions

```toml
# Only scan these ports
allowed_ports = [80, 443, 8080, 8443]

# Always block these ports (even if in allowed_ports)
excluded_ports = [22, 3389, 3306]
```

Evaluation order: excluded_ports wins, then allowed_ports is checked.

## How Scope Is Enforced

Every target-bearing operation (scan, fuzz, stress test, agent run) goes through scope validation:

1. **Private IP check** - If no CIDR rules exist in `allowed_targets` or `excluded_targets`, the target string is parsed. If it resolves to a private/loopback IP (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, IPv6 ULA/link-local), it is rejected. Hostnames that resolve to private IPs are also blocked.
2. **Exclusion check** - If the target matches any `excluded_targets` rule, it is rejected.
3. **Allowed check** - If `allowed_targets` is non-empty, the target must match at least one rule.
4. **Port check** - If `allowed_ports` or `excluded_ports` are configured, ports are filtered accordingly.

**When CIDR rules are present** (any rule with a `cidr` field), the private IP check is skipped. This allows CIDR ranges like `10.0.0.0/8` to match private IPs. Use this for lab/internal testing environments where you need to scan private network ranges.

## Example: Localhost Scope (Safe Testing)

Use this for testing against `127.0.0.1` in a controlled environment:

```toml
# examples/scope-localhost.toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "127.0.0.1"
description = "Localhost"

[[allowed_targets]]
pattern = "localhost"
description = "Localhost"

[[allowed_targets]]
pattern = "*.local"
description = "Local development"
```

**Note:** Direct IP `127.0.0.1` is blocked by the private IP check. Use `localhost` as the hostname instead, or run against a non-loopback address in a lab network.

```bash
eggsec scan localhost --profile quick --scope examples/scope-localhost.toml
```

## Example: Internal Lab Scope

For a dedicated test lab with known CIDR ranges:

```toml
require_explicit_scope = true
max_requests_per_second = 500

[[allowed_targets]]
cidr = "10.10.0.0/16"
description = "Lab network range"

[[allowed_targets]]
pattern = "*.lab.internal"
description = "Lab hostnames"

[[excluded_targets]]
cidr = "10.10.1.1/32"
description = "Lab router management interface"

[[excluded_targets]]
pattern = "gateway.lab.internal"
description = "Network gateway - do not test"

excluded_ports = [22, 3389, 8443]
```

## Private IP Blocking (Known Limitation)

When no CIDR rules are configured, private IPs are blocked before scope rule evaluation. This prevents accidental scanning of internal networks. However, when CIDR rules are present, the private IP check is skipped, allowing the CIDR rules to match private IPs:

| Target | Scope Rule | Result |
|--------|-----------|--------|
| `10.0.0.5` (direct IP) | No CIDR rules | **Blocked** - private IP |
| `10.0.0.5` (direct IP) | `cidr: 10.0.0.0/8` | Allowed - CIDR rule matches |
| `10.0.0.5` (hostname resolving to 10.0.0.5) | No CIDR rules | **Blocked** - DNS resolves to private IP |
| `10.0.0.5` (hostname resolving to 10.0.0.5) | `cidr: 10.0.0.0/8` | Allowed - CIDR rule matches |
| `webserver.lab.local` (resolves to 10.0.0.5) | No CIDR rules | **Blocked** - DNS resolves to private IP |
| `webserver.lab.local` (resolves to 10.0.0.5) | `pattern: *.lab.local` (no CIDR) | **Blocked** - private IP with no CIDR rules |
| `webserver.lab.local` (resolves to 203.0.113.50) | `pattern: *.lab.local` | Allowed |

To test internal systems, use CIDR rules in your scope file (e.g., `cidr = "10.0.0.0/8"`), or use a VPN/tunnel that presents a public-facing address.

## See Also

- [SAFETY.md](SAFETY.md) - Operation risk tiers and authorization requirements
- [agent-workflows.md](agent-workflows.md) - Agent-oriented scope usage
- [lab-safety.md](lab-safety.md) - Safe use of high-risk features
