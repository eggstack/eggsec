# Security Knowledge Corpus Milestone 001 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/security-knowledge-corpus/001-redaction-dead-code-disposition.md`

Source subsystem roadmap:

- `plans/subsystems/security-knowledge-corpus-roadmap.md#milestone-1--redaction-dead-code-disposition`

Repository baseline reviewed: `979dca67` (branch `docs/knowledge-corpus-crate-plans`, plan commit `e251cb42`)

Implementation commits or pull requests:

- this branch — remove `utils/redaction.rs`, add guard check 147, correct `architecture/utils.md`

## 1. Executive finding

The defect is resolved and the latent recurrence is now mechanically prevented.

**Disposition: Option 3 (delete).** `crates/eggsec/src/utils/redaction.rs` — 366 lines, 26 tests, zero production consumers — has been removed, its `pub mod redaction;` declaration dropped, and its module doc comment in `utils/mod.rs` corrected to state the disposition.

This is not "cleanup". It removes a **second, orphaned implementation of a concept the repository already models declaratively**. The repo's real redaction contract is `eggsec-report-model`'s `RedactionState` (`None` / `FullyRedacted` / `PartiallyRedacted` / `Summarized`), carried per evidence item and consumed by `eggsec-output`, `eggsec-db-lab`, and `eggsec-mobile-lab`. `utils/redaction.rs` was a regex masker that nothing called, that no `RedactionState` was derived from, and whose output never reached any sink.

Deleting 26 passing tests is a real loss of tested behavior and is disclosed as such in §10. The mitigation is that the deleted code was unreachable: no runtime path could produce a different result with it present than without it.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Exactly one §6 disposition executed | Option 3 executed; Options 1 and 2 evaluated and rejected on evidence (§3) | pass | |
| Option 1 not skipped silently | Three candidate call sites inspected and rejected with reasons (§3) | pass | |
| Option 2 rejection reasoned | `eggsec-transport` cannot take `regex`; check 108 pins four deps | pass | |
| `make check` green | see §4 | pass | |
| Guard makes failure mode non-recurring | Check 147 added; demonstrated to fail on all 3 conditions | pass | |
| Phase G record references ADR-0005 | `architecture/capability_segregation.md` Phase G | pass | |
| No new workspace dependency | Diff adds no manifest line; `regex` already a direct engine dep (12 other modules) | pass | |
| Deleted test count disclosed | 26, confirmed by `git show HEAD:…redaction.rs \| grep -c '#\[test\]'` | pass | |
| Post-deletion consumer search clean | see §3 | pass | |

## 3. Production implementation evidence

### The decision, and why Option 1 was rejected

Option 1 (adopt) required identifying a genuine call site that should be masking evidence and is not. All three candidates named in the plan were inspected:

1. **`findings::Evidence` persistence.** `Evidence::new` always sets `redacted: false`, which looked like an unmasked evidence path. It is not. The only production construction of `findings::Finding` is the `search_cve` storage mode in `crates/eggsec/src/dispatch/security.rs`, and it sets `evidence: vec![]`. There is no populated evidence path in the engine to mask.

2. **Secret detection.** This is the one place Eggsec provably handles live credential material, and it already has a masking policy — a *different* one. `SecretFinding::value_preview` truncates to 20 chars + `"..."`, asserted by `recon/secrets.rs::test_value_preview_truncation`. Applying `redact_sensitive` (regex masking to `[REDACTED]`) there would impose a second, different policy on a field that is deliberately a *preview*. The plan's own stop condition names this case: "the adopted call site turns out to need a masking policy that differs from `redaction.rs`".

3. **`nse_bridge.rs`.** Maps external NSE evidence and *already* declares redaction declaratively via `.with_redaction(RedactionState::None)` at line 80. The plan's candidate-3 phrasing ("a finding's evidence is echoed to a model") did not survive inspection: this path is a typed DTO mapping, and it uses the report model's redaction vocabulary, not a regex masker.

### The decisive finding

`RedactionState` (`crates/eggsec-report-model/src/envelope.rs:117`) is the redaction contract the repository actually maintains, with four states, consumed by `eggsec-output`, `eggsec-db-lab`, and `eggsec-mobile-lab`. `utils/redaction.rs` never produced a `RedactionState` and was never read by any of those consumers. Deleting it removes a duplicate implementation of an already-solved problem.

### Option 2 rejection

`eggsec-transport`'s `redacted_headers_debug` (operates on `http::HeaderMap`) and `redact_url_for_debug` (on `url::Url`) serve debug formatting, not evidence storage. Unifying would require `regex` inside `eggsec-transport`, which check 108 pins to exactly `bytes`/`http`/`url`/`thiserror`. Not free — rejected, as the plan predicted.

### Post-deletion consumer search

```
rg -n "redact_sensitive|redact_json" crates/ --glob '*.rs'
```

Every remaining hit is a non-consumer:

| Hit | Nature |
|---|---|
| `eggsec-python/src/integrations.rs:173,185,190,199,211` | a `redact_sensitive` struct field / constructor parameter (see §10 finding) |
| `eggsec-python/src/reporters.rs:27` | string literal default `"redact_sensitive"` |
| `eggsec-report-model/tests/roundtrip.rs:119` | string-literal assertion on a `RedactionPolicy` value |
| `tool/protocol/mcp/handlers/server.rs:1612` | JSON key `"redact_sensitive_data"` |
| `utils/mod.rs:8` | this milestone's own doc comment |

Zero call sites reference the removed path.

### Diff

```
 crates/eggsec/src/utils/mod.rs       |   6 +-
 crates/eggsec/src/utils/redaction.rs | 366 -----------------------------------
 scripts/check-architecture-guards.sh |  23 +++
 3 files changed, 28 insertions(+), 367 deletions(-)
```

## 4. Verification executed

### Commands run

```bash
cargo check --workspace --no-default-features
bash scripts/check-architecture-guards.sh
bash scripts/check-architecture-guards.sh   # with the condition artificially reintroduced (negative test)
make check
```

### Results

| Command | Result |
|---|---|
| `cargo check --workspace --no-default-features` | pass — `Finished dev profile … in 1m 19s`, no warnings from the deletion |
| `bash scripts/check-architecture-guards.sh` | pass — all 147 checks, `ALL PASSED: No architecture drift detected.` |
| Guard 147 negative test | **pass** — see below |
| `make check` | **pass** — ran to completion; the final step (guards) printed `ALL PASSED`, and `make` aborts on the first failing recipe, so every preceding step (fmt, 3× `cargo check`, `check-deps`, clippy, doc tests, 6 test suites) succeeded. Took ~9 minutes under CPU contention from unrelated builds in other projects. |

#### 4.1 Negative test for check 147

Check 147 fails if `crates/eggsec/src/utils/redaction.rs` reappears, if `utils/mod.rs` re-exposes `pub mod redaction`, or if an engine-local `fn redact_sensitive`/`fn redact_json` reappears under `crates/eggsec/src/`. All three were artificially reintroduced; all three fired:

```
--- Check 147: removed utils::redaction stays removed ---
FAIL: crates/eggsec/src/utils/redaction.rs reappeared (Phase G removed it: zero production consumers).
FAIL: utils/mod.rs re-exposes the redaction module.
FAIL: engine-local redact_sensitive/redact_json reappeared under eggsec/src.
crates/eggsec/src/utils/redaction.rs:2:pub fn redact_sensitive(s: &str) -> String { s.to_string() }
crates/eggsec/src/utils/redaction.rs:3:pub fn redact_json() {}

=== Summary ===
FAILED: 1 check(s) failed.
EXIT=1
```

The condition was then restored and the suite re-run clean.

Deep checks (`make check-full`, `make check-features-individual`, `make clippy-domain`, `make test-tui-pty`) were **not run** — they are deep checks, not per-milestone, and this milestone changes no feature surface. `make check-deps` was **not run**: no manifest line changed, so the dependency policy is untouched. `make check-python` was **not run**: no Python binding or stub changed.

## 5. Invariant review

| Source-plan invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate | No dispatch, enforcement, `OperationMetadata`, `Capability`, or approval-token code was touched. Diff is one deleted file, one module-doc edit, one shell guard. |
| No `let _ =` / `filter_map(|e| e.ok())` | No Rust logic added. |
| `eggsec-transport` stays exactly `bytes`/`http`/`url`/`thiserror` | Untouched; check 108 passes in the 147-check run. |
| `eggsec-policy` and `eggsec-transport` remain independent | Untouched; checks 121–123 pass. |
| No secret material logged unredacted by anything introduced | Nothing was introduced. |
| Redaction, where it remains, is deterministic and side-effect free | Unchanged: `eggsec-transport`'s debug redaction and `eggsec-report-model`'s `RedactionState` are both untouched. |

## 6. Failure and recovery review

Not applicable in the usual sense — the deleted functions were synchronous, pure, and unreachable. No task, state, persistence, or cancellation semantics were touched. There is no restart or recovery behavior to verify because there was never a live path.

## 7. Migration and compatibility review

No storage, protocol, configuration, or wire migration.

`eggsec::utils::redaction::*` was removed from the public API. This is a **breaking change to a path with zero consumers**, verified by two independent searches (a repo-wide `redaction::` path search returned only prose in the plan documents themselves, and the `redact_sensitive`/`redact_json` symbol search above returned only non-consumers). The crate is not published (`publish = false`, internal workspace), so no external consumer can be affected. Rollback is a single revert of this commit; nothing else depends on the file.

## 8. Security review

**Net effect is a small reduction in attack surface.** The repository now has exactly one redaction mechanism — the declarative `RedactionState` — instead of one declarative mechanism plus an unreachable regex masker whose presence implied a masking capability that was never wired.

- Authorization: untouched. No `Scope`, `Capability`, or `ApprovedOperation` reference was added or removed.
- Secret handling: the deleted module was the only engine code holding a hardcoded credential-shape corpus for *output* masking. Its patterns (bearer/basic/JWT/AWS/PEM/connection-string) remain conceptually represented by `RE_SENSITIVE_KEY`-adjacent handling in the report model's policy surface; no *detection* corpus was touched (`recon/secrets.rs` is unmodified and out of scope until M003).
- Redaction: `eggsec-transport`'s debug redaction unchanged; `RedactionState` unchanged.
- No new network, filesystem, or subprocess surface.

## 9. Documentation and operations

Updated:

- `architecture/capability_segregation.md` — Phase G disposition section rewritten from "defect found, dispositioned separately" to the executed decision, with the three rejected adoption candidates, the `RedactionState` finding, the deleted-test disclosure, and check 147. The Guards bullet and the footer verification note were updated to match.
- `architecture/utils.md` — module table row removed and rows renumbered; Phase G removal note added alongside the Phase D `cache` note; module/file counts corrected 13→12 `pub mod` and 14→13 `.rs` files; the `redaction.rs:10-53` regex-convention bullet replaced; the `### Redaction (redaction.rs)` section removed; the Integration Points table corrected.
- `crates/eggsec/src/utils/mod.rs` — module doc comment updated to state the Phase G disposition and point at the actual redaction contract.

Static guard added:

- **Check 147** — "removed utils::redaction stays removed", in the shape of check 126.

Documentation drift corrected (found during this milestone, see §10):

- `architecture/utils.md` claimed `fuzzer/`, `waf/`, and `proxy/` consumed `redaction`. A repo-wide search for `redact` across all three directories returns **zero** hits — those consumers never existed. The claims were removed rather than preserved, since a guard-free doc claim that is false is worse than no claim.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `PublicationPolicyPy` in `crates/eggsec-python/src/integrations.rs` is an inert data class. Its fields — `redact_sensitive` (default `true`), `include_evidence` (default `false`), `min_severity`, `allowed_finding_types`, `blocked_tags` — are stored and re-emitted in `__str__`/repr, but **no engine logic reads them**. `PublicationPolicyPy` is referenced only by `m.add_class::<…>()` in `lib.rs:1091`. A caller setting `redact_sensitive=True` gets no redaction, because none is implemented anywhere. | A public Python API advertises a security control that does not exist. No active leak today, because `include_evidence` defaults `false` and nothing consumes the policy — but the flag's presence would lead a caller to believe publication masks credentials. | New corrective plan. Out of M001's scope: §5 forbids scope expansion, and "changing behavior at the call site" under Option 1 would have required adopting redaction here, which is exactly the different-policy stop condition. The fix is to either implement `RedactionState`-derived publication masking or remove the inert fields. |
| low | 26 passing `redaction.rs` tests deleted | Loss of tested regex-masking behavior. Mitigated by the code being unreachable. | Accepted. Disclosed here and in `architecture/capability_segregation.md`. |

No critical or high findings. No known unsafe-to-merge condition.

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.**

M003's interface dependency is now discharged: `redaction.rs` is deleted, so `eggsec-secrets` is scoped to `recon/secrets.rs` alone with no `serde_json` requirement. M002 and M004 were already dependency-ready.

M005 remains blocked on M002–M004 plus a maintainer decision that is not a code question.

## 12. Registry updates

- `plans/subsystems/security-knowledge-corpus-roadmap.md` — milestone 1 status → closed; §7 milestone 1 exit conditions recorded as met; §4 candidate table disposition updated; §6 dependency note updated (the M001 → M003 interface edge is discharged).
- `plans/registry.md` — milestone 1 status → closed, with the check 147 guard recorded.