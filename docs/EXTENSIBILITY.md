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

Run every check below before opening a PR. CI will reject changes that fail
any of these.

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
bash scripts/check-architecture-guards.sh
make check-architecture-ci
```

Feature-specific checks (run only if your change touches a feature-gated module):

```bash
# Example: adding a db-pentest feature
cargo check -p eggsec --features db-pentest
cargo test -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest
```

## Detailed Guides

Each extension type has a dedicated guide in `docs/extending/`:

| Topic | Guide |
|-------|-------|
| Adding an operation | `docs/extending/adding-operation.md` |
| Adding a domain | `docs/extending/adding-domain.md` |
| Adding a CLI command | `docs/extending/adding-command.md` |
| Adding a protocol-exposed tool | `docs/extending/adding-tool.md` |
| Adding a TUI action | `docs/extending/adding-tui-action.md` |
| Adding report output | `docs/extending/adding-report.md` |
| Adding a feature flag | `docs/extending/adding-feature.md` |
| Enforcement and dispatch | `docs/ENFORCEMENT_MODES.md` |
| Metadata ownership | `docs/METADATA_OWNERSHIP.md` |
| Capability matrix | `docs/CAPABILITY_MATRIX.md` |
| Tool registration | `docs/TOOL_REGISTRATION.md` |
| Command registry | `docs/COMMAND_REGISTRY.md` |
| Report/evidence model | `docs/REPORT_EVIDENCE_MODEL.md` |
