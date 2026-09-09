# Phase A Plan: Scope Contract Unification

## Status

Status: Executed.

## Objective

Eliminate ambiguous dual scope semantics while preserving current engine authorization behavior and public compatibility where practical.

The current repository exposes both:

- `eggsec_tool_core::Scope`, a protocol-neutral DTO with `allowed_patterns`, `excluded_patterns`, `allowed_ips`, `allow_subdomains`, and a local `is_allowed()` glob matcher;
- `eggsec::config::Scope`, the authoritative policy model with target rules, CIDR/address-set handling, ports, rate limits, explicit-scope requirements, provenance, DNS resolution, exclusion handling, and non-public address policy.

Only the second model may answer the security question "is execution authorized?". The protocol/tool layer may carry a requested or declarative scope specification, but it must not present a competing authorization implementation.

## Baseline files to inspect

```text
crates/eggsec-tool-core/src/request.rs
crates/eggsec/src/config/scope.rs
crates/eggsec/src/config/policy.rs
crates/eggsec/src/config/policy_decision.rs
crates/eggsec/src/tool/mod.rs
crates/eggsec/src/tool/protocol/
crates/eggsec/src/runtime_bridge/
crates/eggsec-runtime/src/request.rs
crates/eggsec-python/src/
docs/SAFETY.md
docs/ENFORCEMENT_MODES.md
docs/ARCHITECTURE.md
docs/python/scope-and-safety.md
```

Confirm actual call sites before editing.

## Non-goals

This phase does not change the meaning of existing engine scope rules, DNS rebinding behavior, non-public address handling, execution profiles, or manual overrides. It does not broaden automated-surface scope. It does not move the complete policy engine into `eggsec-tool-core`.

## Required design

Establish three clearly separated concepts:

1. **Scope specification / transport DTO**: serializable data that a caller may attach to a request. It is not authoritative and contains no method that claims execution authorization.
2. **Loaded/authoritative engine scope**: the existing engine policy object plus provenance and resolver-aware evaluation.
3. **Authorization result**: the decision produced only by `EnforcementContext`/scope policy during approval.

The preferred naming is `ScopeSpec` or `ToolScopeSpec` for the protocol-neutral DTO. If semver compatibility requires retaining `eggsec_tool_core::Scope`, keep it as a deprecated type alias or wrapper with documentation that it is declarative only. Do not keep a public `is_allowed()` method whose semantics differ from engine policy.

## Workstream 1 — Inventory every scope representation and conversion

Identify all occurrences of:

```text
config::Scope
eggsec_tool_core::Scope
tool::Scope
LoadedScope
SessionScope
RuntimeSurface/session scope metadata
Python Scope/ToolScope wrappers
REST/MCP/gRPC scope fields
```

For each occurrence, classify it as:

- authoritative policy state;
- serialized declaration;
- provenance metadata;
- UI/view model;
- compatibility facade.

Document any call site that currently invokes `eggsec_tool_core::Scope::is_allowed()` or otherwise treats the DTO as an authorization result. Those are mandatory migration sites.

## Workstream 2 — Define a protocol-neutral declarative scope type

Introduce a dependency-light type in `eggsec-tool-core` that is explicitly a specification. Preserve wire compatibility where feasible using serde aliases/defaults.

Requirements:

- no authorization-returning methods;
- no permissive default that can be mistaken for engine authorization;
- stable serialization suitable for protocol/Python schemas;
- fields documented as caller intent, not enforcement guarantees;
- conversion to the engine scope must be explicit and fallible where information is incomplete or ambiguous.

If existing fields cannot faithfully express engine `ScopeRule`/CIDR/port semantics, do not silently invent equivalence. Either extend the specification version or treat the simple legacy form as a constrained compatibility subset.

## Workstream 3 — Implement conservative conversion into engine scope

Create a single conversion boundary in the main engine, conceptually:

```rust
impl TryFrom<&ScopeSpec> for config::Scope { ... }
```

or a dedicated `ScopeSpecConverter` if provenance/context is required.

Rules:

- exclusions must never be weakened;
- an unsupported/ambiguous declaration must fail closed on strict surfaces;
- `*` in a transport DTO must not silently become permission to private/link-local/multicast targets contrary to engine defaults;
- hostname/subdomain semantics must be mapped explicitly and covered by tests;
- any legacy `allowed_ips` representation must map through parsed IP/CIDR rules, not string comparison;
- port/rate constraints, if absent from the DTO, remain controlled by engine policy/defaults rather than inferred.

## Workstream 4 — Route all programmatic surfaces through the authoritative conversion

REST, MCP, gRPC, daemon/runtime bridges, tool invocation, and Python tool invocation must not evaluate DTO scope directly.

The required flow is:

```text
transport/request ScopeSpec
    -> parse/validate
    -> engine Scope / LoadedScope with provenance
    -> EnforcementContext
    -> approval decision
    -> dispatch
```

No adapter may shortcut this by calling a matching helper on the DTO.

## Workstream 5 — Python compatibility and naming

The Python package currently exposes both stable engine scope and tool-core bindings. Make the distinction visible:

- `eggsec.Scope` remains the authoritative user-facing engine scope API if that is the current stable contract;
- tool-core binding should become `ToolScopeSpec` (or equivalent);
- if `ToolScope` is retained for compatibility, document/deprecate it rather than silently changing semantics;
- generated/type stubs and schema descriptions must clearly state that tool scope declarations are subject to engine policy.

Add tests proving that a permissive tool scope specification cannot override a restrictive engine scope.

## Workstream 6 — Semantic regression tests

Add direct tests for at least:

- permissive transport spec + restrictive engine scope -> denied;
- restrictive transport spec + permissive engine scope -> transport restriction is preserved if the product contract requires intersection, otherwise document which layer owns it;
- explicit exclusions cannot be lost in conversion;
- wildcard hostname semantics;
- IPv4 and IPv6 literals;
- CIDR rules;
- hostname resolving to private/public mixtures;
- loopback behavior;
- excluded ports and allowed ports;
- no explicit scope on strict automated surfaces;
- serialization round-trip of the specification;
- compatibility deserialization of existing tool-core scope JSON/Python objects.

Prefer property tests for rule conversion where they materially improve coverage.

## Workstream 7 — Remove misleading APIs and documentation

Search for documentation or comments that describe `eggsec-tool-core::Scope` as enforcing authorization. Update architecture, tool-core binding, extensibility, protocol, and Python safety docs.

Add an architecture guard or compile-time ownership test only if necessary to prevent a DTO-level `is_allowed()` implementation from returning. Prefer type/module ownership over grep checks.

## Acceptance criteria

- exactly one implementation decides whether a network target is authorized;
- no protocol-neutral DTO has an authorization-looking `is_allowed()`/`authorize()` method;
- all automated surfaces construct authoritative engine scope before approval;
- legacy wire/Python forms either remain compatible or have a documented migration path;
- no change weakens current engine scope behavior;
- tests cover wildcard, CIDR, port, private/loopback, DNS multi-address, and exclusion semantics;
- `make check` passes;
- `make check-python` passes because tool-core/Python scope bindings are affected;
- relevant feature profiles for REST/gRPC/daemon/tool API compile.

## Completion record

Status: Executed.
Baseline SHA: 2555c7b73e0b0734d569564d27119465a63fc6c5
Final SHA: <filled at commit time>
Compatibility aliases retained:
- Rust: `eggsec_tool_core::Scope` (deprecated alias for `ScopeSpec`),
  `eggsec_tool_core::ToolScopeSpec`, `eggsec::tool::Scope` (deprecated re-export)
- Python: `ToolScope` (alias for `ToolScopeSpec`)
- gRPC: `Scope` message field names/numbers unchanged (documented declarative)
Verification: `make check` (EXIT=0), `make check-python` (EXIT=0, 4450 passed),
`cargo check -p eggsec --features rest-api/grpc-api/tool-api`,
`cargo check -p eggsec-cli` (default + `--no-default-features`),
`cargo check -p eggsec-daemon`, arch guard check 70 PASS.
Notes:
- `ScopeSpec::default()` is now fail-closed deny (was permissive `["*"]`);
  use `ScopeSpec::allow_all()` for an explicit permissive declaration.
- Pre-existing clippy `result_large_err` failure in untouched
  `crates/eggsec-agent/src/scheduler.rs` fixed with a targeted allow so the
  `make check` contract is green.
