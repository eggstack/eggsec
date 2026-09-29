# NSE Runtime Extraction Milestone 006A — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/006-standalone-0-2-0-release.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-006--020-release-and-safe-eggsec-adoption`

Repository baselines reviewed: standalone `eggstack/eggsec-nse@9fe149fbb22480a63e254e91d60083b7a29a8ff4` (plan baseline); release-candidate `ff0d2c09feba1bcd7ba5f7312579690d905be09c`.

Implementation commits or pull requests:

- Standalone `main`: `ff0d2c0` — "release(nse): prepare 0.2.0 candidate (provider surface, accounting fix, workflow/docs)" (version/changelog/README/release-workflow/RELEASING.md + regenerated `Cargo.lock`).
- Tag `v0.2.0` → `ff0d2c0` (annotated; `v0.2.0^{commit} == ff0d2c0`).
- GitHub Release `v0.2.0` (`https://github.com/eggstack/eggsec-nse/releases/tag/v0.2.0`).
- crates.io `eggsec-nse 0.2.0` published from the exact clean candidate via the documented manual-token recovery path (Trusted Publishing registry-side still pending, fail-closed as designed).
- Hosted CI run `36521717829` on the exact candidate SHA: fully green.

## 1. Executive finding

Milestone 006A is complete. `eggsec-nse 0.2.0` is published from a fully qualified source commit with an immutable tag, registry/docs.rs/scratch-consumer verification, and a truthful release record (breaking signature, provider surface, accounting correction, ungated residual). No Eggsec consumer change occurred in this slice. **GO for 006B** (Eggsec adoption + dormant adapter staging).

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Cargo version exactly 0.2.0 | `Cargo.toml` `version = "0.2.0"`; `Cargo.lock` root `0.2.0`; `cargo metadata --no-deps` | pass | — |
| Changelog/release notes describe break + behavior changes | `CHANGELOG.md` 0.2.0 (Added/Changed/Security sections); GitHub Release notes | pass | Breaking `register_vulns_library` + send/write accounting + residual disclosure |
| README installation targets 0.2 | `README.md` `version = "0.2"` | pass | — |
| Release workflow provisions ripgrep before boundary script | `release.yml` explicit `rg` install step (fail-closed, same posture as CI) | pass | Tag-only trigger, min permissions, tag==version + clean-tree gates preserved |
| Release docs no longer overclaim `-D warnings` gate | `RELEASING.md` lint-gate note (accepted debt named); `README.md` build-verify line fixed; plain-clippy gate | pass | `cargo clippy ... -- -D warnings` exits 101 on debt (deprecated `as_utf8`, unreachable pattern, etc.); debt NOT cleared here per plan |
| Exact candidate SHA fully green hosted CI | Run `36521717829` on `ff0d2c0`: msrv/Ubuntu/macOS/Windows/ssh-runtime all success | pass | §4 |
| `cargo publish --dry-run` + package verification pass | dry-run ok; `cargo package` 311 files; dirty-tree dry-run correctly refused (no `--allow-dirty`) | pass | §4 |
| 0.2.0 published and publicly resolvable | `cargo publish` ok; `cargo search`/`cargo info` resolve 0.2.0 | pass | Manual-token recovery path; no credential committed/printed |
| Clean scratch consumer builds registry artifact | `/tmp/opencode/nse-consumer-020`: `=0.2.0` from `registry+https://github.com/rust-lang/crates.io-index`; `cargo check` with `nse,nse-ssh2,sandbox` ok | pass | No Git/path source |
| Published source identity matches qualified candidate | `.cargo_vcs_info.json` in published archive: `sha1 ff0d2c0` == candidate == tag commit | pass | — |
| `v0.2.0` + GitHub Release point to same source commit | `v0.2.0^{commit} == ff0d2c0`; Release notes disclose break/providers/accounting/residual | pass | — |
| docs.rs builds or delay/failure classified | docs.rs serves `eggsec-nse 0.2.0` API docs (built 2026-09-29) | pass | — |
| No Eggsec consumer change in this plan | Eggsec untouched (still crates.io 0.1.0); no adapter merge | pass | — |
| Closure unblocks 006B | This record + registry/roadmap updates | pass | §11 |

## 3. Production implementation evidence

Standalone only (this plan owns the producer boundary):

- `Cargo.toml`: `0.1.0` → `0.2.0` (name/edition/license/MSRV/features unchanged).
- `Cargo.lock`: regenerated root entry `0.2.0`.
- `CHANGELOG.md`: full 0.2.0 entry (provider surface, breaking `register_vulns_library(lua, capability_ctx)`, direction-correct send accounting + HTTP split + live write-limit preflight, 72-file residual disclosure, no-corpus note) + `0.2.0` release link.
- `README.md`: dependency example `0.2`; build-verify clippy line de-overclaimed to plain `--features nse`.
- `.github/workflows/release.yml`: ripgrep provisioning step before `./scripts/check-boundaries.sh` (apt-get with fail-closed fallback error); tag-only semantics, `contents: read` + job `id-token: write`, tag==version/clean-tree gates unchanged; no `pull_request_target`/`workflow_run`/`continue-on-error`.
- `docs/RELEASING.md`: lint-gate note (accepted warning debt named, `-D warnings` must not be re-added without a separate debt-clearing plan); release-history subsection recording 0.1.0 bootstrap + 0.2.0 provenance.
- No provider/runtime semantic change: M005 behavior untouched (release prep only).

## 4. Verification executed

### Commands run

```bash
# Standalone at release candidate ff0d2c0 (clean tree)
cargo fmt --all --check
./scripts/check-boundaries.sh
cargo metadata --no-deps
cargo tree --workspace
cargo check --no-default-features
cargo check --features nse
cargo test --features nse
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo clippy --all-targets --features nse
cargo +1.89.0 check --locked --no-default-features
cargo +1.89.0 check --locked --features nse
cargo publish --dry-run
cargo package --list
cargo package
# Hosted: push main ff0d2c0 → CI run 36521717829 (Ubuntu/macOS/Windows/MSRV/SSH)
# Publication: cargo publish (manual-token recovery path)
# Registry: clean scratch crate eggsec-nse = "=0.2.0" → fetch/tree/check (nse,nse-ssh2,sandbox)
# Identity: .cargo_vcs_info.json sha1 vs candidate vs tag
# docs.rs: https://docs.rs/eggsec-nse/0.2.0 served
```

### Results

- fmt ok; boundaries ok (incl. M005E pins); metadata/tree ok.
- Checks ok: no-default, nse, nse-ssh2, nse+sandbox (0 errors).
- `cargo test --features nse`: **639 passed, 0 failed, 1 ignored across 28 suites** (= 637 M005 baseline + 2 tooling tests).
- Clippy (plain, no `-D warnings`): exit 0, 0 errors. With `-D warnings`: exit 101 on pre-existing accepted debt (documented, not cleared).
- MSRV 1.89: both configurations ok.
- `cargo publish --dry-run` ok (dirty-tree refusal verified pre-commit; no `--allow-dirty`).
- `cargo package`: 311 files, 3.0MiB (512.4KiB compressed); `.crate` sha256 `bf8befe7…0ffe`.
- Hosted run `36521717829` on exact `ff0d2c0`: **msrv success, rust (ubuntu) success, rust (macos) success, rust (windows) success, ssh-runtime success**.
- Publication: `Published eggsec-nse v0.2.0 at registry crates-io` (clean tree, verified build).
- Scratch consumer: resolves `registry+https://github.com/rust-lang/crates.io-index` 0.2.0; `cargo check` with all three features ok.
- Source identity: archive `.cargo_vcs_info.json` sha1 `ff0d2c0` == candidate == `v0.2.0^{commit}`.
- docs.rs: 0.2.0 crate page + API docs served (built 2026-09-29).
- Not run: Windows/macOS locally (carried by hosted matrix); crates.io JSON API direct query (403 via local egress — cargo sparse-index resolution used instead).

## 5. Invariant review

- Package name `eggsec-nse`, version exactly `0.2.0`, edition 2021, MIT, MSRV 1.89 — unchanged except version.
- Zero `eggsec-*` dependencies/imports — untouched (boundary guard green in CI).
- Feature names unchanged — verified via `cargo info` feature list + checks.
- `NseRunReport` serialization untouched — no report-code diff in slice.
- M005 provider/accounting behavior not rewritten — release-prep-only diff.
- 72-file ungated residual documented + guard-pinned — changelog + release notes disclose; pins green.
- No tag before qualification — tag created after hosted green + publication from same SHA.
- No `--allow-dirty`/`--no-verify`; no credential committed/printed — publish log redacted; token only in local cargo store.

## 6. Failure and recovery review

- Dirty-tree `cargo publish --dry-run` correctly refused pre-commit; committed, re-ran clean — no override flag used at any point.
- No publication conflict (0.2.0 absent — confirmed via `cargo search` showing only 0.1.0 pre-publish).
- No yank needed; artifact verified healthy post-publish. Rollback guidance in RELEASING.md stands (Git revision remains Eggsec fallback until 006B adopts).
- No restart/durability semantics involved (release process only).

## 7. Migration and compatibility review

- 0.x minor bump is the correct vehicle: breaking `register_vulns_library` signature + new public provider surface + accounting behavior fix (write-limit now live — sends that previously passed only because the limit was dead now fail closed, intended).
- Consumers must update the one call site to pass `&NseCapabilityContext` (Eggsec adoption owns this in 006B if affected).
- No compatible 0.1.x patch path exists for this surface (signature break forbids it).
- Rollback for Eggsec pre-adoption: prior commit consuming crates.io 0.1.0.

## 8. Security review

- No authorization logic touched; no scope/authority fabrication; no secrets handled (publish token via local credential store only).
- Release workflow keeps minimum permissions, tag-only trigger, no privileged workflow constructs.
- Residual risk disclosed, not hidden: 72-file ungated specialized direct-I/O set (medium, M005E classification stands), mixed advisory files, HTTP preflight byte-hint absence — all carried from 005E §10, now restated in the 0.2.0 changelog/release notes so downstream embedders cannot mistake provider availability for complete scope enforcement.

## 9. Documentation and operations

- Standalone: `CHANGELOG.md` 0.2.0 entry, `README.md` version + lint-gate fix, `docs/RELEASING.md` lint-gate note + release history, `release.yml` ripgrep prerequisite.
- Eggsec: this closure record; plan status `implemented`; registry + subsystem roadmap updates (006A closed, 006B ready).
- Operator impact: none until 006B adoption. Downstream `max_network_bytes_written` users should expect live enforcement (documented in changelog).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low (operational) | Trusted Publishing registry-side entry still pending | Release workflow fails closed at auth; manual-token path required | Crate owner adds Trusted Publisher entry via crates.io web settings (no code change); future releases prefer workflow path |
| low | Accepted clippy warning debt (deprecated `as_utf8`, etc.) | `-D warnings` gate cannot be claimed | Separate debt-clearing plan before re-adding the strict gate |
| info | crates.io JSON API 403 via local egress | No direct API evidence; cargo sparse-index + docs.rs used instead | None — closure evidence does not depend on it |

No high/medium findings. M005E residual list unchanged.

## 11. Roadmap disposition

**Milestone 006A closed; 006B may proceed (GO).** Next: Eggsec adoption of crates.io `eggsec-nse 0.2.0` with dormant adapter replay + automated-exposure quarantine per `006-eggsec-0-2-0-adoption-safe-staging.md`. Automated NSE activation remains deferred to M007.

## 12. Registry updates

- `plans/registry.md`: move 006A to closed; 006B becomes dependency-ready; 006C remains blocked on 006A+006B.
- `plans/subsystems/nse-runtime-extraction-roadmap.md`: M006A status closed with 0.2.0 artifact identity; M006B ready.
