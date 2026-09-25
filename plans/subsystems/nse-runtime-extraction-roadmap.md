# NSE Runtime Extraction Roadmap

Status: active

Long-term references:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Related ADRs:

- none at roadmap creation; create an ADR only if extraction changes a durable cross-subsystem authorization, transport, or public-contract decision.

## 1. Purpose and ownership boundary

This workstream prepares `eggsec-nse` to become a scanner-independent, embeddable Rust NSE compatibility runtime while preserving Eggsec behavior and enforcement semantics.

The runtime owns NSE/Lua compatibility semantics: script and module resolution, execution profiles, limits and cancellation, Lua execution, NSE rule evaluation, library compatibility, capability decisions internal to the runtime, structured NSE reports, built-in scripts, the clean-room compatibility corpus, and NSE-specific convenience APIs.

Eggsec owns product authorization, dispatch, operator/frontend selection of execution profiles, conversion of NSE reports into Eggsec report envelopes, and any adapter from Eggsec transport/scope authority into future NSE host-provider interfaces.

The runtime MUST NOT become an alternate Eggsec authorization layer. Eggsec's canonical pre-dispatch enforcement remains outside the runtime. Conversely, the runtime must not depend on Eggsec engine/report/transport crates merely to express NSE behavior.

## 2. Work classification

### Invariants

- All Eggsec surfaces continue to pass through the canonical Eggsec enforcement and dispatch boundary before NSE execution.
- `eggsec-nse` does not decide whether an Eggsec operation is authorized.
- Equivalent NSE requests produce equivalent structured reports regardless of whether the caller is the runtime CLI helper, Eggsec TUI/dispatch, or Python binding.
- Script-file execution is resolved through the runtime resolver and its profile policy rather than ad-hoc filesystem reads.
- Existing execution-profile semantics remain intact: manual surfaces may use manual profiles; automated surfaces use explicit safe/strict profiles.
- The clean-room corpus remains provenance-tracked and no upstream Nmap script corpus is bundled without an explicit license decision.
- Existing Eggsec public feature names (`nse`, `nse-ssh2`, `nse-sandbox`, stress-testing propagation) remain source-compatible unless a later intentional compatibility decision says otherwise.

### Capabilities

- An embeddable NSE runtime can execute a resolved request and return one complete `NseRunReport`.
- Eggsec manual and programmable surfaces consume the same runtime result contract.
- A later standalone repository can be qualified independently from Eggsec.

### Infrastructure

- Canonical runtime request/execution API.
- One report-assembly path for rules, execution stats, library usage, capability events, resolver diagnostics, compatibility/fidelity, output, and evidence.
- Runtime/Eggsec bridge separation.
- Dependency guards preventing inward `eggsec-*` dependencies after decoupling.
- Downstream dependency consolidation through the `eggsec::nse` compatibility facade.

### Polish

- Compatibility documentation generated or reconciled from authoritative runtime data.
- Crate/repository metadata cleanup and standalone release documentation after the extraction gate passes.

## 3. Non-goals

- Do not claim full Nmap NSE compatibility.
- Do not copy or vendor the upstream Nmap script/nselib corpus as part of this workstream.
- Do not combine extraction with a package rename.
- Do not redesign all NSE networking before the runtime boundary is clean.
- Do not replace the existing 167-library compatibility surface as part of the first milestones.
- Do not broaden Eggsec authorization policy or create a second transport abstraction.
- Do not remove `public_api` or CVE helpers merely because they are not part of a minimal interpreter core; ownership can be revisited after standalone qualification.

## 4. Current state

At repository baseline `2ef67febf17a2ae42e2b8863b140f6c47cb7f387`, `crates/eggsec-nse` is already a substantial domain crate with resolver/profile/limit/capability/reporting infrastructure, Lua executors, NSE library implementations, convenience APIs, and a clean-room compatibility corpus.

The crate is not yet independently extractable for two reasons.

First, execution orchestration is duplicated. `run_cli_with_profile`, Eggsec dispatch/TUI execution, and the Python binding each assemble execution and `NseRunReport` data separately. Their behavior has drifted: the Eggsec dispatch path can bypass `ScriptResolver` for custom files and does not assemble all report sections; the Python path duplicates static `require` fallback logic and does not populate the exact same stats/evidence path as the runtime CLI helper.

Second, the runtime manifest still depends inward on `eggsec-core`, `eggsec-report-model`, and `eggsec-transport`. `bridge.rs` is an Eggsec report-model adapter and belongs in the engine. `http_capability.rs` is an Eggsec-transport-specific adapter whose own documentation records that the Lua HTTP-family libraries still use pre-cutover native/reqwest paths.

Direct consumers are also wider than necessary: `eggsec` correctly re-exports `eggsec_nse` as `eggsec::nse`, but `eggsec-tui` and `eggsec-python` still declare direct optional `eggsec-nse` dependencies. Python source already primarily consumes the engine facade, so that direct edge is unnecessary.

## 5. Target architecture

The end state is:

```text
                    +--------------------------+
                    |      standalone          |
                    |       eggsec-nse          |
                    |--------------------------|
                    | resolver / profiles      |
                    | Lua runtime / rules      |
                    | limits / cancellation    |
                    | capability broker        |
                    | NSE libraries            |
                    | compatibility corpus     |
                    | NseRunRequest            |
                    | NseRunReport              |
                    +-------------+------------+
                                  |
                                  | public runtime API
                                  v
+-------------+        +----------+-----------+        +----------------+
| CLI / TUI   |------->|        eggsec        |<-------| Python / APIs  |
| manual auth |        | auth + dispatch      |        | strict/manual  |
+-------------+        | NSE report bridge    |        +----------------+
                       | optional host adapter |
                       +----------+------------+
                                  |
                                  v
                       eggsec report/transport
```

The canonical runtime entry point may use different final type names, but it MUST have the semantic shape:

```rust
pub struct NseRunRequest {
    pub target: String,
    pub script: NseScriptSource,
    pub script_args: Option<String>,
    pub profile: ResolvedNseExecutionProfile,
    pub host_context: Option<NseHostContext>,
    pub port_context: Option<NsePortContext>,
}

pub fn execute(request: NseRunRequest) -> Result<NseRunReport, NseRunError>;
```

Async or builder variants are acceptable. What matters is that one runtime-owned path performs resolution, execution, rule evaluation, report assembly, compatibility computation, and evidence extraction.

After dependency decoupling, the intended direct dependency graph is:

```text
eggsec-nse
    ^
    |
 eggsec
  ^  ^  ^
  |  |  |
 CLI TUI Python
```

TUI and Python should consume NSE types through `eggsec::nse`, not by declaring their own direct runtime dependency.

## 6. Dependency graph

```text
Milestone 001 — canonical execution/report convergence
    |
    | hard
    v
Milestone 002 — runtime dependency decoupling + consumer consolidation
    |
    | hard
    v
Milestone 003 — standalone repository extraction + git-revision qualification
    |
    | operational
    v
Milestone 004 — release/publish + Eggsec versioned dependency adoption
    |
    | soft
    v
Milestone 005 — provider inversion / deeper runtime portability
```

Milestones 003-005 are roadmap items only. They must receive their own implementation plans when preceding closure evidence exists.

## 7. Milestones

### Milestone 001 — Canonical execution and report convergence

Class: infrastructure

Objective: create one authoritative runtime-owned execution pipeline and migrate all current NSE callers to it without changing authorization or intended execution-profile defaults.

Dependencies: none beyond the current closed Eggsec foundation.

Deliverable boundary: runtime request/result API, resolver-first script handling, consistent report assembly, and migrated CLI helper/Eggsec/Python call sites.

User or operator value: the same NSE request no longer changes report fidelity depending on which supported surface invokes it.

Exit conditions:

- no production surface independently reimplements the runtime orchestration/report sequence;
- file sources are resolved through `ScriptResolver`;
- successful reports consistently carry rules, stats, resolver diagnostics, library usage, capability events, compatibility/fidelity, output, and extracted evidence;
- compatibility tests demonstrate manual and Python surface parity over representative fixtures;
- `make check` is green.

Deferred work: removing inward Eggsec dependencies, physical extraction, provider inversion.

### Milestone 002 — Runtime dependency decoupling and consumer consolidation

Class: infrastructure

Objective: make the in-tree `eggsec-nse` crate independently ownable by removing dependencies on Eggsec engine/report/transport crates and reducing direct consumers to `eggsec`.

Dependencies: hard dependency on Milestone 001 closure.

Deliverable boundary: move the report bridge into Eggsec, isolate or replace the Eggsec-specific transport adapter, add dependency guards, migrate TUI/Python to the engine facade, and prove the runtime crate has zero `eggsec-*` dependencies.

User or operator value: no intended visible behavior change; this is the extraction-readiness boundary.

Exit conditions:

- `cargo tree -p eggsec-nse` contains no workspace `eggsec-*` dependency;
- `bridge.rs` functionality is engine-owned with compatibility paths preserved where appropriate;
- only `eggsec` directly depends on `eggsec-nse`;
- TUI/Python feature behavior remains unchanged;
- clean-room corpus and runtime tests pass independently;
- architecture guards prevent inward dependency regression.

Deferred work: repository split and publication.

### Milestone 003 — Standalone repository extraction and cross-repository qualification

Class: infrastructure

Objective: move the now-independent runtime into its own repository without semantic changes.

Dependencies: hard dependency on Milestone 002 closure.

Deliverable boundary: standalone repository, explicit dependency versions, CI, license/provenance assets, and Eggsec consumption pinned to an exact Git revision/tag during qualification.

Exit conditions: standalone CI and Eggsec full NSE qualification both pass against the exact external revision.

Deferred work: crates.io publication and host-provider redesign.

### Milestone 004 — Versioned release and Eggsec adoption

Class: capability

Objective: publish a semver-tagged runtime release and replace temporary git-revision consumption with the released dependency.

Dependencies: hard dependency on Milestone 003; operational dependency on release infrastructure and package-name availability.

Deliverable boundary: release metadata, versioned dependency, reproducible package verification, and documentation.

Exit conditions: published artifact is reproducible/qualified and Eggsec consumes the released version.

### Milestone 005 — Host provider inversion and portability hardening

Class: infrastructure

Objective: replace remaining direct side-effect implementations with narrow provider interfaces where doing so improves embeddability and authority preservation.

Dependencies: soft dependency on standalone release; may be split further after measurement.

Deliverable boundary: narrow network/filesystem/DNS/clock/random/process provider seams, a native default implementation, and an Eggsec host adapter where useful.

Exit conditions: no monolithic host trait, current behavior preserved, and provider injection is justified by tests or concrete consumers.

## 8. Cross-cutting requirements

### Storage and migration

No persistent schema migration is expected in Milestones 001-002. Serialized `NseRunReport` compatibility must be measured and preserved unless an intentional versioned schema change is approved.

### Protocol and compatibility

Preserve public Eggsec features and the `eggsec::nse` facade. Treat `NseRunReport` JSON shape as a compatibility contract during convergence. Do not silently remove report fields or change profile defaults.

### Security and authorization

Eggsec authorization remains authoritative outside the runtime. Runtime profiles/capability checks are defense-in-depth and NSE execution policy, not replacements for `EnforcementContext`. Automated Eggsec surfaces must not gain access to manual-permissive constructors as a side effect of API convergence.

Script and module resolution must preserve canonical-path containment, symlink-escape rejection, size/extension policy, and profile-based source restrictions.

### Concurrency, cancellation, and recovery

Existing `NseCancellationToken` and execution-limit semantics must survive convergence. Any async wrapper added for orchestration must preserve cancellation and bounded execution; spawned Tokio tasks follow repository timeout requirements. No background durable state is introduced by the first two milestones.

### Observability and audit

One report path must retain profile, compatibility/fidelity, capability-denial/event, rule, resolver, execution-stat, and evidence information. Eggsec audit/report conversion remains outside the runtime.

### Performance and resource use

Convergence must not add an interpreter per reporting step or duplicate script/module loading. Baseline representative corpus runtime should be captured before physical extraction so later cross-repository qualification can detect regressions.

### Documentation and operations

Keep `docs/NSE_COMPATIBILITY.md`, architecture NSE documentation, feature docs, and compatibility-corpus provenance aligned with the canonical pipeline. Historical claims about library counts or support levels should be generated or reconciled from authoritative registries where practical.

## 9. Verification strategy

Every milestone requires `make check`.

Milestone 001 additionally requires focused `eggsec-nse` tests across resolver, profile, rules, limits, reports, runtime smoke/corpus, sandbox, and local protocol fixtures; Eggsec NSE dispatch/TUI tests; and Python NSE tests under the `nse` feature.

Milestone 002 additionally requires dependency-tree and static ownership guards, feature-matrix checks for `nse`, `nse-ssh2`, and `nse-sandbox`, and explicit verification that TUI/Python no longer directly depend on `eggsec-nse`.

Milestones 003-004 must qualify both repositories against an exact runtime revision/release before switching dependency source.

Deep checks such as `make check-features-individual` are required when feature wiring changes.

## 10. Risks and decision points

- The existing NSE convenience API surface is broad. Do not shrink it during extraction preparation merely to simplify ownership.
- `http_capability.rs` is not yet the active backend for all Lua HTTP-family libraries; treating it as if it were would produce a false transport-cutover claim.
- Provider inversion could expand scope substantially. Keep it after dependency decoupling unless a concrete inward dependency cannot otherwise be removed.
- Nmap script/nselib licensing and provenance remain an explicit packaging constraint. The clean-room corpus is the default distributable qualification asset.
- If canonical orchestration requires an incompatible public `NseRunReport` change, stop and record the compatibility decision rather than silently altering serialized output.

No ADR is required for the first two milestones if they preserve current Eggsec authorization and transport ownership. Create one if implementation proposes moving authorization into the runtime, changing the durable transport contract, or changing a cross-repository public contract in a way not already covered by the canonical specification.

## 11. Completion definition

This roadmap is complete when:

1. one authoritative runtime execution/report pipeline is used by supported surfaces;
2. the runtime crate has no inward `eggsec-*` dependency;
3. only Eggsec directly consumes the external runtime and downstream Eggsec surfaces use the engine facade;
4. the runtime exists in a standalone repository with independent CI and provenance assets;
5. Eggsec qualifies and consumes a versioned standalone release;
6. all public feature/profile/report contracts are preserved or intentionally versioned;
7. closure records document dependency, compatibility, security, and corpus evidence.

## 12. Milestone status

| Milestone | Status | Implementation plan | Closure record | Blockers |
|---|---|---|---|---|
| 001 canonical execution/report convergence | ready | `plans/implementation/nse-runtime-extraction/001-canonical-execution-report-convergence.md` | — | — |
| 002 runtime dependency decoupling + consumer consolidation | blocked | `plans/implementation/nse-runtime-extraction/002-runtime-dependency-decoupling.md` | — | Milestone 001 closure |
| 003 standalone repository extraction | not started | — | — | Milestone 002 closure |
| 004 versioned release + Eggsec adoption | not started | — | — | Milestone 003 qualification |
| 005 provider inversion / portability hardening | not started | — | — | post-extraction evidence |
