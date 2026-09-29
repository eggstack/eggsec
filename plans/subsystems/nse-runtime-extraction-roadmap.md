# NSE Runtime Extraction Roadmap

Status: active — Milestones 001-006 closed; M007 protocol-library gating and controlled automated activation active/planned

Long-term references:

- `plans/000-long-term-specification.md#2-primary-product-goals`
- `plans/000-long-term-specification.md#5-crate-ownership`
- `plans/001-terminology-and-domain-model.md#3-execution-terms`
- `plans/001-terminology-and-domain-model.md#5-data-and-reporting-terms`
- `plans/002-long-term-roadmap.md#phase-7--standing-maintenance-and-future-capability-open`

Related ADRs:

- `plans/adrs/ADR-0003-nse-host-provider-boundary.md` — accepted provider/broker boundary for Milestone 005.
- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md` — accepted effect-gating, scope-bearing strict execution, and selective automated activation boundary for Milestone 007.
- `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md` remains controlling for the Eggsec-owned HTTP transport adapter.
- `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md` remains unaffected.

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

**Status update (Milestones 001-002 closed):** both reasons above are resolved. `execute_nse_run` is the single canonical pipeline (001 closure), and `crates/eggsec-nse` now has zero `eggsec-*` dependencies in manifest, source, and tests, with `nse_bridge`/`nse_http_capability` owned by the engine and only `eggsec` consuming the runtime (002 closure). Guards 144/145/146 prevent regression. The crate is therefore extraction-ready in-tree; the remaining work is the physical split itself (Milestone 003), subject to the two preconditions recorded in 002 closure §16.

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
    |
    | release
    v
Milestone 006 — 0.2.0 release + safe Eggsec adoption
    |
    | security activation boundary
    v
Milestone 007 — effect-gated protocols + scope-bearing automated activation
```

Milestones 001-006 are closed. Milestone 007 is active/planned under ADR-0004. It uses the adopted 0.2.0 provider surface, M005E residual inventories, M006 dormant adapter, and M006 quarantine markers as fixed inputs.

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

Status: closed. Closure evidence: `plans/closure/nse-runtime-extraction/003-closure.md`. Standalone revision: `3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2`.

Class: infrastructure

Objective: move the now-independent runtime into its own repository without semantic changes.

Dependencies: hard dependency on Milestone 002 closure.

Deliverable boundary: standalone repository, explicit dependency versions, CI, license/provenance assets, and Eggsec consumption pinned to an exact Git revision/tag during qualification.

Exit conditions: standalone CI and Eggsec full NSE qualification both pass against the exact external revision.

Deferred work: crates.io publication and host-provider redesign.

### Milestone 004 — Versioned release and Eggsec adoption

Status: closed. Closure evidence: `plans/closure/nse-runtime-extraction/004-closure.md`. Published artifact: `eggsec-nse 0.1.0` (crates.io) from standalone `9982c7fc060bb8d9cfe7b5549ba3f8a83336c00c`, tag `v0.1.0`; Eggsec consumes the registry release.

Class: capability

Objective: publish a semver-tagged runtime release and replace temporary git-revision consumption with the released dependency.

Dependencies: hard dependency on Milestone 003 (closed); operational dependency on release infrastructure and package-name availability (satisfied in-milestone).

Deliverable boundary: release metadata, versioned dependency, reproducible package verification, and documentation.

Exit conditions: published artifact is reproducible/qualified and Eggsec consumes the released version.

### Milestone 005 — Host provider inversion and portability hardening

Class: infrastructure

Objective: move selected runtime host side effects behind narrow, per-run provider interfaces so capability policy, cancellation/resource accounting, actual host execution, deterministic testing, authority preservation, and platform specialization share one auditable boundary.

Status: closed (`plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`). M005 implementation is landed on `eggsec-nse/main@9fe149fbb22480a63e254e91d60083b7a29a8ff4` with a fully green hosted post-merge run (`36490773625`: Ubuntu/macOS/Windows/MSRV/SSH all success); the `0.2.0` release/adoption follow-up is dependency-ready.

Dependencies: Milestone 004 is closed. Child dependencies are explicit below.

Provider/broker invariant:

```text
runtime capability decision
-> cancellation/resource preflight
-> narrow provider operation
-> resource accounting
-> capability/report event
```

Provider mechanics never authorize an Eggsec operation. Eggsec's canonical enforcement and `NetworkAuthority` remain external to `eggsec-nse`.

#### M005A — Provider broker foundation and deterministic host services

Status: **closed** (`plans/closure/nse-runtime-extraction/005a-closure.md`). Implementation `675269e` is an ancestor of `eggsec-nse/main@9fe149f`; the 005A provider broker surface and tests ship with the 0.2.0 release/adoption milestone.

Plan: `plans/implementation/nse-runtime-extraction/005-provider-broker-foundation.md` (status: implemented)

Boundary: establish the per-run service bundle, native defaults, additive request injection, broker sequencing, and low-risk clock/random/environment providers with deterministic/concurrent tests.

Exit gate: provider injection is proven on real NSE execution without changing existing caller construction or profile/report semantics.

#### M005B — Authority-preserving network and DNS providers

Status: **closed** (`plans/closure/nse-runtime-extraction/005b-closure.md`). Implementation `0ac9737` is an ancestor of `eggsec-nse/main@9fe149f`; the runtime-neutral DNS/TCP/UDP providers and the resolve-authorize-connect identity contract ship with the 0.2.0 release/adoption milestone.

Plan: `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md` (status: implemented)

Boundary: runtime-neutral DNS/TCP/UDP providers, opaque/runtime-owned handle types, resolve-authorize-connect identity, and migration of the shared/core `socket`/`comm`/`nmap`/`dns` paths.

Exit gate: restricted hostname/CIDR policy selects a concrete allowed endpoint and connects to that same endpoint; actual provider calls own cancellation/accounting; remaining specialized network bypasses are explicitly inventoried.

#### M005C — HTTP provider and Eggsec scoped-transport adapter

Status: **closed** (`plans/closure/nse-runtime-extraction/005c-closure.md`). Standalone implementation `89290f9` is an ancestor of the released `eggsec-nse 0.2.0` source; M006B replayed the Eggsec-side scoped-transport adapter onto current Eggsec `main` as a dormant, engine-owned module with no production caller. Controlled adapter activation remains deferred to M007.

Plan: `plans/implementation/nse-runtime-extraction/005-http-provider-eggsec-adapter.md`

Boundary: runtime-neutral HTTP DTO/provider contract, native reqwest provider, migration of the HTTP-family libraries where parity permits, and activation of Eggsec's existing `HttpTransport` + `NetworkAuthority` seam as an engine-owned provider adapter.

Exit gate: Eggsec-injected HTTP execution carries existing approved authority with no native fallback on denial, while standalone remains Eggsec-independent.

#### M005D — Filesystem, process, and cross-platform host portability

Status: **closed** (`plans/closure/nse-runtime-extraction/005d-closure.md`). Implementation `b3c43b8` is an ancestor of `eggsec-nse/main@9fe149f`; the filesystem/process providers, per-run virtual CWD, and Windows compile-only qualification ship with the 0.2.0 release/adoption milestone.

Plan: `plans/implementation/nse-runtime-extraction/005-filesystem-process-portability.md` (status: implemented)

Boundary: narrow filesystem/process providers, runtime-owned metadata/process DTOs and opaque file handles, per-run virtual CWD, localized Unix/Windows mechanics, and Windows CI qualification.

Exit gate: shared/core filesystem/process paths are brokered, process-global CWD mutation is removed, and declared Windows build/check support is green.

#### M005E — Provider coverage qualification and parent-milestone closure

Status: **closed** (`plans/closure/nse-runtime-extraction/005e-closure.md`). Implementation `c81d84c` is an ancestor of `eggsec-nse/main@9fe149f`; the source-audited host-side-effect inventory, M005 provider composition tests, M005E pinned residual inventories, and the send-accounting correction remain valid. The 0.2.0 recommendation (§11) stands as release-planning input; release/adoption is now dependency-ready (hosted run `36490773625` green on main `9fe149f`; see `plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`).

Plan: `plans/implementation/nse-runtime-extraction/005-provider-coverage-qualification.md`

Boundary: source-derived host-side-effect inventory, cross-domain provider composition, authority/accounting/cancellation qualification, portability matrix, Eggsec consumer verification, documentation reconciliation, and release/versioning recommendation.

Exit gate: all direct host operations in the audited source scope are classified; provider-backed claims match source and guards; no high-severity bypass remains; parent M005 receives an evidence-backed closure disposition.

Child dependency graph:

```text
005A provider/broker foundation
  |\
  | \
  v  v
005B 005D
  |
  v
005C
  \   /
   \ /
   005E qualification/closure
```

Parent Milestone 005 exit conditions:

- no monolithic host trait;
- current native-default behavior remains source-compatible;
- provider-backed operations use one capability-aware broker sequence;
- network resolution/authorization/connection preserves concrete endpoint identity;
- Eggsec provider adapters remain engine-owned and carry existing authority rather than reconstructing it;
- per-run provider/CWD state is isolated;
- Windows qualification is explicit;
- residual specialized direct host operations are inventoried rather than hidden;
- closure determines the semver/release follow-up for the newly public provider surface.


### Milestone 006 — 0.2.0 release and safe Eggsec adoption

Status: closed (`plans/closure/nse-runtime-extraction/006c-closure.md`; 006A `006a-closure.md`, 006B `006b-closure.md`).

Class: capability + infrastructure

Objective: publish the M005 provider surface as `eggsec-nse 0.2.0`, move Eggsec to the registry artifact, replay the qualified engine-owned HTTP adapter against the released contract, and close the cross-repository release boundary without broadening automated NSE execution before protocol-wide capability gating exists.

Why this milestone exists:

- M005E recommends 0.2.0 because `register_vulns_library` has a public 0.x breaking signature change;
- M005 adds substantial provider APIs and corrected send/write accounting;
- the standalone release workflow still needs the ripgrep prerequisite that normal CI already enforces;
- Eggsec's staged HTTP adapter is complete but based on an older branch and must be replayed against the real registry release;
- M005E proved a 72-file ungated specialized direct-I/O residual, so production automated NSE activation must remain deferred.

#### M006A — Standalone 0.2.0 release preparation and publication

Status: **closed** (`plans/closure/nse-runtime-extraction/006a-closure.md`).

Plan: `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md` (status: implemented)

Boundary: version/changelog/release-workflow preparation, exact release-candidate qualification, crates.io publication, registry/docs.rs verification, and immutable `v0.2.0` source identity.

Exit gate: 0.2.0 is published from a fully qualified source commit and clean scratch consumers resolve the crates.io artifact.

Closure: published `eggsec-nse 0.2.0` from `ff0d2c0` (hosted run `36521717829` green on Ubuntu/macOS/Windows/MSRV/SSH; archive VCS identity = tag `v0.2.0` commit; docs.rs built). Release-workflow ripgrep prerequisite fixed; `-D warnings` overclaim reconciled.

#### M006B — Eggsec 0.2.0 adoption and safe adapter staging

Status: **closed** (`plans/closure/nse-runtime-extraction/006b-closure.md`).

Plan: `plans/implementation/nse-runtime-extraction/006-eggsec-0-2-0-adoption-safe-staging.md` (status: implemented)

Boundary: registry dependency/lockfile adoption, replay of the staged `NseHttpTransportProvider` onto current Eggsec main, automated NSE exposure quarantine, manual/TUI/Python requalification, and guard/doc updates.

Exit gate: Eggsec consumes crates.io 0.2.0, the adapter compiles/tests but has no production caller, and automated NSE remains fail-closed/manual-only pending M007.

Closure: registry 0.2.0 adopted (no override), adapter replayed logic-identical with guard-pinned dormancy, three-layer quarantine with manual/TUI preserved, full checks green.

#### M006C — Cross-repository qualification and closure

Status: **closed** (`plans/closure/nse-runtime-extraction/006c-closure.md`).

Plan: `plans/implementation/nse-runtime-extraction/006-cross-repo-qualification-closure.md` (status: implemented)

Boundary: verify release/tag/archive identity, registry-only Eggsec consumption, dormant adapter source truth, automated-exposure quarantine, full consumer checks, and documentation/guard reconciliation.

Exit gate: M006 closes only if the published artifact and Eggsec consumer are fully qualified and no production path activates automated NSE before protocol gating.

Child dependency graph:

```text
006A standalone 0.2.0 release
  |
  v
006B Eggsec adoption + dormant adapter
  |
  v
006C cross-repo qualification / closure
```

Parent Milestone 006 exit conditions:

- `eggsec-nse 0.2.0` is published from an immutable, fully qualified source commit;
- Eggsec resolves the crates.io 0.2.0 artifact with no Git/path override;
- the staged HTTP adapter is replayed onto current main and tested against 0.2.0;
- automated NSE is not exposed to MCP/REST/agent/gRPC while the M005E ungated residual remains;
- manual/TUI NSE behavior remains compatible;
- documentation explicitly states that provider availability is not complete protocol-wide scope enforcement;
- closure sequences the next milestone: protocol-library capability gating + approved-scope/profile threading + controlled adapter activation.

### Milestone 007 — Protocol-library gating and controlled automated activation

Status: active/planned.

Class: security + infrastructure

Objective: make automated NSE structurally fail-closed at the library/effect boundary, migrate the protocol cohort that fits the existing provider contract, publish that standalone hardening, thread Eggsec's approval-time scope/target facts into scoped runtime providers, and only then selectively restore automated NSE discoverability.

Controlling decision:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`.

Why this milestone exists:

- M005E proved a 72-file ungated direct-I/O residual plus 25 advisory-gated/mixed files;
- M006 correctly bounded that residual by making NSE manual-only on automated Eggsec surfaces;
- the runtime pre-registers many Lua globals, so a require-only deny list is insufficient;
- the existing connected DNS/TCP/UDP provider surface can migrate a meaningful blocking cohort without redesigning every protocol;
- the standalone native HTTP provider is not sufficient for an automated hostname/redirect authority claim;
- Eggsec already has an approval-bound `Scope` snapshot, canonical `ScopeAuthority`, scoped Eggfetch transport, and a tested dormant NSE HTTP adapter.

#### M007A — Automated library effect gate and HTTP authority assurance

Status: **closed** (`plans/closure/nse-runtime-extraction/007a-closure.md`; standalone `c9df4d1`).

Plan: `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md`

Implementation repository: `eggstack/eggsec-nse`.

Boundary: complete effect classification for every registered Lua library/global, AgentSafe/CiSafe registration + require gating, unknown-deny behavior, and additive HTTP provider authority assurance.

Exit gate: unsafe/direct/advisory libraries are unreachable under automated profiles through both globals and `require()`; native HTTP is not implicitly authority-bound.

#### M007B — Broker-compatible protocol migration and residual hardening

Status: **closed** (`plans/closure/nse-runtime-extraction/007b-closure.md`; standalone `d4a22f1dbe56f4ccfb17b2a8135aae8395f44f19`, hosted run `36640412317` green on 5/5 jobs). Closed through the corrective plan `plans/implementation/nse-runtime-extraction/007-protocol-migration-corrective.md` after a corrective pass in which the plan's central premise was falsified: 16 of the 17 "still-direct `BrokerCompatible*`" entries had no direct socket effect (the M005E scan substring-matched `BrokeredTcpStream::connect`), and the 17th (`radius`) carried a defect different from the one described. The corrected scan then found four real defects the plan did not anticipate, including a high-severity one: `target.resolve` performed unbrokered DNS while the manifest classified `target` as `Pure`, so an automated-safe registered library could make `CiSafe` emit DNS.

Plan: `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md` (implemented; corrective closure via `007-protocol-migration-corrective.md`)

Implementation repository: `eggstack/eggsec-nse`.

Boundary: classify all 72+25 residual files by effect shape, add an internal brokered stream compatibility layer, migrate all blocking TCP/compatible connected-UDP files supported by the existing provider contract, and reconcile residual pins/eligibility.

Exit gate (met): every promoted module has all automated-relevant network effects provider-backed; residual counts shrink and every remaining file has an explicit manual-only migration class.

Outcome: specialized direct-I/O residual **97 → 22** (15 ungated + 7 advisory), every remaining entry a shape the current provider contract cannot represent; effect-manifest manual-only **106 → 41**, `ProviderBacked` **18 → 84**; M005E baseline (97) frozen in `scripts/nse-m005e-direct-io-baseline.txt` and guard-enforced; manifest↔registration consistency enforced in both directions with an explicit compat allowlist; zero `BrokerCompatible*` entries retain an unexplained direct effect. Eggsec automated NSE unchanged and still quarantined.

#### M007C — Standalone security patch release

Status: **ready for handoff** (unblocked: `007b-closure.md` accepted and hosted standalone CI fully green on the closure SHA).

Plan: `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md`

Implementation repository: `eggstack/eggsec-nse`.

Boundary: public-API compatibility audit, expected 0.2.1 patch publication (or stop/replan as 0.3.0 on any breaking change), exact candidate CI/package qualification, registry/tag/docs.rs verification.

Exit gate: Eggsec has a published immutable registry artifact containing the M007 runtime hardening.

#### M007D — Eggsec approved-scope/profile threading and scoped provider activation

Status: **blocked on accepted M007C closure**.

Plan: `plans/implementation/nse-runtime-extraction/007-approved-scope-provider-activation.md`

Implementation repository: `eggstack/eggsec`.

Boundary: adopt the M007 runtime release; bind approval-time `TargetScope` facts into `ApprovedExecution`; extract generic owned scope authority; implement scoped DNS/TCP/UDP providers; compose the authority-bound HTTP adapter with Eggfetch; map Eggsec strict profiles to NSE safe profiles; route only `execute_approved_execution()` through scoped services.

Exit gate: scope-bearing strict NSE can execute a safe provider-backed fixture with no native fallback, while the scope-less strict entry still rejects NSE and automated metadata remains quarantined.

#### M007E — Controlled automated re-exposure and cross-repository qualification

Status: **blocked on accepted M007D closure**.

Plan: `plans/implementation/nse-runtime-extraction/007-controlled-automated-reexposure-qualification.md`

Implementation repositories: `eggstack/eggsec` + `eggstack/eggsec-nse`.

Boundary: deliberately re-enable MCP/REST/agent/gRPC NSE metadata, replace dormancy guards with exact canonical-call-site guards, run safe positive and zero-contact negative scope/effect tests, reconcile residual counts/docs, and close M007.

Exit gate: automated NSE is discoverable only through the approved scope-bearing path; unsafe/manual-only libraries remain unreachable; scope/port/DNS/redirect negatives make zero unapproved host contact; full local + hosted qualification is green.

Child dependency graph:

```text
007A effect gate + HTTP authority assurance
  |
  v
007B broker-compatible protocol migration
  |
  v
007C standalone security patch release
  |
  v
007D Eggsec approved-scope/provider activation
  |
  v
007E controlled re-exposure + qualification
```

Parent Milestone 007 exit conditions:

- every registered runtime library/global has an automated eligibility/effect classification;
- unknown/direct/advisory libraries are structurally unavailable under AgentSafe/CiSafe;
- the broker-compatible residual cohort is migrated and remaining direct-I/O classes are explicit/manual-only;
- Eggsec consumes the qualified M007 standalone registry release;
- `ApprovedExecution` retains the approval-time scope snapshot and target facts used for strict NSE construction;
- strict NSE HTTP/TCP/UDP use scoped provider composition with no native fallback;
- `execute_approved()` remains unable to execute NSE without the scope-bearing bundle;
- automated metadata is re-enabled only after the path above is qualified;
- safe automated fixtures succeed and unsafe residual/scope-negative fixtures fail before unapproved host contact;
- manual CLI/TUI/Python compatibility remains green;
- closure records before/after residual counts and any M008 long-tail migration recommendation.


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
| 001 canonical execution/report convergence | closed | `plans/implementation/nse-runtime-extraction/001-canonical-execution-report-convergence.md` | `plans/closure/nse-runtime-extraction/001-closure.md` | — |
| 002 runtime dependency decoupling + consumer consolidation | closed | `plans/implementation/nse-runtime-extraction/002-runtime-dependency-decoupling.md` | `plans/closure/nse-runtime-extraction/002-closure.md` | — |
| 003 standalone repository extraction | closed | `plans/implementation/nse-runtime-extraction/003-standalone-repository-extraction.md` | `plans/closure/nse-runtime-extraction/003-closure.md` | — |
| 004 versioned release + Eggsec adoption | closed | `plans/implementation/nse-runtime-extraction/004-versioned-release-and-eggsec-adoption.md` | `plans/closure/nse-runtime-extraction/004-closure.md` | — |
| 005 provider inversion / portability hardening | closed | `plans/implementation/nse-runtime-extraction/005-post-merge-ci-fixture-corrective.md` | fixture corrective closure + amended landing closure; provider implementation landed on standalone main | hosted run `36490773625` green on exact main SHA; 0.2.0 release/adoption dependency-ready |
| 006 0.2.0 release + safe Eggsec adoption | closed | `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md` (006A; 006B-C linked in §7) | `plans/closure/nse-runtime-extraction/006a-closure.md`, `006b-closure.md`, `006c-closure.md` | M007 active/planned; automated activation remains gated |
| 007 protocol-library gating + controlled automated activation | active | `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md` | `plans/closure/nse-runtime-extraction/007a-closure.md` (007A); `plans/closure/nse-runtime-extraction/007b-closure.md` (007B, closed via corrective pass; standalone `d4a22f1`, hosted run `36640412317` green) | M007C is current handoff; M007D-E remain dependency-gated by ADR-0004; automated NSE stays quarantined until 007E |
