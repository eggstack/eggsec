# Scope Model

A scope file lists the hosts, address ranges, and ports approved for an
assessment. Scope enforcement helps prevent accidental testing outside that
boundary, but it does not replace explicit authorization from the system owner.

Scope files use TOML or YAML (`.yml` / `.yaml`). For a complete example, see
[`examples/configs/scope.toml`](../examples/configs/scope.toml).

## Example

```toml
require_explicit_scope = true
max_requests_per_second = 100
allowed_ports = [80, 443, 8080, 8443]
excluded_ports = [22, 3389]

[[allowed_targets]]
pattern = "*.staging.example.com"
description = "Approved staging hosts"

[[excluded_targets]]
pattern = "admin.staging.example.com"
description = "Excluded administrative host"
```

`*.staging.example.com` matches subdomains and the base hostname
`staging.example.com`. Exclusions take precedence over allowed rules.

## Fields

| Field | Type | Purpose |
|-------|------|---------|
| `require_explicit_scope` | Boolean | When true, at least one allowed rule must match. An empty allowlist denies all targets. |
| `max_requests_per_second` | Integer | Optional rate limit from 1 to 10,000. |
| `allowed_targets` | List | Hostname, wildcard, or CIDR rules that permit targets. |
| `excluded_targets` | List | Hostname or CIDR rules that always deny matching targets. |
| `allowed_ports` | List | Optional list of permitted ports. |
| `excluded_ports` | List | Ports that are always denied, even if also allowed. |

A target rule can use `pattern`, `cidr`, and an optional `description`:

```toml
[[allowed_targets]]
pattern = "staging.example.com"

[[allowed_targets]]
cidr = "10.20.0.0/16"
```

Patterns support exact hostnames, `*.suffix` wildcards, and `*` for any
hostname. CIDR ranges can be supplied with `cidr` or as a CIDR-shaped pattern.
Use a broad `*` rule only when that is the intended boundary.

## DNS and address rules

Hostname patterns match the requested hostname. A matching hostname pattern is
also considered for its resolved addresses, so a wildcard rule does not limit
which IP addresses the hostname can resolve to. If a hostname may resolve
outside the approved network, add explicit CIDR exclusions for those addresses
or use address-based rules and exclusions. Run `eggsec scope-explain --help` for the syntax to inspect a target's decision.

CIDR rules match addresses in the specified range. Excluded CIDRs take
precedence over allowed rules. Non-public addresses such as RFC1918, link-local,
CGNAT, and IPv6 ULA require authorization through scope rules. Loopback is
handled separately by scope policy and is not rejected by the non-public
address check.

Some scanner paths also block loopback resolution unless the local fixture
opt-in is set. For an authorized localhost lab, use an explicit localhost scope
and run:

```bash
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 eggsec scan 127.0.0.1 \
  --profile quick --scope examples/scope-localhost.toml \
  --allow-private-resolution
```

The environment variable is a fixture guard. It does not grant permission to
test a system.

## Ports and scope loading

`excluded_ports` always wins. If `allowed_ports` is set, other ports are denied.

The global `--scope` flag accepts a TOML or YAML file. If an explicitly
provided scope path cannot be loaded, Eggsec fails closed. Strict networked
surfaces require an explicit scope manifest. Use `eggsec scope-explain --help`
for the command syntax to inspect a target's scope decision.

## See also

- [Safety and enforcement](SAFETY.md)
- [Enforcement modes](ENFORCEMENT_MODES.md)
- [Build features and system requirements](BUILD.md)
