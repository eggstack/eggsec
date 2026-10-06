# Eggsec Active Planning Registry

This file is the compact control surface for planning. Detailed requirements and completed history remain in source roadmaps, implementation plans, closure evidence, and Git history. It links to source documents rather than duplicating their content.

Canonical direction remains in:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

Durable decisions: `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`, `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md`.

Flat-era files (2026-07 – 2026-09) are grandfathered at the `plans/` top level with inline `Status: Executed` markers and appended completion records. They are immutable historical records referenced by `architecture/*.md`; they are NOT rewritten to the new templates. All new work follows `plans/003-planning-process.md` with new files under `plans/subsystems/`, `plans/implementation/<subsystem>/`, and `plans/closure/<subsystem>/`.

## Status vocabulary

- **proposed** — roadmap or plan exists but is not approved for execution.
- **ready** — dependencies and interfaces are satisfied; plan may be handed off.
- **active** — implementation or closure work is in progress.
- **blocked** — a named dependency or evidence requirement prevents progress.
- **closing** — implementation landed and closure evidence is being gathered.
- **closed** — closure record accepted (flat-era: inline completion record).
- **conditionally closed** — substantial work landed, but a named correctness or operational evidence condition remains.
- **superseded** — replaced by another document.
- **archived** — no longer active and retained for traceability.

## Subsystem standing

| Subsystem | Status | Roadmap(s) | Latest milestone / evidence |
|---|---|---|---|
| architecture-convergence | closed | `plans/architecture-convergence-roadmap-2026-09-08.md` | Phase G closure/measurement; completion records in each phase file |
| dependency-architecture-simplification | closed | `plans/dependency-architecture-simplification-roadmap.md` | Phase J + dispatch-profile parity corrective pass; closure report |
| crate-boundary-ownership | closed | `plans/crate-boundary-consolidation-roadmap-2026-09-16.md` | Phase D closes the roadmap. **Reopened as a bounded new workstream:** Phase G of `architecture/capability_segregation.md` delivered three knowledge-corpus leaf crates under `plans/subsystems/security-knowledge-corpus-roadmap.md` (ADR-0005; closed 2026-10-05, ADR-0006 defers publication); no prior rejection is reopened |
| security-knowledge-corpus | **closed** | `plans/subsystems/security-knowledge-corpus-roadmap.md`; `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md`; `plans/adrs/ADR-0006-knowledge-corpus-publication.md` | **All five milestones closed 2026-10-05.** 001 redaction deleted (guard 147); 002 `eggsec-service-db` (guards 114/148); 003 `eggsec-secrets` (guards 148/149); 004 `eggsec-payloads` (guards 148/150 + `fuzzer_payload_corpus_seam`); 005 **publication deferred** by ADR-0006. Three internal `publish = false` leaf crates with permanent engine facades, zero consumer diffs, and no regression in the corpus test contracts (verified 2026-10-06: payloads 252 / service-db 21 / secrets 11 / udp-scan 23 — all four crates are wired into `make check`) |
| network-transport-egress | closed | `plans/network-dependency-hardening-roadmap-2026-09-11.md`; `plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md` | Eggress 1.0.10 adoption + metadata closure (2026-09-25, implementation `fe5ec2d2`); ADR-0001, ADR-0002 controlling |
| frontend-runtime-tui | active | `plans/frontend-runtime-convergence-roadmap-2026-09-10.md`; `plans/tui-terminal-ownership-corrective-roadmap-2026-09-19.md` | Reopened for `plans/implementation/frontend-runtime-tui/001-pipeline-session-registry-and-resume-picker.md` (scan session store + TUI resume picker) |
| python-programmability | closed | `plans/python-library-roadmap.md`; `plans/python-api-completion-roadmap.md`; `plans/python-api-high-value-roadmap.md`; `plans/python-api-release-5-roadmap.md` | Release 5 phase F compatibility/performance/release closure |
| ci-verification-release | closed | `plans/ci-verification-release-simplification-roadmap.md`; `plans/ci-release-simplification-corrective-closure-index.md` | Phase K evidence-toolchain polish |
| performance-resource-efficiency | closed | `plans/performance-resource-efficiency-roadmap-2026-09-21.md` | Phase F + closure-polish corrective pass |
| daemon-protocol-agent | closed | (single-plan workstream) `plans/ws6-daemon-schema-parity.md` | Executed |
| nse-runtime-extraction | active | `plans/subsystems/nse-runtime-extraction-roadmap.md` | Milestones 001-006 closed; M007A and M007B closed (M007B via a corrective pass whose plan premise the audit falsified); M007C **blocked** (public-API gate found a 74-function major break; 0.2.1 not published); M007C-R **closed** (`eggsec-nse 0.3.0` published from candidate `16cb38e`); M007C-S **closed** (advisory `GHSA-w2g3-v83j-frp2` published, `0.1.0` yanked, `0.2.0` yank deferred to M007D); **M007D is the current handoff**; M007E dependency-gated; automated NSE remains quarantined until 007E |

## Grandfathered file index

### architecture-convergence (closed)

- `plans/architecture-convergence-roadmap-2026-09-08.md`
- `plans/architecture-convergence-phase-a-scope-contract-unification.md`
- `plans/architecture-convergence-phase-b-feature-build-verification-reconciliation.md`
- `plans/architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md`
- `plans/architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md`
- `plans/architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md`
- `plans/architecture-convergence-phase-f-platform-integration-maturity.md`
- `plans/architecture-convergence-phase-g-closure-measurement-documentation.md`

### dependency-architecture-simplification (closed)

- `plans/dependency-architecture-simplification-roadmap.md`
- `plans/dependency-architecture-phase-a-authorization-target-binding.md` through `plans/dependency-architecture-phase-j-measurement-and-closure.md`
- `plans/dependency-architecture-simplification-closure-report.md`
- `plans/dependency-architecture-corrective-closure-pass.md`
- `plans/dependency-architecture-final-polish-pass.md`
- `plans/dependency-architecture-post-polish-corrective-pass.md`
- `plans/dependency-architecture-dispatch-profile-parity-corrective-pass.md`

### crate-boundary-ownership (closed)

- `plans/crate-boundary-consolidation-roadmap-2026-09-16.md`
- `plans/crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`
- `plans/crate-boundary-consolidation-phase-b-report-model-extraction.md`
- `plans/crate-boundary-consolidation-phase-c-policy-enforcement-extraction.md`
- `plans/crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`

### network-transport-egress (closed)

- `plans/network-dependency-hardening-roadmap-2026-09-11.md`
- `plans/network-dependency-phase-a-baseline-invariants.md` through `plans/network-dependency-phase-g-closure-measurement.md`
- `plans/eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md`
- `plans/eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md`
- `plans/eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md`
- `plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md`
- `plans/eggfetch-0.2.0-planning-record-duplication-cleanup-2026-09-22.md`
- `plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md`
- `plans/eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md`
- `plans/eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md`
- `plans/eggress-1.0.8-post-adoption-compatibility-corrective-pass-2026-09-22.md`
- `plans/eggress-1.0.10-adoption-and-metadata-closure-2026-09-24.md`
- `plans/loadtest-authorization-transport-corrective-pass-2026-09-16.md`
- `plans/loadtest-multi-address-socket-binding-corrective-pass-2026-09-17.md`

### frontend-runtime-tui (active)

- `plans/frontend-runtime-convergence-roadmap-2026-09-10.md`
- `plans/frontend-runtime-phase-0-binding-and-parity-guards.md` through `plans/frontend-runtime-phase-3-runtime-contract-hotspot-closure.md`
- `plans/frontend-runtime-tui-full-profile-corrective-pass.md`
- `plans/tui-terminal-ownership-corrective-roadmap-2026-09-19.md`
- `plans/tui-terminal-ownership-phase-a-single-writer-logging-boundary.md`
- `plans/tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md`
- `plans/tui-broad-profile-warning-debt-cleanup-2026-09-22.md`
- `plans/tui-confirmation-test-intent-corrective-pass-2026-09-22.md`

### python-programmability (closed)

- `plans/python-library-roadmap.md`
- `plans/python-bindings-phase-a-foundation.md` through `plans/python-bindings-phase-f-major-tool-expansion.md`
- `plans/python-bindings-corrective-verification-pass.md`
- `plans/python-bindings-final-validation-polish-pass.md`
- `plans/python-bindings-active-api-scope-ci-pass.md`
- `plans/python-api-completion-roadmap.md`
- `plans/python-api-high-value-roadmap.md`
- `plans/python-api-milestone-a-engine-operation-model.md` through `plans/python-api-milestone-g-extensibility-stabilization.md`
- `plans/python-api-release-1-api-convergence.md`
- `plans/python-api-release-1-2-closure-pass.md`
- `plans/python-api-release-1-2-corrective-integration-pass.md`
- `plans/python-api-release-1-4-final-polish-pass.md`
- `plans/python-api-release-1-4-final-readiness-follow-up.md`
- `plans/python-api-release-1-4-operational-correction-pass.md`
- `plans/python-api-release-2-network-programmability.md`
- `plans/python-api-release-3-major-subsystem-completion.md`
- `plans/python-api-release-4-stateful-remote-execution.md`
- `plans/python-api-release-5-roadmap.md`
- `plans/python-api-release-5-phase-a-tool-schema-integration.md` through `plans/python-api-release-5-phase-f-compatibility-performance-release-closure.md`
- `plans/python-api-release-5-corrective-closure.md`
- `plans/python-api-release-closure-pass.md`
- `plans/python-api-corrective-integration-pass.md`
- `plans/python-api-final-integration-release-readiness.md`

### ci-verification-release (closed)

- `plans/ci-verification-release-simplification-roadmap.md`
- `plans/ci-verification-release-simplification-closure-report.md`
- `plans/ci-simplification-phase-a-policy-baseline.md` through `plans/ci-simplification-phase-f-documentation-and-closure.md`
- `plans/ci-release-simplification-corrective-closure-index.md`
- `plans/ci-release-simplification-corrective-phase-g-publishability.md` through `plans/ci-release-simplification-corrective-phase-k-evidence-toolchain-polish.md`
- `plans/ci-failure-remediation-2026-07-27.md`
- `plans/phase-b-gap-closure-performance-benchmarks-guards.md`

### performance-resource-efficiency (closed)

- `plans/performance-resource-efficiency-roadmap-2026-09-21.md`
- `plans/performance-phase-a-baseline-and-measurement-harness.md` through `plans/performance-phase-f-secondary-hotspots-and-closure.md`
- `plans/performance-closure-polish-corrective-pass-2026-09-21.md`

### daemon-protocol-agent (closed)

- `plans/ws6-daemon-schema-parity.md`

## Dependency-ready implementation plans

- `plans/implementation/nse-runtime-extraction/007-approved-scope-provider-activation.md` — **ready for handoff (M007D, current NSE boundary)**. Its hard dependencies are now closed: `007c-r-closure.md` and the `007c-s-closure.md` §13 disposition. Adopt the verified crates.io `0.3.0` artifact, retain approval-time target facts in `ApprovedExecution`, compose scoped DNS/TCP/UDP/HTTP providers from canonical Eggsec authority, and enable strict NSE only on the scope-bearing execution entry while metadata remains quarantined. **Two obligations are inherited, not optional:**
  1. **Inherited medium finding from `007b-closure.md` §12:** `broker_dns_lookup` gates `DnsResolution` on `DenyAll` only and does not evaluate per-target membership for the resolved name, so runtime DNS policy is not yet bound to approved scope. M007D must close that before re-exposure.
  2. **Deferred `0.2.0` yank from `007c-s-closure.md` §13.5:** `0.2.0` was deliberately left published and unyanked because yanking it would strand the principal consumer on `^0.2.0`. Once M007D has adopted `0.3.0`, execute the yank and re-verify the four yank properties recorded in the addendum. `0.2.0` remains affected until then.

### security-knowledge-corpus (proposed 2026-10-05)

Governed by `plans/adrs/ADR-0005-knowledge-corpus-crate-ownership.md` and
`plans/subsystems/security-knowledge-corpus-roadmap.md`. All five plans were validated
by extraction spike before writing: each target was copied into a scratch crate and
compiled with its full original test suite passing (payload corpus 233, secret detection
11, service tables 21). Targets are measured at ~0–1 `crate::` coupling because
`Severity` is already owned by the `eggsec-core` leaf — the same rationale that justified
`eggsec-policy`.

- `plans/implementation/security-knowledge-corpus/001-redaction-dead-code-disposition.md` — **closed 2026-10-05**; closure `plans/closure/security-knowledge-corpus/001-closure.md`. Option 3 (delete) executed: `utils/redaction.rs` removed (366 lines, 26 tests, zero production consumers). Guard check 147 added and demonstrated to fail on all three conditions. M003's interface blocker is discharged — `redaction.rs` is not moving into `eggsec-secrets`.
- `plans/implementation/security-knowledge-corpus/002-service-fingerprint-db-extraction.md` — **closed 2026-10-05**; closure `plans/closure/security-knowledge-corpus/002-closure.md`. `scanner/service_data.rs` moved byte-identically into `crates/eggsec-service-db` (21 tests green, single `rustc-hash` edge, **zero** consumer diffs via the `pub use eggsec_service_db as service_data` facade). Guard 114 re-pinned; guard 148 added.
- `plans/implementation/security-knowledge-corpus/004-payload-corpus-extraction.md` — **closed 2026-10-05**; closure `plans/closure/security-knowledge-corpus/004-closure.md`. 34 pure-data modules (7,084 lines) into `crates/eggsec-payloads`, **233 tests green**; zero diffs in `eggsec-python`/`eggsec-tui`. **Amended 2026-10-06:** the milestone's premise that the remaining 6 were "live-probe" generators was false — their `get_payloads()` functions are pure data. Those 6 payload sets moved too, so the crate owns **all 40** variants and no longer panics; the cross-variant caches moved into the corpus with them. Guard 150, the corpus crate's own all-40 test, and `crates/eggsec/tests/fuzzer_payload_corpus_seam.rs` assert it.
- `plans/implementation/security-knowledge-corpus/003-secret-detection-extraction.md` — **closed 2026-10-05**; closure `plans/closure/security-knowledge-corpus/003-closure.md`. `recon/secrets.rs` moved into `crates/eggsec-secrets` with a **one-line** body diff (the `Severity` import); 11 tests green, `eggsec-core` the only workspace edge, and **zero** consumer diffs — so `eggsec-python`'s exhaustive 30-arm `SecretType` match compiles untouched. Check 149 pins the owner, the facade, and freezes the AWS-scoped entropy gate at `3.5`.
- `plans/implementation/security-knowledge-corpus/005-publication-qualification.md` — **closed 2026-10-05 (deferral)**; closure `plans/closure/security-knowledge-corpus/005-closure.md`, decision in `plans/adrs/ADR-0006-knowledge-corpus-publication.md` (accepted). Publication **deferred**: corpora still evolving (14 commits since 2026-06-01, `+550 payloads`), release qualification proven expensive (NSE M007C, 74-function break), `eggsec-core` would need publishing on a shared version line, **no external consumer identified**, and `eggsec-payloads` would ship a `get_payloads` that panics for 6 of 40 variants. Reopening requires all four ADR conditions. No code changed.

### nse-runtime-extraction


- `plans/implementation/nse-runtime-extraction/007-standalone-security-patch-release.md` — **blocked (M007C)**. The §3 public-API gate was run and failed: `cargo semver-checks check-release --baseline-version 0.2.0` reports one failed major lint, `function_missing` — **74 public functions removed** (70 `register_<mod>_library(&Lua)`, 4 `helpers::*`), none source-compatible at the original path. §3's stop/replan remedy was applied: 0.2.1 was not published, no tag or release was created, and `0.2.0` remains the published artifact. Closure record: `plans/closure/nse-runtime-extraction/007c-closure.md` (archived gate output in `007c-semver-report.txt`).
- `plans/implementation/nse-runtime-extraction/007-breaking-0-3-0-release.md` — **closed** (`plans/closure/nse-runtime-extraction/007c-r-closure.md`). `eggsec-nse 0.3.0` published from the exact candidate `16cb38ee78f680bd07739dc3fc1ef776c9f19c9c` via the manual-token recovery path; the §3 gate was re-run on the candidate and reports 196 checks / 1 fail / exactly the known 74 `function_missing` items, byte-identical to the archived M007C gate; §10 qualification 15/15 green and hosted CI 5/5 on that SHA; published archive VCS identity equals the candidate and the immutable `v0.3.0` tag points at it; docs.rs built; three registry-only scratch consumers build for `nse`, `nse-ssh2`, `nse,sandbox`. Archived gate output: `007c-r-semver-report.txt`. Guard hardening landed rather than deferred: the two withdrawn helpers are now enforced absent by `compile_fail` doctests, and the `nse_production_code()` truncation hole is closed by an untruncated sweep (the hole was live — `helpers.rs` carries 45 production lines below its test module).
- `plans/implementation/nse-runtime-extraction/007-breaking-release-security-advisory-disposition.md` — **closed** (`plans/closure/nse-runtime-extraction/007c-s-closure.md` §13 execution addendum). Advisory `GHSA-w2g3-v83j-frp2` published 2026-10-02 with the source-verified range `>= 0.1.0, < 0.3.0`, patched `0.3.0`, severity medium derived from the threat model rather than from the word "bypass"; `v0.1.0` and `v0.2.0` helper source is byte-identical, which is why the range covers both. `0.1.0` **yanked** and the yank verified (registry metadata, existing lockfile still builds it, fresh `^0.1` resolution refused, unconstrained consumer reaches `0.3.0`). `0.2.0` **deliberately not yanked**: the principal consumer requires `^0.2.0` and `0.2.0` is the only `0.2.x`, so yanking it would strand fresh resolution — plan §10's stop condition; `0.2.0` remains affected and the advisory range is unchanged. The `0.2.0` yank is inherited by M007D.
- `plans/implementation/nse-runtime-extraction/007-protocol-migration-corrective.md` — **closed** (`plans/closure/nse-runtime-extraction/007b-closure.md`). The plan's central premise was falsified by the audit it ordered: 16 of the 17 "still-direct `BrokerCompatible*`" entries had no direct socket effect (the M005E scan substring-matched `BrokeredTcpStream::connect`), and `radius` — the 17th — carried a different defect than described. The corrected scan then found four real defects the plan did not anticipate, including a high-severity one (`target.resolve` unbrokered DNS in a `Pure`-classified registered library).
- `plans/implementation/nse-runtime-extraction/007-broker-compatible-protocol-migration.md` — **closed (corrective)** (`plans/closure/nse-runtime-extraction/007b-closure.md`). Standalone `699d374` landed the migration; the corrective pass closed it at `d4a22f1` with residual 97 → 22, manifest manual-only 106 → 41, `ProviderBacked` 18 → 84, and a green hosted run on the closure SHA.
- `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md` — **closed** (`plans/closure/nse-runtime-extraction/007a-closure.md`). Standalone effect manifest (159 entries), registration + require gates with post-registration scrub, HTTP authority assurance, and full standalone verification landed as `eggsec-nse@c9df4d1`; GO for M007B.
- `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md` — **closed** (`plans/closure/nse-runtime-extraction/006a-closure.md`). Published `eggsec-nse 0.2.0` from fully qualified source `ff0d2c0` (hosted run `36521717829` green; archive VCS identity = tag commit; docs.rs built), fixed the release-workflow ripgrep prerequisite, reconciled the `-D warnings` overclaim, and disclosed the breaking/provider/accounting changes plus residual risk.
- `plans/implementation/nse-runtime-extraction/005-post-merge-ci-fixture-corrective.md` — **closed** (`plans/closure/nse-runtime-extraction/005-post-merge-ci-fixture-corrective-closure.md`). The missing-ripgrep fixture is hermetic (empty-temp-dir child PATH with proven `rg` absence; fail-fast check above `dirname` with `printf`-builtin diagnostic), standalone main is `9fe149fbb22480a63e254e91d60083b7a29a8ff4`, and hosted run `36490773625` on that exact SHA is fully green (Ubuntu/macOS/Windows/MSRV/SSH).
- `plans/implementation/nse-runtime-extraction/005-provider-stack-landing-ci-corrective.md` — **closed** (`plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md`, restored from conditional via second corrective addendum with observed run `36490773625`). The linear M005A-E stack plus the hermetic tooling fix are landed on standalone main.
- `plans/implementation/nse-runtime-extraction/005-provider-coverage-qualification.md` — **closed** (`plans/closure/nse-runtime-extraction/005e-closure.md`). Source-audit provider coverage, cross-domain composition/cancel/accounting qualification, send-accounting correction, and 0.2.0 recommendation are merged on `eggsec-nse/main`.
- `plans/implementation/nse-runtime-extraction/005-http-provider-eggsec-adapter.md` — **closed** (`plans/closure/nse-runtime-extraction/005c-closure.md`). The standalone HTTP provider work is merged and released in `eggsec-nse 0.2.0`; M006B replayed the Eggsec-side scoped-transport adapter onto current `main` as `crates/eggsec/src/nse_http_provider.rs`. The adapter is compiled/tested but intentionally dormant with no production caller; controlled activation is deferred to M007.
- `plans/implementation/nse-runtime-extraction/005-authority-preserving-network-dns.md` — **closed** (`plans/closure/nse-runtime-extraction/005b-closure.md`). Runtime-neutral DNS/TCP/UDP providers, opaque handles, resolve-authorize-connect identity, and shared/core network migration are merged on `eggsec-nse/main`.
- `plans/implementation/nse-runtime-extraction/005-filesystem-process-portability.md` — **closed** (`plans/closure/nse-runtime-extraction/005d-closure.md`). Filesystem/process providers, per-run CWD, localized platform mechanics, and Windows qualification are merged on `eggsec-nse/main`.
- `plans/implementation/nse-runtime-extraction/004-versioned-release-and-eggsec-adoption.md` — **closed** (`plans/closure/nse-runtime-extraction/004-closure.md`). Published `eggsec-nse 0.1.0` (standalone `9982c7f`, tag `v0.1.0`) and moved Eggsec from the temporary Git revision to the crates.io package with full requalification.
- `plans/implementation/nse-runtime-extraction/003-standalone-repository-extraction.md` — **closed** (`plans/closure/nse-runtime-extraction/003-closure.md`). Standalone extraction and cross-repository qualification are complete at a pinned revision. Publication/provider inversion remain deferred.
- `plans/implementation/nse-runtime-extraction/002-runtime-dependency-decoupling.md` — **closed** (`plans/closure/nse-runtime-extraction/002-closure.md`). Runtime crate has zero `eggsec-*` dependencies; report bridge and scoped-transport HTTP adapter moved into the engine; only `eggsec` directly depends on `eggsec-nse`; TUI/Python consume through `eggsec::nse`; guards 144/145/146 enforce it.
- `plans/implementation/nse-runtime-extraction/001-canonical-execution-report-convergence.md` — **closed**. One runtime-owned resolver/execution/report pipeline; runtime CLI, Eggsec manual dispatch/TUI, and Python migrated.

## Blocked work

- `plans/implementation/nse-runtime-extraction/006-eggsec-0-2-0-adoption-safe-staging.md` — **closed** (`plans/closure/nse-runtime-extraction/006b-closure.md`). Eggsec consumes registry `eggsec-nse 0.2.0` (no override); the staged adapter is replayed logic-identical with no production caller; automated NSE is quarantined at metadata/listing/execution layers with manual/TUI preserved; full checks green.
- `plans/implementation/nse-runtime-extraction/006-cross-repo-qualification-closure.md` — **closed** (`plans/closure/nse-runtime-extraction/006c-closure.md`). Parent M006 closed: release/tag/archive identity, registry-only consumption, dormant adapter, quarantine, and full cross-repo checks verified with no stop conditions; GO for M007.
- `plans/implementation/nse-runtime-extraction/007-controlled-automated-reexposure-qualification.md` — **blocked on accepted M007D closure (M007E)**. Re-enable automated NSE metadata only after safe-path qualification, replace dormancy guards with exact call-site guards, prove zero-contact negative cases, and close M007 across both repositories.

## Closure work and current control points

All closure evidence for flat-era work lives inline in the files above (appended completion records) and in `architecture/` deep-dives. Current foundation evidence relevant to future work includes:

| Area | Controlling evidence |
|---|---|
| Enforcement/dispatch ownership | architecture-convergence Phase C + closure report; `architecture/dispatch.md`, `architecture/config.md` |
| Crate boundaries | crate-boundary Phase D; `architecture/overview.md` dependency map |
| Transport/egress | Eggress 1.0.10 closure; ADR-0001, ADR-0002; `architecture/transport_eggfetch.md`, `architecture/network_dependency_closure.md` |
| Frontend/runtime | frontend-runtime Phase 3 + TUI terminal-ownership Phase B; `architecture/tui.md`, `architecture/runtime_bridge.md` |
| Python API | Release 5 phase F; `architecture/python_api.md` |
| CI/release | Phase K; `docs/VERIFICATION.md` |
| Performance | Phase F + polish corrective; `architecture/performance.md` |
| NSE runtime extraction | `plans/adrs/ADR-0003-nse-host-provider-boundary.md`; `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`; `plans/subsystems/nse-runtime-extraction-roadmap.md`; `plans/closure/nse-runtime-extraction/007b-closure.md`; `plans/closure/nse-runtime-extraction/007c-closure.md`; `plans/closure/nse-runtime-extraction/007c-r-closure.md`; `plans/closure/nse-runtime-extraction/007c-s-closure.md`; `plans/implementation/nse-runtime-extraction/007-approved-scope-provider-activation.md` (current handoff, M007D); automated NSE stays quarantined until M007E |
