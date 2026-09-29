# NSE Runtime Extraction Milestone 006C — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/006-cross-repo-qualification-closure.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Repository baselines reviewed: Eggsec `c9d493fd` (M006B commit); standalone `eggstack/eggsec-nse@ff0d2c09feba1bcd7ba5f7312579690d905be09c` (unchanged since release — no drift).

Prior closures referenced:

- `plans/closure/nse-runtime-extraction/006a-closure.md` (M006A: 0.2.0 publication).
- `plans/closure/nse-runtime-extraction/006b-closure.md` (M006B: adoption + dormant adapter + quarantine).

Implementation commits or pull requests:

- No new production commits in this slice — this is the qualification/closure pass over the M006A (standalone) + M006B (Eggsec) landed state. Planning-only updates (this record, plan status, registry, roadmap) accompany it.

## 1. Executive finding

Milestone 006 as a whole is complete and closes. The published `eggsec-nse 0.2.0` artifact, its immutable source identity, and the Eggsec consumer state were re-verified end to end in this slice: registry-only resolution, dormant engine-owned adapter with zero production callers, three-layer automated-exposure quarantine with manual/TUI NSE functional, full consumer checks (lib/NSE/TUI/Python/policy/dependency/architecture/feature-sweep/`make check`/`make check-python`), and documentation without enforcement overclaims. No stop condition triggered. **GO for M007** (protocol-library capability gating + approved-scope/profile threading + controlled adapter activation), which is now unblocked and ready to author from this closure evidence.

## 2. Requirement-to-evidence matrix

| Requirement (plan §7) | Evidence | Result | Notes |
|---|---|---|---|
| 1. Published 0.2.0 artifact/source/tag identity verified | crates.io resolves `eggsec-nse 0.2.0`; tag `v0.2.0^{commit} == ff0d2c0` == candidate; archive `.cargo_vcs_info.json` sha1 `ff0d2c0`; standalone `origin/main` still `ff0d2c0` (no drift) | pass | 006A evidence re-confirmed live |
| 2. docs.rs state recorded | `https://docs.rs/eggsec-nse/0.2.0` serves built API docs (built 2026-09-29) | pass | Re-fetched in this slice |
| 3. Eggsec manifest + lockfile resolve crates.io 0.2.0 | `Cargo.toml` `0.2.0`; lockfile `0.2.0` + `registry+...crates.io-index` + checksum `bf8befe7…0ffe`; `cargo tree -p eggsec --features nse,cli -i eggsec-nse` → `eggsec-nse v0.2.0`; `cargo metadata --no-deps` ok | pass | §4 |
| 4. No Git/path/patch source | Guard 144 source regex green; override scan (`patch.crates-io`, git/path refs) empty; `make check-deps` ok | pass | — |
| 5. Full relevant Eggsec verification passes | lib 1721 / NSE 237 / TUI 903+12 / Python 231 / policy 81, all 0-fail; `make check-features-individual` 89/89; `make check` green; `make check-python` PASSED | pass | §4 (006C-owned reruns) |
| 6. Adapter code present, compiled, tested | `nse_http_provider.rs` in engine (cfg `nse`); combo check `nse-ssh2,nse-sandbox,cli` ok; 8/8 adapter tests green against registry 0.2.0 | pass | — |
| 7. Adapter has no production caller | `rg NseHttpTransportProvider` → module only; constructors → module tests only; zero `with_host_services`/`NseHostServices` in engine/TUI/Python; guard 145 dormancy pin green | pass | §4 audit |
| 8. Automated NSE disabled, strict dispatch fails closed | Metadata all-false (catalog test); 4 listings omit NSE (registration test); `execute_approved*` reject synthesized bundles (2 boundary tests); `NseTool` unregistered; REST/gRPC explicit exposure checks stand | pass | Quarantine tests re-run green in-slice |
| 9. Manual/TUI NSE functional | NSE suites 237 green; manual path (`evaluate` + `run_cli_with_profile`, native defaults, no host-services injection) untouched; `run_nse` code-reviewed native | pass | — |
| 10. TUI/Python indirect consumers | Guard 145 facade checks green (`pub use eggsec_nse as nse`; TUI/Python `nse = ["eggsec/nse"]`); TUI 903 / Python 231 green | pass | — |
| 11. Architecture/dependency guards reflect 0.2.0 + dormant state | Guards ALL PASSED (144 = 0.2.0 registry; 145 = dormancy pin; 146 canonical); `make check-deps` ok | pass | — |
| 12. No provider-wide enforcement overclaim | Source scan over `architecture/`, `docs/`, skill, registry, roadmap: no activation/completeness claim; skill + arch docs state quarantine + residual explicitly | pass | §4 |
| 13. Closure sequences M007 | §11: GO for M007 with boundary + inputs defined | pass | M007 ready to author |

## 3. Production implementation evidence

None new (qualification-only slice). Landed state under review:

- Standalone: `ff0d2c0` (0.2.0 prep), tag `v0.2.0`, GitHub Release (non-draft, 2026-09-29), crates.io `0.2.0`, docs.rs built. No post-release standalone commits.
- Eggsec `c9d493fd`: 0.2.0 dependency, replayed dormant adapter (doc-only diff vs `4ade61a`), metadata quarantine, strict-entry guard, 11 regression tests, guard/doc updates.

## 4. Verification executed

### Commands run

```bash
# A — registry artifact identity (standalone side)
git rev-parse 'v0.2.0^{commit}'            # == ff0d2c0
git rev-parse origin/main                  # == ff0d2c0 (no drift)
gh release view v0.2.0 --repo eggstack/eggsec-nse
cargo search eggsec-nse                    # 0.2.0 listed
# docs.rs re-fetched: https://docs.rs/eggsec-nse/0.2.0 served
# (006A scratch-consumer + .cargo_vcs_info.json evidence stands; artifact immutable)

# B — Eggsec registry-source qualification
cargo tree -p eggsec --features nse,cli -i eggsec-nse
cargo metadata --no-deps
make check-deps

# C — consumer behavior qualification
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
cargo test -p eggsec-policy --lib
make check-features-individual
make check
make check-python

# D — dormant-adapter proof (source audit)
rg -l 'NseHttpTransportProvider' crates/ --glob='*.rs'            # module only
rg -n 'NseHttpTransportProvider::new' crates/ --glob='*.rs'       # module tests only
rg -n 'with_host_services|NseHostServices' engine+TUI+Python     # zero hits
# manual path review: dispatch/api.rs run_nse builds NseRunRequest::new (no injection)

# E — quarantine proof
cargo test -p eggsec --features nse,cli --lib quarantined        # 3/3 green
# + catalog/registration/boundary tests above; NseTool unregistered (doc + code)

# F — doc/guard reconciliation scan
rg -i 'adapter.*active|automated NSE.*support|complete.*scope enforcement' architecture/ docs/ skill/ plans/
```

### Results

- Identity: tag commit = candidate = archive VCS = `origin/main` (`ff0d2c0`); Release non-draft; crates.io 0.2.0 listed; docs.rs served.
- Source: tree/metadata/lockfile all registry-0.2.0; no overrides; `make check-deps` ok.
- Consumer: lib **1721/0**, NSE suites **237/0**, TUI **903/0 (+12 ignored)**, Python **231/0**, policy **81/0**; feature sweep **89 passed, 0 failed, 0 skipped**; `make check` green (incl. guards ALL PASSED); `make check-python` PASSED.
- Dormancy: audits exactly as required (module-only type, test-only constructors, zero host-services injection).
- Quarantine: 3/3 focused tests green; layers reviewed (§5 of 006B still accurate).
- Docs: no stale activation/completeness claim found; one scan hit is the roadmap exit-condition requirement itself, one is the skill's correct quarantine statement, one is a factual optional-module listing.
- No stop condition triggered (no identity divergence, no override need, no automated exposure, no premature activation, no manual regression, no overclaim).

## 5. Invariant review

All M006 invariants hold in the closed state (verified, not assumed):

- 0.2.0 tag/source/archive identity immutable and traceable (§4A).
- Eggsec resolves 0.2.0 from crates.io only (§4B).
- Only Eggsec directly consumes the runtime; TUI/Python behind `eggsec::nse` (guard 145 + suite results).
- Adapter engine-owned (`crates/eggsec/src/nse_http_provider.rs`), standalone Eggsec-independent (untouched since release).
- Adapter has no production dispatch caller (§4D).
- Automated NSE disabled/fail-closed (§4E); manual CLI/TUI NSE works (§4C).
- No source override or temporary patch (guard 144 + scans).
- M005E residual inventories documented and unchanged (no standalone diff since `ff0d2c0`; Eggsec docs restate, not alter).
- No claim of complete protocol-wide NSE scope enforcement (§4F).

## 6. Failure and recovery review

- No failures in-slice; every required check passed on first execution against the landed state.
- The two M006B implementation-time corrections (metadata-derived registration assertion; `NonBaselineCapability` override in boundary tests) were already recorded in the 006B closure and their corrected tests re-ran green here.
- Rollback posture unchanged: Eggsec pre-adoption commit (0.1.0) remains the consumer rollback; 0.2.0 yank is not indicated (artifact healthy, docs built, consumers green).

## 7. Migration and compatibility review

- No new migration in-slice. 006A/006B compatibility findings stand: 0.x minor vehicle correct; write-limit enforcement live; metadata flip removes a false advertisement (tool was never registered), not a working path; general strict-entry guard additionally hardens 5 pre-existing manual-only ops with zero test impact.
- `NseRunReport` serialization untouched across M006 (no runtime diff since M005).

## 8. Security review

- Closed posture at three layers (enumeration / approval-adjacent / execution) re-verified against the committed tree, not merely cited from 006B.
- Strict canonical dispatch cannot execute NSE through a hidden route: both `ApprovedOperation` (`execute_approved`) and `ApprovedExecution` (`execute_approved_execution`) entries reject; tool-registry dispatch cannot resolve an NSE tool; REST/gRPC explicit exposure checks stand where their metadata registries apply.
- Manual dispatch remains native/default-provider with `ManualPermissive` confined to manual surfaces (CLI handler + `run_nse` NOTE).
- Residual risk: M005E medium (72-file ungated set) unchanged and still manual-only bounded; low items from 006B (strict-`approve()` token issuance without exposure check; Trusted Publishing pending) carried — both are explicit M007 inputs, not M006 blockers.

## 9. Documentation and operations

- No doc changes required in-slice: 006B reconciliation already truthful, verified by the §4F scan. Historical M005 closures untouched.
- Operator impact: none. No config/CLI/TUI/Python surface change; no new diagnostics to document.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium (carried, M005E) | 72-file ungated specialized direct-I/O residual | Manual-only bounded; quarantine holds | **M007**: protocol-library capability gating |
| low (carried, M006B) | Strict `approve()` may still issue NSE tokens (execution blocked downstream) | Token alone executes nothing | M007 may add surface-exposure to approval |
| low (carried, M006A) | Trusted Publishing registry-side entry pending | Manual-token path remains release route | Crate-owner website step |
| info | M007 plan file does not exist yet | Nothing blocked; authoring input complete | Author M007 from this closure (§11) |

No high/critical findings. No new findings of any severity.

## 11. Roadmap disposition

**Parent Milestone 006: CLOSED.** All three child slices (006A release, 006B adoption, 006C qualification) are implemented with accepted closures; all 13 acceptance criteria pass (§2); no stop condition triggered; remaining items are sequenced M007 work, not blockers.

**GO for M007 — protocol-library capability gating + approved-scope/profile threading + controlled adapter activation.** M007 is unblocked and ready to author; required inputs from this closure:

- Adopted surface: crates.io `eggsec-nse 0.2.0` (`ff0d2c0`), immutable.
- Dormant adapter ready for threading: `NseHttpTransportProvider::new(transport, authority, insecure_tls)` + mapping/authority/TLS tests (8) in `crates/eggsec/src/nse_http_provider.rs`.
- Quarantine to lift deliberately: catalog flags + strict-entry guard + listing tests (all marked M007).
- Residual to gate: M005E 72-file ungated set + mixed advisory files (pins + `docs/PROVIDERS.md` M005E section).
- Non-goals carried: no broad metadata redesign beyond the quarantine already landed; `TargetPolicyKind::ExplicitScopeRequired` must survive re-enablement.
- Suggested M007 exit: scoped provider injection on strict paths, automated surfaces re-exposed selectively with tests, quarantine tests revised (not deleted), residual inventory shrunk with guard updates.

## 12. Registry updates

- `plans/registry.md`: M006 (006A/B/C) closed; subsystem standing updated; M007 recorded as ready-to-author (no file yet).
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: Milestone 006 closed with artifact identity; M007 sequencing note updated.
