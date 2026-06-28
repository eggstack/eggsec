# Dual-Mode Enforcement Contract

Eggsec intentionally supports two usage families with distinct enforcement postures:

- **Manual operator posture** (CLI/TUI): Human-directed security assessment. Operators may proceed through warnings and explicit confirmations where appropriate. This mode is designed to remain productive and should not inherit agent-grade strictness by default.
- **Automated agent posture** (MCP, security agent, CI, REST): Programmatic noninteractive execution. Strict, explicitly scoped, non-overridable. Manual overrides are never honored.

The contract below is the source of truth for how enforcement behaves per execution surface. All later implementation phases must follow this contract to prevent drift in either direction: over-hardening manual use, or under-hardening agent use.

## Terminology

| Term | Definition |
|------|------------|
| **Execution surface** | Where the request originates: CLI, TUI, MCP server, security agent, CI pipeline, REST API. |
| **Execution profile** | Enforcement behavior, represented by `ExecutionProfile` (`ManualPermissive`, `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`). |
| **Manual permissive** | Human-directed default mode (`ManualPermissive`). Warnings and confirmation prompts are available; operator may override low-risk classes. |
| **Manual guarded** | Strict human mode (`ManualGuarded`). Equivalent to CLI `--strict-scope` and future TUI guarded toggle. No discretion path. |
| **Agent strict** | Noninteractive/model-controlled strict posture (`AgentStrict`). Cannot self-approve scope expansion or override any enforcement. Handler defensively rebuilds `AgentStrict`; runtime validates profile at construction. |
| **Scope provenance** | Whether scope came from an explicit manifest (`ConfigFile`, `CliScopeFile`, `GeneratedPreset`) versus `DefaultEmpty` (no manifest provided). |
| **Manual override** | Explicit operator acceptance of specific confirmation classes. Only valid in `ManualPermissive` contexts. Honored and audited only there. |
| **Confirmation class** | Machine-readable class (`ConfirmationClass`) requiring explicit operator action before dispatch. |

## Surface Behavior Matrix

| Execution Surface | Intended Posture | `ExecutionProfile` | Explicit Scope Manifest Required | `Warn` May Dispatch | `RequireConfirmation` May Dispatch After Override | Manual Override Flags Honored | Policy Re-evaluated Before Dispatch |
|---|---|---|---|---|---|---|---|
| CLI default | Manual permissive | `ManualPermissive` | No (warnings for safe scope misses) | Yes | Yes (with matching `ManualOverride`) | Yes | No (single evaluation) |
| CLI `--strict-scope` | Manual guarded | `ManualGuarded` | Yes (networked operations) | No | No (treated as deny) | No | No (single evaluation) |
| TUI default | Manual permissive | `ManualPermissive` | No (warnings for safe scope misses) | Yes | Yes (with matching `ManualOverride`) | Yes | No (single evaluation) |
| TUI guarded | Manual guarded | `ManualGuarded` | Yes (networked operations) | No | No (treated as deny) | No | No (single evaluation) |
| MCP server | Agent strict | `McpStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| Security agent | Agent strict | `AgentStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |
| CI | Agent strict | `CiStrict` | Yes (target/networked operations) | No (treated as deny) | No (treated as deny) | No | No (single evaluation) |
| REST API | Agent strict | `McpStrict` or `CiStrict` | Yes (networked operations) | No (treated as deny) | No (treated as deny) | No | Yes |

**Key invariant**: `ManualPermissive` behavior must not bleed into MCP, security agent, CI, or strict REST. Agent strict behavior must not become the default for normal CLI/TUI manual use.

## Outcome Semantics

`EnforcementOutcome` wraps a `PolicyDecision` with profile-aware dispatch semantics:

| Outcome | Manual Permissive | Manual Guarded | Automated (CI/MCP/Agent) |
|---------|-------------------|----------------|--------------------------|
| `Allow` | Dispatch permitted | Dispatch permitted | Dispatch permitted |
| `Warn` | Dispatch permitted; warnings must be visible and audited | Treated as deny | Treated as deny |
| `RequireConfirmation` | Dispatch permitted **only** after matching `ManualOverride` classes are present | Treated as deny | Treated as deny |
| `Deny` | Dispatch never permitted | Dispatch never permitted | Dispatch never permitted |

**Invariant**: Automated surfaces must treat `Warn` conservatively (as denial) and must treat `RequireConfirmation` as denial. Only `ManualPermissive` may dispatch on `Warn` or `RequireConfirmation` (with matching override).

## Manual Discretion Classes

`ConfirmationClass` variants represent categories of conditions that trigger `RequireConfirmation` under `ManualPermissive`:

| Class | `as_str()` | Override Mechanism | Notes |
|-------|-----------|-------------------|-------|
| `OutOfScope` | `out-of-scope` | `--allow-out-of-scope` or `--yes` | Low-risk scope confirmation |
| `TargetExpansion` | `target-expansion` | `--allow-out-of-scope` or `--yes` | Low-risk scope confirmation |
| `HighRisk` | `high-risk` | `--allow-high-risk` or `--allow-db-pentest` | Requires dedicated flag and reason |
| `NonBaselineCapability` | `nonbaseline-capability` | `--allow-nonbaseline-capability` | Requires dedicated flag |
| `PrivateResolution` | `private-resolution` | `--allow-private-resolution` | Requires dedicated flag |
| `CrossHostRedirect` | `cross-host-redirect` | `--allow-cross-host-redirect` | Requires dedicated flag |
| `TrafficInterception` | `traffic-interception` | `--allow-web-proxy` | Requires dedicated web-proxy flag |
| `ExplicitExclusion` | `explicit-exclusion` | `--allow-explicit-exclusion` | Requires dedicated flag and audit reason |

### `--yes` Scope

`--yes` (`assume_yes`) is intentionally narrow. It suppresses low-risk manual prompts for:
- `OutOfScope`
- `TargetExpansion`

`--yes` does **not** authorize:
- `HighRisk`
- `NonBaselineCapability`
- `PrivateResolution`
- `CrossHostRedirect`
- `TrafficInterception`
- `ExplicitExclusion`

Those classes require their dedicated `--allow-*` flags. This prevents accidental authorization of high-risk or sensitive operations through prompt suppression.

## Hard-Deny Classes

The following conditions produce hard denial and must **never** be converted to manual confirmation:

| Condition | Rationale |
|-----------|-----------|
| Missing compile-time feature | Build configuration error; cannot proceed |
| Invalid target | Unresolvable or malformed target |
| Scope parse/check error | Scope configuration is broken |
| Capability explicitly denied by policy | Policy explicitly blocks this capability |
| Risk not allowed by execution policy | Operation exceeds policy risk limits |
| Missing explicit scope manifest in automated mode | Strict profiles require explicit scope for networked operations |
| Agent/model-supplied override attempt | Automated surfaces cannot self-approve scope expansion |

## Policy Invariants

These invariants hold across all execution paths:

1. **Manual permissive isolation**: Manual permissive behavior must not bleed into MCP, security agent, CI, or strict REST.
2. **Agent strict isolation**: Agent strict behavior must not become the default for normal CLI/TUI manual use.
3. **Override scope**: Manual override flags are only honored in `ManualPermissive` contexts.
4. **Scope provenance**: Scope provenance for automated networked execution must come from `LoadedScope`, not raw `Scope`.
5. **Shared evaluation**: Every dispatch path must eventually flow through `EnforcementContext::evaluate()`.
6. **Re-evaluation**: Agent/MCP dispatch must re-evaluate enforcement immediately before dispatch.
7. **Constructor intent**: Programmatic constructors for agent-facing servers should require explicit enforcement context or be clearly test-only.

## Examples

### CLI manual scan with missing scope

**Scenario**: Operator runs `eggsec scan example.com` without a scope file.

**Expected**: `EnforcementOutcome::Warn` (safe scope-selection miss for passive/safe-active StandardAssessment). Warning is visible. Scan proceeds. No hard denial.

### CLI manual positive allowlist miss

**Scenario**: Scope has `[[allowed_targets]] pattern = "*.lab.internal"` but operator scans `example.com`.

**Expected**: `EnforcementOutcome::RequireConfirmation` with class `out-of-scope`. Operator must pass `--allow-out-of-scope` to proceed. `--yes` also suppresses this prompt.

### CLI strict positive allowlist miss

**Scenario**: Same as above but with `--strict-scope` (`ManualGuarded`).

**Expected**: `EnforcementOutcome::Deny`. No discretion path. Hard denial.

### MCP missing explicit manifest

**Scenario**: MCP server receives a networked tool call with `DefaultEmpty` scope.

**Expected**: `EnforcementOutcome::Deny`. `LoadedScope::is_explicit_manifest()` returns false for `DefaultEmpty`. Strict profiles require explicit manifest for networked operations.

### Security agent with high-risk nonbaseline capability not allowlisted

**Scenario**: Agent requests an `IntrusiveFuzz` capability but policy has not added it to `allowed_capabilities`.

**Expected**: `EnforcementOutcome::Deny`. Non-baseline capabilities (`IntrusiveFuzz`, `LoadTest`, etc.) require explicit listing in `allowed_capabilities` for strict profiles. `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect` are baseline and allowed by default.

### TUI manual high-risk action

**Scenario**: Operator triggers a high-risk action in TUI (e.g., WAF stress test).

**Expected**: TUI preflight shows `RequireConfirmation` with class `high-risk`. Operator must pass `--allow-high-risk` flag with a reason. `--yes` does not suppress this prompt.
