# NSE Runtime Extraction Milestone 007A — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-007--protocol-library-gating-and-controlled-automated-activation`

Applicable ADRs:

- `plans/adrs/ADR-0004-nse-automated-activation-boundary.md`
- `plans/adrs/ADR-0003-nse-host-provider-boundary.md`

Repository baselines reviewed: Eggsec `4e734c9b` (unchanged in this slice — no Eggsec production activation per plan §1); standalone `eggstack/eggsec-nse@ff0d2c09feba1bcd7ba5f7312579690d905be09c` (`0.2.0`).

Implementation commits or pull requests:

- standalone `c9df4d1` — `feat(nse): M007A automated library effect gate and HTTP authority assurance` (manifest + registration/require gates + HTTP assurance + test migration + guards + docs; 8 files, +2587/−210).

## 1. Executive finding

Milestone 007A is complete. The standalone runtime now carries a complete, fail-closed automated-library eligibility boundary before any Eggsec automated NSE surface is re-enabled: all 159 registered Lua libraries/globals are classified, AgentSafe/CiSafe runs cannot reach manual-only libraries through either direct globals or dynamic `require()`, manual compatibility is preserved, and automated HTTP requires explicit authority-bound assurance so the native reqwest provider is never silently treated as scope-authoritative. No Eggsec production activation occurred in this slice (per plan). All 12 acceptance criteria pass. **GO for M007B.**

## 2. Requirement-to-evidence matrix

| Requirement (plan §8) | Evidence | Result | Notes |
|---|---|---|---|
| 1. Every registered Lua library/global has an effect classification | `src/effect_manifest.rs`: 159 `LibraryEffectEntry` records (80 `ManualOnlyDirectIo`, 26 `ManualOnlyAdvisory`, 35 `Pure`, 18 `ProviderBacked`); M007A guard asserts every `register_*_library` call in `register_libraries()` has a manifest entry | pass | 146 register calls covered; 13 extra entries carry explicit compat rationale |
| 2. Unknown classification is automated-denied | `automated_library_eligibility()` defaults unknown names to `ManualOnlyDirectIo`; `unknown_library_resolves_to_manual_only` + `automated_dynamic_require_blocked_for_representative_unsafe` (includes `unknown_lib_xyz`) green | pass | ADR-0004 §3 deny-by-default |
| 3. AgentSafe/CiSafe never register direct/advisory residual globals | `gate_then_register()` (27 high-risk call sites) + `scrub_ineligible_globals()` post-registration scrub for all 106 manual-only libs; `automated_direct_globals_absent_for_unsafe_libraries` (10 representative libs × 2 profiles) green; safe globals retained in same test | pass | Scrub was added when review found ~79 manual-only libs bypassing per-site gating |
| 4. Dynamic require uses the same classification | `setup_require()` consults the manifest (no second allowlist); `BlockedByPolicy` reports; 16 migrated `local_protocol_tests` denial tests + require unit tests green | pass | §3 |
| 5. Required-module reports expose policy denial | `report.libraries` carries `blocked-by-policy` warnings via `from_required_module`; `assert_library_blocked_by_manifest` helper asserts them | pass | — |
| 6. Manual profiles retain current compatibility | `manual_direct_globals_present_for_same_libraries` (all 14 representative globals present) + all manual-success local-protocol tests + full suite green | pass | `gate_then_register`/`scrub` early-return on manual kinds |
| 7. Native HTTP is not authority-bound | `http_authority_bound: false` in all native constructors; `with_authority_bound_http()` is the only `true` source (guard-enforced); `authority_bound_flag_is_propagated` green | pass | — |
| 8. AgentSafe native HTTP fails before provider contact | `automated_profile_native_http_denied_before_provider_call` (Denied + 0 provider calls) and `ci_safe_native_http_denied_before_provider_call` green | pass | Plan invariant 10–11 |
| 9. Authority-bound HTTP usable under AgentSafe subject to runtime policy | `automated_profile_authority_bound_http_can_be_invoked` green; `manual_profile_native_http_still_works` green | pass | Eggsec adapter becomes first authority-bound provider in M007D |
| 10. M005E inventories remain guard-pinned | `check-boundaries.sh` M005E sections untouched and passing; 72/25 sets map to manual-only classes (representative pins guarded); residual counts unchanged | pass | Augmented, not deleted, per plan invariant |
| 11. No breaking public API | Additive only: new module + re-exports, new builder method, new flag defaulting `false`; `cargo package` verifies; MSRV 1.89 check passes | pass | No `NseLibraryDescriptor` layout change |
| 12. Closure unblocks M007B | This record + §11 | pass | M007B ready for handoff |

## 3. Production implementation evidence

Standalone-only slice (zero Eggsec dependency, per invariant). Landed behavior in `c9df4d1`:

- `src/effect_manifest.rs` (new, 1678 lines): manifest + `automated_library_eligibility`, `is_automated_library_safe`, `eligible_for_profile`, `is_automated_profile`, `classify`, `classified_libraries`, `eligibility_counts`; compiles with `nse` feature off.
- `src/executor_core.rs`: `gate_then_register()` for 27 high-risk libraries (sslcert, tls, ftp, smtp, mysql, postgres, mssql, redis, mongodb, ldap, snmp, smb, smb2, rdp, vnc, ntp, imap, dhcp, dhcp6, upnp, openssl, brute, ssh, xdmcp, omp2, libssh2); `scrub_ineligible_globals()` post-registration removal of all remaining ineligible globals under automated profiles; dynamic-`require()` policy denial with `BlockedByPolicy` report.
- `src/providers.rs`: additive `http_authority_bound: bool`, `with_authority_bound_http()`, `is_http_authority_bound()`; `broker_http_request()` denies AgentSafe non-authority hostname requests before provider contact (CiSafe denied regardless; manual unchanged).
- `src/lib.rs`: module wiring + additive re-exports.
- Tests: `tests/effect_manifest_tests.rs` (new, 22 tests: eligibility API, coverage floors, 4 direct-global/require regression tests, 5 HTTP authority tests); `tests/local_protocol_tests.rs` (16 denial tests migrated from network-capability assertions to manifest-block assertions; duplicate-assert cleanup; FTP `hits()`→`control_hits()` correction).
- Guards/docs: `scripts/check-boundaries.sh` M007A section (manifest coverage, scrub presence, residual pins, HTTP-default pin); `docs/PROVIDERS.md` M007A section.

Planned but absent (by design): no Eggsec-side activation, no M005E file migration (M007B), no release (M007C).

## 4. Verification executed

### Commands run

```bash
cargo fmt --all --check
bash scripts/check-boundaries.sh
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --features nse
cargo package --allow-dirty   # --allow-dirty: work intentionally uncommitted at check time; committed as c9df4d1 after
```

### Results

- `cargo fmt --all --check`: pass (after one `cargo fmt --all` normalization of new test code).
- `check-boundaries.sh`: `standalone boundary and provenance checks passed` (incl. new M007A section; one guard iteration fixed an `rg` multiline-literal error).
- `cargo check --no-default-features` / `--features nse` / `--features nse-ssh2` / `--features nse,sandbox`: pass (92–94 pre-existing dead-code/deprecation warnings, unchanged class).
- `cargo test --features nse`: all suites green, 0 failed — lib 204, effect_manifest 22, local_protocol 61, plus 25 further integration suites (full list in run log; e.g. provider composition, resolver, sandbox, corpus). Doc-tests: 1 ignored (pre-existing).
- `cargo clippy --all-targets --features nse`: no errors; only pre-existing test-code warnings (`context_fidelity_tests`, `runtime_corpus_tests`, `sandbox_tests`).
- `cargo +1.89.0 check --locked --features nse`: pass (MSRV holds).
- `cargo package --allow-dirty`: `Packaged 313 files` + verify compile ok. (Plain `cargo package` refuses on dirty tree as designed; committed immediately after as `c9df4d1`.)
- Explicit new-test runs: `effect_manifest_tests` 22/22, `local_protocol_tests` 61/61.

## 5. Invariant review

| Plan invariant (§4) | Evidence | Result |
|---|---|---|
| Zero Eggsec dependency in standalone | `check-boundaries.sh` Eggsec-dependency guard green; Eggsec repo untouched (`git status` clean at `4e734c9b`) | holds |
| ManualPermissive/CompatibilityLab compatible | manual-presence test + all manual-success protocol tests + full suite green | holds |
| Unsafe modules not merely require-blocked; direct globals fail | scrub + direct-global absence tests (incl. previously ungated `pop3`/`sip`/`tftp`/`smbauth`) | holds |
| Unknown eligibility deny-by-default | default arm + unknown require test | holds |
| No residual promoted without provider-backed evidence | `pop3`→DirectIo / `smb`→Advisory pins (test + guard); counts ≥70 manual-only floor | holds |
| M005E pins active and augmented | M005E guard sections unchanged + passing; M007A section additive | holds |
| Native HTTP available manually | `manual_profile_native_http_still_works` + manual HTTP protocol tests | holds |
| AgentSafe + native HTTP cannot perform hostname request | denial-before-provider tests (AgentSafe + CiSafe, 0 calls) | holds |
| Provider denial means zero provider contact | call-count asserts in all three HTTP denial tests | holds |
| MSRV 1.89 | `cargo +1.89.0 check` pass | holds |

## 6. Failure and recovery review

Not a runtime-lifecycle slice: no persistence, daemon, lease, or cancellation-path changes. Fail-closed properties verified instead: scrub ignores per-global removal errors with a `tracing::warn` (one missing global cannot break executor construction); require-denial surfaces as a Lua runtime error plus a structured `BlockedByPolicy` report (no silent empty success); HTTP denial returns `Denied` before any provider contact (no partial-send state). Malformed input (`require("")`, unknown names) follows the same deny path (covered by `InvalidName`/`Missing` arms and the unknown-lib test).

## 7. Migration and compatibility review

No schema, protocol, or config migration. Backward compatibility: public API strictly additive (verified by `cargo package` + MSRV check); manual-profile Lua behavior unchanged (presence tests + manual protocol successes); `NseLibraryDescriptor` layout untouched per plan constraint. Rollback: standalone commit `c9df4d1` reverts cleanly (single-commit slice, no cross-repo coupling).

## 8. Security review

Authorization boundary (standalone side): automated profiles deny by default; 106 manual-only libraries unreachable via globals or `require()`; unknown names fail closed; HTTP authority cannot be inherited from capability pre-checks (explicit builder flag only). No secrets, paths, or privilege changes in this slice. DoS bounds unchanged (existing limits model untouched). Audit: denials are recorded in `report.libraries` warnings and `BlockedByPolicy` required-module reports; scrub removals emit `tracing::debug`, failures `tracing::warn`. No `let _ =` / `filter_map(ok)` silencing introduced (per repo gotchas; scrub logs instead).

## 9. Documentation and operations

- `docs/PROVIDERS.md`: new M007A section (manifest table + counts, gate/scrub/require mechanics, HTTP assurance contract, regression-test map, guard regeneration note).
- `scripts/check-boundaries.sh`: new M007A section (4 guard groups).
- `src/effect_manifest.rs` module docs: class semantics, API surface, invariants, known `enforcement_status` divergence note.
- Operator diagnostics: `eligibility_counts()` / `classified_libraries()` for inventory inspection; `blocked-by-policy` warnings in run reports.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | 13 manifest entries exceed the 146 `register_*` call sites (compat rationale, not reconciled one-to-one) | Guard enforces coverage in the register→manifest direction only; an orphaned manifest entry would not fail the guard | **CLOSED in M007B** (`plans/closure/nse-runtime-extraction/007b-closure.md` §3.4): verified as 12 manifest-only entries plus 4 further unregistered modules with no manifest entry; both directions now enforced with `scripts/nse-registration-compat-entries.txt` |
| low | `gate_then_register` (27 sites) + scrub overlap: per-site gating is now redundant for libraries the scrub also covers | Harmless defense-in-depth; slight maintenance surface | Optional M007B cleanup: keep both (preferred) or collapse to scrub-only with guard |
| low | Clippy/pre-existing warnings (92–94 lib warnings: dead code, deprecated `as_utf8`, unreachable pattern) unchanged | No new warnings introduced; noise only | Out of scope; tracked by existing repo hygiene |

No critical/high/medium findings. No stop condition triggered (plan §9: coverage derived deterministically; no static-only reliance; no breaking API; native HTTP not implicitly trusted; manual compatibility preserved).

## 11. Roadmap disposition

Milestone closed and next dependency may proceed: **GO for M007B** (`007-broker-compatible-protocol-migration.md`, blocked on accepted M007A closure). M007C/D/E remain gated behind their stated predecessors. Automated NSE stays quarantined until 007E qualification — this slice enables no Eggsec automated surface.

## 12. Registry updates

- `plans/registry.md`: M007A `ready for handoff` → closed (this record); M007B `blocked on accepted M007A closure` → ready for handoff; handoff-boundary note updated.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: milestone 007 status advanced past M007A (M007B current handoff).
- `plans/implementation/nse-runtime-extraction/007-automated-library-effect-gate.md`: Status `ready for handoff` → `closed`.
