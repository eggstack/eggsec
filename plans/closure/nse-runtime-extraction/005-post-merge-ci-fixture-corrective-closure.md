# NSE Runtime Extraction Milestone 005 — Post-Merge CI Fixture Corrective Closure

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/005-post-merge-ci-fixture-corrective.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-005--host-provider-inversion-and-portability-hardening`

Prior closure reconciled by this pass:

- `plans/closure/nse-runtime-extraction/005-provider-stack-landing-ci-corrective-closure.md` (was `conditionally closed`, restored to `closed` by this pass)

Repository baselines reviewed: Eggsec `88ee459c` (pre-reconciliation HEAD); standalone pre-corrective `1134c289b71a07fda21a8554782fd8101df5396f` (origin/main before push).

Implementation commits:

- standalone `9fe149fbb22480a63e254e91d60083b7a29a8ff4` — `ci(nse): make missing-ripgrep fixture hermetic` (single commit on top of `1134c28`, pushed fast-forward to `eggsec-nse/main`).
- this Eggsec reconciliation commit (registry/roadmap/closure updates; no production code).

## 1. Executive finding

The M005 post-merge CI defect is corrected from observed hosted evidence. The negative tooling test no longer assumes any fixed system directory excludes `rg`; the missing-tool environment is a hermetic empty temporary directory proven to contain no resolvable `rg` before the script runs. Hosted GitHub Actions run `36490773625` on the exact new standalone main SHA `9fe149fbb22480a63e254e91d60083b7a29a8ff4` concludes **success** with all five required jobs green (Ubuntu, macOS, Windows, MSRV 1.89, SSH runtime). The Ubuntu log proves both tooling tests pass in the environment where apt-installed ripgrep is present (`test script_fails_fast_when_ripgrep_missing ... ok`, `test script_passes_when_ripgrep_available ... ok`, `test result: ok. 2 passed`).

No provider trait, DTO, broker, native provider, library migration, accounting behavior, or profile semantics changed: `git diff 1134c28..9fe149f --stat` touches exactly two files (`scripts/check-boundaries.sh`, `tests/boundary_check_tooling_tests.rs`). Parent M005 is restored to `closed` and the `eggsec-nse 0.2.0` release/adoption follow-up is restored to **dependency-ready**.

## 2. Requirement-to-evidence matrix

| Requirement (plan §13) | Evidence | Result | Notes |
|---|---|---|---|
| 1. Negative test no longer assumes `/usr/bin:/bin` excludes ripgrep | `MINIMAL_PATH` const removed; child `PATH` is an empty temp dir | pass | §3 |
| 2. Child environment proves `rg` is unavailable | directory scan (`path_contains_rg`) + child-shell `command -v rg` probe, both asserted before invoking the script | pass | §3 |
| 3. Script exits 127 before normal guards in hermetic no-`rg` env | prerequisite check moved above `cd`/`dirname`; direct empty-PATH invocation exits 127 | pass | §3, §4 |
| 4. Diagnostic once, no success marker | `printf`-builtin diagnostic; test asserts marker count == 1 and success marker absent | pass | §4 (local + hosted) |
| 5. Positive test passes with normal PATH | inherited-PATH test unchanged; green locally and on hosted Ubuntu | pass | §4 |
| 6. Boundary guards unchanged in semantics, pass normally | guard regexes untouched; `./scripts/check-boundaries.sh` green locally and in hosted Ubuntu/macOS steps | pass | §4 |
| 7. Corrective change pushed to `eggsec-nse/main` | fast-forward `1134c28..9fe149f`, `ls-remote` confirms origin/main at `9fe149f` | pass | §4 |
| 8. Hosted run on exact new main SHA concludes success | run `36490773625`, conclusion `success`, `headSha` equals final main | pass | §4 |
| 9. Ubuntu/macOS/Windows/MSRV/SSH all success | per-job conclusions recorded from run API | pass | §4 |
| 10. No provider/runtime production behavior changes | two-file diff stat; smoke suites green | pass | §3, §4 |
| 11. Prior landing/CI closure amended with observed run | second corrective addendum appended; status restored to `closed` | pass | §9 |
| 12. Parent M005 restored to closed only after hosted success | roadmap §7 + §12 updated after run `36490773625` completed | pass | §9, §11 |
| 13. 0.2.0 release/adoption dependency-ready only after hosted success | registry `Blocked work` updated after run completion | pass | §9, §11 |
| 14. Staged HTTP adapter + protocol-gating sequencing unchanged | no adapter branch advanced; gating still sequenced after release/adoption | pass | §7 |

## 3. Production implementation evidence

Standalone diff `1134c28..9fe149f` (2 files, no workflow change):

- `scripts/check-boundaries.sh`: the `command -v rg` prerequisite moves above `cd "$(dirname "$0")/.."` and the heredoc diagnostic (which required the external `cat`) becomes a `printf`-builtin emission. The fail-fast path now uses shell builtins only (`command`, `printf`, `exit`), so it works with an empty tool `PATH` where even `dirname` is unresolvable. Diagnostic text and exit code 127 are unchanged; no guard regex was rewritten.
- `tests/boundary_check_tooling_tests.rs`: the negative test builds an empty temporary directory (`eggsec-nse-no-rg-<pid>-<counter>` under the platform temp dir), asserts no `rg` file exists in it, asserts a fresh child shell cannot resolve `rg` under that `PATH`, then invokes the script via an absolute Bash path (`/usr/bin/bash` or `/bin/bash`, probed) with the script referenced by absolute path (`CARGO_MANIFEST_DIR`-anchored). The positive test is unchanged (inherited PATH).

Why the original verification did not catch it: the defect is layout-dependent. On developer machines `rg` typically lives outside `/usr/bin:/bin` (e.g. `~/.cargo/bin`), so the fixed-PATH fixture passed locally; only the hosted Ubuntu runner, where the corrected workflow installs ripgrep via apt into `/usr/bin`, exposed the contamination. The hermetic design removes the layout assumption entirely.

## 4. Verification executed

### Standalone local (on `9fe149f` before push)

```bash
cargo fmt --all --check                                          # ok (after cargo fmt)
cargo test --features nse --test boundary_check_tooling_tests -- --nocapture  # 2 passed
./scripts/check-boundaries.sh                                    # standalone boundary and provenance checks passed
PATH=<empty-tmpdir> /bin/bash $(pwd)/scripts/check-boundaries.sh  # exit 127, one diagnostic, builtin-only path
cargo check --features nse                                      # 0 errors; 92 pre-existing warnings (matches 005E baseline)
cargo test --features nse --test provider_composition_tests     # 8 passed
cargo +1.89.0 check --locked --features nse                     # ok (92 pre-existing warnings)
```

### Standalone landing

```text
$ git push origin main
   1134c28..9fe149f  main -> main
$ git ls-remote origin main
9fe149fbb22480a63e254e91d60083b7a29a8ff4  refs/heads/main
```

`git log 1134c28..main` enumerates exactly one commit (`9fe149f`); no rebase, no squash, no merge commit. The five M005A-E implementation SHAs remain ancestors of main.

### Hosted post-merge CI (observed, not predicted)

Run `36490773625` — https://github.com/eggstack/eggsec-nse/actions/runs/36490773625 — on `headSha` `9fe149fbb22480a63e254e91d60083b7a29a8ff4`, conclusion **success**:

| Job | Conclusion |
|---|---|
| `rust (ubuntu-latest)` | success (boundary script, fmt, metadata, checks, full `cargo test --features nse`, clippy, package) |
| `rust (macos-latest)` | success |
| `rust (windows-latest)` | success (compile/check qualification; boundary + ssh steps skipped per portability plan) |
| `msrv` (Rust 1.89) | success |
| `ssh-runtime` | success |

`headSha` equals the `ls-remote` origin/main SHA above: the successful run is on the exact final main. Ubuntu log evidence: `test script_fails_fast_when_ripgrep_missing ... ok`, `test script_passes_when_ripgrep_available ... ok`, `test result: ok. 2 passed` for `boundary_check_tooling_tests`, zero `FAILED` lines in the full-suite step.

### Eggsec planning repository (documentation-only reconciliation)

```bash
bash scripts/check-architecture-guards.sh   # green (see §9)
```

## 5. Invariant review

Per source-plan §4, each invariant holds:

- No M005 provider trait, DTO, broker, native provider, library migration, accounting behavior, or profile semantics changed (two-file tooling diff; provider smoke suites green).
- `scripts/check-boundaries.sh` is not weakened: guard regexes untouched; the script still fails closed (exit 127) on a missing prerequisite and enforces all M005A-E guards when `rg` is present.
- The explicit ripgrep CI installation remains in the workflow (unchanged file); the negative test no longer depends on it being absent from any fixed directory.
- No host-specific path encodes "tool absent".
- Windows compile/check, MSRV 1.89, and SSH runtime jobs all ran and succeeded on the exact main SHA.
- No crates.io publication occurred; Eggsec remains on `eggsec-nse 0.1.0`; the staged Eggsec HTTP adapter branch was not advanced; protocol-gating remains a separate sequenced follow-up.
- M005/0.2.0 release readiness is restored only on the observed green run recorded above.

## 6. Failure and recovery review

- The red hosted run `36486527159` (Ubuntu tooling-test failure, exit 0 vs expected 127) is the classified trigger; root cause is fixture contamination, not a guard or provider defect — the real boundary script passed in that same failing job after ripgrep installation.
- No runtime lifecycle, cancellation, restart, or persistence behavior changes (test/script tooling only).
- If a future hosted run on a later main diverges, that divergence is new evidence to classify separately; this closure pins its claims to run `36490773625` on SHA `9fe149f`.
- No `continue-on-error`, no skipped required jobs, no shell-fallback masking: the Ubuntu job ran the full matrix including the hermetic negative test.

## 7. Migration and compatibility review

No runtime or consumer migration. Standalone main moves `1134c28` → `9fe149f` (test/tooling only). Eggsec dependency stays at published `eggsec-nse 0.1.0`. The staged Eggsec-side HTTP adapter branch is untouched; activation still waits for the 0.2.0 adoption milestone and the NSE enforcement-metadata prerequisite. Protocol-library capability gating remains sequenced after release/adoption and is not absorbed here.

## 8. Security review

No authorization, scope, transport, secret-handling, or report-surface change. The edited script path is CI tooling executed with the repository's own permissions; the fail-fast diagnostic contains no secrets. The hermetic test creates empty temp directories (no executable content, no fake `rg` placeholder) and cleans up best-effort after the run.

## 9. Documentation and operations

- Prior corrective closure `005-provider-stack-landing-ci-corrective-closure.md`: status restored to `closed`; second corrective addendum appended with the observed run ID/URL, per-job outcomes, and explicit supersession of the §7 "expected outcomes" statements.
- `005a/005b/005c/005d/005e` closures: each receives a brief second addendum noting the fixture defect and pointing at this closure as the observed-evidence source of truth (their implementation evidence is unchanged).
- Implementation plan `005-post-merge-ci-fixture-corrective.md`: status `ready for handoff` → `implemented`.
- Roadmap `plans/subsystems/nse-runtime-extraction-roadmap.md`: M005 status `corrective pass required` → `closed`; milestone table updated; header status `active` → `closed` (all milestones 001–005 closed; 0.2.0 release/adoption and protocol gating are sequenced follow-ups, not roadmap milestones).
- `plans/registry.md`: subsystem row `active` → `closed`; fixture corrective listed as closed with this closure record; landing corrective restored to `closed`; 0.2.0 release/adoption moved from blocked to dependency-ready.
- Operator note: contributors running the negative test locally need no special setup; the hermetic fixture works regardless of where `rg` is installed.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | Hosted CI log retention is external (GitHub); closure pins run ID/URL but log content may age out | traceability of log-line quotes degrades over time | none now; run conclusion + per-job outcomes are recorded in the run API |
| — | `eggsec-nse 0.2.0` release/adoption + protocol-library capability gating | sequenced follow-ups, not blockers | separate milestone plans; do not reopen this pass |

No high-severity or correctness finding remains. No publication occurred in this pass (confirmed: only `git push origin main`; no `cargo publish`).

## 11. Roadmap disposition

Milestone M005 **closed**; subsystem roadmap **closed**. The `eggsec-nse 0.2.0` release/adoption follow-up is **dependency-ready** (GO for release/adoption planning using the 005E recommendation as input). Protocol-library capability gating remains sequenced after release/adoption.

## 12. Registry updates

- `plans/registry.md` subsystem standing: `nse-runtime-extraction` → `closed`, latest-evidence pointer to this closure.
- `plans/registry.md` dependency-ready plans: fixture corrective → `closed` (this record); landing corrective → `closed` (addendum-amended); 005A-E slices remain `closed`.
- `plans/registry.md` blocked work: 0.2.0 release/adoption → **dependency-ready** (unblocked by this closure); protocol gating stays sequenced after release/adoption.
- `plans/subsystems/nse-runtime-extraction-roadmap.md` §7 M005 status + §12 milestone table + header status updated as in §9.
