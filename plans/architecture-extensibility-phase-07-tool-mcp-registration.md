# Architecture Extensibility Phase 7: Tool and MCP Registration Modernization

## Objective

Modernize tool, MCP, REST, and gRPC registration so programmatic exposure is driven by canonical metadata rather than scattered manual registration code. The goal is to reduce drift between `OperationMetadata`, `DomainDescriptor`, `ToolIntegration`, protocol registration, and capability documentation while preserving strict fail-closed enforcement for all programmatic surfaces.

This phase follows Phase 6's command registry work. It should not depend on every CLI command being registry-backed, but it should use the corrected metadata model from earlier phases.

## Current context

Eggsec has multiple related concepts:

- `OperationMetadata`: canonical operation risk/capability/exposure flags.
- `DomainDescriptor`: domain grouping and integration metadata.
- `ToolIntegration`: per-domain tool metadata, including MCP default exposure and opt-in feature requirements.
- `ToolDispatcher`: raw internal dispatch implementation.
- `EnforcedDispatcher`: strict wrapper requiring `ApprovedOperation`.
- MCP/REST/gRPC registration and filtering logic.

The previous cleanup adopted Model A for programmatic exposure: broad exposure flags indicate metadata-level permission to register/call when compiled, scoped, and policy-authorized. These flags do not imply default safe execution.

This phase should make that model explicit in code.

## Non-goals

- Do not make high-risk tools execute by default.
- Do not weaken `EnforcedDispatcher`.
- Do not remove strict profile checks.
- Do not add new hazardous capabilities.
- Do not move authorization into protocol adapters.
- Do not require every domain to have MCP exposure.

## Design target

Create a protocol-neutral programmatic registration model that can drive MCP, REST, gRPC, and agent-visible tool lists.

Suggested type shape:

```rust
pub struct ToolRegistration {
    pub tool_id: &'static str,
    pub operation_id: &'static str,
    pub display_name: &'static str,
    pub source: ToolRegistrationSource,
    pub feature: Option<&'static str>,
    pub required_mcp_feature: Option<&'static str>,
    pub mcp_exposed_by_default: bool,
    pub rest_exposable: bool,
    pub grpc_exposable: bool,
    pub agent_exposable: bool,
    pub category: ToolCategory,
}
```

The exact type shape can differ, but the model must distinguish:

- tool implementation exists;
- operation metadata permits surface exposure;
- domain integration permits MCP exposure by default;
- opt-in MCP feature is required;
- runtime enforcement approves a specific target/request.

## Work item 1: Inventory current tool/protocol registration

Document every place tools are registered or filtered.

Inspect at minimum:

- `crates/eggsec/src/tool/mod.rs`
- `crates/eggsec/src/tool/dispatcher.rs`
- MCP server handlers and tool listing code
- REST tool execution/listing/preflight code
- gRPC execute/listing code
- agent scan execution paths
- feature-gated tool registrations for db-pentest, web-proxy, C2, search, pipeline, etc.

Deliverable:

- Add or update `docs/TOOL_REGISTRATION.md` with:
  - current registration sources;
  - strict dispatch path;
  - raw dispatch exceptions;
  - feature-gated tools;
  - default-exposed vs opt-in MCP tools;
  - known legacy/manual registrations.

Acceptance criteria:

- Maintainers can see where a tool appears in MCP/REST/gRPC/agent lists.
- The doc distinguishes registration from runtime authorization.

## Work item 2: Add canonical tool registration builder

Implement a function that derives registration rows from `OperationMetadata` and `DomainDescriptor`.

Suggested functions:

```rust
pub fn all_tool_registrations() -> Vec<ToolRegistration>;
pub fn mcp_tool_registrations(profile: McpProfilePolicy) -> Vec<ToolRegistration>;
pub fn rest_tool_registrations() -> Vec<ToolRegistration>;
pub fn agent_tool_registrations() -> Vec<ToolRegistration>;
```

This can initially coexist with manual registration. Do not remove `create_default_registry()` immediately unless the migration is straightforward and fully tested.

Required behavior:

- Resolve operation metadata by operation/tool ID.
- Include feature gate metadata.
- Include domain `ToolIntegration` where present.
- Preserve opt-in MCP semantics for db-pentest, web-proxy, C2, and any other hazardous/defense-lab tools.
- Mark default MCP exposure separately from metadata-level `mcp_exposable`.

Acceptance criteria:

- Registration builder compiles with no default features.
- It can enumerate known tool metadata without side effects.
- It does not require optional domain crates to be linked unless needed for actual execution.

## Work item 3: Use registration builder for listing/preflight, not execution first

Start by using metadata-driven registration for read-only surfaces:

- MCP `tools/list`
- REST tool listing endpoint
- gRPC tool listing, if present
- preflight metadata responses
- docs/debug output if present

Do not immediately route execution through the new registration unless tests are strong. Execution should still use existing `EnforcedDispatcher::dispatch_checked()`.

Acceptance criteria:

- Tool lists now derive from canonical registration metadata.
- Execution behavior remains unchanged.
- Feature-gated and opt-in tools appear or are hidden according to the chosen model.

## Work item 4: Preserve strict runtime enforcement

Review every programmatic execution path and ensure it still follows:

1. parse request;
2. resolve operation/tool metadata;
3. build `OperationDescriptor`;
4. evaluate through `EnforcementContext`;
5. require `ApprovedOperation`;
6. call `EnforcedDispatcher::dispatch_checked()`.

Required tests:

- MCP strict denies high-risk operation under default policy.
- REST strict denies high-risk operation under default policy.
- Agent strict denies high-risk operation under default policy.
- Opt-in MCP tools are not listed as default-exposed without their feature/profile requirements.
- Tool name mismatch with `ApprovedOperation` remains rejected.

Acceptance criteria:

- Programmatic registration cannot bypass approval-token checks.
- Existing enforced dispatch regression tests still pass.

## Work item 5: Metadata consistency tests for tool registration

Add tests:

- every tool registration resolves to `OperationMetadata`;
- default MCP-exposed tools have `mcp_exposable = true`;
- opt-in MCP tools have `mcp_exposed_by_default = false`;
- hazardous domains are never default MCP-exposed;
- high-risk agent-exposable operations are denied by default `AgentStrict` policy unless explicitly allowed;
- feature-gated registrations declare non-empty features.

Prefer a dedicated `crates/eggsec/tests/tool_registration.rs` if the test file grows too large.

Acceptance criteria:

- Drift between metadata and protocol lists fails tests.
- High-risk exposure semantics remain documented and enforced.

## Work item 6: Move manual registration comments into metadata

Where current registration code has comments explaining feature gates or safety posture, move durable facts into metadata docs or `docs/TOOL_REGISTRATION.md`.

Examples:

- db-pentest MCP is opt-in;
- C2 MCP is opt-in;
- web-proxy MCP is opt-in;
- mobile dynamic has no programmatic exposure;
- raw packet/stress tools require strict policy approval.

Acceptance criteria:

- Code comments are shorter and point to metadata ownership docs.
- Safety posture is not duplicated inconsistently across files.

## Work item 7: Optional execution migration for one safe tool

If the listing/preflight migration is clean, migrate execution lookup for one safe tool through the canonical registration path while still using `EnforcedDispatcher`.

Recommended tool:

- `recon` or `search`.

Do not migrate high-risk tools in this phase unless necessary.

Acceptance criteria:

- At least one safe tool can prove the registration-to-execution bridge.
- No behavior change for denied strict operations.

## Safety requirements

- Metadata-driven registration is not authorization.
- Default MCP exposure must remain conservative.
- Programmatic execution must require `ApprovedOperation`.
- Agent paths must not accept manual override flags.
- Optional features must not become runtime authorization.

## Files likely to change

- `crates/eggsec/src/tool/mod.rs`
- `crates/eggsec/src/tool/dispatcher.rs`
- `crates/eggsec/src/tool/protocol/mcp/**`
- `crates/eggsec/src/tool/protocol/rest.rs`
- `crates/eggsec/src/tool/protocol/grpc.rs`
- `crates/eggsec/src/agent/**`
- `crates/eggsec/src/domain/mod.rs`
- `crates/eggsec/src/config/policy.rs`
- `crates/eggsec/tests/metadata_consistency.rs`
- optionally `crates/eggsec/tests/tool_registration.rs`
- `docs/TOOL_REGISTRATION.md`
- `docs/METADATA_OWNERSHIP.md`
- `docs/CAPABILITY_MATRIX.md`

## Validation commands

Run:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo test -p eggsec --lib
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec --test enforcement_matrix
```

Feature checks:

```bash
cargo check -p eggsec --features rest-api,tool-api
cargo check -p eggsec --features db-pentest-mcp,rest-api,tool-api
cargo check -p eggsec --features web-proxy-mcp,rest-api,tool-api
cargo check -p eggsec --features c2-mcp,rest-api,tool-api
```

If a new `tool_registration` test exists:

```bash
cargo test -p eggsec --test tool_registration
cargo test -p eggsec --features db-pentest-mcp --test tool_registration
cargo test -p eggsec --features web-proxy-mcp --test tool_registration
cargo test -p eggsec --features c2-mcp --test tool_registration
```

## Completion criteria

Phase 7 is complete when:

- Programmatic tool listings derive from canonical registration metadata.
- Opt-in MCP semantics are preserved.
- High-risk exposure flags are clearly distinguished from default runtime approval.
- Strict execution paths still require `ApprovedOperation`.
- Tool registration drift is covered by tests.

## Handoff note

This phase should make future MCP/API additions much safer. After it lands, new tools should be added by updating `OperationMetadata`, `DomainDescriptor`/`ToolIntegration`, and the relevant execution adapter, with tests catching missing or inconsistent registration state.
