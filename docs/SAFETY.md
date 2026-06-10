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
| **ManualPermissive** | Warnings (not denials) for safe scope ambiguity / ScopeMissing / TargetOutOfScope when no positive rules declared and no exclusions/feature/risk/capability/hazard denials; downgrades use `classify_denial_reasons` + `may_downgrade_to_warning`. Higher-risk, exclusions, or declared positive scope rules remain hard denials. | Default CLI/TUI |
| **ManualGuarded** | Denies missing scope, out-of-scope targets, ambiguous scope for target-bearing ops | CLI with `--strict-scope` |
| **CiStrict** | Non-interactive, deterministic, strict; explicit manifest required; positive capability allow enforced for non-baseline | CI/CD pipelines |
| **McpStrict** | Always strict, scope manifest (`LoadedScope::is_explicit_manifest()`) required for networked ops; warnings treated as denials; capabilities populated via `required_capabilities_for_tool_call` + `operation_descriptor_for_mcp_call`; MCP profile layer (visibility/target/arg restrictions) overlays shared enforcement decision | MCP server |
| **AgentStrict** | Always strict, cannot self-approve scope; explicit manifest required; per-scan `enforcement.evaluate` immediately before dispatch in `execute_scan_with_depth` (in addition to startup gating in `handle_agent`) | Autonomous agent |

`LoadedScope` provenance (`ScopeSource`: DefaultEmpty vs. ConfigFile/CliScopeFile/GeneratedPreset) is the source of truth for strict automated manifest checks inside `EnforcementContext::evaluate`. `requires_explicit_manifest_for` + `is_explicit_manifest()` produce the canonical denial reason for automated networked operations.

### Usage Examples

```bash
# Manual permissive (default)
eggsec scan example.com --profile quick

# Manual strict
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# Strict MCP (enforcement wired at construction via with_enforcement / create_mcp_router / run_stdio)
eggsec codegg-mcp --scope scope.toml --stdio

# Strict autonomous agent (enforcement passed through AgentConfig; re-evaluated per-scan)
eggsec agent run --portfolio portfolio.json --scope scope.toml
```

MCP and autonomous agent callers cannot use warn-only or downgrade flags. Enforcement is always in Rust code paths (`EnforcementContext::evaluate` central boundary), not prompt-level instructions. Legacy MCP helpers (`policy_decision_for_mcp_call`) and direct `evaluate_operation_policy` are deprecated for denial paths; prefer `operation_descriptor_for_mcp_call` + `policy_decision_for_mcp_call_with_enforcement` (via `EnforcementContext`) to ensure required capabilities, provenance, and DenialClass/positive-capability logic are consistent. Preferred MCP production constructor: `McpServer::with_enforcement`.

## Policy Decision Records

Every target-bearing operation produces a structured policy decision with:
- Unique decision ID
- Operation mode and risk level
- Target normalization and scope matching
- Required features and policy flags
- Denial reasons (when blocked)

Use `eggsec policy-explain --json` to view a policy decision without executing.
