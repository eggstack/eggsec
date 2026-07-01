# Templates and Checklists

Compact templates for each extension type.

---

## 1. New Operation Metadata

```rust
// In crates/eggsec/src/config/policy.rs
// Add to OPERATION_METADATA array
OperationMetadata {
    id: "your-operation-id",        // kebab-case
    display_name: "Your Operation",
    aliases: &["alt-name"],         // alternate tool IDs that resolve to this
    mode: OperationMode::Active,    // Active or Passive
    risk: OperationRisk::SafeActive,// See operations.md for variants
    required_capabilities: &[Capability::ActiveProbe], // what the op needs
    required_features: &["your-feature"],              // Cargo features
    manual_exposable: true,     // CLI/TUI operator use
    tui_exposable: true,        // TUI tab/action
    mcp_exposable: false,       // MCP protocol listing
    rest_exposable: false,      // REST API listing
    grpc_exposable: false,      // gRPC listing
    agent_exposable: false,     // Agent dispatch
}
```

Verify after adding:

```bash
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test feature_matrix
```

---

## 2. New Domain Descriptor

```rust
// In crates/eggsec/src/domain/mod.rs
// Add to DOMAIN_DESCRIPTORS array
DomainDescriptor {
    id: "your-domain",
    display_name: "Your Domain",
    description: "Brief description of what this domain covers.",
    required_feature: Some("your-feature"), // None for always-available
    operations: &["your-operation-id"],     // must exist in OperationMetadata
    cli_integration: CliIntegration { /* ... */ },
    tui_integration: TuiIntegration { /* ... */ },
    mcp_integration: McpIntegration { /* ... */ },
    report_integration: ReportIntegration { /* ... */ },
    tool_integration: Some(ToolIntegration { /* ... */ }),
}
```

Verify after adding:

```bash
cargo test -p eggsec --test metadata_consistency
```

---

## 3. New Command Registration

```rust
// In crates/eggsec/src/commands/registry.rs
// Add to REGISTERED_COMMANDS array
CommandRegistration {
    command_id: "your-command",
    operation_id: "your-operation-id",  // must exist in OperationMetadata
    category: CommandCategory::SideEffectingNetwork,
    feature_gate: Some("your-feature"),
    cli_visible: true,
    tui_visible: true,
    programmatic_visible: false,
    cli_interactive_only: false,
    registry_backed: true,
    dispatch_mode: CommandDispatchMode::RegistryBacked,
}
```

Verify after adding:

```bash
cargo test -p eggsec --test command_registry
```

---

## 4. New Tool Exposure Decision

Decision tree:

1. Does it have `OperationMetadata`? Add/verify first.
2. Is it domain-specific? Use domain `ToolIntegration`.
3. Is it a base tool? Use `ToolRegistration` directly.
4. Set `mcp_metadata_exposable` based on MCP need.
5. Set `mcp_default_visible` only for conservative-default tools.
6. REST/gRPC/agent exposure requires separate flags.

---

## 5. New TUI Action

Steps:

1. Declare `Tab` variant in `Tab` enum.
2. Create `TabSpec` with task configuration.
3. Create `TuiActionSpec` pointing to `OperationMetadata`.
4. Use `EnforcementFacade` for preflight and dispatch.
5. Do not duplicate risk/capability/scope semantics.

---

## 6. New Report/Evidence Output

Steps:

1. Define domain-specific finding types in your domain crate.
2. Implement `to_report_envelope()` conversion.
3. Use `ReportEnvelope`, `FindingRecord`, `EvidenceItem` from `eggsec-output`.
4. Classify evidence with `RedactionState`.
5. Add to `EvidenceManifest`.

---

## 7. New Cargo Feature

Steps:

1. Add to `crates/eggsec/Cargo.toml` `[features]` section.
2. Add to `tests/feature_matrix.rs` `classify_feature()`.
3. Add dependency edges to `FEATURE_DEPENDENCIES` if needed.
4. Update `KNOWN_EGGSEC_FEATURES`.
5. Run: `cargo test -p eggsec --test feature_matrix`

---

## 8. PR Checklist

```markdown
## PR Checklist

- [ ] Code compiles: `cargo check --workspace --no-default-features`
- [ ] Formatting: `cargo fmt --all --check`
- [ ] Unit tests: `cargo test -p eggsec --lib`
- [ ] Metadata consistency: `cargo test -p eggsec --test metadata_consistency`
- [ ] Command registry: `cargo test -p eggsec --test command_registry`
- [ ] Feature matrix: `cargo test -p eggsec --test feature_matrix`
- [ ] Architecture guards: `bash scripts/check-architecture-guards.sh`
- [ ] Full CI reproduction: `make check-architecture-ci`
```
