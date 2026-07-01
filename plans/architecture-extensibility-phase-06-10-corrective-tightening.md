# Corrective Handoff Plan: Phase 6–10 Registry, Exposure, and Feature Tightening

## Objective

Tighten the architecture work that landed after Phases 6–10. The implementation substantially improved the repo by adding a command registry, tool registration model, TUI metadata specs, an enforcement facade, a normalized report envelope, and feature matrix tests. This corrective pass should preserve those gains while fixing the semantic drift and continuity issues that appeared during implementation.

This is a targeted cleanup pass. Do not continue to Phase 11 CI wiring until the registry/exposure semantics are stable and the plan directory remains coherent for handoff history.

## Current state summary

Recent implementation added:

- `crates/eggsec/src/commands/registry.rs`
- `crates/eggsec/tests/command_registry.rs`
- `crates/eggsec/src/tool/registration.rs`
- `crates/eggsec/tests/tool_registration.rs`
- `crates/eggsec-tui/src/app/action_spec.rs`
- `crates/eggsec-tui/src/app/enforcement_facade.rs`
- `crates/eggsec-output/src/envelope.rs`
- `crates/eggsec-output/tests/report_envelope.rs`
- `crates/eggsec/tests/feature_matrix.rs`
- docs for command registry, tool registration, report/evidence model, feature matrix, and TUI architecture.

The broad direction is correct. Remaining issues are mostly semantic precision, test correctness, and handoff continuity:

1. `ToolRegistration::mcp_exposed_by_default` currently defaults to `OperationMetadata::mcp_exposable` for standalone operations. This conflates metadata-level exposability with default MCP listing.
2. `CommandRegistration::manual_only` is ambiguous. Docs say CLI/TUI manual-only, while tests treat `manual_only` as not TUI-visible.
3. The command registry is partly a true registry and partly a catalog of legacy commands. That needs explicit naming and tests.
4. A registry unit test checks duplicates using `dedup()` without sorting, so it only catches adjacent duplicates.
5. The feature matrix test duplicates Cargo feature declarations manually. This is acceptable short term but should be hardened or explicitly documented as a snapshot.
6. The `full` aggregate includes advanced/lab-only features. This may be intentional, but it needs an explicit profile distinction.
7. `plans/architecture-extensibility-phase-09-report-evidence-unification.md` was removed after execution. Previous cleanup established that executed plans should remain available for handoff/audit continuity.

## Non-goals

- Do not remove the command registry.
- Do not remove the tool registration model.
- Do not change the normalized report envelope unless tests reveal a compile or serialization issue.
- Do not add new security capabilities.
- Do not wire CI yet; that belongs to Phase 11 after this cleanup.
- Do not weaken manual CLI/TUI operator discretion.
- Do not weaken strict MCP/REST/gRPC/agent enforcement.

## Work item 1: Split MCP exposure semantics in `ToolRegistration`

### Problem

`ToolRegistration` currently has `mcp_exposed_by_default`, but for standalone operations it is initialized from `OperationMetadata::mcp_exposable`. Under the documented Model A, `mcp_exposable` means metadata-level programmatic exposability when compiled, registered, scoped, and policy-authorized. It does not necessarily mean default listing on MCP profiles.

This reintroduces a drift class already corrected in the domain metadata layer: "tool can be exposed under policy" is not the same as "tool is default-listed for an MCP profile."

### Required changes

Replace or augment `ToolRegistration` fields so they distinguish:

- `mcp_metadata_exposable`: derived from `OperationMetadata::mcp_exposable`.
- `mcp_default_visible`: whether the tool is shown in the default MCP tool list for a profile.
- `required_mcp_feature`: opt-in feature gate for MCP exposure.
- `mcp_profile_visibility`: optional future field or helper that answers visibility for `ops-agent`, `coding-agent`, etc.

Suggested struct shape:

```rust
pub struct ToolRegistration {
    pub tool_id: &'static str,
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub source: ToolRegistrationSource,
    pub feature: Option<&'static str>,
    pub mcp_metadata_exposable: bool,
    pub mcp_default_visible: bool,
    pub required_mcp_feature: Option<&'static str>,
    pub rest_exposable: bool,
    pub grpc_exposable: bool,
    pub agent_exposable: bool,
    pub category: ToolCategory,
}
```

### Default visibility policy

Define a conservative default policy for MCP listing:

- Domain `ToolIntegration` remains authoritative for domain tools: use `tool.mcp_exposed_by_default`.
- Standalone base tools should only be default-visible if they are low-risk or explicitly allowlisted for the MCP profile.
- High-risk operations may remain `mcp_metadata_exposable = true` but should not automatically become `mcp_default_visible = true`.
- Profile-specific allowlists may exist, but should be named and tested.

Recommended helper:

```rust
fn default_mcp_visible_for_operation(meta: &OperationMetadata) -> bool {
    matches!(meta.risk, OperationRisk::Passive | OperationRisk::SafeActive)
        && meta.mcp_exposable
        && meta.required_features.is_empty()
}
```

Adjust the exact policy to match project intent, but make it explicit.

### Required tests

Add/update tests in `tool_registration.rs`:

- `mcp_metadata_exposable` matches `OperationMetadata::mcp_exposable`.
- `mcp_default_visible` does not automatically equal `mcp_metadata_exposable` for high-risk operations.
- High-risk standalone operations are not default-visible unless explicitly allowlisted with a test explaining why.
- Domain tools still respect `ToolIntegration::mcp_exposed_by_default`.
- Opt-in MCP tools with `required_mcp_feature` are not default-visible.

### Acceptance criteria

- No field name implies default listing when it only means metadata-level exposure.
- `mcp_tool_registrations("ops-agent")` and `mcp_tool_registrations("coding-agent")` are based on explicit profile visibility, not raw `mcp_exposable`.
- Tests would fail if a high-risk operation becomes default MCP-visible accidentally.

## Work item 2: Clarify command visibility semantics

### Problem

`CommandRegistration::manual_only` is ambiguous. The field comment says manual-only means CLI/TUI and not programmatic. Tests currently assert manual-only commands should not be TUI-visible, which contradicts the comment because TUI is manual.

### Required changes

Replace `manual_only` with explicit visibility/surface fields.

Recommended fields:

```rust
pub struct CommandRegistration {
    pub command_id: &'static str,
    pub operation_id: Option<&'static str>,
    pub display_name: &'static str,
    pub category: CommandCategory,
    pub feature: Option<&'static str>,
    pub cli_visible: bool,
    pub tui_visible: bool,
    pub programmatic_visible: bool,
    pub interactive_only: bool,
    pub registry_backed: bool,
}
```

Definitions:

- `cli_visible`: appears as a CLI command or CLI help target.
- `tui_visible`: appears in TUI tab/action listing.
- `programmatic_visible`: may be exposed through MCP/REST/gRPC/agent command-like surfaces. This should usually be false for CLI commands; tool registration governs tools.
- `interactive_only`: requires human interaction and must not be used by automated surfaces.
- `registry_backed`: descriptor/execution path is actually registry-backed, not merely cataloged.

If this is too many fields for this pass, use a smaller enum:

```rust
pub enum CommandVisibility {
    CliOnly,
    TuiOnly,
    CliAndTui,
    ProgrammaticServer,
    CatalogOnly,
}
```

But avoid preserving the current ambiguous `manual_only` name.

### Required test changes

- Replace `manual_only_not_exposed_programmatically` with tests that check explicit fields.
- TUI-visible manual commands should be allowed where intended.
- Interactive-only commands must not be programmatic-visible.
- Frontend server commands should not be TUI-visible unless intentionally represented as lifecycle controls.
- Registry-backed pilot commands must have `registry_backed = true`.
- Legacy/catalog entries must have `registry_backed = false` unless dispatch actually uses the registry.

### Acceptance criteria

- Field names reflect actual semantics.
- No test says manual-only implies not TUI-visible.
- Docs explain which commands are catalog-only vs registry-backed.

## Work item 3: Distinguish registry-backed entries from catalog-only legacy entries

### Problem

`REGISTERED_COMMANDS` includes both true pilot registry entries and many legacy/catalog entries. That is useful for docs and diagnostics, but it can overstate how much dispatch has actually moved to the registry.

Examples:

- Pilot commands: `recon`, `scan-ports`, `scan-endpoints`, `fingerprint`.
- Catalog-only/legacy entries include commands with `operation_id = None`, feature-specific commands, and commands whose descriptor path still lives entirely in legacy handlers.

### Required changes

Add an explicit distinction.

Recommended field:

```rust
pub dispatch_mode: CommandDispatchMode,

pub enum CommandDispatchMode {
    RegistryBacked,
    LegacyWrapped,
    CatalogOnly,
    ServerLifecycle,
    HelperOnly,
}
```

Define expected behavior:

- `RegistryBacked`: descriptor comes from registry and execution path can use registry metadata.
- `LegacyWrapped`: legacy handler remains but registry metadata is used for help/descriptor/preflight where possible.
- `CatalogOnly`: present for help/docs/diagnostics only; no descriptor builder expected.
- `ServerLifecycle`: starts a service surface, not an operation.
- `HelperOnly`: config/report/help-like command.

### Required tests

- Only `RegistryBacked` entries are required to build descriptors.
- `CatalogOnly` entries are not used for enforcement preflight unless they also carry a valid operation descriptor builder.
- Pilot commands are `RegistryBacked`.
- Entries with `operation_id = None` cannot be `RegistryBacked` unless they have a custom descriptor builder.

### Docs updates

Update `docs/COMMAND_REGISTRY.md`:

- explain the dispatch modes;
- list pilot registry-backed commands;
- list remaining catalog/legacy groups;
- state that the registry is incremental.

### Acceptance criteria

- The registry no longer overstates implementation completeness.
- Tests distinguish descriptor-capable entries from docs-only entries.

## Work item 4: Fix duplicate command ID unit test

### Problem

The unit test in `registry.rs` calls `dedup()` without sorting. That only catches adjacent duplicates. The integration test sorts first and is correct, but the unit test should not be misleading.

### Required changes

Either:

- sort before `dedup()`, or
- use a `HashSet`, or
- remove the weaker unit test and rely on the integration test.

Recommended implementation:

```rust
let mut seen = rustc_hash::FxHashSet::default();
for reg in REGISTERED_COMMANDS {
    assert!(seen.insert(reg.command_id), "duplicate command id: {}", reg.command_id);
}
```

### Acceptance criteria

- Duplicate command IDs are caught regardless of ordering.

## Work item 5: Restore Phase 9 plan continuity

### Problem

`plans/architecture-extensibility-phase-09-report-evidence-unification.md` was removed after execution. The project has repeatedly used the `plans/` directory as handoff/audit history. Removing executed plans makes it harder to review what was intended versus what landed.

### Required changes

- Restore `plans/architecture-extensibility-phase-09-report-evidence-unification.md` from git history.
- Add a short status note at the top:

```md
> Status: executed; retained for handoff/audit continuity.
```

- If the team prefers archiving executed plans elsewhere, create a documented `plans/archive/` convention and move old plans consistently. Do not delete individual plans ad hoc.

### Acceptance criteria

- Phase 9 plan exists again or is present in an explicit archive location.
- The plan directory has a clear retention convention.

## Work item 6: Harden feature matrix against Cargo drift

### Problem

`feature_matrix.rs` currently duplicates `crates/eggsec/Cargo.toml` feature keys and dependency edges manually. This is a good start, but it is still a second source of truth.

### Required decision

Choose one of two models.

#### Model A: Parse Cargo.toml in tests

Preferred if adding/using a TOML parser dev-dependency is acceptable.

Required changes:

- Parse `crates/eggsec/Cargo.toml` in `feature_matrix.rs`.
- Extract `[features]` keys and dependency strings.
- Compare metadata feature strings against actual Cargo features.
- Compare `KNOWN_EGGSEC_FEATURES` against actual Cargo features, or remove the manual list entirely.
- Check dependency edges from actual Cargo data rather than copied `FEATURE_DEPENDENCIES`.

#### Model B: Keep static snapshot but make it explicit

Acceptable short term if dependency changes are undesirable.

Required changes:

- Rename `KNOWN_EGGSEC_FEATURES` to `SNAPSHOT_EGGSEC_FEATURES`.
- Document that it is a snapshot of Cargo features.
- Add a test or doc note requiring updates whenever `Cargo.toml` features change.
- Prefer a simple text check that `docs/FEATURE_MATRIX.md` and the snapshot mention all metadata feature strings.

### Recommended approach

Use Model A if the repo already has `toml`, `toml_edit`, or similar in dev dependencies. Otherwise use Model B now and create a follow-up TODO for Cargo parsing.

### Acceptance criteria

- Metadata feature strings are validated against the real Cargo feature list or against an explicitly named snapshot.
- Tests no longer imply the manual list is intrinsically authoritative.

## Work item 7: Clarify `full` feature profile semantics

### Problem

The current feature dependency test expects `full` to include advanced/lab-only domain features. That may be intentional, but it conflicts with the general principle that advanced/lab-only features should not be enabled accidentally by broad/default profiles.

### Required changes

Decide and document one of two models.

#### Model A: `full` means developer/lab everything

If this is the intended behavior:

- Document that `full` is not a conservative user/default profile.
- State that `full` is for development, integration testing, and explicit lab builds.
- Add or document a safer profile such as `standard`, `full-safe`, or `manual-standard` if useful.
- Ensure README and feature matrix do not recommend `full` casually.

#### Model B: Split broad safe profile from lab profile

If `full` should be user-facing:

- Remove advanced/lab-only features from `full`.
- Add a separate `lab-full` or `all-labs` aggregate.
- Preserve backwards compatibility if users may rely on `full` by documenting the change or retaining an alias with warning.

### Recommended approach

Use Model A for this corrective pass unless there is a strong reason to change Cargo semantics now. It is less disruptive. Then consider a future split if needed.

### Required tests/docs

- Update `docs/FEATURE_MATRIX.md` to define `full` explicitly.
- Update tests to assert the chosen model.
- If `full` remains broad, add a test or doc line that it is not default and not authorization.

### Acceptance criteria

- `full` semantics are explicit.
- No documentation suggests `full` is a safe/default production profile unless it is actually narrowed.

## Work item 8: Review tool/profile listing docs after semantic split

After changing tool registration fields, update docs:

- `docs/TOOL_REGISTRATION.md`
- `docs/METADATA_OWNERSHIP.md`
- `docs/CAPABILITY_MATRIX.md`
- `docs/FEATURE_MATRIX.md` if it mentions protocol exposure.

Required wording:

- Metadata exposable: operation may be registered/exposed under feature/profile and policy.
- Default visible: appears in a profile's default tool list.
- Opt-in feature: requires explicit Cargo feature and usually explicit configuration/profile.
- Runtime approved: requires `EnforcementContext::evaluate()` and `ApprovedOperation` where strict.

Acceptance criteria:

- Docs use these terms consistently.
- A high-risk tool with metadata exposure is not described as default safe or default listed unless it actually is.

## Work item 9: Validation and compile checks

Run at minimum:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec-output --test report_envelope
cargo test -p eggsec-tui --lib
```

Feature checks:

```bash
cargo check -p eggsec --features tool-api,rest-api
cargo check -p eggsec --features grpc-api
cargo check -p eggsec --features db-pentest-mcp,tool-api,rest-api
cargo check -p eggsec --features web-proxy-mcp,tool-api,rest-api
cargo check -p eggsec --features c2-mcp,tool-api,rest-api
cargo check -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
```

If any command is platform-sensitive, record the skipped command and required environment.

## Completion criteria

This corrective pass is complete when:

- Tool registration distinguishes metadata-level exposure from default MCP visibility.
- MCP profile listings are explicit and tested.
- Command visibility fields no longer conflate CLI, TUI, interactive-only, and programmatic semantics.
- Registry-backed vs catalog-only command entries are explicit.
- Duplicate command ID tests are order-independent.
- Phase 9 plan continuity is restored.
- Feature matrix validation either parses Cargo features or clearly names the static snapshot model.
- `full` feature semantics are explicit and not presented as conservative/default if it includes advanced/lab-only features.
- Relevant docs are updated with consistent terminology.

## Handoff note

After this pass, the repo should be ready for Phase 11 CI architecture guards. Do not wire CI against the current registry/exposure semantics until this tightening pass lands, otherwise CI may lock in ambiguous field meanings.
