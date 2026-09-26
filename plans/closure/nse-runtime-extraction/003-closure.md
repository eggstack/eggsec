# NSE Runtime Extraction Milestone 003 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/nse-runtime-extraction/003-standalone-repository-extraction.md`

Source subsystem roadmap:

- `plans/subsystems/nse-runtime-extraction-roadmap.md#milestone-003--standalone-repository-extraction-and-cross-repository-qualification`

Repository baseline reviewed: `d743ef2e6064a2a9d2ab575e1c4e3f996f3f7c61`

Implementation commits or pull requests:

- `3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2` — standalone `eggstack/eggsec-nse` runtime qualification commit.
- `f6c1ea3a5402aae2bca99efb4c261984d64907a8` — Eggsec exact-revision integration, source removal, guards, feature sweep, and documentation.

## 1. Executive finding

Milestone 003 is complete. The runtime was moved with history into the canonical standalone repository, given explicit package metadata and standalone docs/provenance, and qualified by independent Linux/macOS CI at an immutable commit. The standalone CI exercised the SSH runtime against local OpenSSH. Eggsec now consumes precisely that commit through one optional engine dependency; its former local runtime crate is removed. Eggsec's facade, TUI/Python feature forwarding, authorization/dispatch, report conversion, and HTTP capability adapter remain engine-owned.

Standalone GitHub Actions run [36214729637](https://github.com/eggstack/eggsec-nse/actions/runs/36214729637) passed on the qualified revision.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| Preserve source history and exact content | `git subtree split`; extracted tree `a46ffe78a6fe179203cc25bddaea8f4e7e39c912` matched the source tree at baseline | pass | Filtered-history seed `6e5abf5fcb9a9a28df78914495aad1471a17c284`; provenance records the source baseline. |
| Standalone metadata, docs, provenance, and packageability | Standalone `cargo metadata`, boundary script, and `cargo package` | pass | 296 package files; archive preflight succeeded; no Eggsec crate dependency. |
| Runtime behavior and compatibility corpus | Standalone `cargo test --features nse` | pass | 558 passed, 1 ignored; includes clean-room fixtures. |
| Feature combinations and MSRV | Standalone Linux/macOS CI and Rust 1.89 jobs | pass | Linux ran tests/clippy/package; macOS compiled/package-checked; MSRV no-default and `nse` checks passed. |
| Real SSH runtime path | Standalone Linux CI local `sshd` integration | pass | Disposable local endpoint and deterministic test credentials; no public target. |
| Exact external dependency | Eggsec `Cargo.toml`, `Cargo.lock`, metadata and dependency tree | pass | `eggsec-nse` resolves to `3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2`; Cargo Deny requires `rev`. |
| Eggsec feature/adapter behavior | Engine NSE suites; TUI NSE tests; Python NSE tests; combined SSH/sandbox/CLI check | pass | 1,710 engine unit tests; 237 adapter/integration tests; TUI 903 passed/12 ignored; Python 231 passed. |
| Full workspace and feature verification | `make check`; `make check-features-individual`; `make check-python` | pass | See §4 for exact results and the one transient Python socket-budget failure that passed on isolated rerun. |
| Future publication/provider work disposition | Roadmap and registry update | pass | Milestone 004 planning is unblocked; operational release/name checks remain implementation gates. Milestone 005 may be planned, with implementation evidence-gated. |

## 3. Production implementation evidence

The standalone repository is `https://github.com/eggstack/eggsec-nse`. It owns runtime source, fixtures, runtime tests, README/license, compatibility and provenance docs, contributor guidance, boundary checks, and CI. Its CI qualifies Linux and macOS, checks Rust 1.89, runs package verification, and exercises SSH using local OpenSSH. Its current qualified revision is `3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2`.

Eggsec removed `crates/eggsec-nse` from workspace membership and source. Only `crates/eggsec/Cargo.toml` declares the optional runtime dependency, pinned to the exact Git revision. `Cargo.lock` records the same SHA. Cargo Deny allows only this Git URL and still requires a `rev`. Guards 144–146 enforce the absence of the in-tree package, exact external pin, single direct consumer, facade/feature forwarding, and engine-owned adapters.

The Eggsec report bridge (`nse_bridge`) and scoped HTTP adapter (`nse_http_capability`) remain engine-owned. No protocol, report DTO, feature-name, authorization, or execution-profile behavior was intentionally changed.

## 4. Verification executed

### Commands run

```bash
# Standalone checkout at 3f57e6c33c8fb17f39ddfcd3bde80a2bde6769a2
cargo fmt --all --check
scripts/check-boundaries.sh
cargo metadata --no-deps
cargo tree --workspace
cargo check --no-default-features
cargo check --features nse-ssh2
cargo check --features nse,sandbox
cargo +1.89.0 check --locked --features nse
cargo test --features nse
cargo package

# Eggsec at the matching lockfile revision
cargo check -p eggsec --features nse-ssh2,nse-sandbox,cli
cargo test -p eggsec --features nse,cli --lib
cargo test -p eggsec --features nse,cli --test nse_bridge_tests --test nse_integration_tests --test nse_real_scripts --test nse_tests
cargo test -p eggsec-tui --features nse
cargo test -p eggsec-python --features nse
make check-deps
make check-features-individual
make check
make check-python
```

### Results

- Standalone: `cargo test --features nse` — 558 passed, 1 ignored; `cargo package` succeeded; format, boundary, metadata, feature, and MSRV checks passed. Standalone Clippy CI passed (existing warning debt is recorded by CI; no Clippy errors).
- Standalone CI: run 36214729637 passed for Linux, macOS, Rust 1.89, package, and local SSH runtime qualification.
- Eggsec combined `nse-ssh2,nse-sandbox,cli` check passed (four existing unrelated warnings).
- Eggsec NSE unit tests — 1,710 passed. Four NSE engine/adapter integration suites — 237 passed.
- TUI NSE tests — 903 passed, 12 ignored. Python NSE tests — 231 passed.
- `make check-deps` passed after adding the single exact-revision Git source allow-list entry.
- `make check-features-individual` passed (exit 0), covering declared engine, TUI, domain, daemon/CLI, and Python feature profiles.
- `make check` passed after correcting a stale feature-matrix expectation that still named the removed local package. It includes fmt, workspace checks, dependency policy, clippy, contract tests, and architecture guards.
- The first full `make check-python` run had 4,453 passes and one socket cleanup budget failure (one transient socket remained at the immediate sample). The isolated test passed, then a full rerun of `make check-python` exited 0.
- TUI NSE tests: 903 passed, 12 ignored; Python NSE tests: 231 passed. Full `make check-python` passed on rerun.

## 5. Invariant review

- Eggsec authorization remains outside the runtime. The extraction changes only dependency ownership; dispatch still uses the canonical Eggsec `EnforcementContext` and approved execution flow.
- Strict automated surfaces remain routed through canonical engine dispatch. The engine facade and adapter tests pass; TUI/Python continue forwarding `nse` through `eggsec/nse` without direct runtime dependencies.
- Runtime features and Eggsec feature names remain intact; the feature sweep passed.
- The runtime continues to own canonical request execution, resolver policy, profiles, limits, cancellation, compatibility/fidelity reports, and serialization. Standalone corpus and Eggsec adapters pass.
- Resolver containment and capability restrictions remain within the standalone runtime and its regression suites; no authorization claim was transferred to the runtime.
- The clean-room corpus and provenance assets moved together; no upstream Nmap corpus was added.
- TLS provider policy remains ring-only for Eggsec-owned rustls users; runtime packaging uses its declared standalone dependency policy.

## 6. Failure and recovery review

No persistent runtime state, daemon protocol, restart behavior, or migration was introduced. Runs remain request-scoped and cancellation/resource-limit semantics are qualified by runtime tests. SSH CI uses a local disposable service with cleanup. Build fan-out remains bounded by existing Cargo and test harness behavior; this milestone adds no runtime task spawning.

## 7. Migration and compatibility review

No persistent-data migration or serialized DTO change occurred. The source graph changed from workspace path membership to a Git source pinned to one commit. Rollback is possible by restoring the prior workspace revision; future upgrades require updating the exact `rev`, lockfile, and cross-repository qualification evidence together. The Git source is temporary pending Milestone 004's registry release.

## 8. Security review

Eggsec remains the authorization and scope authority; runtime profiles/capabilities do not replace Eggsec enforcement. No secrets or credentials are persisted: SSH CI uses generated disposable test credentials. Runtime path containment, capability denial, limits, cancellation, and sandbox tests remain in the standalone project. The Eggsec static guard fails if the dependency becomes branch-only, if a second direct consumer appears, or if the runtime returns as a local workspace member. Cargo Deny now allows only the canonical NSE runtime source and requires an exact `rev`.

## 9. Documentation and operations

Updated workspace and NSE architecture docs, compatibility and feature docs, Python architecture docs, release/dependency guidance, architecture guards, feature sweep, and the Eggsec NSE skill. Historical baseline documents are explicitly marked as pre-extraction evidence; grandfathered plans were not rewritten. Standalone operating and provenance material lives with the external runtime.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| low | The runtime remains a Git dependency until its first crates.io release; crate-name availability and release infrastructure were not independently confirmed during extraction. | Publication cannot begin without checking registry availability and release permissions. | Milestone 004 must verify the name, release credentials/workflow, and reproducible archive before publishing. |

No correctness or security findings remain for Milestone 003.

## 11. Roadmap disposition

Milestone closed; the next dependency may proceed. Milestone 004 plan authoring is unblocked by this closure, while crate-name availability and release infrastructure remain operational gates before implementation. Milestone 005 plan authoring may proceed; provider-interface implementation remains conditional on measured behavior or concrete consumer evidence. No corrective implementation plan is required for this milestone.

## 12. Registry updates

- Mark Milestone 003 closed and link this record from the roadmap.
- Mark the 003 implementation plan closed and record its implementation baseline and external SHA.
- Update the registry to record 001–003 closed; unblock Milestone 004 planning while retaining its operational gates; permit Milestone 005 planning while preserving its evidence gate.
- Keep the NSE subsystem active for future milestones; no implementation work is presently blocked.
