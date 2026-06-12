# Safety and Scope Enforcement

Eggsec is a security testing toolkit designed for **authorized testing only**.

## Scope Enforcement

All target-bearing operations go through scope validation:
- Direct IP addresses (e.g., `127.0.0.1`) are blocked by default
- Scope rules define allowed targets
- Operations outside scope are rejected

## Operation Risk Tiers

Eggsec classifies operations by risk level:

| Risk Level | Description | Default |
|------------|-------------|---------|
| Passive | Read-only operations | Allowed |
| SafeActive | Port scanning, fingerprinting | Allowed |
| Intrusive | Fuzzing, injection testing | Blocked |
| LoadTest | Load testing | Blocked |
| StressTest | Stress testing | Blocked |
| RawPacket | Raw packet operations | Blocked |
| CredentialTesting | Auth testing (auth-test CLI only; local `Auth*` types; see architecture/auth.md) | Blocked |
| ExploitAdjacent | Exploit-adjacent testing (e.g. chained primitives) | Blocked |
| (wireless passive) | Passive WiFi recon (iwlist scan, analysis only; no tx/injection/deauth/handshake). Detects security types (incl. WPS/hidden/transition), weak configs, and passive rogue/Evil-Twin heuristic. | Allowed under SafeActive (feature-gated `wireless`; requires root/CAP_NET_ADMIN + wireless-tools/iwlist; authorized lab/defense use only). Use --dry-run for unprivileged planning/CI. --known-good suppresses heuristic for baselines. See docs/WIRELESS.md. |
| (wireless active, Phase 1) | Active WiFi attacks (deauth, disassoc). Phase 1 implemented: pure-Rust 802.11 frame crafting, Linux raw socket injection, targeted/broadcast deauth, dry-run, packet budgets, policy gate (`Intrusive` risk + `wireless-advanced` feature). See `docs/WIRELESS.md`, `plans/wireless-active-attacks-loadout-design-plan.md`. | Blocked by default; requires `wireless-advanced` feature + `--allow-active-wireless` + lab context |
| (mobile dynamic) | Dynamic/runtime mobile app testing (controlled ADB/logcat/proxy/perms under `mobile-dynamic`). Phase 1 (Android ADB core + logcat) and Phase 2a (proxy Level-1 + runtime permissions + traffic summary + correlation) complete 2026-06-12; design + gating in `plans/dynamic-mobile-testing-loadout-design-plan.md` and handoff plans. Phase 3/4a (Frida + CorrelationEngine) delivered 2026-06-12 under single mobile-dynamic. | Blocked by default; requires `mobile-dynamic` feature + lab context + overrides (like wireless-advanced) |
| (mobile static) | Static analysis of user-supplied .apk/.ipa in lab (manifest, permissions, transport config, secrets, debug/backup flags, exported components). No execution, no device interaction. | Allowed under SafeActive (feature-gated `mobile`; dynamic phases per `plans/dynamic-mobile-testing-loadout-design-plan.md`) |
| RemoteExecution | Remote command execution | Blocked |
| AgentAutonomous | Agent-driven operations | Blocked |

High-risk operations (e.g. intrusive fuzzing, stress testing, raw packets, credential testing) must be explicitly enabled in your config file. Mobile static analysis is gated behind the `mobile` feature but classified under SafeActive (no execution, lab binaries only); dynamic (Phase 1 + Phase 2a, complete 2026-06-12; Phase 3/4a delivered 2026-06-12) is gated behind `mobile-dynamic` and the additional `--allow-dynamic-mobile` runtime confirmation; design in `plans/dynamic-mobile-testing-loadout-design-plan.md`.

## Authorization Requirements

Before using Eggsec:
1. Ensure you have explicit authorization to test the target
2. Understand the scope of your testing engagement
3. Review and configure operation policies appropriately
4. Never test production systems without authorization

## Configuration

Operation policies are configured in your config file:

```toml
[execution_policy]
require_explicit_scope = true
allow_intrusive_fuzzing = false
allow_stress_testing = false
```

The `mobile` feature (static-only APK/IPA analysis for lab binaries) must be enabled at build time (`--features mobile` or `--features full`). Mobile static is intended for authorized lab/defense use on user-supplied .apk/.ipa files only; no execution or device interaction occurs. The `mobile-dynamic` feature (Android ADB + logcat + Phase 2a proxy + traffic-capture + runtime-permission operations, complete 2026-06-12; Phase 3/4a delivered under single mobile-dynamic) must be enabled at build time for `eggsec mobile dynamic ...`; design in `plans/dynamic-mobile-testing-loadout-design-plan.md`. See `architecture/feature_matrix.md` for feature flags and `docs/CAPABILITIES.md` (Mobile App Security section) for coverage.

See `architecture/feature_matrix.md` for feature flags.

## Operating Modes

Eggsec operates in three modes:

- **standard-assessment**: Ordinary scoped scanning, fuzzing, API testing, WAF detection
- **defense-lab**: Local/private WAF regression, Synvoid validation, protocol edge testing
- **hazardous-lab**: Raw packets, flood stress, proxy rotation, distributed stress

Each CLI command's help text indicates its mode. Use `eggsec policy-explain` to inspect decisions before running traffic-generating operations.

## Execution Profiles

Eggsec distinguishes caller trust contexts through execution profiles. All paths route through the shared `EnforcementContext::evaluate(descriptor)` (in `config/policy_decision.rs`), which centralizes scope-provenance checks, `DenialClass` classification for downgrade decisions, positive capability allow checks for strict profiles, and risk/feature/policy enforcement.

| Profile | Behavior | Use Case |
|---------|----------|----------|
| **ManualPermissive** | Warn for safe scope ambiguity (no positive rules); RequireConfirmation for operator-discretion cases (explicit positive-scope out-of-scope, exclusions, high-risk, non-baseline caps, private-resolution, cross-host-redirect, target-expansion). `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. required for their classes (manual-only, audited on decision). Strict profiles/MCP/agent treat RequireConfirmation as Deny; never honor overrides. | Default CLI/TUI |
| **ManualGuarded** | Hard-deny (no overrides) for missing scope, out-of-scope targets, ambiguous scope, high-risk etc. for target-bearing ops | CLI with `--strict-scope` |
| **CiStrict** | Hard-deny (no overrides); non-interactive, deterministic, strict; explicit manifest required; positive capability allow enforced for non-baseline | CI/CD pipelines |
| **McpStrict** | Hard-deny (no overrides); always strict, scope manifest (`LoadedScope::is_explicit_manifest()`) required for networked ops; warnings treated as denials; capabilities populated via `required_capabilities_for_tool_call` + `operation_descriptor_for_mcp_call`; MCP profile layer (visibility/target/arg restrictions) overlays shared enforcement decision | MCP server |
| **AgentStrict** | Hard-deny (no overrides); always strict, cannot self-approve scope; explicit manifest required; per-scan `enforcement.evaluate` immediately before dispatch in `execute_scan_with_depth` (in addition to startup gating in `handle_agent`) | Autonomous agent |

`LoadedScope` provenance (`ScopeSource`: DefaultEmpty vs. ConfigFile/CliScopeFile/GeneratedPreset) is the source of truth for strict automated manifest checks inside `EnforcementContext::evaluate`. `requires_explicit_manifest_for` + `is_explicit_manifest()` produce the canonical denial reason for automated networked operations.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution.

**Baseline capabilities for strict automated profiles** (`McpStrict`, `AgentStrict`, `CiStrict`): `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect` (positive capability allow not required). All other capabilities require explicit `allowed_capabilities` in `ExecutionPolicy` (plus matching risk/feature gates). Strict profiles never downgrade or confirm; they treat RequireConfirmation as Deny with no overrides. **ManualPermissive** (default) uses Warn for safe scope ambiguity when no positive rules; RequireConfirmation for operator-discretion cases (explicit positive-scope out-of-scope, exclusions, high-risk, non-baseline caps, private-resolution, cross-host-redirect, target-expansion). `--yes` narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-*` flags for others (manual-only, audited). Missing features and impossible cases are always hard Deny. Strict profiles/MCP/agent never honor overrides.

### Usage Examples

```bash
# Manual permissive (default) - safe scope ambiguity warns
eggsec scan example.com --profile quick

# Manual permissive with explicit override for RequireConfirmation cases
# (--yes is narrow: only out-of-scope/target-expansion; use dedicated for private/redirect)
eggsec scan example.com --scope scope.toml --allow-out-of-scope --manual-override-reason "authorized boundary test"
eggsec scan example.com --scope scope.toml --allow-high-risk --yes
eggsec scan 10.0.0.5 --allow-private-resolution --manual-override-reason "lab private target"
eggsec waf-stress https://lab.example --allow-high-risk --manual-override-reason "authorized Synvoid WAF regression"

# Manual strict (hard-deny, no overrides)
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# Strict MCP (enforcement wired at construction via with_enforcement / create_mcp_router / run_stdio)
eggsec codegg-mcp --scope scope.toml --stdio

# Strict autonomous agent (enforcement passed through AgentConfig; re-evaluated per-scan)
eggsec agent run --portfolio portfolio.json --scope scope.toml
```

MCP, CI, agent, and ManualGuarded callers cannot use warn-only or downgrade/override flags. Enforcement is always in Rust code paths (`EnforcementContext::evaluate` central boundary), not prompt-level instructions. Strict profiles and `--strict-scope` treat RequireConfirmation as hard Deny with no overrides. MCP enforcement uses `operation_descriptor_for_mcp_call` + `policy_decision_for_mcp_call_with_enforcement` (via `EnforcementContext`) to ensure required capabilities, provenance, and DenialClass/positive-capability logic are consistent. Preferred MCP production constructor: `McpServer::with_enforcement`.

## Policy Decision Records

Every target-bearing operation produces a structured policy decision with:
- Unique decision ID
- Operation mode and risk level
- Target normalization and scope matching
- Required features and policy flags
- Denial reasons (when blocked)

Use `eggsec policy-explain --json` to view a policy decision without executing.

## High-Risk Feature Safety

**All high-risk features should only be used against systems you own or have explicit written authorization to test.**

### Stress Testing

**Risk: Denial of Service**

Stress testing generates high volumes of traffic against a target. It can overwhelm target services, saturate network links, trigger IDS/IPS alerts, and impact co-located services on shared infrastructure.

**Requirements:** Written authorization, isolated lab environment, `--rate-limit` and `--concurrency` caps, shutdown plan.

```bash
eggsec stress "$TARGET" \
  --rate-limit 100 \
  --concurrency 10 \
  --duration 60 \
  --scope scopes/lab.toml
```

### Packet / Raw Socket Operations

**Risk: Network disruption, requires elevated privileges**

Raw packet operations (crafted packets, IP spoofing, packet capture) require root/sudo and can disrupt network connections if misconfigured.

**Requirements:** Root/sudo, isolated network, `stress-testing` feature flag.

```bash
sudo eggsec packet capture \
  --interface eth1 \
  --filter "tcp port 80" \
  --output captures/
```

### WAF Evasion-Resistance Testing

**Risk: May trigger security responses**

WAF bypass testing sends payloads designed to evade web application firewalls. It can trigger WAF alerts, IP blocks, and security team responses.

**Requirements:** Authorization from both application owner and WAF operator; coordination with security operations.

```bash
eggsec waf detect "$TARGET" \
  --scope scopes/authorized.toml \
  --rate-limit 50
```

### Proxy / Tor Usage

**Risk: Legal considerations, route leaks, attribution issues**

Using Eggsec through proxies or Tor may violate provider ToS, leak real IP, or have legal implications. Verify proxy configuration and use only with authorized targets.

### Authentication Testing

**Risk: Account lockout, credential exposure, legal exposure**

Auth testing attempts credential stuffing, brute force, or session manipulation. It can lock out legitimate accounts and generate audit logs requiring explanation.

**Requirements:** Written authorization, dedicated test accounts, coordination with auth team, rate limiting.

```bash
eggsec auth-test "$TARGET" \
  --wordlist test-credentials.txt \
  --max-attempts 50 \
  --concurrency 2 \
  --scope scopes/authorized.toml
```

See `docs/AUTH_LAB.md` for full defense-lab usage guide.

### Rate and Concurrency Limits

**Always** use rate and concurrency limits with high-risk features:

| Flag | Purpose | Recommended Range |
|------|---------|-------------------|
| `--rate-limit` | Max requests per second | 10-500 depending on target |
| `--concurrency` | Max parallel connections | 5-50 depending on target |
| `--timeout` | Per-request timeout | 10-30 seconds |

Scope files can enforce rate limits globally:

```toml
max_requests_per_second = 100
```

### Private Lab Recommendation

Run high-risk features in an isolated environment:

| Method | Isolation Level | Setup Effort |
|--------|----------------|--------------|
| Docker containers | Good | Low |
| VMs (Vagrant, libvirt) | Good | Medium |
| Dedicated lab network | Best | High |
| Cloud sandbox (VPC) | Good | Medium |

```bash
docker network create --driver bridge eggsec-lab
docker run -d --name target --network eggsec-lab vulnerable-app:latest

cat > /tmp/lab-scope.toml << 'EOF'
require_explicit_scope = true
[[allowed_targets]]
cidr = "172.20.0.0/16"
description = "Docker lab network"
EOF

eggsec scan target --scope /tmp/lab-scope.toml
```

### Monitoring and Rollback

When running high-risk features:
1. Monitor the target for service degradation, error spikes, or resource exhaustion
2. Have a kill switch (Ctrl+C or `eggsec stop`)
3. Log everything (Eggsec logs all operations)
4. Have a rollback plan
5. Document what was tested, when, and results
