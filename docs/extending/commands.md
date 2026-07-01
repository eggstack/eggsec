# Adding New CLI Commands

This guide covers how to register, implement, and migrate CLI commands using the
command registry defined in `crates/eggsec/src/commands/registry.rs`.

## What CommandRegistration Owns

Every CLI command is declared as a `CommandRegistration` entry in the
`REGISTERED_COMMANDS` static array. This is **metadata and routing, not
authorization**. The registry provides:

- Stable command IDs for dispatch and lookup.
- `OperationMetadata` linkage for descriptor generation.
- Visibility flags controlling where the command appears (CLI, TUI, MCP).
- A dispatch mode classification for the execution path.

Authorization remains the responsibility of `EnforcementContext::evaluate()`. The
registry never gates execution.

## Field Reference

### command_id

Stable string matching the CLI subcommand name (e.g., `"recon"`,
`"scan-ports"`). Must be unique across all entries. Used by `lookup_command()`,
`build_descriptor_for_command()`, and `suggest_command()`.

### operation_id

Canonical operation ID in `ALL_OPERATION_METADATA`, if applicable. `None` for
config/helper/server commands that have no operation metadata. When `Some`, the
value must resolve through `metadata_for_tool_id()` (canonical IDs and known
aliases are both valid).

### category

`CommandCategory` variant classifying the command. Affects diagnostics, test
matrix coverage, and visibility defaults:

| Variant | Meaning |
|---|---|
| `SideEffectingNetwork` | Network operations requiring enforcement (scans, fuzz, stress) |
| `LocalFileDomain` | Local file or domain-specific operations (DB, mobile, reports) |
| `PassiveAnalytical` | Read-only analysis (explain, AI analyze) |
| `ConfigOutputHelper` | Configuration, help, diagnostics (config, doctor, plan) |
| `FrontendServer` | Server daemons (REST, MCP, gRPC, agent) |
| `LegacySpecial` | Commands with no metadata or unique dispatch needs |

### feature

Optional Cargo feature gate. `None` means always compiled. When `Some`, the
string must match a feature in `Cargo.toml` (validated by
`tests/feature_matrix.rs`).

### cli_visible

Whether the command appears as a CLI `--help` target and accepts CLI arguments.
Set `true` for any command invoked from the shell.

### tui_visible

Whether the command appears in TUI tab listings and action menus. Only side
effects commands intended for interactive use from the TUI. Server, helper, and
config commands are typically `false`.

### programmatic_visible

Whether the command may be exposed through MCP, REST, gRPC, or agent surfaces.
Currently `false` for all entries; used for future programmatic API expansion.

### cli_interactive_only

Marks CLI-helper, config, and report-style commands that should not appear in
TUI or programmatic surfaces.

**Warning**: This does **not** mean "all human-interactive surfaces." TUI
visibility is controlled separately by `tui_visible`. A command can be
`tui_visible = true` and `cli_interactive_only = false` if it is intended for
operator use from both CLI and TUI. Tests enforce that `cli_interactive_only`
implies `!tui_visible` and `!programmatic_visible`.

### registry_backed

Whether the descriptor and execution path uses registry metadata. When `true`,
the command's `build_descriptor()` call must succeed and the handler builds its
`OperationDescriptor` through `CommandContext::describe_from_registry()`.

### dispatch_mode

How the command's execution is routed. See the next section.

## CommandDispatchMode Variants

| Variant | Meaning | When to use |
|---|---|---|
| `RegistryBacked` | Descriptor and dispatch use registry metadata (Phase 6 pilot pattern) | New commands that build descriptors via `describe_from_registry()` |
| `LegacyWrapped` | Routes through the legacy `handle_command()` match dispatch | Existing commands not yet migrated |
| `CatalogOnly` | Listed for discoverability but never dispatched | Future catalog entries |
| `ServerLifecycle` | Server daemon lifecycle (serve, mcp-serve, agent, grpc) | Long-running server processes |
| `HelperOnly` | Read-only helper/diagnostic (config, doctor, plan, preflight) | Non-side-effecting commands |

### Selection criteria

Choose `RegistryBacked` when:

- The command performs a side effect (network scan, fuzz, stress).
- The handler builds its `OperationDescriptor` via
  `ctx.describe_from_registry()`.
- The command should be inspectable through the registry for preflight and
  enforcement evaluation.

Choose `LegacyWrapped` for commands still dispatching through
`handle_command()` match arms. These are candidates for migration.

Choose `HelperOnly` or `ServerLifecycle` for commands that never flow through
`EnforcementContext::evaluate()`.

## How to Add a New CLI Command

### 1. Register in the command registry

Add an entry to `REGISTERED_COMMANDS` in
`crates/eggsec/src/commands/registry.rs`:

```rust
CommandRegistration {
    command_id: "my-new-command",
    operation_id: Some("my-new-command"),
    display_name: "My New Command",
    category: CommandCategory::SideEffectingNetwork,
    feature: None,               // or Some("my-feature") if feature-gated
    cli_visible: true,
    tui_visible: true,
    programmatic_visible: false,
    cli_interactive_only: false,
    registry_backed: true,
    dispatch_mode: CommandDispatchMode::RegistryBacked,
}
```

Ensure the `operation_id` resolves through `metadata_for_tool_id()`. If the
operation does not yet exist in `ALL_OPERATION_METADATA`, add it there first.

### 2. Add the operation metadata

If the operation ID does not already exist, add an `OperationMetadata` entry in
`crates/eggsec/src/config/policy.rs` (or wherever `ALL_OPERATION_METADATA` is
defined). The `command_id` and `operation_id` are usually identical but can
differ when a CLI subcommand name differs from the canonical operation ID.

### 3. Implement the handler

Create a handler in `crates/eggsec/src/commands/handlers/`. The handler should:

1. Parse CLI arguments.
2. Build the `OperationDescriptor` via `ctx.describe_from_registry()`:

```rust
pub fn handle(ctx: &CommandContext, args: MyArgs) -> Result<()> {
    let descriptor = ctx
        .describe_from_registry("my-new-command", Some(args.target.clone()))
        .expect("registry-backed command must have metadata");

    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // ... execute the operation ...
}
```

3. Register the subcommand in the CLI argument parser.
4. Wire the handler into `handle_command()` or the new dispatch path.

### 4. Add the CLI subcommand

Add the subcommand to the CLI argument parser in
`crates/eggsec-cli/src/main.rs` (or the relevant CLI crate) with the matching
`command_id` string.

### 5. Run tests

```bash
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test enforcement_matrix
cargo clippy --lib -p eggsec
```

## Migrating from LegacyWrapped to RegistryBacked

### Before migration

A legacy command dispatches through `handle_command()` match arms and builds
descriptors inline (or not at all):

```rust
// Legacy pattern (inside handle_command match)
"my-command" => {
    let descriptor = OperationDescriptor {
        operation: "my-command".to_string(),
        // ... inline construction ...
    };
    let decision = ctx.evaluate_and_enforce_operation(descriptor)?;
    // execute
}
```

### Migration steps

1. **Ensure operation metadata exists.** The `operation_id` must resolve via
   `metadata_for_tool_id()`. Add it to `ALL_OPERATION_METADATA` if missing.

2. **Update the registry entry.** Change `registry_backed` to `true` and
   `dispatch_mode` to `RegistryBacked`:

```rust
CommandRegistration {
    command_id: "my-command",
    operation_id: Some("my-command"),
    // ...
    registry_backed: true,
    dispatch_mode: CommandDispatchMode::RegistryBacked,
}
```

3. **Replace inline descriptor construction.** In the handler, use
   `ctx.describe_from_registry()`:

```rust
let descriptor = ctx
    .describe_from_registry("my-command", Some(target))
    .expect("registry-backed command must have metadata");
```

4. **Remove the inline `OperationDescriptor` construction.** The registry lookup
   replaces hand-built descriptors.

5. **Verify.** Run the full test suite:

```bash
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test enforcement_matrix
cargo clippy --lib -p eggsec
```

### Pilot commands

The following commands have completed migration and serve as reference
implementations:

| Command | Registry entry |
|---|---|
| `recon` | `handlers/recon.rs` |
| `scan-ports` | `handlers/scan.rs` |
| `scan-endpoints` | `handlers/scan.rs` |
| `fingerprint` | `handlers/scan.rs` |

## How Descriptors Are Built and Used

### Build path

1. The handler calls `ctx.describe_from_registry(command_id, target)`.
2. This delegates to `build_descriptor_for_command()` in
   `commands/registry.rs`.
3. `build_descriptor_for_command()` calls `lookup_command()` to find the
   `CommandRegistration`, then `reg.build_descriptor(target)`.
4. `build_descriptor()` calls `reg.metadata()` to get the `OperationMetadata`,
   then `metadata.descriptor_for_target(target)` to produce the
   `OperationDescriptor`.

### Enforcement path

1. The handler calls `ctx.evaluate_and_enforce_operation(descriptor)`.
2. This calls `self.enforcement.evaluate(&descriptor)` which is
   `EnforcementContext::evaluate()`.
3. The evaluation checks scope, risk tier, capabilities, feature gates, and
   policy flags against the current execution profile.
4. Returns `EnforcementOutcome` (Allow, Warn, RequireConfirmation, or Deny).
5. Under `ManualPermissive`, `RequireConfirmation` is checked against
   `ManualOverride` flags. Under strict profiles, it is treated as Deny.

### Preflight

The same `build_descriptor_for_command()` path is used by the CLI `preflight`
command, REST `POST /api/v1/tools/{tool_id}/preflight`, and MCP
`eggsec_preflight` tool. This ensures that preflight evaluation uses the same
metadata source as dispatch.

## Tests

### command_registry

Located at `crates/eggsec/tests/command_registry.rs`. Validates:

- All command IDs are unique.
- All `operation_id` values resolve to `OperationMetadata`.
- Feature-gated entries declare a non-empty feature string.
- `RegistryBacked` side-effecting commands build descriptors successfully.
- `LegacyWrapped` entries have valid metadata when present but do not require
  descriptor generation.
- `cli_interactive_only` commands are not `programmatic_visible`.
- Pilot commands (`recon`, `scan-ports`, `scan-endpoints`, `fingerprint`) have
  the expected registry properties.
- `dispatch_mode` is consistent with `registry_backed`, `operation_id`, and
  visibility flags.
- `suggest_command()` returns close matches for near-miss inputs.

### enforcement_matrix

Located at `crates/eggsec/tests/enforcement_matrix.rs`. Validates the dual-mode
enforcement contract across all execution surfaces:

- Manual permissive (CLI, TUI) allows safe operations; requires confirmation
  for scope misses.
- Manual guarded (CLI strict, TUI strict) denies scope misses; ignores manual
  overrides.
- MCP, agent, REST, CI strict profiles deny scope misses; never produce
  `Warn` or `RequireConfirmation`.
- Risk tiers, capabilities, feature gates, and exclusions are enforced
  consistently across surfaces.
- Permissive never hard-denes safe in-scope operations.
- Strict never produces `RequireConfirmation` or `Warn`.

### Required invariants for new commands

When adding a command, the following tests must pass:

```bash
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test enforcement_matrix
cargo clippy --lib -p eggsec
```

If the command is feature-gated, also run:

```bash
cargo test -p eggsec --test feature_matrix
```

## Warnings

1. **Registry metadata is not authorization.** The registry provides metadata for
   descriptor generation and diagnostics. All side-effecting operations still
   flow through `EnforcementContext::evaluate()` before execution. Do not add
   authorization checks to the registry.

2. **`cli_interactive_only` does not mean all human-interactive surfaces.** TUI
   visibility is controlled by `tui_visible`, not `cli_interactive_only`. A
   command can be visible in both CLI and TUI (`cli_visible = true`,
   `tui_visible = true`) while not being `cli_interactive_only`. Only
   CLI-helper, config, and report commands that should never appear in TUI or
   programmatic surfaces should set `cli_interactive_only = true`.

3. **Side-effecting registry-backed commands must build descriptors.** The test
   `registry_backed_side_effecting_commands_build_descriptors` enforces that
   every `RegistryBacked` command in the `SideEffectingNetwork` category has an
   `operation_id` and produces a valid `OperationDescriptor` from
   `build_descriptor()`. Failing this means the dispatch bridge cannot evaluate
   the command.

4. **Feature strings must match `Cargo.toml`.** The `tests/feature_matrix.rs`
   test validates that feature strings in registry entries and operation metadata
   match actual Cargo features. Add new feature strings to `KNOWN_EGGSEC_FEATURES`
   in that test file when introducing new features.
