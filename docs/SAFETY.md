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
| CredentialTesting | Auth testing | Blocked |
| ExploitAdjacent | Exploit-adjacent testing (e.g. chained primitives) | Blocked |
| RemoteExecution | Remote command execution | Blocked |
| AgentAutonomous | Agent-driven operations | Blocked |

High-risk operations must be explicitly enabled in your config file.

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
| **ManualPermissive** | Warn for safe scope ambiguity (no positive rules); RequireConfirmation for operator-discretion cases (explicit positive-scope out-of-scope, exclusions, high-risk, non-baseline caps, etc). CLI overrides RequireConfirmation via --yes / --allow-out-of-scope / --allow-high-risk etc (manual-only, audited). | Default CLI/TUI |
| **ManualGuarded** | Hard-deny (no overrides) for missing scope, out-of-scope targets, ambiguous scope, high-risk etc. for target-bearing ops | CLI with `--strict-scope` |
| **CiStrict** | Hard-deny (no overrides); non-interactive, deterministic, strict; explicit manifest required; positive capability allow enforced for non-baseline | CI/CD pipelines |
| **McpStrict** | Hard-deny (no overrides); always strict, scope manifest (`LoadedScope::is_explicit_manifest()`) required for networked ops; warnings treated as denials; capabilities populated via `required_capabilities_for_tool_call` + `operation_descriptor_for_mcp_call`; MCP profile layer (visibility/target/arg restrictions) overlays shared enforcement decision | MCP server |
| **AgentStrict** | Hard-deny (no overrides); always strict, cannot self-approve scope; explicit manifest required; per-scan `enforcement.evaluate` immediately before dispatch in `execute_scan_with_depth` (in addition to startup gating in `handle_agent`) | Autonomous agent |

`LoadedScope` provenance (`ScopeSource`: DefaultEmpty vs. ConfigFile/CliScopeFile/GeneratedPreset) is the source of truth for strict automated manifest checks inside `EnforcementContext::evaluate`. `requires_explicit_manifest_for` + `is_explicit_manifest()` produce the canonical denial reason for automated networked operations.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution.

**Baseline capabilities for strict automated profiles** (`McpStrict`, `AgentStrict`, `CiStrict`): `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect` (positive capability allow not required). All other capabilities require explicit `allowed_capabilities` in `ExecutionPolicy` (plus matching risk/feature gates). Strict profiles never downgrade or confirm; they treat RequireConfirmation as Deny with no overrides. **ManualPermissive** (default) uses Warn for safe scope ambiguity when no positive rules; RequireConfirmation for operator-discretion cases (explicit positive-scope out-of-scope, exclusions, high-risk, non-baseline caps, etc). Missing features and impossible cases are always hard Deny. CLI may satisfy RequireConfirmation via manual-only overrides.

### Usage Examples

```bash
# Manual permissive (default) - safe scope ambiguity warns
eggsec scan example.com --profile quick

# Manual permissive with explicit override for RequireConfirmation cases
eggsec scan example.com --scope scope.toml --allow-out-of-scope --manual-override-reason "authorized boundary test"
eggsec scan example.com --scope scope.toml --allow-high-risk --yes
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
