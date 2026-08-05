# Phase D Plan: Operation, Command, Domain, and Tool Metadata Consolidation

## Status

Ready for implementation after Phase C.

## Objective

Establish one canonical declaration for every externally invokable operation and
derive command, domain, tool, protocol-exposure, alias, feature, risk, and target
views from that declaration wherever practical.

This phase must remove contradictions among `OperationMetadata`,
`CommandRegistration`, `DomainDescriptor`, tool registration, frontend visibility,
and protocol exposure rather than adding more consistency tests between separate
hand-maintained tables.

## Preconditions

- Phase A provides validated target-policy-aware operation construction.
- Phase B establishes target/address semantics used by metadata target policy.
- Phase C provides one exhaustive feature registry.

Do not begin broad registry removal until those primitives are stable.

## Scope

Primary areas:

```text
crates/eggsec/src/config/policy.rs
crates/eggsec/src/commands/registry.rs
crates/eggsec/src/domain/mod.rs
crates/eggsec/src/tool/metadata.rs
crates/eggsec/src/tool/registration.rs
crates/eggsec/src/tool/protocol/
crates/eggsec/src/runtime_bridge/
crates/eggsec/src/dispatch/
crates/eggsec/src/cli/
crates/eggsec-tui/src/
crates/eggsec-python/src/operation_registry.rs
crates/eggsec/tests/metadata_consistency.rs
crates/eggsec/tests/command_registry.rs
crates/eggsec/tests/tool_registration.rs
crates/eggsec/tests/feature_matrix.rs
docs/COMMAND_REGISTRY.md
docs/TOOL_REGISTRATION.md
docs/METADATA_OWNERSHIP.md
docs/FEATURE_MATRIX.md
docs/EXTENSIBILITY.md
docs/extending/
```

Confirm actual paths before editing. Some tool metadata may be re-exported or
split across submodules.

## Non-goals

This phase does not:

- rewrite all command handlers;
- replace Clap;
- change user-facing command names without a compatibility requirement;
- expose hazardous operations to automated surfaces;
- remove domain crates;
- add runtime reflection or dynamic registration;
- require procedural macros if a normal declarative macro/static table is
  sufficient;
- preserve transitional registries merely because documentation currently names
  them.

## Canonical declaration requirements

Each operation declaration must own or reference exactly one value for:

```text
canonical operation ID
display name
operation mode
risk tier
intended uses
required capabilities
target policy
required engine features
aliases
manual CLI exposure
TUI exposure
MCP metadata exposure
MCP default visibility
REST exposure
gRPC exposure
agent exposure
runtime TaskKind mapping, where applicable
domain membership, where applicable
report kind/evidence support, where applicable
```

Not every field must live in one enormous struct. A canonical operation record
may reference domain or adapter records, but there must be one ownership path
and no contradictory duplicate values.

Keep the important distinction between:

- metadata-exposable through a protocol;
- visible by default in a specific protocol/profile;
- enabled by the required Cargo feature;
- allowed by runtime policy and scope.

Exposure metadata never authorizes execution.

## Workstream 1 — Build a contradiction inventory

Before refactoring, compare the current registries and document confirmed
conflicts in the implementation PR. Check at least:

- operation mode and risk;
- target policy;
- required feature names;
- CLI command ID versus operation ID;
- alias mappings;
- TUI visibility;
- MCP/REST/gRPC/agent exposure;
- domain category and strict-surface support;
- report/evidence support;
- command categories and dispatch modes.

Resolve each conflict by referencing actual behavior and documented safety
intent. Do not choose whichever value is most permissive.

Known areas requiring explicit review include high-risk domains such as database
assessment, traffic interception, C2, mobile dynamic analysis, stress/raw packet
operations, and remote execution.

## Workstream 2 — Define the canonical operation catalog

Implement a catalog that is static, inspectable, and usable at compile time. A
macro is acceptable when it produces normal Rust constants and clear compiler
errors. Example conceptual shape:

```rust
operation_catalog! {
    ScanPorts {
        id: "scan-ports",
        aliases: ["scan", "scan_ports"],
        mode: StandardAssessment,
        risk: SafeActive,
        target: ExplicitScopeRequired,
        features: [],
        capabilities: [ActiveProbe],
        domain: Scanner,
        surfaces: {
            cli: true,
            tui: true,
            mcp_metadata: true,
            mcp_default: true,
            rest: true,
            grpc: true,
            agent: true,
        },
    }
}
```

Avoid requiring every operation to carry irrelevant fields. Use defaults only
when they are conservative and obvious. Hazardous or domain-specific exposure
must be explicit.

## Workstream 3 — Derive operation metadata and alias lookup

Generate or derive:

- `all_operation_metadata()`;
- canonical ID lookup;
- alias-to-canonical lookup;
- `operation_matches_tool_id()`;
- validated descriptor construction from Phase A;
- surface exposure predicates;
- feature/capability lists.

Reject duplicate canonical IDs and aliases during tests or compilation. An alias
must resolve to one operation only. Remove surprising aliases that map helper
commands to unrelated execution operations unless a real compatibility contract
requires them.

## Workstream 4 — Replace command registry duplication

Classify command entries into two categories:

1. operation-backed commands derived from the canonical catalog;
2. helper/lifecycle commands that are genuinely not operations.

For operation-backed commands, derive display name, feature, visibility, and
operation identity. Keep only command-specific information such as Clap variant
mapping or handler dispatch if it cannot be derived.

For helper/lifecycle commands, use a smaller dedicated registry only if
inspection/documentation needs it. Do not force configuration or server lifecycle
commands into security operation metadata.

Retire `registry_backed` and transitional `LegacyWrapped` distinctions when they
no longer describe a real migration boundary. The final command path should not
need phase-era flags.

## Workstream 5 — Replace domain descriptor duplication

Domain declarations should reference canonical operation IDs and derive operation
mode, risk, features, capabilities, target policy, and surface exposure from the
catalog.

Keep domain-owned metadata only where it is truly domain-level:

```text
domain ID and description
domain category
documentation link
domain-wide optional crate/feature ownership
domain-level evidence/baseline support when not operation-specific
```

If evidence/baseline support differs per operation, move it to operation or
report-adapter metadata rather than copying it at domain level.

Remove comments and tests that claim all domains exist while the function or
registry conditionally omits them. Choose and document one model:

- complete catalog with availability state; or
- available-only catalog with a separate complete definition.

The complete-catalog model is preferred for documentation and introspection.

## Workstream 6 — Replace tool registration duplication

Tool registrations for MCP/REST/gRPC should derive canonical operation ID,
feature requirements, and metadata exposure from the catalog. Keep adapter-owned
schema and implementation factory references separate.

A tool implementation must not become exposed merely because metadata exists.
Registration should require all of:

```text
canonical operation permits surface
required feature is enabled
adapter implementation exists
profile/default visibility permits listing
```

Preflight and execution must use the same canonical record.

## Workstream 7 — Align runtime, TUI, and Python operation registries

### Runtime

Map `TaskKind` to canonical operation IDs in one place. Runtime descriptors must
be constructed from the catalog, not by copying risk/feature/target fields.

### TUI

TUI tab/action visibility should consume catalog/domain views. TUI-specific
layout and input state remain local to the TUI.

### Python

The Python stable/provisional/experimental operation registry should reference
canonical engine IDs and capability metadata rather than recreating engine risk
or scope declarations. Python maturity classification remains binding-specific
and may remain separate.

## Workstream 8 — Replace consistency tests with construction tests

Delete tests whose only purpose is to keep two hand-maintained tables equal after
one table is derived.

Retain direct tests for:

- unique IDs and aliases;
- target-policy construction;
- fail-closed feature lookup;
- exposure/default-visibility distinctions;
- adapter implementation existence for exposed operations;
- hazardous operations not default-visible on strict programmatic surfaces;
- command/TaskKind mapping completeness;
- serialization/API compatibility where public.

Do not retain large static expected lists unless the list itself is a public
compatibility contract.

## Workstream 9 — Documentation reconciliation

Update metadata ownership and extension docs to show one addition workflow.
A contributor adding an operation should not edit five complete registries.
Document the exact remaining steps, which should normally be:

1. add canonical operation declaration;
2. implement handler/domain execution;
3. add adapter schema/factory only for supported surfaces;
4. add direct behavioral tests;
5. update user documentation.

Remove references to pilot phases, transitional registry-backed commands, and
historical field names from active architecture documentation.

## Validation commands

```bash
cargo fmt --all -- --check
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo check -p eggsec --features rest-api,grpc-api
cargo check -p eggsec-cli
cargo check -p eggsec-tui
make check-python   # when Python registry integration changes
```

Remove obsolete test commands from `make check` only in Phase I unless a deleted
test target would otherwise break the build.

## Migration strategy

1. Add the canonical catalog alongside existing registries.
2. Generate operation lookup and descriptor construction first.
3. Switch policy/preflight/tool lookup to the catalog.
4. Derive command operation-backed rows.
5. Derive domain operation rows.
6. Derive protocol registrations and runtime mappings.
7. Update TUI/Python consumers.
8. Delete transitional duplicate fields and snapshot tests.
9. Update documentation.

Each intermediate commit must compile. Avoid a single unreviewable replacement
commit.

## Rollback considerations

If macro expansion obscures diagnostics or IDE navigation, replace it with typed
static declarations and ordinary helper functions. Do not roll back to multiple
independent canonical tables.

## Acceptance criteria

1. Canonical operation ID, aliases, mode, risk, target policy, features, and
   capabilities have one owner.
2. Surface metadata/default visibility distinctions have one owner.
3. Operation-backed command metadata is derived from the canonical catalog.
4. Domain operation metadata is derived from the canonical catalog.
5. Tool registration consumes canonical operation metadata.
6. Runtime `TaskKind` mapping consumes canonical operation IDs.
7. TUI visibility does not duplicate engine risk/feature metadata.
8. Python operation metadata references canonical engine IDs.
9. Duplicate IDs and aliases fail tests or compilation.
10. Hazardous operations are not accidentally default-visible on strict
    automated surfaces.
11. Transitional `registry_backed`/phase-era distinctions are removed when no
    longer meaningful.
12. Tests validate behavior and catalog construction rather than mirrored lists.
13. Active docs describe one operation-addition workflow.
14. No capability is removed and no exposure is broadened without an explicit
    reviewed correction.
15. No package or release is published.
