# Phase 2 Handoff Plan: First-Class ExecutionSurface

## Goal

Introduce a first-class `ExecutionSurface` type that describes where an operation originates, then derive the correct `ExecutionProfile` and enforcement posture from that surface. This separates caller identity from enforcement behavior and prevents entrypoints from hand-rolling inconsistent profile selection.

The outcome should be a single source of truth for these mappings:

- CLI manual -> `ManualPermissive`.
- TUI manual -> `ManualPermissive`.
- CLI strict -> `ManualGuarded`.
- TUI guarded -> `ManualGuarded`.
- MCP server -> `McpStrict`.
- Security agent -> `AgentStrict`.
- CI -> `CiStrict`.
- REST API -> strict by default, pending Phase 7 refinement.

## Rationale

`ExecutionProfile` currently describes enforcement semantics. It does not fully encode caller origin. That leads to brittle code: each entrypoint has to know how to pick a profile and whether manual overrides should be honored. This already creates risk around the security-agent path, where a top-level CLI default can accidentally remain manual-permissive unless corrected elsewhere.

`ExecutionSurface` should become the semantic bridge between entrypoints and enforcement.

## Files likely to change

Primary:

- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/config/mod.rs`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/runner.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec/src/commands/handlers/serve.rs`
- `crates/eggsec/src/commands/handlers/agent.rs`

Secondary/test files:

- `crates/eggsec/src/config/policy_decision.rs`
- New or existing tests under `crates/eggsec/tests/` if integration-style testing is easier.

## Proposed design

Add a new enum near `ExecutionProfile` in `crates/eggsec/src/config/policy.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionSurface {
    CliManual,
    TuiManual,
    CliManualStrict,
    TuiManualStrict,
    McpServer,
    SecurityAgent,
    Ci,
    RestApi,
}
```

Add implementation helpers:

```rust
impl ExecutionSurface {
    pub fn profile(self) -> ExecutionProfile { ... }
    pub fn is_manual(self) -> bool { ... }
    pub fn is_agent_controlled(self) -> bool { ... }
    pub fn honors_manual_override(self) -> bool { ... }
    pub fn requires_explicit_manifest_for_networked(self) -> bool { ... }
    pub fn label(self) -> &'static str { ... }
}
```

Expected mappings:

```rust
pub fn profile(self) -> ExecutionProfile {
    match self {
        ExecutionSurface::CliManual | ExecutionSurface::TuiManual => ExecutionProfile::ManualPermissive,
        ExecutionSurface::CliManualStrict | ExecutionSurface::TuiManualStrict => ExecutionProfile::ManualGuarded,
        ExecutionSurface::McpServer => ExecutionProfile::McpStrict,
        ExecutionSurface::SecurityAgent => ExecutionProfile::AgentStrict,
        ExecutionSurface::Ci => ExecutionProfile::CiStrict,
        ExecutionSurface::RestApi => ExecutionProfile::McpStrict, // temporary strict posture until Phase 7 adds ApiStrict or split mode
    }
}
```

Do not add `ApiStrict` in this phase unless it is trivial and clearly cleaner. The minimal useful change is the caller-origin enum.

## Enforcement construction helper

Add a helper that constructs the right `EnforcementContext` from a surface:

```rust
impl EnforcementContext {
    pub fn for_surface(
        surface: ExecutionSurface,
        policy: ExecutionPolicy,
        loaded_scope: LoadedScope,
    ) -> Self {
        match surface.profile() {
            ExecutionProfile::ManualPermissive => Self::manual_permissive(policy, loaded_scope),
            ExecutionProfile::ManualGuarded => Self::manual_guarded(policy, loaded_scope),
            ExecutionProfile::CiStrict => Self::ci_strict(policy, loaded_scope),
            ExecutionProfile::McpStrict => Self::mcp_strict(policy, loaded_scope),
            ExecutionProfile::AgentStrict => Self::agent_strict(policy, loaded_scope),
        }
    }
}
```

If `ExecutionSurface` should be stored for audit/UI later, also add `surface: Option<ExecutionSurface>` to `EnforcementContext`. If that is too invasive, defer storage to Phase 5/TUI and Phase 10/audit. For Phase 2, the critical requirement is central construction.

## CLI entrypoint changes

In `crates/eggsec-cli/src/main.rs`, replace direct `ExecutionProfile` selection with `ExecutionSurface` selection.

Expected logic:

```rust
let execution_surface = match cli.command.as_ref() {
    Some(eggsec::cli::Commands::Ci(_)) => eggsec::config::ExecutionSurface::Ci,
    #[cfg(feature = "rest-api")]
    Some(eggsec::cli::Commands::Agent(_)) => eggsec::config::ExecutionSurface::SecurityAgent,
    #[cfg(feature = "rest-api")]
    Some(eggsec::cli::Commands::McpServe(_))
    | Some(eggsec::cli::Commands::CodeggMcp(_)) => eggsec::config::ExecutionSurface::McpServer,
    #[cfg(feature = "rest-api")]
    Some(eggsec::cli::Commands::Serve(_)) => eggsec::config::ExecutionSurface::RestApi,
    _ if cli.strict_scope => eggsec::config::ExecutionSurface::CliManualStrict,
    _ => eggsec::config::ExecutionSurface::CliManual,
};
```

Then derive:

```rust
let execution_profile = execution_surface.profile();
```

And construct or update `CommandContext` using this surface/profile.

If feature-gated match arms are awkward, use helper functions in the CLI module to avoid invalid feature combinations.

## CommandContext changes

Add optional surface storage:

```rust
pub execution_surface: ExecutionSurface,
```

Update constructors:

- `CommandContext::new(...)` should default to `ExecutionSurface::CliManual`.
- Add `with_execution_surface(surface)` that updates both stored surface and `EnforcementContext` through `EnforcementContext::for_surface(...)`.
- Keep `with_execution_profile(...)` temporarily if many tests depend on it, but mark it as transitional in comments.

Important: manual overrides should be ignored unless `execution_surface.honors_manual_override()` and profile is `ManualPermissive`.

Inside `evaluate_and_enforce_operation`, the current profile check is already protective. This phase can leave the deep evaluator mostly unchanged.

## TUI changes

TUI should use `ExecutionSurface::TuiManual` for default startup instead of constructing `manual_permissive` directly.

In `crates/eggsec-tui/src/app/mod.rs`, default enforcement should be created through the new helper. In `runner.rs`, where config/scope is loaded, create:

```rust
let surface = ExecutionSurface::TuiManual;
app.enforcement = EnforcementContext::for_surface(surface, policy, loaded_scope.clone());
```

Do not implement the TUI guarded toggle in this phase; that belongs to Phase 5.

## MCP changes

In `handle_mcp_serve`, construct enforcement via `ExecutionSurface::McpServer` rather than calling `mcp_strict` directly. The result should still be `McpStrict`.

This is mostly a readability and invariant improvement.

## Agent changes

This phase should prepare the surface mapping for agent strictness, but Phase 3 will do the full correction and defense-in-depth. It is acceptable in Phase 2 to make `main.rs` map `Commands::Agent(_)` to `SecurityAgent`; Phase 3 will harden the agent handler/runtime.

## REST changes

Map `Commands::Serve(_)` to `ExecutionSurface::RestApi` and derive strict enforcement context, but do not fully retrofit REST dispatch yet. Phase 7 will complete REST enforcement. If this phase stores the surface in `CommandContext`, `handle_serve` can start receiving the correct posture even before dispatch logic is changed.

## Tests to add

Add unit tests for `ExecutionSurface` mapping:

- `CliManual.profile() == ManualPermissive`
- `TuiManual.profile() == ManualPermissive`
- `CliManualStrict.profile() == ManualGuarded`
- `TuiManualStrict.profile() == ManualGuarded`
- `McpServer.profile() == McpStrict`
- `SecurityAgent.profile() == AgentStrict`
- `Ci.profile() == CiStrict`
- `RestApi.profile().is_strict()`

Add tests for helper predicates:

- Only CLI/TUI manual surfaces honor manual overrides.
- MCP/security-agent/CI/REST are agent-controlled or automated as appropriate.
- Strict surfaces require explicit manifest for networked execution.

If `CommandContext` gets `execution_surface`, add tests proving `with_execution_surface(SecurityAgent)` produces `AgentStrict` enforcement.

## Acceptance criteria

- `ExecutionSurface` exists and is exported from `eggsec::config`.
- Entry points derive `ExecutionProfile` from `ExecutionSurface`, not ad hoc profile selection.
- CLI default remains manual permissive.
- CLI `--strict-scope` remains manual guarded.
- TUI default remains manual permissive, now via `ExecutionSurface::TuiManual`.
- MCP maps to `McpStrict` via `ExecutionSurface::McpServer`.
- Agent maps to `AgentStrict` via `ExecutionSurface::SecurityAgent`.
- REST maps to a strict profile placeholder pending Phase 7.
- Unit tests cover the mapping and helper predicates.

## Suggested validation

Run:

```bash
cargo fmt --all
cargo test -p eggsec --lib config::policy
cargo test -p eggsec --lib config::policy_decision
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-tui
```

If feature combinations are expensive, at minimum run the default checks plus one `rest-api` check because MCP/agent/serve command variants are feature-gated.

## Non-goals

- Do not complete REST enforcement in this phase.
- Do not add full enforcement matrix tests yet.
- Do not implement TUI guarded toggle yet.
- Do not redesign `ExecutionProfile`.
- Do not remove existing constructors until follow-up phases prove all call sites are migrated.

## Common pitfalls

- Do not make `CliManual` strict by default.
- Do not honor manual overrides for `SecurityAgent`, `McpServer`, `Ci`, or `RestApi`.
- Do not rely on raw `Scope` for automated profile decisions.
- Do not introduce a new enum that duplicates `ExecutionProfile` without adding caller-origin semantics.
