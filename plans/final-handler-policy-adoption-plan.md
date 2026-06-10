# Final Handler Policy Adoption Plan

## Purpose

This is the last intended pass for the current Eggsec policy-hardening thread. The MCP IPv6 parser bug has been fixed, IPv6 tests were added, MCP risk/denial tests were strengthened, and the core policy architecture is in place.

The remaining gap is verification and adoption: confirm that target-bearing command handlers use the shared policy evaluator, migrate any high-risk handlers that are still scope-only, and record validation results.

Keep this pass narrow. Do not add new security features or new scan behavior.

## Current Known-Good State

The following pieces are already in place and should not be redesigned:

- `OperationDescriptor`
- `PolicyDecision`
- `evaluate_operation_policy`
- `CommandContext::evaluate_and_enforce_operation`
- config-aware `policy-explain`
- feature-gate checks and `missing_features`
- MCP profile restrictions
- MCP tool-risk classification
- MCP denial helpers with structured policy-decision data
- IPv6 hostname parsing fix for bare IPv6 and bracketed IPv6
- MCP tests for IPv6 parsing, coding-agent target policy, tool-risk classification, and structured denial data

## Non-Goals

Do not broaden coding-agent MCP permissions.

Do not remove or weaken defense-lab/hazardous-lab gates.

Do not add new probes, payloads, or scan techniques.

Do not refactor all command handlers for style-only reasons.

Do not treat policy denials as vulnerabilities in reports.

## Phase 1: Create Command Handler Policy Audit

Create:

- `docs/internal/POLICY_HANDLER_AUDIT.md`

Run locally:

```bash
rg "evaluate_and_enforce_operation|evaluate_operation_policy|ensure_scope|ensure_scope_url|check_scope" \
  crates/eggsec/src/commands \
  crates/eggsec/src/commands/handlers \
  crates/eggsec/src/tool/protocol \
  crates/eggsec/src/api \
  crates/eggsec/src/server
```

If some directories do not exist, note that in the audit and continue.

The audit should have this table shape:

```markdown
| Path | Entry point | Target-bearing? | Current policy path | Risk tier | Status | Notes |
|---|---|---:|---|---|---|---|
| crates/eggsec/src/commands/handlers/stress.rs | handle_stress | yes | evaluate_and_enforce_operation | StressTest | migrated | requires stress-testing feature |
```

Classify each handler as one of:

- `migrated`: uses `ctx.evaluate_and_enforce_operation` or direct `evaluate_operation_policy` with an `OperationDescriptor`.
- `scope-only`: uses scope helpers but does not evaluate operation risk.
- `no-target`: does not accept or operate on a target.
- `deferred`: not migrated in this pass; must include exact reason.
- `feature-gated`: behind a Cargo feature; audit should state which feature.

## Phase 2: Prioritize High-Risk Handler Migration

High-risk handlers must not remain `scope-only` unless there is a precise documented reason.

Audit and migrate these first:

- `stress`
- `waf_stress`
- `packet`
- `proxy`
- `remote`
- `exec`
- `nse`
- `agent`
- REST/API tool execution
- MCP tool execution
- gRPC tool execution

For each target-bearing high-risk handler, build an `OperationDescriptor` with accurate values.

Suggested examples:

### Stress / WAF stress

```rust
OperationDescriptor {
    operation: "stress".to_string(),
    mode: OperationMode::HazardousLab,
    risk: OperationRisk::StressTest,
    intended_uses: vec![IntendedUse::DistributedSystemStress, IntendedUse::SynvoidRegression],
    target: Some(target.clone()),
    required_features: vec!["stress-testing".to_string()],
    required_policy_flags: vec![],
    requires_private_or_local_target: true,
    requires_explicit_scope: true,
}
```

### Packet / protocol edge

```rust
OperationDescriptor {
    operation: "packet".to_string(),
    mode: OperationMode::DefenseLab,
    risk: OperationRisk::RawPacket,
    intended_uses: vec![IntendedUse::ProtocolEdgeValidation],
    target: Some(target.clone()),
    required_features: vec!["packet-inspection".to_string()],
    required_policy_flags: vec![],
    requires_private_or_local_target: true,
    requires_explicit_scope: true,
}
```

### NSE safe profile

```rust
OperationDescriptor {
    operation: "nse".to_string(),
    mode: OperationMode::DefenseLab,
    risk: OperationRisk::SafeActive,
    intended_uses: vec![IntendedUse::CodingAgentVerification],
    target: Some(target.clone()),
    required_features: vec!["nse".to_string()],
    required_policy_flags: vec![],
    requires_private_or_local_target: true,
    requires_explicit_scope: true,
}
```

Adjust risk and intended use to match the actual command behavior. Do not blindly copy these examples if the command has different semantics.

Acceptance criteria:

- No high-risk target-bearing handler is only scope-checked.
- JSON mode denials return serialized `PolicyDecision` or an error containing structured policy data.
- Human mode denials remain readable.
- Existing allowed workflows continue to work.

## Phase 3: Ensure MCP/API/gRPC Tool Execution Uses Policy Decisions

MCP has `policy_decision_for_mcp_call` and `to_mcp_error_with_decision`. Verify these helpers are used in actual request dispatch, not just unit tests.

Tasks:

- Inspect MCP `tools/call` routing and stdio dispatch.
- Inspect REST/API tool execution paths, if present.
- Inspect gRPC tool execution paths, if present.
- Ensure denied target/tool calls carry structured policy information.

Acceptance criteria:

- Coding-agent MCP denial for `stress`, `packet`, `proxy`, `remote`, or `exec` returns structured policy data.
- Coding-agent MCP public-target denial returns structured policy data.
- Ops-agent remains broader but still respects execution policy/scope where the execution path uses the core engine.
- Tests cover at least one actual dispatch-level denial, not only helper-level denial.

## Phase 4: Add Minimal Regression Tests for Migrated Handlers

Do not try to integration-test every scanner. Add focused policy-denial tests where feasible.

Recommended tests:

- stress handler denies without `allow_stress_testing`.
- packet handler denies without `allow_raw_packets` or without `packet-inspection` feature.
- WAF regression profile requires scope for private/lab modes.
- JSON-mode denial includes `decision_id`, `operation`, `operation_risk`, and `denied_reasons`.

If full command-handler tests are too heavy, add lower-level tests around descriptor construction helpers.

Acceptance criteria:

- Tests cover migrated policy behavior without making external network requests.
- Tests are deterministic and do not depend on system network state.

## Phase 5: Record Validation Results

Add a short validation note:

- `docs/internal/POLICY_VALIDATION_RESULTS.md`

Include:

```markdown
# Policy Validation Results

Date: YYYY-MM-DD
Branch/commit: <commit>

## Commands Run

- [ ] cargo fmt --all
- [ ] cargo test --workspace
- [ ] cargo test --workspace --all-features
- [ ] cargo clippy --workspace --all-targets --all-features -- -D warnings
- [ ] cargo build -p eggsec-cli
- [ ] cargo build -p eggsec-cli --features stress-testing
- [ ] cargo build -p eggsec-cli --features packet-inspection
- [ ] cargo build -p eggsec-cli --features "rest-api ai-integration"
- [ ] cargo build -p eggsec-cli --features full

## Results

Document pass/fail, exact failure text if any, and whether failure is code-related or environmental.
```

Do not claim a command passed unless it was actually run.

## Phase 6: Required Local Validation

Run:

```bash
cargo fmt --all
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Feature builds:

```bash
cargo build -p eggsec-cli
cargo build -p eggsec-cli --features stress-testing
cargo build -p eggsec-cli --features packet-inspection
cargo build -p eggsec-cli --features "rest-api ai-integration"
cargo build -p eggsec-cli --features full
```

Manual policy checks:

```bash
eggsec policy-explain \
  --target http://127.0.0.1:8080 \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json

eggsec policy-explain \
  --target https://example.com \
  --profile waf-regression \
  --scope examples/scope-localhost.toml \
  --json

eggsec plan \
  --target http://127.0.0.1:8080 \
  --profile protocol-edge \
  --scope examples/scope-localhost.toml \
  --format json
```

## Stopping Condition

This thread is complete when:

1. `docs/internal/POLICY_HANDLER_AUDIT.md` exists.
2. High-risk target-bearing handlers are migrated or explicitly documented as deferred with reason.
3. MCP/API/gRPC dispatch paths use structured policy denials where applicable.
4. Minimal regression tests cover at least the most important migrated denials.
5. `docs/internal/POLICY_VALIDATION_RESULTS.md` records actual command results.
6. No known bare IPv6 parser bug remains.

After this pass, treat the policy-hardening work as complete unless validation exposes a concrete defect.
