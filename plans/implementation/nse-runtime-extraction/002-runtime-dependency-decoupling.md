# NSE Runtime Extraction Milestone 002 — Runtime Dependency Decoupling and Consumer Consolidation

Status: blocked

Repository baseline: `2ef67febf17a2ae42e2b8863b140f6c47cb7f387`

Source roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-002--runtime-dependency-decoupling-and-consumer-consolidation`

Long-term requirements:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#4-policy-and-transport-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Applicable ADRs:

- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md` — controlling only for Eggsec-owned transport adaptation; this milestone must not introduce a competing transport owner.
- `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md` — preserve current selective-reuse boundary.

Primary class: infrastructure

## 1. Objective

After Milestone 001 closes, make the in-tree `eggsec-nse` crate independently ownable by removing all dependencies on Eggsec workspace crates, relocating Eggsec-specific adapters to the engine, and making `eggsec` the sole direct workspace consumer of `eggsec-nse`.

This milestone establishes the hard extraction gate. It does not yet create or publish the standalone repository.

## 2. Why this milestone is blocked

Hard dependency:

- Milestone 001 must close with one canonical execution/report pipeline and parity evidence.

Dependency decoupling before execution convergence would make it harder to distinguish ownership changes from behavior changes and could freeze the current caller drift across repository boundaries.

The implementation agent must not begin this plan until the Milestone 001 closure record explicitly recommends Milestone 002 readiness.

## 3. Current implementation evidence

At the baseline, `crates/eggsec-nse/Cargo.toml` directly depends on:

- `eggsec-core`;
- `eggsec-report-model`;
- `eggsec-transport`.

The inward edges have identifiable ownership causes:

- `crates/eggsec-nse/src/bridge.rs` converts `NseRunReport` into Eggsec `ReportEnvelope` data and uses Eggsec severity/report contracts. That is an Eggsec composition/reporting adapter, not NSE runtime semantics.
- `crates/eggsec-nse/src/http_capability.rs` is coupled to `eggsec-transport::{HttpTransport, NetworkAuthority, ...}`. Its own integration status does not make it the canonical backend for all Lua HTTP-family libraries; several runtime libraries still use native/reqwest paths under NSE capability checks.
- Other runtime modules should be audited for `eggsec_core` usage and converted to runtime-local or standard/public dependency types where the type is genuinely NSE-owned.

Consumer ownership is also wider than necessary:

- `crates/eggsec/src/lib.rs` already exposes `pub use eggsec_nse as nse` under the `nse` feature, which is the desired compatibility facade.
- `eggsec-tui` declares a direct optional `eggsec-nse` dependency in addition to `eggsec`.
- `eggsec-python` declares a direct optional `eggsec-nse` dependency even though its NSE source predominantly uses `eggsec::nse::*`.
- CLI consumption is already mediated by the engine rather than requiring a separate direct runtime edge.

The intended post-milestone graph is therefore practical without a broad frontend redesign.

## 4. Invariants that must not regress

- Eggsec authorization remains engine-owned and external to the runtime.
- `eggsec-transport` remains implementation-neutral; no new dependency from transport into the runtime or runtime into Eggsec policy is introduced.
- The runtime does not gain a dependency on `eggsec-policy`, `eggsec-runtime`, `eggsec-output`, frontend crates, or the Eggsec engine.
- `eggsec::nse::*` remains a compatibility facade for Eggsec consumers.
- TUI and Python retain their existing user-visible NSE feature switches and behavior.
- Report conversion to Eggsec's `ReportEnvelope` remains available from the engine side.
- Runtime capability/profile semantics remain unchanged.
- The clean-room corpus stays runtime-owned and provenance-tracked.
- No physical extraction occurs until dependency and consumer guards are green.

## 5. Scope

### In scope

- Inventory every `eggsec-*` reference reachable from `eggsec-nse`.
- Move Eggsec report-envelope conversion from the runtime crate into `eggsec`.
- Remove or relocate the Eggsec-specific transport adapter from the runtime dependency graph.
- Replace any incidental `eggsec-core` type use with runtime-owned/public dependency types where semantically appropriate.
- Add a manifest/dependency static guard that `eggsec-nse` has zero `eggsec-*` dependencies.
- Preserve an Eggsec compatibility path for the moved report bridge.
- Remove direct `eggsec-nse` dependencies from TUI/Python where the `eggsec::nse` facade is sufficient.
- Update feature forwarding so visible behavior is unchanged.
- Add a workspace guard that only the `eggsec` package directly depends on `eggsec-nse`.
- Requalify runtime, engine, TUI, Python, and feature matrices.
- Update architecture and contributor documentation to reflect ownership.

### Explicitly out of scope

- Creating the standalone repository.
- Publishing `eggsec-nse`.
- Renaming the package.
- Replacing every native socket/HTTP implementation with provider traits.
- Forcing all NSE HTTP-family libraries onto `eggsec-transport`.
- Moving Eggsec authorization into runtime capability checks.
- Removing public convenience APIs or compatibility libraries.
- Changing the clean-room corpus licensing/provenance model.

## 6. Required production changes

### Core/domain

Audit `eggsec-nse` for all direct and transitive workspace-specific type use.

The end-state runtime source must compile without importing any `eggsec_*` crate.

Where `eggsec-core` is used only for generic shared values such as severity or common DTOs, choose one of:

1. keep the concept engine-side if it belongs to Eggsec reporting;
2. define an NSE-specific runtime representation if it is semantically part of the runtime;
3. use an existing public ecosystem/std type if there is no Eggsec-specific semantic value.

Do not copy an Eggsec domain type into the runtime merely to remove a dependency.

### Storage and migrations

None expected.

### Protocol and DTOs

Move `bridge.rs` conversion into an engine-owned module, for example `crates/eggsec/src/nse_bridge.rs` or an existing output/report adapter namespace.

Preserve current caller ergonomics where dependency-safe. An acceptable compatibility shape is:

```rust
#[cfg(feature = "nse")]
pub use eggsec_nse as nse;

// Engine-owned adapter, optionally re-exported under an Eggsec namespace.
```

Do not force the standalone runtime to know about `ReportEnvelope`.

### Runtime and concurrency

No runtime execution behavior should change after Milestone 001. This is an ownership/dependency pass.

If removal of `http_capability.rs` exposes a concrete runtime dependency on an Eggsec transport type that is actually needed for production execution, stop and classify the seam. Prefer moving the adapter to Eggsec over designing a new provider layer inside this milestone.

Provider inversion belongs to a later roadmap milestone unless it is the minimal required mechanism to remove a real inward dependency.

### Frontend or operator surface

TUI:

- replace direct `eggsec_nse::...` imports with `eggsec::nse::...` where possible;
- remove `dep:eggsec-nse` from the TUI `nse` feature;
- retain `nse = ["eggsec/nse"]` plus any truly local dependencies if required.

Python:

- remove the direct optional `eggsec-nse` dependency if no source requires it after Milestone 001;
- change `nse = ["eggsec/nse", "dep:eggsec-nse"]` to engine-only forwarding;
- preserve public Python API/DTO behavior.

CLI:

- verify no direct edge is introduced.

### Security and authorization

No runtime dependency removal may bypass Eggsec's outer authorization boundary.

If an Eggsec transport adapter is relocated, it must remain downstream of the appropriate engine authority/scope decisions. Do not replace an authority-preserving adapter with an unrestricted client merely to make the runtime standalone.

Runtime-native side effects remain governed by existing NSE profile/capability checks until the later provider-inversion milestone. This is not permission to claim those checks are equivalent to Eggsec authorization.

### Documentation and static guards

Add an architecture guard with at least these assertions:

- `crates/eggsec-nse/Cargo.toml` contains no dependency whose package/path resolves to `eggsec-*`;
- no runtime source imports `eggsec_...`;
- only `crates/eggsec/Cargo.toml` directly references `eggsec-nse` among production workspace consumers, excluding any explicit test-only qualification harness if later justified.

Update architecture ownership/dependency maps and NSE integration docs.

## 7. Ordered work packages

### Work package A — Dependency and ownership inventory

Intent:

Prove the exact inward dependency causes before editing.

Required changes:

- enumerate all `eggsec_*` imports in runtime source/tests/examples;
- classify each as runtime semantic, Eggsec adapter, test-only, or accidental;
- capture `cargo tree -p eggsec-nse` before state;
- capture direct workspace consumers of `eggsec-nse`.

Acceptance evidence:

- every inward edge has a documented disposition;
- no unexplained dependency remains.

### Work package B — Move the Eggsec report bridge

Intent:

Make Eggsec reporting an engine composition concern.

Required changes:

- move `NseRunReport -> ReportEnvelope` conversion into `eggsec`;
- move bridge-specific tests into Eggsec integration tests;
- split mixed runtime/bridge tests so runtime evidence remains in `eggsec-nse`;
- preserve compatibility imports/call sites where practical.

Acceptance evidence:

- `eggsec-nse` no longer depends on `eggsec-report-model`;
- Eggsec report-envelope conversion tests pass.

### Work package C — Remove Eggsec transport coupling

Intent:

Eliminate the runtime's direct `eggsec-transport` edge without misrepresenting current network behavior.

Required changes:

- audit all callers of `http_capability.rs`;
- if it is only an Eggsec adapter/unused migration seam, move it to Eggsec or remove it with tests/docs updated;
- if a runtime-owned abstraction is genuinely needed, introduce only the narrowest runtime-neutral interface necessary and provide an Eggsec adapter outside the runtime;
- do not rewrite all Lua libraries in this pass unless required by actual callers.

Acceptance evidence:

- `eggsec-nse` no longer depends on `eggsec-transport`;
- existing runtime HTTP/network compatibility tests remain green;
- docs accurately state which paths use native/reqwest behavior versus any Eggsec adapter.

### Work package D — Remove residual `eggsec-core` coupling

Intent:

Finish runtime semantic independence.

Required changes:

- relocate Eggsec-only type conversions;
- replace only genuinely generic primitives with runtime/public equivalents;
- update tests accordingly.

Acceptance evidence:

- `eggsec-nse` manifest and source have no `eggsec-core` dependency/import;
- no duplicate Eggsec domain model was introduced.

### Work package E — Collapse downstream consumers through Eggsec

Intent:

Establish one host ownership edge before physical extraction.

Required changes:

- migrate TUI direct imports to `eggsec::nse`;
- remove TUI direct optional runtime dependency;
- remove Python direct optional runtime dependency when confirmed unused;
- preserve feature forwarding and compilation;
- scan all workspace manifests for direct runtime edges.

Acceptance evidence:

- production dependency graph is `eggsec-nse <- eggsec <- {CLI,TUI,Python,...}`;
- feature behavior remains unchanged.

### Work package F — Add extraction guards and architecture evidence

Intent:

Make the clean boundary durable.

Required changes:

- add dependency/source guards;
- update architecture overview/NSE integration docs;
- document exact runtime dependency tree after decoupling;
- document which tests/corpus assets are runtime-owned versus Eggsec integration-owned;
- update registry/roadmap status only when closure evidence exists.

Acceptance evidence:

- a new inward `eggsec-*` dependency fails repository verification;
- a new direct TUI/Python runtime edge fails repository verification.

## 8. Failure, cancellation, restart, and contention semantics

No execution lifecycle semantics are intentionally changed. Milestone 001 behavior is the baseline.

Moving adapters must not alter cancellation propagation, resource limits, process-level TLS initialization, or per-run state isolation.

If relocating a network adapter changes the actual connection path or scope/authority semantics, stop: that is no longer a pure dependency-decoupling change and requires a separately bounded transport/security plan.

## 9. Compatibility and migration

This milestone should be source-compatible for ordinary Eggsec consumers:

- `eggsec::nse::*` remains the supported Eggsec-facing namespace.
- TUI/Python users should see no feature/API change.
- Engine report conversion remains available even though its owner changes.
- `eggsec-nse` public runtime types remain stable unless Milestone 001 closure already documented an intentional change.

Internal direct imports from TUI/Python may change. That is an ownership cleanup, not a public behavior change.

Do not switch to a git/crates.io external dependency in this milestone; keep the crate in the workspace until the zero-inward-dependency and one-direct-consumer gates are proven.

## 10. Required tests

### Focused unit tests

- runtime modules compile/test without Eggsec workspace crates;
- moved report conversion unit tests in engine;
- any runtime-neutral adapter interface tests if one is minimally required.

### Integration tests

- Eggsec NSE report-envelope bridge;
- TUI NSE report view/use through engine facade;
- Python NSE feature/API through engine facade;
- runtime corpus independent of Eggsec reporting.

### Restart and recovery tests

Not applicable; no durable lifecycle change.

### Contention and cancellation tests

Re-run Milestone 001 canonical runtime contention/cancellation coverage to prove the ownership move did not change execution semantics.

### Security and negative tests

- dependency guard catches `eggsec-*` runtime dependency;
- runtime source guard catches `eggsec_...` import;
- transport relocation does not remove existing profile/capability denials;
- Eggsec integration still requires outer authorization before execution.

### Migration and compatibility tests

- all existing NSE feature combinations compile;
- TUI/Python feature forwarding works without direct runtime dependency;
- `eggsec::nse` exports required downstream symbols;
- `NseRunReport` serialization remains compatible.

## 11. Required verification commands

At minimum after Milestone 001 closure:

```bash
cargo tree -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo check -p eggsec --features nse,cli
cargo test -p eggsec --features nse,cli
cargo check -p eggsec-tui --features nse
cargo test -p eggsec-tui --features nse
cargo check -p eggsec-python --features nse
cargo test -p eggsec-python --features nse
make test-architecture-guards
make check
```

Because feature dependency wiring changes:

```bash
make check-features-individual
```

Also record a workspace manifest search proving direct production consumers.

If the exact guard target name differs, add the guard to the existing architecture-guard suite rather than creating an unintegrated script.

## 12. Documentation updates

- `architecture/overview.md` dependency/ownership map.
- NSE architecture/integration documentation.
- `docs/NSE_COMPATIBILITY.md` only where ownership/path descriptions change.
- contributor/AGENTS/skill guidance describing canonical NSE imports.
- feature documentation for TUI/Python forwarding if current comments imply a direct runtime dependency.
- plan registry and subsystem roadmap status after closure.

Do not modify grandfathered historical plan records.

## 13. Acceptance criteria

1. `eggsec-nse` has zero direct dependencies on any `eggsec-*` crate.
2. Runtime source imports no `eggsec_...` crate.
3. Eggsec report-envelope conversion is engine-owned and tested.
4. The Eggsec-specific transport coupling is removed from the runtime dependency graph without weakening authority/scope behavior.
5. Only `eggsec` directly depends on `eggsec-nse` among production workspace crates.
6. TUI and Python consume NSE through `eggsec::nse` and retain existing feature behavior.
7. Runtime corpus/tests run independently of Eggsec report/transport/engine crates.
8. Static guards prevent inward runtime dependencies and duplicate direct consumers from reappearing.
9. `nse`, `nse-ssh2`, and `nse-sandbox` build/behavior remains qualified.
10. `NseRunReport` compatibility remains intact.
11. `make check` and relevant feature/dependency checks pass.
12. Closure evidence explicitly recommends whether Milestone 003 physical extraction is ready.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- Milestone 001 is not closed or its closure record reports unresolved report/orchestration defects;
- removing an inward dependency requires moving Eggsec authorization into the runtime;
- removing `eggsec-transport` would weaken actual scope/authority behavior;
- a broad provider architecture is required across many libraries to proceed;
- a public compatibility break is required but not already approved/documented;
- another workspace crate has a legitimate direct-runtime ownership need that cannot be satisfied through the engine facade;
- work expands into creating/publishing the external repository.

## 15. Closure evidence required

The closure record must contain:

- Milestone 001 closure reference and readiness finding;
- implementation commit(s);
- before/after `eggsec-nse` dependency tree;
- complete inventory/disposition of prior `eggsec-*` imports;
- location/API of the moved report bridge;
- disposition of `http_capability.rs` and evidence that network semantics were not misrepresented/weakened;
- workspace direct-consumer before/after inventory;
- TUI/Python feature-forwarding evidence;
- architecture-guard evidence;
- runtime corpus result;
- `make check` and feature-matrix results;
- residual findings by severity;
- explicit go/no-go recommendation for standalone extraction.

## 16. Handoff notes

This is a boundary-cleanup pass, not a network rewrite.

The safest expected disposition is:

- runtime semantics stay in `eggsec-nse`;
- report conversion moves up into `eggsec`;
- Eggsec-specific transport adaptation moves up into `eggsec` or is removed if it is only a dormant migration seam;
- downstream UI/bindings import through `eggsec::nse`.

Do not delete broad NSE convenience APIs solely to obtain a smaller manifest. Extraction changes ownership first; API pruning, provider inversion, and package decomposition require separate evidence and plans.
