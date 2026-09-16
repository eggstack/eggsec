# Phase C — Policy and enforcement boundary extraction

Status: Executed

Date: 2026-09-16

Depends on: Phases A-B

Roadmap: `crate-boundary-consolidation-roadmap-2026-09-16.md`

## Purpose

Separate Eggsec's authorization/enforcement semantic engine from ordinary configuration loading and, if the cleaned boundary passes the dependency/cycle gate, promote it to a dependency-light `eggsec-policy` crate.

The current `config` module owns two fundamentally different responsibilities:

- loading application settings from files/environment and composing `EggsecConfig`;
- defining and evaluating execution policy, operation metadata/risk/capabilities, scope, target rules, approval binding, denials, and enforcement outcomes.

The second responsibility is now large enough and stable enough to deserve an independent semantic boundary. The extraction must not recreate the rejected `eggsec-net` design. Network I/O, DNS acquisition, and the transport implementation remain outside the policy crate.

## Gate C0 — Make the boundary independently compilable before adding the crate

Do not begin by moving files. First refactor the policy cluster in place so that it can be described by a dependency list that does not include the rest of `eggsec`.

Inventory all references from these modules to engine/config/runtime state:

```text
config/policy.rs
config/policy_approval.rs
config/policy_catalog.rs
config/policy_decision.rs
config/policy_target.rs
config/scope.rs
config/scope_address.rs
config/scope_spec.rs
config/scope_resolver.rs
config/scope_transport.rs
config/feature_registry.rs
```

Classify every edge as one of:

- pure policy data/logic;
- feature-availability input;
- DNS/resolution I/O;
- transport bridge;
- config-file loading;
- engine operation/dispatch coupling;
- frontend/process-host coupling.

Create `eggsec-policy` only when the pure policy set can compile without `eggsec`, `eggsec-transport`, Tokio, HTTP/TLS clients, filesystem access, or frontend crates.

Record the proposed dependency graph and cycle analysis before creating the workspace member.

## Target ownership

### `eggsec-policy` should own

- `OperationRisk`;
- `OperationMode`;
- `ExecutionProfile` / `ExecutionSurface` / intended-use vocabulary;
- `Capability` and denial classes;
- `ExecutionPolicy` as the serializable policy data type;
- `OperationDescriptor`, operation metadata, tool-id/operation alias matching, and policy catalog data;
- target-policy descriptors and pure normalization needed for authorization;
- `Scope`, `TargetScope`, `ScopeRule`, `ScopeSource`, `LoadedScope` if they can be kept I/O-free;
- address classification/private-range semantics;
- pure scope matching over already-resolved destination facts;
- `PolicyDecision` and preflight/enforcement result types;
- `ApprovedOperation` and approval-token binding/verification semantics;
- deterministic policy evaluation over explicit inputs.

### The `eggsec` engine should continue to own

- config-file discovery/loading and environment overrides;
- `EggsecConfig`, scan/http/API/recon/path/process-host settings;
- compile-time feature discovery/registry adapter;
- concrete DNS resolution;
- any Hickory/Tokio resolver implementation;
- the `ScopeAuthority` adapter implementing `eggsec_transport::NetworkAuthority`;
- conversion from frontend/dispatch/runtime request state into policy inputs;
- policy audit emission and report-summary conversion when those require engine/output types.

### `eggsec-transport` continues to own

- transport checkpoint DTOs;
- `NetworkAuthority`;
- transport resolver facts and approved connection binding;
- HTTP request/response contract;
- no Eggsec policy implementation.

## Workstream 1 — Decouple feature availability from policy evaluation

Policy evaluation currently knows about Eggsec operation requirements and compile-time feature state. The extracted crate must not use engine `cfg!(feature = ...)` as its runtime feature oracle.

Introduce an explicit immutable policy input representing available features/capabilities. Preferred shape:

```text
FeatureAvailability / EnabledFeatures
    - constructed by the engine from `config::feature_registry` or its successor
    - queried by policy evaluation
    - serializable only if useful; otherwise plain value object
    - no Cargo-feature macros inside `eggsec-policy`
```

Avoid a global singleton. Avoid making policy call back into the engine.

`OperationMetadata.required_features` may remain a stable list of feature identifiers in the policy catalog. The engine maps its compiled/available feature registry to the input set before evaluation.

Add tests proving the same operation can be evaluated against different supplied feature sets without recompiling the policy crate.

## Workstream 2 — Make scope evaluation consume resolution facts, not perform DNS

The policy crate may decide whether a target/address is authorized, but it must not acquire network facts itself.

Refactor scope APIs toward pure forms such as:

```text
normalize target -> target descriptor
classify supplied IP address
scope.evaluate_target(target_descriptor, resolved_addresses)
scope.evaluate_addresses(allowed/excluded facts)
```

Concrete host resolution remains in the engine/transport composition layer.

### Resolver rule

Do not move `SystemResolver`, Hickory integration, `tokio::net::lookup_host`, or other concrete resolver behavior into `eggsec-policy`.

If `HostResolver` exists only to make scope unit tests deterministic, prefer converting the core policy API to supplied address facts and keep resolver adapters outside. A resolver trait may remain in the engine as a compatibility/test seam, but it is not part of the reusable policy kernel unless a concrete need remains after refactoring.

### TOCTOU rule

Do not weaken the existing transport binding invariant. Policy authorization of a hostname/address set does not authorize arbitrary later resolution. `eggsec-transport::validate_binding` and approved connection candidates remain authoritative at the dispatch boundary.

## Workstream 3 — Keep the transport-policy bridge in the engine

`ScopeAuthority` is an adapter between two independent contracts:

- Eggsec policy/scope semantics;
- `eggsec-transport::NetworkAuthority` checkpoint facts.

Move it out of the `config` namespace if that improves ownership, for example:

```text
crates/eggsec/src/policy_bridge/
  feature_availability.rs
  resolver.rs
  transport.rs
```

The exact module name is flexible. The dependency direction is not:

```text
eggsec-policy      eggsec-transport
       ^                 ^
        \               /
         \             /
              eggsec
        composition/bridge
```

Never introduce `eggsec-policy -> eggsec-transport` or `eggsec-transport -> eggsec-policy` merely to avoid a small adapter.

## Workstream 4 — Extract config-independent policy types

After Workstreams 1-3, move pure modules into the new crate in dependency order.

Suggested order:

1. policy vocabulary (`OperationRisk`, mode/profile/surface/intended use, capability, denial classes);
2. operation target/descriptor/metadata/catalog;
3. scope/address/pure target matching;
4. decision/result types;
5. approval binding/token;
6. evaluation/preflight/enforcement algorithms.

Keep configuration loading in `eggsec`; deserialize policy-owned structs directly as fields of `EggsecConfig` where appropriate.

Do not duplicate types during migration. Use temporary `pub use eggsec_policy::...` facades rather than maintaining parallel enums/structs.

## Workstream 5 — Design the `eggsec-policy` manifest as a leaf semantic crate

Expected dependencies may include only what the pure algorithms require, for example:

```text
serde
thiserror
url
ipnetwork
sha2        # only if approval binding still uses it
uuid        # only if decision/token identity still uses it
eggsec-core # only for a genuine shared primitive
```

Minimize the set based on actual code after decoupling.

Forbidden dependencies:

```text
eggsec
eggsec-transport
eggsec-runtime
eggsec-output / eggsec-report-model
tokio
hickory-resolver
reqwest / hyper
rustls / tokio-rustls
axum / tonic
clap / ratatui / crossterm
rusqlite / sqlx
filesystem/process/signal abstractions
```

Add a manifest-graph guard for these directions.

## Workstream 6 — Preserve the `eggsec::config` compatibility surface

The main engine currently re-exports many policy types from `config`. Preserve internal/downstream source compatibility where it does not obscure ownership:

```rust
pub use eggsec_policy::{
    ApprovedOperation, Capability, EnforcementContext, ExecutionPolicy,
    ExecutionProfile, OperationDescriptor, OperationMode, OperationRisk,
    PolicyDecision, Scope, TargetScope, ...
};
```

`eggsec::config::*` may remain a compatibility facade during the 0.1 line, but new engine code should import policy types from the canonical crate/module to prevent `config` from becoming the conceptual owner again.

Document canonical ownership in rustdoc and architecture docs.

If a facade would require an invalid dependency direction, do not preserve it. In practice `eggsec -> eggsec-policy` is the intended direction, so engine re-exports are safe.

## Workstream 7 — Keep audit/report conversion above policy

Do not make `eggsec-policy` depend on output/report types merely to produce `PolicySummary`, audit envelopes, JSON reports, or logging.

Instead:

- policy returns typed decisions/outcomes;
- engine audit code maps them to `EnforcementAuditEvent`;
- output/report code maps them to `eggsec-report-model::PolicySummary`;
- frontends render them.

This keeps policy usable without report or logging dependencies.

## Workstream 8 — Preserve and relocate tests by responsibility

Move pure policy tests into `eggsec-policy`, including:

- risk/profile/capability allow/deny matrices;
- operation metadata/alias invariants;
- target normalization that is purely policy-related;
- scope rule matching against supplied addresses;
- private/public/reserved address classification;
- denial classification;
- confirmation-class behavior;
- approval token creation/binding/replay/mismatch tests;
- deterministic feature-availability tests;
- policy evaluation table tests.

Keep integration tests in `eggsec` for:

- config loading into policy types;
- real/concrete resolver behavior;
- feature-registry -> `EnabledFeatures` mapping;
- `ScopeAuthority` transport checkpoints;
- DNS mixed-answer/re-resolution/redirect/proxy policy invariants;
- dispatch approval binding;
- CLI/TUI/MCP/REST/gRPC/agent surface enforcement.

The extraction must improve test isolation without moving network integration tests into the leaf crate.

## Workstream 9 — Reconcile the prior Phase E decision record

Update `architecture/capability_segregation.md` rather than silently contradicting it.

Record:

- `eggsec-net` remains rejected;
- why `eggsec-policy` is a different boundary (complete semantic authorization domain, not a network middle layer);
- dependency delta;
- API boundary;
- cycle analysis;
- migration cost;
- measured compile/dependency effect;
- retained engine transport/resolver bridge;
- any extraction gate that failed.

Update architecture overview, config/policy docs, AGENTS/skills guidance, and architecture guards accordingly.

## Gate C1 — Final extraction decision

After the in-place decoupling, create `eggsec-policy` only if all of these are true:

1. the candidate source set has no dependency on engine modules except through values that can be passed explicitly;
2. no transport/runtime/frontend dependency is required;
3. the engine can depend one-way on the policy crate without a cycle;
4. configuration loading can deserialize/re-export policy types without duplicating them;
5. at least policy tests can run in isolation with a narrower dependency graph than `eggsec`;
6. the move makes the conceptual ownership clearer enough to justify the workspace member and release/versioning cost.

If any criterion fails, keep the cleaned `policy` module inside `eggsec`, document the blocker, and retain all other Phase C decoupling improvements. Do not force the crate.

## Required verification

If the crate is created:

```text
cargo check -p eggsec-policy
cargo test -p eggsec-policy
cargo tree -p eggsec-policy
cargo check -p eggsec --no-default-features
cargo test -p eggsec --lib
cargo check -p eggsec-cli
cargo check -p eggsec-tui
cargo check -p eggsec-daemon
cargo check --workspace --no-default-features
cargo package -p eggsec-policy --no-verify
make check-feature-profiles
make check-features-individual
make test-architecture-guards
make check
make check-python
```

Also rerun all existing network-policy/transport-contract/approval-binding integration suites explicitly, not only through aggregate `make` targets.

If extraction is rejected, run the same engine/workspace suites and record the dependency blocker.

## Acceptance criteria

1. Policy evaluation no longer obtains compile-time feature state from hidden/global engine context; feature availability is explicit input.
2. Pure scope evaluation consumes supplied resolution facts and performs no DNS/network I/O.
3. Concrete resolver behavior and `ScopeAuthority` remain engine-side bridges.
4. `eggsec-transport` remains independent of policy implementation.
5. If created, `eggsec-policy` has no Tokio, HTTP/TLS, filesystem, frontend, database, output, or engine dependency.
6. Policy DTOs/algorithms have a single canonical owner; no long-lived duplicate policy language is created.
7. Existing `eggsec::config` paths remain compatibility facades where dependency-safe.
8. Approval-token target/operation binding and strict-surface enforcement remain unchanged or stronger.
9. Existing DNS/redirect/proxy/TLS scope invariants remain green.
10. The Phase E `eggsec-net` rejection remains in force and is reconciled in architecture documentation.
11. The extraction decision includes measured dependency/cycle evidence.

## Expected files touched

- new `crates/eggsec-policy/` if Gate C1 passes;
- root `Cargo.toml` and release package graph if created;
- `crates/eggsec/src/config/` policy/scope modules;
- engine policy/resolver/transport bridge module(s);
- `crates/eggsec/src/audit.rs` and policy-summary conversions only as needed for imports;
- policy and network-policy integration tests;
- architecture docs/guards/AGENTS/skills references;
- this plan completion record.

## Completion record template

(Template retained; execution record follows under "Completion record".)

## Completion record

- Baseline SHA: `5a3b2a0499fdc3a3700dc973633d7659191ea7b5` (post-Phase-B main).
  Final SHA: recorded at commit time (see `git log`).
- Gate C0 dependency inventory (all references from the 10 candidate modules
  classified before creating the member):
  - pure policy data/logic: `policy.rs` (serde only), `policy_target.rs`
    (serde/url/ipnetwork + 2 tracing debug logs, dropped), `policy_catalog.rs`
    (policy+target types, incl. one dead engine-coupled
    `derive_operation_integration` method with zero callers — removed from the
    kernel, kept as an engine free function), `scope_address.rs` (serde only),
    `policy_approval.rs` (policy types only);
  - feature-availability input: `policy_decision.rs` queried the engine
    registry via global `is_feature_enabled` (`cfg!` oracle) — decoupled via
    explicit `EnabledFeatures`;
  - DNS/resolution I/O: `policy_decision.rs` called `TargetScope::parse*` +
    `Scope::is_target_allowed` (DNS inside evaluation) — decoupled via
    supplied `TargetScope` facts; `scope.rs` I/O methods (`is_target_allowed*`,
    `is_excluded`, `validate_url`, `from_file` via std::fs/toml/yaml) and
    `scope_resolver.rs` (`ToSocketAddrs`) stay engine-side;
  - transport bridge: `scope_transport.rs` (`ScopeAuthority` over
    `eggsec_transport::NetworkAuthority`) stays engine-side;
  - config-file loading: `Scope::from_file`, loader paths stay engine-side;
  - engine operation/dispatch coupling: catalog `derive_operation_integration`
    → engine free function; `From<&LoadedScope> for SessionScope`
    (eggsec-runtime) → engine `session_scope_from_loaded` free function
    (orphan-rule-forced); `From<&PolicyDecision> for PolicySummary` →
    engine `policy_summary_from_decision` free function (WS7);
  - frontend/process-host coupling: none inside the candidate set.
  - Proposed graph verified before creating the member: leaf with direct deps
    `serde`, `serde_json`, `thiserror`, `url`, `ipnetwork`, `sha2`, `uuid`,
    `hex`, `rustc-hash`; no `eggsec`, `eggsec-transport`, Tokio, HTTP/TLS,
    filesystem, or frontend crates.
- Pure modules/types isolated: `features.rs` (`EnabledFeatures`, new),
  `policy.rs` (vocabulary + `ExecutionPolicy` + descriptor), `target.rs`
  (normalization), `catalog.rs` (metadata registry), `scope.rs` (`Scope`/
  `TargetScope`/`ScopeRule`/`ScopeSource`/`LoadedScope` + `evaluate_facts`/
  `evaluate_addresses`), `address.rs` (classification), `decision.rs`
  (`PolicyDecision`, outcomes, `EnforcementContext` with explicit inputs,
  pure `evaluate_*`/`preflight_operation`), `approval.rs`
  (`ApprovedOperation`), `lib.rs` re-exports.
- Feature-availability API chosen: immutable `EnabledFeatures(HashSet<String>)`
  with `from_names` + `From`/`FromIterator`; engine snapshots via
  `policy_bridge::features::current_enabled_features()` (registry oracle test
  included). Same-operation/different-sets test proves no kernel recompile.
- Resolver/transport bridge final owner: `crates/eggsec/src/policy_bridge/`
  (`features.rs`, `resolver.rs` incl. `HostResolver`/`SystemResolver`,
  `resolve_target_facts*`, `ScopeResolution`, `load_scope_from_file`,
  `transport.rs` incl. `ScopeAuthority`). Dependency direction
  `eggsec-policy ← eggsec → eggsec-transport` (no policy↔transport edge).
- Gate C1 decision: EXTRACTED. All six criteria hold: pure set compiles
  without engine/transport/runtime/frontend deps; engine depends one-way with
  no cycle (`cargo tree -d` clean); config deserializes/re-exports policy
  types without duplication; 80 kernel tests run isolated with a 9-direct-dep
  closure; ownership is unambiguous (`architecture/capability_segregation.md`
  reconciles the `eggsec-net` rejection).
- `eggsec-policy` dependency tree (direct): `hex`, `ipnetwork`, `rustc-hash`,
  `serde`, `serde_json`, `sha2`, `thiserror`, `url`, `uuid`. Transitive
  closure is pure-only (icu/idna via url, digest/generic-array via sha2,
  getrandom via uuid). Verified free of tokio/reqwest/hyper/rustls/axum/
  tonic/clap/ratatui/rusqlite/sqlx/hickory/eggsec/transport/runtime/output.
- Path dependency graph before → after: before, `eggsec` owned policy
  semantics internally; after, `eggsec → eggsec-policy` (new leaf) with
  `config/*` as facades, `policy_bridge/` as the I/O adapter layer, and no
  new edges into `eggsec-transport` (still zero workspace deps).
- Compatibility re-exports/API changes (pre-1.0, all recorded):
  - `eggsec::config::*` paths preserved (policy/scope/decision/approval/
    catalog/target/address/resolver/transport facades + `EnabledFeatures` +
    `session_scope_from_loaded` + resolve fns + `ScopeResolution`).
  - `TargetScope::parse*` / `Scope::from_file` associated fns removed
    (orphan-rule-forced; use `policy_bridge::resolver::{resolve_target_facts*,
    resolve_hostname_facts*, load_scope_from_file}`); ~30 call sites updated
    (engine, TUI ×4, Python ×2 modules, 6 integration test files).
  - `Scope::is_target_allowed*` / `is_excluded` / `validate_url` now require
    `ScopeResolution` in scope (trait import updates).
  - `TryFrom<&ScopeSpec> for Scope` and `From<&PolicyDecision> for
    PolicySummary` / `From<&LoadedScope> for SessionScope` impls replaced by
    same-behavior free functions (orphan-rule-forced).
  - Pure `EnforcementContext::{evaluate, approve, approve_manual}` and
    `evaluate_operation_policy` / `evaluate_enforcement` /
    `preflight_operation` take explicit `EnabledFeatures` + `Option<&TargetScope>`;
    engine facades preserve legacy signatures (snapshot + resolve + legacy
    `InvalidTarget` hard denial for unresolvable/CIDR-without-IP targets).
  - `ApprovedOperation::new` stays `pub(crate)` to the kernel; engine facades
    delegate token issuance to the kernel (no construction outside).
- Policy/transport/approval test results: `cargo test -p eggsec-policy`
  80 passed; engine `make test-ci` (`-p eggsec --features rest-api,cli`)
  2830 passed / 0 failed (incl. network_policy_invariants 12, transport
  contract 12, eggfetch parity 5, enforcement matrix); `make check` green
  (fmt, no-default checks, deny, clippy incl. new crate, doc tests,
  output/report-model/policy/eggfetch/TUI suites, guards).
  `make check-python`: 4453 passed, 1 flaky socket-budget failure
  (`test_socket_cleanup_after_close`, +1 socket under load; passes 3/3 in
  isolation; no socket code touched by this change).
- Architecture guard/doc changes: new guards Checks 121–123 + Check 76/100
  path updates; `Makefile` clippy/test lists include `eggsec-policy`;
  `architecture/capability_segregation.md` Phase C acceptance section;
  `architecture/overview.md` (20 crates, policy row, direction, guardrails);
  `architecture/config.md` Phase C ownership section + facade file table;
  `docs/ARCHITECTURE.md`, `docs/ENFORCEMENT_MODES.md`,
  `docs/CI_ARCHITECTURE_GUARDS.md` canonical-owner notes; `README.md` +
  `AGENTS.md` (20 crates, clippy list, policy boundary, bridge paths) +
  `config/AGENTS.override.md` + `eggsec-config` skill Phase C section.
- Residual debt / Phase D blockers:
  - Engine `EnforcementContext` facade duplicates approval-match logic shape
    (delegates issuance, but keeps its own evaluate/approve wrappers); a
    future pass could migrate call sites to pure kernel + explicit facts and
    shrink the facade. Not blocking.
  - `derive_operation_integration` free function is currently uncalled
    (was dead as a method too); keep or remove in Phase D.
  - `check-features-individual` (deep-checks oracle, not per-PR) not run
    locally beyond `stress-testing` spot-check; CI deep-checks cover the rest.
  - Python `check-python` flaky socket-budget test is environment-sensitive
    and unrelated; consider raising its tolerance or quarantining.
