# Eggsec Extensibility Guide

Contributor-facing reference for adding operations, domains, commands, tools, TUI actions,
report output, and features to the Eggsec workspace.

## Core Invariants

1. **Metadata-first**: Every security action starts with `OperationMetadata` in
   `crates/eggsec/src/config/policy.rs`. No command, tool, or domain integration
   may be added without a canonical metadata entry.

2. **Domains don't authorize**: `DomainDescriptor` in
   `crates/eggsec/src/domain/mod.rs` groups operations under a domain umbrella
   (CLI/TUI/MCP/report integrations, feature gates, dry-run/evidence support).
   It never performs authorization or network I/O.

3. **Strict dispatch uses `ApprovedOperation`**: REST, MCP, agent, and gRPC
   surfaces require an `ApprovedOperation` token produced exclusively by
   `EnforcementContext::approve()` before dispatch via
   `EnforcedDispatcher::dispatch_checked()`. No raw dispatch fallback exists.

4. **Listing is not authorization**: `ToolRegistration` and
   `mcp_tool_registrations_default_visible()` control protocol exposure.
   A tool being listed does not mean it is safe to execute. Strict surfaces
   must evaluate policy at runtime regardless of listing state.

## Extension Decision Tree

```text
Do you need a new security action?
  -> Add OperationMetadata first.
  -> Decide whether it belongs to an existing domain or a new DomainDescriptor.
  -> Add command/tool/TUI/report integration only after metadata exists.

Do you need a new protocol-exposed tool?
  -> Add/verify OperationMetadata.
  -> Add ToolRegistration or domain ToolIntegration.
  -> Ensure strict dispatch uses ApprovedOperation.

Do you need a new manual CLI command?
  -> Add CommandRegistration.
  -> Choose RegistryBacked, LegacyWrapped, HelperOnly, ServerLifecycle, or CatalogOnly.
  -> Wire preflight/enforcement appropriately.
```

## File Ownership Map

| Extension Type | Primary Files | Metadata | Tests |
|---------------|--------------|----------|-------|
| Operation | `crates/eggsec/src/config/policy.rs` | `OperationMetadata` | `metadata_consistency`, `feature_matrix` |
| Domain | `crates/eggsec/src/domain/mod.rs`, domain crate | `DomainDescriptor` | `metadata_consistency`, `tool_registration`, `feature_matrix` |
| Command | `crates/eggsec/src/commands/registry.rs`, handler | `CommandRegistration` | `command_registry`, `enforcement_matrix` |
| Tool exposure | `crates/eggsec/src/tool/registration.rs` | `ToolRegistration` | `tool_registration`, `enforced_dispatch_regression` |
| TUI action | `crates/eggsec-tui/src/app/action_spec.rs` | `TuiActionSpec` | `eggsec-tui --lib` |
| Report output | `crates/eggsec-output/src/envelope.rs` | `ReportEnvelope` | `report_envelope` |
| Feature | `crates/eggsec/Cargo.toml`, `tests/feature_matrix.rs` | Feature string | `feature_matrix`, `cargo check --features ...` |

## Required Local Checks

Run the authoritative Make target before opening a PR. CI will reject
changes that fail any of these.

```bash
make check-architecture-ci
make check-feature-profiles   # if feature-gated code changed
```

`make check-architecture-ci` reproduces the full architecture guard CI
job locally: formatting, no-default build, lib tests, metadata consistency,
command registry, tool registration, feature matrix, enforcement matrix,
enforced dispatch regression, report envelope, and static drift guards.

For the expanded command list and per-extension test mapping, see
[`docs/extending/testing.md`](docs/extending/testing.md) and
[`docs/CI_ARCHITECTURE_GUARDS.md`](docs/CI_ARCHITECTURE_GUARDS.md).

## Detailed Guides

Each extension type has a dedicated guide in `docs/extending/`:

| Topic | Guide |
|-------|-------|
| Adding an operation | [`docs/extending/operations.md`](docs/extending/operations.md) |
| Adding a domain | [`docs/extending/domains.md`](docs/extending/domains.md) |
| Adding a CLI command | [`docs/extending/commands.md`](docs/extending/commands.md) |
| Adding a protocol-exposed tool | [`docs/extending/tool-exposure.md`](docs/extending/tool-exposure.md) |
| Adding a TUI action | [`docs/extending/tui-actions.md`](docs/extending/tui-actions.md) |
| Adding report output | [`docs/extending/report-evidence.md`](docs/extending/report-evidence.md) |
| Adding a feature flag | [`docs/extending/features.md`](docs/extending/features.md) |
| Testing and pre-handoff checks | [`docs/extending/testing.md`](docs/extending/testing.md) |
| Copyable templates | [`docs/extending/templates.md`](docs/extending/templates.md) |
| Enforcement and dispatch | [`docs/ENFORCEMENT_MODES.md`](docs/ENFORCEMENT_MODES.md) |
| Metadata ownership | [`docs/METADATA_OWNERSHIP.md`](docs/METADATA_OWNERSHIP.md) |
| Capability matrix | [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) |
| Tool registration | [`docs/TOOL_REGISTRATION.md`](docs/TOOL_REGISTRATION.md) |
| Command registry | [`docs/COMMAND_REGISTRY.md`](docs/COMMAND_REGISTRY.md) |
| Report/evidence model | [`docs/REPORT_EVIDENCE_MODEL.md`](docs/REPORT_EVIDENCE_MODEL.md) |
