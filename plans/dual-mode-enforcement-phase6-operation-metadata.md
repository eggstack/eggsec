# Phase 6 Handoff Plan: Metadata-Derived Operation Descriptors

## Goal

Replace hand-built and string-classified `OperationDescriptor` construction with canonical operation metadata. Every externally invokable Eggsec operation should have one metadata declaration that drives policy descriptors, protocol exposure, capability/risk declarations, feature gates, and eventually documentation.

This phase should reduce drift between CLI handlers, TUI actions, MCP tools, REST tools, and agent workflows.

## Current context

The dual-mode enforcement work now has a solid caller-origin model:

- `ExecutionSurface` maps caller origin to `ExecutionProfile`.
- CLI, MCP, REST, TUI, and security agent now evaluate shared `EnforcementContext` before dispatch.
- REST was retrofitted in the corrective pass, but it currently uses a local string-based `operation_descriptor_for_rest_tool()` bridge.
- TUI builds descriptors from tab specs and targeted overrides.
- Agent has a local `operation_descriptor_for_agent_scan()` helper.
- MCP builds descriptors for tool calls through MCP-specific helpers.

Those bridges are acceptable as intermediate safety fixes, but the architecture should now converge on one canonical metadata layer.

## Rationale

Without canonical operation metadata, the same operation can accidentally differ by surface:

- REST may classify a tool as `SafeActive` while MCP classifies it as `Intrusive`.
- TUI may omit capabilities that MCP requires.
- Feature-gated operations may be exposed without `required_features` in one protocol.
- Agent descriptors may require explicit scope while REST or TUI forgets.
- Documentation and behavior can drift.

The enforcement layer can only be as good as the descriptor passed into it. This phase makes descriptors deterministic and centrally declared.

## Primary files likely to change

- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/src/tool/mod.rs`
- `crates/eggsec/src/tool/registry.rs` or equivalent registry module
- `crates/eggsec/src/tool/protocol/rest.rs`
- `crates/eggsec/src/tool/protocol/mcp/handlers/server.rs`
- `crates/eggsec/src/tool/protocol/mcp/...` descriptor helper modules
- `crates/eggsec-tui/src/tabs/spec.rs`
- `crates/eggsec-tui/src/app/operation.rs`
- `crates/eggsec/src/agent/enforcement.rs`
- Tests under `crates/eggsec/src/config`, `crates/eggsec/src/tool`, and `crates/eggsec-tui`

## Proposed design

Add a canonical metadata type near `OperationDescriptor` or in a new module such as `crate::tool::metadata`.

Suggested shape:

```rust
#[derive(Debug, Clone, Copy)]
pub struct OperationMetadata {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: OperationMode,
    pub risk: OperationRisk,
    pub intended_uses: &'static [IntendedUse],
    pub required_features: &'static [&'static str],
    pub required_policy_flags: &'static [&'static str],
    pub required_capabilities: &'static [Capability],
    pub target_policy: TargetPolicyKind,
    pub manual_exposable: bool,
    pub tui_exposable: bool,
    pub mcp_exposable: bool,
    pub rest_exposable: bool,
    pub agent_exposable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPolicyKind {
    NoTarget,
    OptionalTarget,
    TargetRequired,
    ExplicitScopeRequired,
    PrivateOrLocalRequired,
}
```

Add descriptor generation:

```rust
impl OperationMetadata {
    pub fn descriptor_for_target(&self, target: Option<String>) -> OperationDescriptor {
        OperationDescriptor {
            operation: self.id.to_string(),
            mode: self.mode,
            risk: self.risk,
            intended_uses: self.intended_uses.to_vec(),
            target,
            required_features: self.required_features.iter().map(|s| s.to_string()).collect(),
            required_policy_flags: self.required_policy_flags.iter().map(|s| s.to_string()).collect(),
            requires_private_or_local_target: matches!(self.target_policy, TargetPolicyKind::PrivateOrLocalRequired),
            requires_explicit_scope: matches!(
                self.target_policy,
                TargetPolicyKind::ExplicitScopeRequired | TargetPolicyKind::PrivateOrLocalRequired
            ),
            required_capabilities: self.required_capabilities.to_vec(),
        }
    }
}
```

Keep this deliberately simple. Do not introduce macros unless the declaration volume becomes painful.

## Step 1: Add metadata registry

Create a registry module, for example:

- `crates/eggsec/src/tool/metadata.rs`

Provide:

```rust
pub fn operation_metadata(id: &str) -> Option<&'static OperationMetadata>;
pub fn all_operation_metadata() -> &'static [OperationMetadata];
pub fn metadata_for_tool_id(tool_id: &str) -> Option<&'static OperationMetadata>;
```

Start with the operations already exposed through the tool registry and major CLI/TUI tabs:

- `recon`
- `scan`
- `scan-ports`
- `scan-endpoints`
- `fingerprint`
- `fuzz`
- `waf-detect`
- `waf-bypass`
- `waf-stress`
- `load`
- `stress`
- `packet`
- `graphql`
- `oauth`
- `auth-test`
- `nse`
- `db-pentest`
- `proxy-*` web proxy tools
- `c2`
- `wireless-*`

Feature-gated operations can be declared unconditionally with `required_features`, or behind cfg if the underlying enum variants/types are feature-gated. Prefer unconditional metadata declarations when enum variants are always available.

## Step 2: Integrate tool registry

When registering a tool, ensure a metadata entry exists.

Add a debug/test validation:

```rust
#[test]
fn every_registered_tool_has_operation_metadata() { ... }
```

If the tool registry does not expose all IDs cheaply, add a helper for test builds.

Do not block feature-gated tools from compiling when features are disabled. Use `cfg` in tests or a feature-aware allowlist.

## Step 3: Replace REST descriptor helper

Replace `operation_descriptor_for_rest_tool()` string matching with metadata lookup:

```rust
let metadata = metadata_for_tool_id(tool_id)
    .ok_or_else(|| EggsecError::Config(format!("missing operation metadata for tool '{}'", tool_id)))?;
let descriptor = metadata.descriptor_for_target(Some(target.to_string()));
```

REST should fail closed when metadata is missing for an executable tool. Listing a tool without metadata should be considered a registry/test failure.

Preserve the corrective-pass behavior that REST dispatches only after enforcement. Also tighten REST behavior if still needed: strict REST should dispatch only on `Allow`; `Warn` should fail closed until a local/manual REST mode exists.

## Step 4: Replace MCP descriptor construction

Find MCP helper code that maps tool IDs to operation/risk/capabilities. Replace with metadata lookup, preserving any MCP-specific profile restrictions.

MCP should fail closed if metadata is missing or if `mcp_exposable == false`.

Expected flow:

1. Validate MCP tool availability/profile.
2. Lookup metadata.
3. Verify `metadata.mcp_exposable`.
4. Build descriptor from metadata and target.
5. Evaluate enforcement.
6. Dispatch only on `Allow`.

## Step 5: Replace TUI descriptor construction where practical

`crates/eggsec-tui/src/tabs/spec.rs` already has tab metadata. Do not duplicate risk/capability declarations there long-term.

Preferred result:

- `TabSpec` includes `operation_id: Option<&'static str>`.
- `App::build_current_operation_descriptor()` looks up `OperationMetadata` by `operation_id` and uses it to build the descriptor.
- Tab-specific overrides remain only for runtime details that metadata cannot know, such as dry-run mode changing risk from `Intrusive` to `SafeActive`.

For dry-run or mode-sensitive tabs, allow a small adjustment layer:

```rust
let mut descriptor = metadata.descriptor_for_target(target);
if tab_is_dry_run { descriptor.risk = OperationRisk::SafeActive; }
```

Keep such overrides explicit and tested.

## Step 6: Replace agent descriptor construction where practical

Agent scan descriptors can use metadata for known scan types such as `recon`, `scan`, `pipeline`, `fuzz`, or `waf-detect`.

For dynamic scan types, use a safe fallback:

- If the scan type maps to known metadata, use metadata.
- If not, classify conservatively based on scan depth and keywords as today, but mark it as a fallback and test it.

Agent fallback should remain strict: `requires_explicit_scope = true` for target-bearing operations.

## Step 7: Add metadata validation tests

Required tests:

- Every registered tool ID has metadata.
- Every metadata ID is unique.
- No metadata has empty `id` or `display_name`.
- Every `agent_exposable` or `mcp_exposable` operation has `requires_explicit_scope` via target policy when target-bearing.
- High-risk metadata declares at least one nonbaseline capability where appropriate.
- Feature-gated operations declare their feature name.
- REST descriptor generation from metadata matches expected risk/capability for representative tools.
- TUI descriptor generation matches metadata for representative tabs.
- MCP descriptor generation matches metadata for representative tools.

## Step 8: Documentation

Update architecture docs:

- `docs/ENFORCEMENT_MODES.md`
- `architecture/overview.md`
- `architecture/tui.md`
- REST/MCP docs if present

Document that operation metadata is the source of truth for descriptor generation and protocol exposure.

## Acceptance criteria

- `OperationMetadata` exists and can generate `OperationDescriptor`.
- REST descriptor construction uses metadata, not string-classification.
- MCP descriptor construction uses metadata, not protocol-local classification.
- TUI descriptor construction uses metadata for normal tabs.
- Agent uses metadata for known scan types and conservative fallback for unknown scan types.
- Missing metadata for an externally executable tool is a test failure or runtime fail-closed error.
- Tests cover metadata uniqueness, registry coverage, exposure flags, and representative descriptor generation.

## Validation commands

Run:

```bash
cargo fmt --all
cargo test -p eggsec --features rest-api --lib
cargo test -p eggsec --features rest-api --test '*'
cargo test -p eggsec-tui
cargo check -p eggsec-cli --features rest-api
cargo check -p eggsec-tui
```

If feature names differ, use the feature set that exposes MCP, REST, db-pentest, web-proxy, and relevant TUI tabs.

## Non-goals

- Do not implement type-level `ApprovedOperation` yet.
- Do not extract domain crates yet.
- Do not change manual CLI/TUI posture.
- Do not add a local/manual REST mode.
- Do not require perfect metadata for purely internal helper functions that are not externally invokable.
