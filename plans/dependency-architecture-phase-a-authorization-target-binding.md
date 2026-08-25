# Phase A Plan: Authorization Token and Target-Binding Correction

## Status

Status: Executed.

Executed. Changes landed in main.

Phase A is fully implemented. `OperationMetadata::try_descriptor_for_target()`
makes construction fallible and target-policy-aware. `ApprovedOperation` binds
normalized target identity. `validate_request_binding()` prevents dispatch token
reuse across targets. Surface/profile mismatches are rejected before approval.
Seven regression tests in `enforced_dispatch_regression.rs` cover target
mismatch, missing target, smuggled target, conflicting targets, and surface
profile mismatch.

## Objective

Make an approval token a complete, self-contained proof that one exact operation,
for one exact target identity where applicable, was authorized under one exact
execution surface and policy context.

The phase closes the current gap in which target-required operation metadata can
produce a descriptor with `target = None`, strict approval may proceed without
scope evaluation, and `dispatch_checked()` skips target comparison when the
approved descriptor has no target.

## Why this phase is first

This is a correctness and authorization issue, not a metadata-cleanup preference.
Later phases will consolidate operation declarations and move frontend adapters.
Those changes must build on a sound approval model rather than preserving a
semantically incomplete token behind cleaner APIs.

## Scope

Primary implementation areas:

```text
crates/eggsec/src/config/policy.rs
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/tool/dispatcher.rs
crates/eggsec/src/tool/metadata.rs
crates/eggsec/src/runtime_bridge/
crates/eggsec/src/tool/protocol/rest.rs
crates/eggsec/src/tool/protocol/grpc.rs
crates/eggsec/src/tool/protocol/mcp/
crates/eggsec/src/agent/
crates/eggsec/src/commands/
crates/eggsec-python/src/
crates/eggsec/tests/
crates/eggsec-python/tests/
```

The exact file list must be confirmed before editing. Do not assume every surface
constructs descriptors through the same helper.

## Non-goals

This phase does not:

- redesign the entire metadata registry;
- change which operations are allowed by policy;
- add new target normalization libraries unless current `url`/IP primitives are
  insufficient;
- change DNS address-set policy beyond what is necessary to bind the original
  target identity; full DNS/address semantics belong to Phase B;
- remove legacy dispatch surfaces solely because they are awkward;
- add new evidence or architecture-guard frameworks;
- alter manual release behavior.

## Required design decisions

### 1. Operation descriptors must be valid by construction

Replace unrestricted public-field construction for authorization-relevant
fields with validated constructors or builders. At minimum, construction must
apply `TargetPolicyKind`:

- `NoTarget`: reject a supplied target unless the operation explicitly permits
  an ignored/display-only argument;
- `OptionalTarget`: accept `None` or a validated target;
- `TargetRequired`: reject `None` and empty targets;
- `ExplicitScopeRequired`: reject `None` and mark the operation as requiring an
  explicit scope decision;
- `PrivateOrLocalRequired`: reject `None` and retain the stronger target-policy
  requirement for enforcement.

A constructor should return a typed error containing operation ID, target-policy
kind, and the reason construction failed.

### 2. Approval must validate surface/profile coherence

`EnforcementContext::approve()` and `approve_manual()` currently receive a
surface separately from the context profile. Before issuing an approval token,
verify that:

```text
surface.profile() == enforcement_context.execution_profile
```

Reject mismatches through a dedicated error variant. Do not silently normalize
or trust the caller-provided surface.

### 3. The token must bind normalized operation and target identity

`ApprovedOperation` should contain or expose an immutable authorization binding
with at least:

```text
canonical operation ID
canonical/normalized target identity, if target-bearing
original target for audit display
execution surface
execution profile
policy decision identifier/hash as currently supported
```

The normalized target representation must be deterministic for the same logical
input. It may initially bind normalized host/URL/IP identity without binding DNS
addresses; Phase B will extend address-set semantics.

Avoid storing only free-form strings when a small enum/struct can distinguish:

```text
URL target
hostname/domain target
IP target
CIDR target
local file/resource target
no target
```

Do not broaden the phase into a universal resource URI system.

### 4. Dispatch must fail closed on missing or mismatched binding

`EnforcedDispatcher::dispatch_checked()` must validate:

- request tool resolves to the approved canonical operation;
- the request contains a target whenever the approved operation requires one;
- the request target normalizes to the same identity as the approved target;
- no request target is accepted when the approved operation is targetless unless
  the operation's policy explicitly permits an auxiliary target field;
- request parameters cannot supply a second conflicting target;
- aliases resolve before comparison but do not weaken target checks.

Where `request.target` and `params["target"]` both exist, define one canonical
source and reject disagreement. Do not accept equality with either of two
conflicting fields.

### 5. Entrypoints must consume the same fallible construction API

Migrate strict surfaces first:

1. REST;
2. MCP;
3. gRPC;
4. agent;
5. runtime/daemon bridge;
6. Python async/sync dispatch paths.

Then migrate manual CLI/TUI preflight and dispatch helpers. Transitional wrappers
may remain briefly, but they must call the validated constructor and must not
construct an `OperationDescriptor` by public-field literal.

## Workstream 1 — Inventory all descriptor construction paths

Search for:

```bash
rg -n 'OperationDescriptor\s*\{|descriptor_for_target|approve\(|approve_manual\(' crates
```

Produce a short implementation note in the commit or plan status update listing:

- each construction helper;
- each literal construction site;
- whether the target originates in CLI args, request DTOs, runtime task DTOs, or
  metadata;
- whether target validation currently occurs before or after descriptor creation;
- whether the path is manual or strict.

This inventory is implementation guidance, not a new generated artifact.

## Workstream 2 — Introduce validated target and descriptor construction

Implement the smallest set of types needed to enforce target-policy validity.
Preferred shape:

```rust
pub enum OperationTarget {
    None,
    Url(NormalizedUrlTarget),
    Host(NormalizedHostTarget),
    Ip(IpAddr),
    Cidr(IpNetwork),
    Resource(String),
}

impl OperationMetadata {
    pub fn try_descriptor_for_target(
        &self,
        target: Option<&str>,
        hint: Option<TargetKind>,
    ) -> Result<OperationDescriptor, DescriptorError>;
}
```

The exact public API may differ. Required properties are fallibility, target-policy
validation, deterministic normalization, and no public mutation of authorization
fields after construction.

Keep serialization compatibility where required by public APIs. If existing
serialized descriptor fields are public contract, use custom serialization or a
compatibility DTO rather than retaining unrestricted constructors.

## Workstream 3 — Harden approval issuance

Add checks before evaluating or tokenizing:

- descriptor belongs to known canonical metadata or an explicitly supported
  internal operation class;
- descriptor target policy is satisfied;
- context profile matches surface profile;
- target-bearing strict operations have the scope provenance required by policy;
- approval decision references the exact descriptor being stored.

A descriptor rejected as structurally invalid should not be converted into a
normal policy denial. Return a distinct validation/configuration error so callers
can differentiate malformed invocation from an authorized-but-denied operation.

## Workstream 4 — Harden dispatch binding

Refactor `dispatch_checked()` around one comparison function, for example:

```rust
fn validate_request_binding(
    approval: &ApprovedOperation,
    request: &ToolRequest,
) -> Result<(), DispatchBindingError>
```

This helper should be unit-testable without executing a real tool.

Required rejection cases:

- target-required approval attempted with no target;
- request target differs only after normalization in a way that changes host,
  scheme policy, port, path policy, IP, or CIDR identity;
- request target is empty;
- `params.target` conflicts with the typed request target;
- alias points to a different canonical operation;
- approval surface/profile mismatch;
- targetless operation receives a target that would alter execution scope;
- a token approved for one target is reused for another request.

## Workstream 5 — Preserve manual and strict behavior distinctions

Manual override semantics must remain unchanged. This phase must not make manual
CLI/TUI use stricter than the documented policy except for malformed or
mismatched authorization bindings.

Strict surfaces must continue to reject `Warn` and `RequireConfirmation`
outcomes. Manual surfaces may continue to approve warnings and explicitly
permitted confirmation classes.

The constructor and dispatch-binding checks apply equally to manual and strict
surfaces; only policy outcome handling differs.

## Workstream 6 — Python and runtime compatibility

Confirm that Python-facing operation construction and runtime bridge DTO
conversion cannot bypass the validated constructor.

For Python:

- sync and async engines must produce the same structural validation result;
- malformed target errors should map to stable Python exception/result kinds;
- no Python method should accept `None` for a target-required operation and then
  execute against a target embedded in params;
- stubs/docs must be updated only when public signatures or exception behavior
  change.

For runtime/daemon:

- `TaskKind` to operation resolution must supply the target before approval;
- the approved request bundle must retain the validated target binding;
- session-surface normalization must precede approval;
- request DTO informational surface fields must not override session surface.

## Tests

Add direct semantic tests rather than new grep guards.

Minimum Rust regression coverage:

1. every `TargetRequired`, `ExplicitScopeRequired`, and
   `PrivateOrLocalRequired` metadata entry rejects `None`;
2. `NoTarget` behavior is explicit and tested;
3. surface/profile mismatch cannot produce `ApprovedOperation`;
4. approved target A cannot dispatch request target B;
5. approved alias/canonical operation matching succeeds only for the same
   canonical operation;
6. conflicting typed and parameter targets are rejected;
7. strict approval without explicit scope fails for target-bearing operations;
8. a targetless operation cannot smuggle a network target through params;
9. manual warning/override behavior remains unchanged for valid descriptors;
10. runtime bridge and at least one REST/MCP path use the validated API.

Minimum Python regression coverage when applicable:

- sync and async target-validation parity;
- stable error classification for missing required targets;
- no execution occurs after structural validation failure.

## Validation commands

Use only the direct commands needed for this phase:

```bash
cargo fmt --all -- --check
cargo test -p eggsec --lib
cargo test -p eggsec --test enforced_dispatch_regression --features rest-api
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test tool_registration --features rest-api
cargo check -p eggsec --features rest-api,grpc-api
make check-python   # only when Python-facing code changed
```

Do not require `make check-full` unless the implementation changes broad feature
metadata or optional-domain construction paths.

## Migration and compatibility strategy

- Introduce the validated constructor and binding tests first.
- Migrate strict surfaces before making old constructors private.
- Migrate manual and Python/runtime paths.
- Remove or restrict literal/public construction only after all in-tree callers
  compile.
- Retain serialized field names where they are part of a public protocol.
- Add temporary deprecated wrappers only when external Rust users need a
  transition; wrappers must fail closed and call the new constructor.

## Rollback considerations

If target normalization causes unexpected compatibility failures, revert only
the normalization representation while retaining the structural requirements:
required targets, surface/profile validation, and exact request-to-approval
binding must not be rolled back.

## Acceptance criteria

Phase A is complete only when:

1. authorization-relevant `OperationDescriptor` fields cannot be freely mutated
   after validated construction;
2. target-required policies reject missing and empty targets;
3. strict approval rejects a surface/profile mismatch;
4. `ApprovedOperation` stores a canonical operation binding;
5. target-bearing approvals store a normalized target binding;
6. `dispatch_checked()` requires exact binding equality;
7. conflicting request target representations are rejected;
8. no strict surface constructs a descriptor through an unchecked literal;
9. runtime and Python paths cannot bypass construction validation;
10. manual override semantics remain unchanged for valid requests;
11. direct bypass regression tests fail on the pre-phase implementation and pass
    after the correction;
12. no new architecture grep guard is introduced for behavior enforced by types
    and tests;
13. no package or release is published.
