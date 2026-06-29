# Phase 8 Handoff Plan: Enforcement Matrix Test Suite

## Goal

Create a comprehensive enforcement matrix test suite that protects the dual-mode contract across all execution surfaces. The matrix should catch both categories of regression:

1. Manual CLI/TUI becoming too strict to be useful.
2. Agent/MCP/REST/CI becoming too permissive or honoring manual discretion.

This phase converts the architecture into durable regression coverage.

## Current context

The repo already has useful focused tests:

- `ExecutionSurface` mapping tests.
- Agent strictness tests.
- Manual override tests.
- TUI posture/preflight tests.
- REST strict enforcement tests.

Those tests are good, but scattered. The matrix suite should become the canonical cross-surface guardrail.

## Files likely to change

- New: `crates/eggsec/tests/enforcement_matrix.rs`
- Or if integration tests are awkward, new module: `crates/eggsec/src/config/enforcement_matrix_tests.rs`
- `crates/eggsec/src/config/policy_decision.rs` for helper visibility if needed
- `crates/eggsec/src/config/scope.rs` for test helpers if needed
- `crates/eggsec/src/tool/metadata.rs` if Phase 6 is in place

## Matrix dimensions

### Execution surfaces

Cover at least:

- `ExecutionSurface::CliManual`
- `ExecutionSurface::CliManualStrict`
- `ExecutionSurface::TuiManual`
- `ExecutionSurface::TuiManualStrict`
- `ExecutionSurface::McpServer`
- `ExecutionSurface::SecurityAgent`
- `ExecutionSurface::Ci`
- `ExecutionSurface::RestApi`

### Scope states

Cover:

- `LoadedScope::default_empty()`
- Explicit allow match
- Explicit positive allowlist miss
- Explicit exclusion match
- Private/local target allowed by explicit local scope
- Public target resolving/treated as private-resolution where supported by descriptor/decision helpers

### Risk tiers

Cover representative tiers:

- `Passive`
- `SafeActive`
- `Intrusive`
- `LoadTest`
- `StressTest`
- `RawPacket`
- `CredentialTesting`
- `DbPentest`
- `TrafficInterception`
- `RemoteExecution`
- `C2Operation`
- `AgentAutonomous`

Do not test every risk tier against every surface if that creates excessive runtime. Use parameterized helpers and representative cases.

### Capabilities

Cover:

- Baseline capability: `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`
- Nonbaseline capability: `RawPacketProbe`, `CredentialTesting`, `DatabaseAssessment`, `TrafficInterception`, `RemoteExecution`, `C2Simulation`
- Explicitly allowed capability
- Explicitly denied capability

### Overrides

Cover:

- No override
- `assume_yes`
- `allow_out_of_scope`
- `allow_high_risk`
- `allow_private_resolution`
- `allow_cross_host_redirect`
- `allow_nonbaseline_capability`
- `allow_web_proxy`
- `allow_db_pentest`
- Irrelevant override for a required class
- Overrides supplied under strict/automated surfaces

## Test helper design

Create small helper structs to reduce boilerplate:

```rust
struct MatrixCase {
    name: &'static str,
    surface: ExecutionSurface,
    scope: ScopeFixture,
    descriptor: DescriptorFixture,
    policy: PolicyFixture,
    manual_override: ManualOverride,
    expected: ExpectedOutcome,
}

enum ExpectedOutcome {
    Allow,
    Warn,
    RequireConfirmation(&'static [ConfirmationClass]),
    Deny,
}
```

Add helpers:

```rust
fn enforcement(surface: ExecutionSurface, policy: ExecutionPolicy, scope: LoadedScope) -> EnforcementContext;
fn safe_descriptor(target: &str) -> OperationDescriptor;
fn intrusive_descriptor(target: &str) -> OperationDescriptor;
fn descriptor_with_capability(target: &str, cap: Capability) -> OperationDescriptor;
fn explicit_scope_allow(pattern: &str) -> LoadedScope;
fn explicit_scope_exclude(pattern: &str) -> LoadedScope;
```

Keep helpers in the test file unless they are broadly useful.

## Required tests

### Surface mapping invariants

- CLI manual and TUI manual map to `ManualPermissive`.
- CLI/TUI strict map to `ManualGuarded`.
- MCP maps to `McpStrict`.
- Security agent maps to `AgentStrict`.
- CI maps to `CiStrict`.
- REST maps to strict profile.
- Only CLI/TUI manual honor manual overrides.

### Manual permissive invariants

- Safe passive/safe-active target with default empty scope allows or warns, not hard deny.
- Positive allowlist miss requires confirmation.
- Positive allowlist miss plus matching manual override can proceed through `CommandContext`-style override path.
- High-risk operation requires dedicated high-risk/capability flag.
- `assume_yes` does not permit high-risk, private-resolution, cross-host redirect, traffic interception, explicit exclusion, or nonbaseline capability.
- Explicit denied capability hard denies even in manual mode.
- Missing compile-time feature hard denies even in manual mode.

### Manual guarded invariants

- Positive allowlist miss denies.
- Missing explicit/required scope denies where descriptor requires it.
- Manual overrides are ignored.
- `RequireConfirmation` is not dispatchable.

### MCP invariants

- Missing explicit manifest denies target-bearing explicit-scope operations.
- Positive allowlist miss denies.
- Manual override flags have no effect.
- Nonbaseline capability not allowlisted denies.
- `Warn` and `RequireConfirmation` are not dispatchable.

### Security agent invariants

- Same as MCP, plus:
- Agent surface requires `AgentStrict`.
- Agent descriptors require explicit scope for target-bearing scans.
- Warnings are treated as denial in agent runtime tests.

### REST invariants

- REST requires explicit manifest for target-bearing explicit-scope operations.
- REST dispatches only on `Allow` if Phase 7 is complete.
- REST positive allowlist miss denies.
- REST missing metadata or non-rest-exposable metadata fails closed if Phase 6 is complete.
- REST ignores manual overrides.

### CI invariants

- CI strict behavior matches automated strict behavior.
- CI does not honor manual overrides.
- CI requires explicit scope where descriptor requires it.

## CommandContext-style tests

Policy-level tests should not be the only coverage. Add a few `CommandContext::evaluate_and_enforce_operation()` tests to verify manual override handling:

- Manual permissive with required class + matching override returns `Ok` and marks `manual_override_used`.
- Manual permissive with irrelevant override returns error.
- Manual guarded with matching override still errors.
- Agent/MCP/REST context with matching override still errors.

## Metadata integration tests

If Phase 6 metadata exists, add matrix tests that generate descriptors from metadata for representative operations:

- `recon` baseline/manual/MCP/REST.
- `fuzz` intrusive/manual confirmation/strict denial unless policy allows.
- `db-pentest` defense-lab/high-risk.
- `proxy-start` traffic interception.
- `packet` raw-packet capability.

## Acceptance criteria

- A dedicated enforcement matrix suite exists.
- It covers all execution surfaces.
- It covers manual permissive vs strict/automated behavior.
- It tests scope, risk, capability, and override axes.
- It proves manual discretion does not leak into MCP/agent/REST/CI.
- It proves agent strictness does not become the default for CLI/TUI manual.
- Tests are readable enough to extend when new tools/surfaces are added.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --features rest-api --test enforcement_matrix
cargo test -p eggsec --lib config::policy_decision
cargo test -p eggsec-tui
```

If the matrix is a unit-test module rather than integration test, run the relevant module path.

## Non-goals

- Do not change enforcement semantics unless a test reveals a real contradiction with the mode contract.
- Do not require every possible operation/risk/tool combination in the first pass.
- Do not add type-level dispatch tokens yet.
- Do not extract domain crates yet.
