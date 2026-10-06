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
| `require_explicit_scope` | bool | No | When `true`, targets must match an `allowed_targets` rule — and at least one rule must exist. When `false` with no rules, only non-public addresses are rejected. |
| `max_requests_per_second` | int | No | Rate limit (1..=10000). Null means no limit. `0` and values above 10000 are rejected at load. |
| `allowed_targets` | list | No | Rules defining permitted targets. Empty list + `require_explicit_scope = true` = deny all. |
| `excluded_targets` | list | No | Rules that override `allowed_targets`. Exclusion always wins. |
| `allowed_ports` | list | No | Restrict scanning to specific ports. Null means all non-excluded ports. |
| `excluded_ports` | list | No | Ports always blocked regardless of `allowed_ports`. |
| `scope_file` | string | No | Recorded provenance path; not a matching rule. |

`Scope` is the pure policy type in `eggsec-policy` (no filesystem, no DNS).
Filesystem and DNS stay engine-side in `policy_bridge::resolver`:
`load_scope_from_file()` picks the parser by extension (`.yaml`/`.yml` → YAML,
otherwise TOML), and `resolve_hostname_facts_with()` turns a target string into
a `TargetScope` through a `HostResolver` (`SystemResolver` by default).

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

Every target-bearing operation (scan, fuzz, stress test, agent run) is evaluated
by `Scope::evaluate_facts(&TargetScope)` — pure policy that takes resolved
address facts supplied by the engine's resolver bridge (`HostResolver`,
`SystemResolver` by default). Adapters never check scope themselves.

1. **Exclusion check** — if the target matches any `excluded_targets` rule, it is rejected. Exclusion always wins.
2. **Empty allowlist** — if `allowed_targets` is empty and `require_explicit_scope = true`, deny all. If it is empty and `require_explicit_scope = false`, only **non-public** addresses are rejected.
3. **Allowed check** — when `allowed_targets` is non-empty, the target must match at least one rule. Resolved addresses are evaluated individually when DNS facts are available, so a hostname resolving to both a public and a private address does not silently pass.
4. **Non-public fallback** — a target that matches no allowed rule is rejected if it is a non-public address.
5. **Port check** — ports are filtered by `is_port_allowed()`: `excluded_ports` always wins, then `allowed_ports` applies when set.

**Loopback is exempt.** `127.0.0.0/8` / `::1` classify as `Loopback`, not merely
non-public, and are never blocked by the non-public checks — they are inherently
local and represent no scope violation. This makes localhost testing work with
an explicit rule instead of requiring a CIDR workaround.

**CIDR rules are checked like any other rule.** A `cidr` field is *not* a switch
that disables the non-public check; it is an alternative matcher inside the same
rule. An `allowed_targets` entry with a `cidr` (either the `cidr` field or a
CIDR-shaped `pattern`) matches any of the target's resolved addresses within
that network, which is what allows internal ranges like `10.0.0.0/8` to be
authorized. Pattern matching supports exact hosts, `*.suffix` wildcards (which
also match the bare suffix), the glob-all `"*"`, and CIDR-shaped patterns.

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

**Note:** loopback (`127.0.0.0/8`, `::1`) is exempt from the non-public address
checks, so `127.0.0.1` and `localhost` are both usable here. Keep the explicit
rules anyway — without them, `require_explicit_scope = true` still denies all.

```bash
eggsec scan localhost --profile quick --scope examples/scope-localhost.toml
```

`--scope` is a **global** flag (`eggsec --scope <path> <command>`, or after the
subcommand — clap marks it `global = true`). An operator-supplied path that does
not exist is a hard error (fail closed); with no `--scope`, Eggsec falls back to
`find_scope_file()` and, failing that, to `LoadedScope::default_empty()`.
`LoadedScope::is_explicit_manifest()` distinguishes a real manifest from that
default-empty fallback — strict networked surfaces require the former.

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

## Non-Public Address Handling

`Scope` blocks non-public addresses (`RFC1918`, link-local, CGNAT, IPv6 ULA) in
two places: when the allowlist is empty and `require_explicit_scope = false`,
and as a fallback when a target matches no allowed rule. A `cidr` rule is a
matcher, not a bypass — it authorizes a private range explicitly rather than
switching the check off.

| Target | Scope Rule | Result |
|--------|-----------|--------|
| `127.0.0.1` (loopback) | any | Allowed — loopback is exempt |
| `10.0.0.5` (direct IP) | No rules, `require_explicit_scope = false` | **Blocked** — non-public |
| `10.0.0.5` (direct IP) | `cidr: 10.0.0.0/8` | Allowed — CIDR rule matches |
| `10.0.0.5` (direct IP) | `pattern: *.example.com` only | **Blocked** — matches no rule, and non-public |
| `10.0.0.5` (direct IP) | No rules, `require_explicit_scope = true` | **Blocked** — empty allowlist denies all |
| `webserver.lab.local` (resolves to 10.0.0.5) | `cidr: 10.0.0.0/8` | Allowed — one resolved address matches |
| `webserver.lab.local` (resolves to 203.0.113.50) | `pattern: *.lab.local` | Allowed |
| `webserver.lab.local` (resolves to both public and private) | `pattern: *.lab.local` | **Blocked** — per-address evaluation rejects the private address |

To authorize internal systems, use CIDR rules in your scope file (e.g.
`cidr = "10.0.0.0/8"`), or present a public-facing address through a VPN/tunnel.

## See Also

- [ENFORCEMENT_MODES.md](ENFORCEMENT_MODES.md) — how scope combines with profiles and overrides
- [SAFETY.md](SAFETY.md) - Operation risk tiers and authorization requirements
- [AGENT.md](AGENT.md) - Agent configuration and operation
- `architecture/config.md` — enforcement/scope deep dive
- `examples/configs/scope.toml` — fully annotated example
