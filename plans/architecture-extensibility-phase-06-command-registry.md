# Architecture Extensibility Phase 6: Command Registry Refactor

## Objective

Refactor the CLI/TUI command dispatch surface away from an ever-growing central match/handler pattern toward a typed, metadata-aware command registry. The goal is not to remove every match immediately; the goal is to establish a durable registration model that reduces handler sprawl, keeps operation metadata close to execution adapters, and preserves the central enforcement boundary.

This phase builds on the corrected `OperationMetadata`, `DomainDescriptor`, `CapabilityMatrixRow`, and metadata consistency tests. Command dispatch must become easier to extend without weakening manual CLI/TUI semantics or strict programmatic enforcement.

## Current context

The main `eggsec` crate still owns substantial command orchestration. `CommandContext` carries config, loaded scope, execution surface/profile, enforcement state, manual override flags, and output mode. Handlers build `OperationDescriptor`s, call enforcement, and then dispatch or execute domain-specific code. This is acceptable for safety, but scaling is poor: adding a command often requires touching the central CLI enum, the central handler match, help/docs, metadata, TUI descriptors, and sometimes tool registration separately.

The desired architecture is:

- `OperationMetadata` remains the canonical per-operation policy metadata.
- `DomainDescriptor` remains the canonical domain grouping/integration metadata.
- A new command registry maps command IDs to dispatch adapters.
- The registry can be inspected by CLI/TUI/docs/tests without executing commands.
- Runtime execution still flows through `CommandContext` and `EnforcementContext`.
- Manual CLI/TUI discretion remains separate from MCP/agent strictness.

## Non-goals

- Do not redesign CLI argument parsing wholesale.
- Do not remove all existing handler modules in this phase.
- Do not make domain crates decide authorization.
- Do not expose new MCP/REST/agent tools.
- Do not change manual override semantics.
- Do not convert every legacy command at once unless the conversion remains small and low-risk.

## Design target

Introduce a command registry layer in `crates/eggsec/src/commands/` or a nearby module. Suggested shape:

```rust
pub struct CommandRegistration {
    pub command_id: &'static str,
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub category: CommandCategory,
    pub feature: Option<&'static str>,
    pub manual_only: bool,
    pub tui_visible: bool,
    pub builds_descriptor: fn(&CommandInvocation) -> Result<OperationDescriptor>,
    pub execute: CommandExecutor,
}

pub enum CommandExecutor {
    Sync(fn(&mut CommandContext, CommandInvocation, ApprovedOperation) -> Result<CommandResult>),
    Async(/* boxed or typed future adapter, as appropriate */),
    Legacy(fn(CommandContext, Command) -> BoxFuture<'static, Result<()>>),
}
```

This exact type shape is not mandatory. Keep it idiomatic for the current async setup. The important properties are:

- registration is static and inspectable;
- descriptors are built before side effects;
- approval tokens are passed into execution for side-effecting operations;
- legacy handlers can be wrapped during migration;
- metadata IDs are validated against `OperationMetadata`.

## Work item 1: Inventory current command dispatch paths

Create or update a short internal inventory of current command handlers.

Required checks:

- Identify every arm in `handle_command()` or equivalent central dispatcher.
- Categorize commands as:
  - side-effecting network operation;
  - local-file/domain operation;
  - passive analytical command;
  - config/output/helper command;
  - frontend/server command;
  - legacy/special command.
- Mark whether each command already has `OperationMetadata`.
- Mark whether each command can be moved to a registry in this phase.

Suggested output location:

- `docs/COMMAND_REGISTRY.md`, or
- a section in `docs/ARCHITECTURE.md`, if small.

Acceptance criteria:

- Future maintainers can see which commands are registry-backed and which remain legacy.
- The inventory distinguishes no-dispatch commands like CI/config/help from side-effecting commands.

## Work item 2: Add a minimal command registration type

Implement a small registration model that can coexist with the current dispatcher.

Required fields:

- stable command ID;
- canonical operation ID where applicable;
- feature gate where applicable;
- command category;
- execution surface eligibility, at least manual vs programmatic distinction;
- descriptor builder or metadata lookup hook;
- execution adapter.

Recommended file locations:

- `crates/eggsec/src/commands/registry.rs`
- re-export from `crates/eggsec/src/commands/mod.rs` or `commands/handlers/mod.rs` as appropriate.

Acceptance criteria:

- Registry compiles with no default features.
- Registry is static or lazily static without runtime I/O.
- Registry entries do not authorize operations.
- Registry entries can be enumerated by tests.

## Work item 3: Pilot with low-risk commands

Start with a small set of straightforward commands that already have stable metadata and simple descriptor generation.

Recommended pilot commands:

- `recon`
- `scan-ports`
- `fingerprint`
- `scan-endpoints`
- optionally `search` if it is passive and no-target.

Avoid starting with db-pentest, mobile dynamic, C2, proxy intercept, or raw packet operations. Those are important but have more nuanced feature/policy/runtime behavior.

Pilot requirements:

- Registry-backed command still calls the same underlying execution logic.
- Enforcement outcome remains identical to legacy path.
- Manual permissive and strict behavior remains unchanged.
- Existing CLI args continue to work.

Acceptance criteria:

- At least two low-risk commands are represented in the registry.
- Tests prove registry metadata resolves to `OperationMetadata`.
- No user-visible CLI behavior changes except possibly improved diagnostics.

## Work item 4: Add dispatch bridge while preserving legacy fallback

Create a dispatch bridge:

1. Try to resolve a command through the registry.
2. If registry-backed, execute via the registry path.
3. If not registry-backed, fall back to the existing legacy handler path.

The fallback must be explicit and documented. Do not silently bypass enforcement in the registry path.

Required behavior:

- Side-effecting registry commands build `OperationDescriptor` first.
- Use existing `CommandContext::evaluate_and_enforce_operation()` or an equivalent typed approval flow.
- Pass `ApprovedOperation` into execution where practical.
- If legacy handlers still perform their own enforcement, do not double-execute preflight side effects.

Acceptance criteria:

- Registry-backed and legacy-backed commands both work.
- Tests verify at least one registry-backed command and one legacy command execute through expected paths.
- No strict surface can use the registry to bypass `EnforcementContext`.

## Work item 5: Metadata consistency tests for command registration

Add tests in `crates/eggsec/tests/metadata_consistency.rs` or a new `command_registry.rs` integration test.

Required tests:

- Every registry entry with `operation_id` resolves to `OperationMetadata`.
- Registry command IDs are unique.
- Registry operation IDs are either canonical metadata IDs or documented aliases.
- Feature-gated registry entries declare a non-empty feature string.
- Side-effecting registry entries have a descriptor builder.
- Commands marked manual-only are not exposed through programmatic registries.

Acceptance criteria:

- Adding a registry command without metadata fails tests.
- Duplicate command IDs fail tests.
- Feature-gated commands without feature metadata fail tests.

## Work item 6: Improve diagnostics

Use the registry to improve command diagnostics without large UI changes.

Possible improvements:

- Unknown command suggestions based on registered command IDs.
- Clear feature-missing error from registry metadata.
- Help/documentation pointers from `OperationMetadata`/`DomainDescriptor`.

Keep this small.

Acceptance criteria:

- Feature-missing errors are at least as clear as before.
- No extra startup cost or I/O is introduced.

## Safety requirements

- The registry is metadata and routing, not authorization.
- Execution remains centrally mediated by `EnforcementContext`.
- Manual CLI/TUI may continue to present warnings/confirmation according to `ManualPermissive` semantics.
- Strict and automated surfaces must still fail closed.
- Legacy fallback must be documented as transitional and covered by tests.

## Files likely to change

- `crates/eggsec/src/commands/mod.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`
- `crates/eggsec/src/commands/registry.rs` (new)
- `crates/eggsec/src/config/policy.rs` if command metadata needs small additions
- `crates/eggsec/tests/metadata_consistency.rs`
- optionally `crates/eggsec/tests/command_registry.rs`
- `docs/ARCHITECTURE.md`
- optionally `docs/COMMAND_REGISTRY.md`

## Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
```

If pilot commands touch scanner/recon features, run relevant CLI smoke checks if available. If no CLI smoke tests exist, add small unit/integration tests around registry descriptor generation.

## Completion criteria

Phase 6 is complete when:

- A command registry exists and is documented.
- At least two low-risk commands are registry-backed.
- Legacy fallback remains explicit.
- Registry entries are validated against operation metadata.
- No command path bypasses central enforcement.
- Documentation identifies what remains legacy and why.

## Handoff note

After Phase 6, future command migrations should be incremental. Do not attempt a single all-command conversion unless tests are already strong enough to catch enforcement and CLI behavior drift.
