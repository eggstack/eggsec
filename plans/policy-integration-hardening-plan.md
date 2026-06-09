# Policy Integration Hardening Plan

## Purpose

This plan is the next pass after the lab-defense workflow refinement work. The repo now has the right primitives: `OperationRisk`, `ExecutionPolicy`, `OperationMode`, `IntendedUse`, `PolicyDecision`, `ProbeRisk`, `policy-explain`, `scope-explain`, and Synvoid defense-lab documentation. The remaining work is to make those primitives load-bearing across execution paths rather than partially-adopted vocabulary.

The main goals are:

1. Wire `PolicyDecision` through every target-bearing execution path.
2. Align `plan`/`scan`/profiles with the new operation taxonomy.
3. Resolve semantic drift between `ProbeRisk` and `OperationRisk`.
4. Standardize scope examples and parser expectations.
5. Add MCP, CLI, report, and JSON contract tests so future changes cannot bypass policy by accident.

This pass should preserve low-level/stress capabilities for Synvoid WAF hardening and authorized distributed-system validation. The goal is stronger boundaries and better auditability, not feature removal.

## Current State Summary

The repo already includes:

- `crates/eggsec/src/config/policy.rs` with `OperationRisk`, `ExecutionPolicy`, `OperationMode`, and `IntendedUse`.
- `crates/eggsec/src/config/policy_decision.rs` with structured `PolicyDecision` records.
- `crates/eggsec/src/probe.rs` with `ProbeIntent` and `ProbeRisk`.
- CLI-level `PolicyExplain` and `ScopeExplain` commands.
- Updated safety docs explaining `standard-assessment`, `defense-lab`, and `hazardous-lab`.
- `docs/lab/SYNVOID_DEFENSE_LAB.md` documenting Synvoid-oriented lab workflows.

Known gaps:

- `PlanArgs` still appears generic, with `profile: String` and default `default`; it is not clearly aligned to the richer `ScanProfile` / policy model.
- `ProbeRisk` and `OperationRisk` overlap but do not line up cleanly.
- `ProbeRisk::ExploitAdjacent` currently maps to `OperationRisk::AgentAutonomous`, which is semantically questionable.
- Some docs use older labels such as `ActiveScan` and `IntrusiveFuzz`, while code uses `SafeActive` and `Intrusive`.
- Scope examples may mix `pattern = "10.0.0.0/8"` and `cidr = "10.0.0.0/8"`; this should match the parser exactly.
- It is not yet clear that every target-bearing command goes through one policy decision path.

## Non-Goals

Do not remove `stress`, `packet`, `waf-stress`, `proxy`, `cluster`, NSE, Synvoid profiles, or other lab-defense tools.

Do not weaken or bypass scope enforcement to make tests easier.

Do not expose hazardous-lab tools to the coding-agent MCP profile.

Do not add new exploit behavior. This pass is about policy correctness, profile consistency, and testability.

Do not create duplicate policy systems. Consolidate with the current `config::policy` and `probe` modules.

## Phase 1: Inventory All Target-Bearing Execution Paths

Create an implementation checklist in code comments or a short internal doc while working. Identify every command/tool path that can touch a target, network endpoint, file-derived target list, packet interface, remote worker, or agent portfolio target.

Audit at minimum:

- `scan`
- `scan-ports`
- `scan-endpoints`
- `fingerprint`
- `fuzz`
- `waf`
- `waf-stress`
- `graphql`
- `oauth`
- `auth-test`
- `recon`
- `load`
- `stress`
- `packet`
- `proxy`
- `icmp`
- `traceroute`
- `cluster`
- `remote`
- `exec`
- `nse`
- `browser`
- `wireless`
- `agent run`
- `serve` / REST tool execution
- `mcp-serve`
- `codegg-mcp`
- `grpc`

Acceptance criteria:

- Each target-bearing path is categorized as either target-bearing or explicitly non-target-bearing.
- Each target-bearing path has a declared `OperationMode`, `OperationRisk`, and at least one `IntendedUse`.
- Each path either uses the shared policy evaluator or has a TODO with a failing/ignored test documenting missing coverage.

## Phase 2: Create One Shared Policy Evaluation Entry Point

Add a single high-level policy evaluation function if one does not already exist.

Suggested API shape:

```rust
pub struct OperationDescriptor {
    pub operation: String,
    pub mode: OperationMode,
    pub risk: OperationRisk,
    pub intended_uses: Vec<IntendedUse>,
    pub target: Option<String>,
    pub required_features: Vec<String>,
    pub required_policy_flags: Vec<String>,
    pub requires_private_or_local_target: bool,
    pub requires_explicit_scope: bool,
}

pub fn evaluate_operation_policy(
    descriptor: &OperationDescriptor,
    config: &EggsecConfig,
    scope: &Scope,
) -> PolicyDecision
```

Adjust names and module placement to match the existing codebase. The important point is that command handlers, tool handlers, MCP handlers, and agent workflows should not each reinvent policy checks.

Policy evaluation should include:

- target normalization
- scope allow matches
- scope exclusion matches
- private/local detection where relevant
- execution policy flag checks
- feature flag checks where possible
- risk budget checks
- operation mode checks
- warnings for broad or ambiguous targets
- structured denial reasons

Acceptance criteria:

- The evaluator returns a complete `PolicyDecision` for both allowed and denied operations.
- Denied decisions are never silently converted into generic `anyhow` errors before reaching CLI/MCP/API/report layers.
- Unit tests cover allowed localhost, denied public target, excluded target, missing policy flag, missing feature, and hazardous-lab operation.

## Phase 3: Align `PlanArgs` and Plan Execution With Scan Profiles

Update `PlanArgs` so planning is profile-aware and policy-aware.

Current risk: `PlanArgs` uses a raw `String` profile defaulting to `default`. This can drift from the actual scan profile enum and the defense-lab profile semantics.

Recommended changes:

- Use the existing `ScanProfile` enum if feasible.
- If `PlanArgs` must remain string-based for compatibility, parse it immediately into a canonical profile type and return a structured error for unknown profiles.
- Ensure supported profile names include the current defense-lab profiles: `defense-lab`, `synvoid-local`, `waf-regression`, `protocol-edge`, and `nse-safe`.
- Make `plan` emit one `PolicyDecision` per planned stage.
- Add `--json` compatibility through the global JSON flag or explicit format handling.
- Ensure `plan` sends no traffic.

Suggested output model:

```rust
pub struct PlanOutput {
    pub target: Option<String>,
    pub profile: String,
    pub operation_mode: OperationMode,
    pub max_risk: OperationRisk,
    pub stages: Vec<PlannedStage>,
    pub policy_decisions: Vec<PolicyDecision>,
    pub skipped_stages: Vec<SkippedStage>,
}
```

Acceptance criteria:

- `eggsec plan --target http://127.0.0.1:8080 --profile waf-regression --scope examples/scope-localhost.toml --json` emits policy decisions without executing network traffic.
- Unknown profile names fail with a clear error listing valid profiles.
- Defense-lab profiles require local/private/scope-approved targets at planning time.
- Plan output is covered by golden JSON tests.

## Phase 4: Resolve `ProbeRisk` vs `OperationRisk` Semantics

Audit the relationship between `ProbeRisk` and `OperationRisk`.

Current concern: `ProbeRisk::ExploitAdjacent` maps to `OperationRisk::AgentAutonomous`, but these describe different dimensions. Exploit-adjacent is about behavior; agent-autonomous is about executor context.

Recommended options:

Option A, preferred if minimal churn:

- Add `OperationRisk::ExploitAdjacent`.
- Map `ProbeRisk::ExploitAdjacent` to `OperationRisk::ExploitAdjacent`.
- Keep `AgentAutonomous` for actions initiated by the autonomous agent, or represent it through an `ExecutionContext` enum.

Option B, if `OperationRisk` should remain stable:

- Rename `ProbeRisk::ExploitAdjacent` to something that maps accurately to an existing risk.
- Add comments/tests explaining the mapping.

Potential long-term model:

```rust
pub enum ExecutionContext {
    HumanCli,
    Tui,
    RestApi,
    GrpcApi,
    OpsMcp,
    CodingMcp,
    AgentAutonomous,
    Ci,
}
```

This lets `AgentAutonomous` stop acting like a risk tier. An autonomous agent can attempt a passive operation or a stress operation; these are different axes.

Acceptance criteria:

- No semantically questionable risk mapping remains.
- Tests cover all `ProbeRisk -> OperationRisk` mappings.
- Docs distinguish operation risk from executor context if that split is introduced.
- Existing serialized values are considered; if breaking changes occur, note them in docs or migration notes.

## Phase 5: Standardize Scope Rule Syntax and Examples

Audit the actual `Scope` parser and docs/examples.

Clarify whether CIDRs must use:

```toml
[[allowed_targets]]
cidr = "10.0.0.0/8"
```

or whether this is also valid:

```toml
[[allowed_targets]]
pattern = "10.0.0.0/8"
```

Then standardize every example accordingly.

Files to inspect and update:

- `README.md`
- `docs/SAFETY.md`
- `docs/lab/SYNVOID_DEFENSE_LAB.md`
- `docs/CAPABILITIES.md`
- `examples/*.toml`
- Tests/fixtures using scope files

Acceptance criteria:

- All CIDR examples match the real parser.
- `scope-explain` test fixtures include domain pattern, localhost, private CIDR, public IP, and excluded target.
- Invalid scope syntax produces a clear config validation error.

## Phase 6: Wire Policy Decisions Into Command Denials

Every denied target-bearing command should return a `PolicyDecision`, either in human-readable form or JSON.

Behavior:

- Human output: concise denial plus relevant reasons and remediation hints.
- JSON output: serialized `PolicyDecision` with stable field names.
- Logs: include `decision_id`.
- Reports: include policy summaries for executed scans.

Acceptance criteria:

- `--json` denial output is valid JSON and includes `allowed: false`.
- Human denial output includes operation, mode, risk, target, and denied reasons.
- Commands do not bypass policy checks when invoked through aliases.
- Denials from nested pipeline stages are preserved in plan/report output.

## Phase 7: Harden MCP Profile Boundaries

Audit MCP tool registration and dispatch.

Coding-agent MCP must not expose or invoke:

- `stress`
- `waf-stress` if it can generate intrusive/stress traffic beyond the coding profile
- `packet`
- raw packet send/capture tools
- `proxy` rotation
- `remote`
- `exec`
- broad recon
- cluster/distributed execution
- external target scans by default
- load testing

Coding-agent MCP may expose bounded tools such as:

- validate local/scope target
- re-check a known finding
- local/private port check with tiny limits
- local/private fingerprinting
- limited safe endpoint checks
- limited safe fuzz checks if scope and policy allow
- WAF detection in local/private scope with conservative budget

Ops-agent MCP may expose broader tools, but it must still call the shared policy evaluator.

Acceptance criteria:

- Tool registry has per-tool risk and MCP-profile metadata.
- Coding-agent profile cannot invoke hazardous tools by canonical name or alias.
- Ops-agent profile receives policy denials when config/scope does not allow the operation.
- MCP denial responses include serialized `PolicyDecision` or a stable wrapper containing it.
- Tests cover at least one denied coding-agent stress/raw/remote call and one allowed coding-agent local verification call.

## Phase 8: Agent and API Integration

Ensure autonomous agent and server paths use the same policy logic as CLI.

Agent requirements:

- Portfolio targets are checked against scope before scheduling.
- Scheduled operations include `OperationDescriptor` metadata.
- Agent logs include `decision_id` for allowed and denied operations.
- Agent respects budgets/cooldowns and cannot exceed CLI-equivalent policy limits.
- `AgentAutonomous` should be represented as executor context if Phase 4 introduces that split.

REST/gRPC requirements:

- Tool execution endpoints call shared policy evaluator.
- Denials serialize as structured policy responses.
- API docs/examples mention scope and policy behavior.

Acceptance criteria:

- Agent tests include denied external target and allowed local/private lab target.
- REST/MCP/gRPC tool execution paths do not duplicate policy checks.
- Policy denial response shape is stable across CLI JSON, MCP, and REST where feasible.

## Phase 9: Report Integration

Add policy summary sections to reports for relevant formats.

Minimum JSON fields:

```json
{
  "policy_summary": {
    "operation_mode": "defense-lab",
    "max_risk": "intrusive",
    "decisions": [...],
    "denied_count": 0,
    "warning_count": 1
  }
}
```

Human/Markdown/HTML reports should include:

- mode
- risk budget
- intended use
- target/scope summary
- skipped/denied stages
- budget termination reason if applicable

SARIF/JUnit should remain conservative. Do not convert policy denials or stress metrics into vulnerabilities unless they represent an actionable finding.

Acceptance criteria:

- JSON reports contain policy decision summaries.
- Markdown/HTML reports separate vulnerabilities, observations, policy denials, and stress/load metrics.
- Golden report fixtures are updated.

## Phase 10: Documentation Cleanup

Clean up docs after implementation.

Required updates:

- Replace stale risk labels such as `ActiveScan` / `IntrusiveFuzz` if they do not match code.
- Document serialized enum names for `OperationRisk`, `OperationMode`, and `IntendedUse`.
- Explain the difference between risk and executor context if introduced.
- Ensure `policy-explain` and `scope-explain` examples match actual CLI syntax.
- Ensure Synvoid defense-lab docs use valid scope TOML.
- Add a small MCP profile table documenting allowed/denied categories.

Acceptance criteria:

- Docs and code use the same terms.
- Every documented command example is either tested or manually verified.
- The README still presents Eggsec as a scope-enforced defense-validation engine, not a generic offensive automation toolkit.

## Phase 11: Test Matrix

Add tests that lock down the policy contract.

Minimum tests:

- `OperationRisk` policy defaults block intrusive/load/stress/raw/credential/remote/autonomous.
- `OperationMode` default max risk is correct.
- `IntendedUse` serialization is stable.
- `PolicyDecision` golden JSON for allowed and denied decisions.
- `ProbeRisk -> OperationRisk` mappings are explicit and correct.
- `plan` emits policy decisions for `waf-regression`.
- `policy-explain` performs no network traffic and emits JSON/human output.
- `scope-explain` handles localhost/private/public/excluded targets.
- CLI aliases do not bypass policy.
- Coding-agent MCP denies hazardous tools.
- Ops-agent MCP still respects policy.
- Agent denies out-of-scope portfolio targets.
- Reports include policy summary.

Testing constraints:

- No tests should send external network traffic.
- Stress/raw packet tests should be mock, dry-run, or ignored by default.
- Use local fixture servers where required.
- Prefer golden JSON snapshots for policy/report output.

## Suggested Implementation Order

1. Inventory target-bearing paths.
2. Add or consolidate `OperationDescriptor` and shared evaluator.
3. Fix `ProbeRisk` / `OperationRisk` mapping.
4. Align `PlanArgs` and plan execution with profiles/policy decisions.
5. Wire denials through CLI command handlers.
6. Harden MCP profile registration and dispatch.
7. Wire agent/API paths to shared evaluator.
8. Add report policy summaries.
9. Standardize scope syntax examples.
10. Update docs and add golden tests.

## Validation Commands

Run at minimum:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --workspace --all-features
cargo build -p eggsec-cli
cargo build -p eggsec-cli --features stress-testing
cargo build -p eggsec-cli --features packet-inspection
cargo build -p eggsec-cli --features "rest-api ai-integration"
cargo build -p eggsec-cli --features full
```

Manual/dry-run validation after implementation:

```bash
eggsec policy-explain \
  --target http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml

eggsec policy-explain \
  --target https://example.com \
  --profile hazardous-lab \
  --scope examples/scope-localhost.toml \
  --json

eggsec scope-explain \
  --target 10.0.0.5 \
  --scope examples/scope-synvoid-lab.toml \
  --json

eggsec plan \
  --target http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json

eggsec codegg-mcp --help
eggsec mcp-serve --help
```

## Handoff Notes

Treat this as an integration-hardening pass. The previous pass added the right primitives. This pass should make them unavoidable.

The most important invariant is: no target-bearing operation should execute without producing or consuming a shared policy decision. If a command cannot yet be fully migrated, mark it explicitly and add a test/TODO so it cannot be mistaken for covered behavior.
