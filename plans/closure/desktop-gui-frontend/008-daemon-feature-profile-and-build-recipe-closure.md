# Desktop GUI Frontend Milestone 008 — Closure Status

Status: closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/008-daemon-feature-profile-and-build-recipe.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Repository baseline reviewed: `0c38c403`

Implementation commits or pull requests:

- `7332baea` — the separately-owned fix for the inherited `E0616` defect that blocked the profile's build (see §3).
- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's capability boundary is complete. The daemon that serves the desktop GUI now has **one named, documented, machine-checked engine feature profile**, and a reproducible build recipe that produces the binary and records exactly what went into it.

The measurement that motivated the milestone was a falsification: `eggsec-daemon`'s `full-executor` cargo feature enabled `eggsec/cli`, and the engine's `cli` feature enables only `clap` — so a `--full-executor` daemon linked the engine with **none** of the engine's 28-member `full` aggregate. Milestone 006's stated user value ("the GUI reaches the engine's full feature set") was therefore unreachable with a stock daemon build.

What landed:

- **One profile, owned by the engine.** `crates/eggsec/src/config/daemon_profile.rs` declares `DAEMON_SERVICE_PROFILE` (20 features), `DAEMON_SERVICE_EXCLUDED` (the other 30), and `DAEMON_SERVICE_FORBIDDEN` (the absolute-no subset). The module is compiled unconditionally — it is data, not a capability.
- **The derivation is not hand-written.** `required_profile_features()` reads `eggsec_policy::ALL_OPERATION_METADATA` and collects the distinct non-empty `required_features`, unions the `cli` host gate and the declared transitive members, and intersects that with the features the daemon's task kinds can actually reach. The catalog-derived names are never transcribed.
- **The build recipe.** `scripts/build-daemon-profile.sh` pins the cargo invocation, refuses a dirty tree without `--allow-dirty`, and writes a metadata sidecar with the commit SHA, the dirty flag, `rustc -Vv`, the resolved toolchain and the MSRV floor, the target triple, the resolved engine feature set, the `Cargo.lock` hash, the binary size, and its SHA-256. It performs no signing, no bundling, no packaging, and no npm step.
- **Conformance is machine-checked.** The static half lives in `crates/eggsec/tests/feature_matrix.rs` and runs in every `make check`; the runtime half lives in `crates/eggsec-daemon/tests/feature_profile.rs` and compares the **compiled** feature set to the documented list. Guard 163 pins the manifest to the engine's contract.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| One named profile with a per-feature rationale for all 50 features | `docs/DAEMON_FEATURE_PROFILE.md` — the `## Included features` table (20) and the `## Excluded features` table (30) | pass | Every one of the engine's 50 registry features carries a rationale |
| The engine owns the list; the daemon consumes it | `crates/eggsec/src/config/daemon_profile.rs`; guard 163 | pass | The manifest's `eggsec/<feature>` entries are compared for **set equality** against `DAEMON_SERVICE_PROFILE` |
| The required set is derived, not transcribed | `required_profile_features()` reads `ALL_OPERATION_METADATA` | pass | See the recorded deviation in §7 |
| The profile is fail-closed on drift | `profile_violations()` reports an unknown name, a forbidden name, a missing required name, a duplicate, and a name that is neither required nor a declared transitive member | pass | Six negative tests cover the forbidden, serving-surface, exposure-marker, aggregate, missing-required, and unknown-name shapes |
| `full-executor` remains a working alias | `full-executor = ["service"]`; test `the_full_executor_alias_builds_the_same_profile` | pass | Every existing invocation keeps working |
| The daemon's default build still links no engine | `default = []` on `crates/eggsec-daemon`; test `the_default_daemon_build_still_links_no_engine` | pass | The profile is opt-in |
| The compiled feature set equals the documented list | `crates/eggsec-daemon/tests/feature_profile.rs` — `the_compiled_engine_feature_set_equals_the_documented_profile` | pass | 20 profile tests green |
| The forbidden features are absent from the compiled set | `the_compiled_feature_set_excludes_the_forbidden_features` | pass | `rest-api`, `grpc-api`, `ws-api`, `insecure-tls`, `test-helpers`, `full`, and the three MCP exposure markers |
| The profile enables the `cli` host gate | `service` enables `eggsec/cli`; guard 104 asserts it | pass | `runtime_bridge`, `commands`, and `dispatch` are `#[cfg(feature = "cli")]` |
| The profile is a build-time property, not a runtime one | `the_profile_is_a_build_time_property_not_a_runtime_one` | pass | The wire-side publication is milestone 006's |
| The protocol version is unchanged | `the_protocol_version_is_unchanged` | pass | Stays 2 |
| The runtime `--full-executor` flag is unchanged | `the_daemon_runtime_flag_is_unchanged` | pass | It is a different mechanism from the cargo feature |
| The prerequisite map distinguishes build-time, runtime-privilege, and runtime-binary | `docs/DAEMON_FEATURE_PROFILE.md` §"System prerequisites" — one row per `all_domain_ids()` domain | pass | `test_every_profile_domain_has_a_platform_prerequisite_row` |
| The stale `libpcap-dev` claim is corrected | `docs/PLATFORM.md`, `docs/FEATURE_MATRIX.md`, `crates/eggsec/src/platform/prereqs.rs`, `scripts/check-features-individual.sh` | pass | `packet-inspection` and `stress-testing` are pure-Rust `pnet` |
| The stale `libssl-dev` claim is corrected | The same four files | pass | `nse` **vendors** OpenSSL and needs a C toolchain |
| The build recipe is reproducible and records its inputs | `scripts/build-daemon-profile.sh`; `make build-daemon-profile` | pass | The script reads the profile from the engine's source of truth and refuses to build on a disagreement |
| The recipe performs no packaging | The script's header and body | pass | No signing, no bundling, no installer, no updater, no npm step |
| The prerequisite probe is advisory | The script's probe calls the engine's own platform matrix through `eggsec doctor`'s JSON mode | pass | It reports; it never blocks the build |
| No new dependency | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` unchanged | pass | — |

## 3. Production implementation evidence

**The profile contract** (`crates/eggsec/src/config/daemon_profile.rs`). Three statics and two functions. `DAEMON_SERVICE_PROFILE` lists the 20 included engine features; `DAEMON_SERVICE_EXCLUDED` lists the other 30; `DAEMON_SERVICE_FORBIDDEN` lists the absolute-no subset. `required_profile_features()` derives the required set from the policy catalog. `profile_violations()` is **pure** and deterministic — it takes a candidate profile and reports what is wrong with it, which is what makes the negative tests possible.

**The manifest** (`crates/eggsec-daemon/Cargo.toml`). The `service` feature enables `dep:eggsec` plus the 20 engine features. `full-executor = ["service"]` is the deprecated alias, with a comment recording that the daemon's **runtime** `--full-executor` flag is a different mechanism and is unaffected.

**The cfg gates.** `crates/eggsec-daemon/src/main.rs` and `crates/eggsec-daemon/src/host.rs` now gate on `service` rather than `full-executor`, and the `--full-executor` flag's help text states that the binary must be **built** with the cargo feature for the flag to have any effect. The not-built branch warns with the exact `cargo install` invocation instead of a vague message.

**The prerequisite map.** `docs/DAEMON_FEATURE_PROFILE.md` carries one row per domain id from `all_domain_ids()` — the six the engine actually knows — with the columns `Domain | Build-time | Runtime-privilege | Runtime-binary`. The three categories are kept apart: only `nse` has a build-time requirement (a C toolchain for the vendored OpenSSL), the privilege rows are Linux-only and scoped to live capture and active wireless, and the runtime-binary rows (`iwlist`, Chrome/Chromium) are runtime-only.

**The build recipe** (`scripts/build-daemon-profile.sh`). It resolves the profile from `DAEMON_SERVICE_PROFILE` rather than re-declaring it, refuses to build when the manifest and the engine's contract disagree, pins `cargo build --release --locked -p eggsec-daemon --features <profile> --target <triple>`, writes the sidecar, and prints the runtime invocation the operator should then use.

**The conformance tests.** The static half — `crates/eggsec/tests/feature_matrix.rs` — parses the daemon's manifest, the engine's registry, the policy catalog, and the profile document, and asserts that all four agree. It runs in every `make check`. The runtime half — `crates/eggsec-daemon/tests/feature_profile.rs` — compares the engine's **compiled** feature set to the documented list and asserts the forbidden features are absent.

**Guard 163** pins the manifest to `DAEMON_SERVICE_PROFILE` by set equality and pins `full-executor` to the alias shape. Guard 104 was **strengthened** rather than deleted: it now parses the manifest with `tomllib` and asserts that `service` enables `dep:eggsec` and `eggsec/cli`, and that `full-executor` is exactly `["service"]`.

**A separately-owned blocking fix.** The `websocket` engine feature had **never compiled**: `WebSocketFuzzer::url` was `pub(crate)` inside the `eggsec-payloads` corpus crate, so the engine's `AdvancedFuzzer` implementation could not read the target it must dial (`E0616`). The defect was invisible because `websocket` is not in the engine's default feature set, so `make check` never compiled it — it surfaced only under `--all-features`, and it **blocked this milestone's profile build outright**. The plan's own stop condition anticipated it and required the fix to be a "separate, separately-owned change"; it was landed as commit `7332baea`, before the milestone work, by exposing the target through a `pub fn url(&self) -> &str` accessor rather than a public field, so the corpus crate keeps control of the struct's shape.

## 4. Verification executed

### Commands run

```bash
bash -n scripts/build-daemon-profile.sh
cargo check -p eggsec --no-default-features
cargo test -p eggsec --features cli --test feature_matrix
cargo check -p eggsec-daemon --features service
cargo test -p eggsec-daemon --features service --test feature_profile
cargo test -p eggsec-daemon --features service
bash scripts/check-architecture-guards.sh
```

### Results

| Command | Result | Detail |
|---|---|---|
| `bash -n scripts/build-daemon-profile.sh` | pass | shell syntax clean |
| `cargo check -p eggsec --no-default-features` | pass | the profile module is compiled unconditionally |
| `cargo test -p eggsec --features cli --test feature_matrix` | pass | **43 passed, 0 failed** |
| `cargo check -p eggsec-daemon --features service` | pass | the profile build is green **after** the `E0616` fix |
| `cargo test -p eggsec-daemon --features service --test feature_profile` | pass | **20 passed, 0 failed** |
| `cargo test -p eggsec-daemon --features service` | pass | the whole crate's suite is green under the profile |
| `bash scripts/check-architecture-guards.sh` | pass | **ALL PASSED** (guard 163 added) |
| `make check-deps` | **fail (inherited)** | Pre-existing advisories; see §10. |

**Not run:** `make check-python` (requires `.venv-ci/`). `make check` cannot complete because it stops at `make check-deps`; every step before it passes, and the steps after it were run directly and are green.

## 5. Invariant review

| Invariant | Evidence |
|---|---|
| The engine's feature registry stays the single source of truth | The profile **derives** from `ALL_FEATURES`, `FULL_MEMBERS`, and `FULL_EXCLUDED_WITH_REASON`. The milestone adds no second hand-maintained feature list; the one list it does declare is checked against the registry by test. |
| The engine's `full` aggregate is unchanged | `full` keeps its 28 members and its 21 documented exclusions; `the_engine_full_aggregate_is_unchanged` and `full_exclusions_are_documented` are green. |
| No new dependency | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` gain no entry. `deny.toml` is a closed allow-list, and the profile's features are all pre-existing engine features — the milestone adds no crate. |
| Ring-only TLS | The profile adds no TLS stack and changes no existing one. The engine's `insecure-tls` is never in the profile — it is in the forbidden set. |
| Workspace Tokio baseline (`default-features = false`, per-crate feature lists) | The daemon's own Tokio feature list is unchanged. The profile adds no Tokio capability. |
| The MSRV floor stays 1.89 | The profile's feature set pulls no crate whose MSRV exceeds the floor; the recipe records the toolchain and refuses one older than 1.89. |
| The daemon's default build still links no engine | `default = []` is unchanged; the profile is opt-in. |
| No protocol change | The protocol version stays 2 and `DaemonCapabilities` is untouched. The profile is a build-side contract; the wire-side publication is milestone 006's. |
| `RuntimeTaskExecutor` is unchanged by this milestone | Untouched — the declaration seam is milestone 009's. |

## 6. Failure and recovery review

- **Duplicate delivery.** Not applicable; the profile is a build-side contract, not a runtime service.
- **Cancellation races.** Not applicable.
- **Daemon restart.** Not applicable.
- **Partial persistence failure.** Not applicable.
- **Stale generation or lease.** Not applicable.
- **Contention and resource release.** Not applicable.
- **Malformed or unauthorized input.** The profile's validator is **fail-closed**: an unknown feature name, a forbidden feature, a missing required feature, a duplicate, or a name outside the declared scope are all violations, not warnings. Six negative tests cover the shapes.
- **Bounded event and artifact behavior.** The build recipe's release build is bounded by the toolchain; the sidecar is a fixed-shape record.

## 7. Migration and compatibility review

- **Schema migration.** None.
- **Backward compatibility.** `full-executor = ["service"]` keeps every existing `--features full-executor` invocation building the same profile. The runtime `--full-executor` flag is unchanged. A binary built without the profile still works and still falls back to a no-op executor — with a warning that now names the exact install command.
- **Protocol negotiation.** Unchanged; the version stays 2.
- **Configuration behaviour.** The profile adds no configuration surface. The build recipe's `--profile`, `--target`, `--out`, and `--allow-dirty` flags are build-time only.
- **Rollback limitations.** Reverting the milestone restores the old, under-featured daemon. Guard 163 would then fail, which is the intended signal.
- **Legacy-path status.** `docs/DAEMON_FEATURE_PROFILE.md` names the deprecated alias and marks it as such; nothing depends on the alias being removed.

## 8. Security review

- **Authorization enforcement.** Unchanged; the profile changes which engine features are compiled into a daemon, not how a request is authorized. Every request still passes through `EnforcementContext::evaluate()` and the canonical executor.
- **Scope checks.** Unchanged.
- **Secret handling.** The sidecar records the commit SHA, the toolchain, the feature set, and the binary's hash — no secret. The build recipe does not read or write any credential.
- **Path validation.** The build recipe writes only under the caller-supplied `--out` directory.
- **Privilege boundaries.** The profile's runtime-privilege rows are documented, not granted: the recipe's probe **reports** that live capture needs `CAP_NET_RAW` and active wireless needs `CAP_NET_ADMIN` and monitor mode. It never escalates.
- **Transport checkpoints.** The profile excludes every serving surface (`rest-api`, `grpc-api`, `ws-api`), so the daemon gains no second network surface. The daemon's transport remains the Unix domain socket.
- **Denial-of-service bounds.** The build recipe is a local build step; it performs no network access.
- **Redaction.** Not applicable.
- **Audit behaviour.** Unchanged.

One security-relevant consequence is worth recording: the profile deliberately **excludes** `db-pentest-mssql-tiberius`, which keeps `tiberius 0.12.3` → `tokio-rustls 0.24.1` → the `rustls 0.21.12` family out of the **shipped** daemon closure. It does **not** remove those crates from `cargo deny`'s view, because `deny.toml` sets `[graph] all-features = true` — the profile narrows the shipped binary, not the advisory set.

## 9. Documentation and operations

- **New:** `docs/DAEMON_FEATURE_PROFILE.md` (the profile, the per-feature rationales, the prerequisite map, and the build recipe), `scripts/build-daemon-profile.sh`.
- **Updated:** `crates/eggsec/src/config/mod.rs`, `crates/eggsec/src/config/feature_registry.rs` (the doc comment recording that `FULL_EXCLUDED_WITH_REASON` is scoped to `full` and that a second aggregate exists), `crates/eggsec/src/platform/prereqs.rs`, `docs/PLATFORM.md`, `docs/FEATURE_MATRIX.md`, `scripts/check-features-individual.sh`, `architecture/overview.md`, `architecture/daemon.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/VERIFICATION.md`.
- **Operator diagnostics.** `make build-daemon-profile` builds the profile and prints the runtime invocation to use next; `make check-daemon-profile` runs the conformance suites; the sidecar next to the binary records exactly what was built.
- **Static guards.** Guard 163 (new). Guard 104 strengthened from a literal string match to a `tomllib` parse.
- **Recovery instructions.** The recipe refuses to build on a dirty tree; re-run with `--allow-dirty` when the operator accepts an unreproducible build, and the sidecar records the dirty flag either way.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | `make check-deps` fails on pre-existing advisories (`zip 2.4.2` — RUSTSEC-2026-0367/0368/0369 — the `rustls 0.21.12` family via `tokio-rustls 0.24.1` → `tiberius 0.12.3` → `eggsec`, and the GUI-reachable `glib`, `glib-macros`, and `unic-*` entries). | The dependency-policy gate is red at the baseline. | **Inherited, not attributable to this milestone.** `deny.toml` is a closed allow-list and the profile narrows only the shipped binary — `cargo deny` resolves every optional closure regardless. Recording the exceptions is a maintainer decision (owner + review-by date). No `deny.toml` entry was added here. |
| low | The plan's "15 catalog-derived names" was wrong: the catalog actually names **19** distinct `required_features`. | A plan-relative count only; no code depended on it. | Corrected in this closure record and in the module's documentation. |
| low | The daemon's `service` profile needs a C toolchain (vendored OpenSSL under `nse`), so it cannot be built in an environment without one, and it is therefore a deep check rather than part of `make check`. | `make check` does not compile the profile. | Accepted. `make check-daemon-profile` is wired into `make check-feature-profiles`. |
| low | The socket-path default is duplicated in six production sites. | A future change to the default must touch all six. | Not this milestone's scope; recorded in the subsystem roadmap. |
| medium | **The `service` profile compiles engine modules the default build never reaches, and those modules carry pre-existing clippy debt.** `cargo clippy -p eggsec-daemon --features service` reports **80** findings across roughly twenty engine files (`packet/parse_impl.rs`, `packet/traceroute.rs`, `packet/craft.rs`, `packet/cli.rs`, `packet/hexdump.rs`, `packet/fixture.rs`, `scanner/ports/spoofed.rs`, `scanner/ports/mod.rs`, `dispatch/security.rs`, `dispatch/canonical_execution.rs`, `tool/service.rs`, `tool/scripting.rs`, `tool/session.rs`, `tool/traits.rs`, `tool/implementations/oast.rs`, `storage/postgres.rs`, `stress/udp.rs`, `stress/icmp.rs`, `stress/warning.rs`, `wireless/active/attacks/deauth.rs`) — large enum variants, format strings ending in a newline, redundant closures and bindings, unused imports, manual checked division, and argument-count lints. None of the files is one this milestone touched. | The profile **builds** cleanly (`cargo check` passes; the only diagnostics are warnings), but the profile is not clippy-clean under `-D warnings`. `make clippy` is unaffected: it lints the engine with `--features cli` only, so this debt is outside the mandatory contract. | **Not fixed here** — 80 lints across twenty engine files is an engine-wide cleanup with real regression risk, well outside this milestone's boundary. `make check-daemon-profile` therefore gates on `cargo check` and its Makefile comment says why. A separate engine lint-cleanup milestone should discharge it. |
| low | **The same two prerequisite claims also appear in the Python bindings' wheel-profile data and in the Python subsystem's documents** — `crates/eggsec-python/wheel-profiles.json`, `crates/eggsec-python/validation/profiles.json`, `crates/eggsec-python/python/eggsec/_capabilities.json`, `crates/eggsec-python/python/eggsec/_feature_guard.py`, `crates/eggsec-python/python/eggsec/__init__.py`, `crates/eggsec-python/pyproject.toml`, `crates/eggsec-python/README.md`, `docs/python/packaging.md`, `docs/python/API_CAPABILITY_MATRIX.md`, `docs/python/versioning.md`, and `.opencode/skills/eggsec-python/SKILL.md`. The Python feature guard **reports** these names to an operator as the packages to install. | A Python operator is told to install a package that is not required. | **Deliberately not changed here.** The Python wheel-profile data is validated by the Python subsystem's own contract (`make check-python`), which cannot run in this environment (no `.venv-ci/`), and the data files, the feature guard, the docs, and the skill are validated against each other. Changing one without the others would introduce a drift of its own. This is a **Python-subsystem corrective**, not a desktop-GUI one. |
| low | The two prerequisite claims also appear in **historical records**, which were left intact on purpose: `docs/python/GRADUATION_AUDIT.md`, `docs/python/GRADUATION_REVIEW.md`, `docs/python/RELEASES_1_4_FINAL_READINESS.md` (audit and release records), `architecture/network_dependency_baseline.md` (a dated 2026-09-12 measurement), `architecture/capability_segregation.md` (an ADR-0005 decision rationale), and the flat-era plans under `plans/` (immutable history per the planning process). | A reader of a historical record sees the claim as it stood. | Correct as history. The **current** operator-facing statements — `AGENTS.md`, `docs/BUILD.md`, `docs/extending/testing.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `architecture/networking.md`, `architecture/python_api.md`, `scripts/test_python_nse_runtime.sh`, `Makefile`, and the desktop-GUI skill — were corrected in this milestone. |
| low | `Dockerfile:12-13` and `.github/workflows/deep-checks.yml:30` still install `libpcap-dev` and `libssl-dev`. | None: installing extra packages is not a false requirement claim, only an unnecessary one. | Optional cleanup. Not a correctness defect. |

## 11. Roadmap disposition

**Milestone closed and next dependency may proceed.** Milestone 006 may proceed on the strength of this milestone: the daemon now has a profile whose feature set is documented, derived from the policy catalog, and machine-checked against the compiled binary. The remaining milestone-006 work — publishing the enabled-feature set additively over the wire, with a denial-driven fallback — is unchanged by this milestone and was not started here.

## 12. Registry updates

- `plans/registry.md`: the milestone 008 entry changes from **planned 2026-10-23** to **closed 2026-10-23**, and names this closure record.
- `plans/subsystems/desktop-gui-frontend-roadmap.md`: the §4 current-state section, the milestone 8 entry, and the §12 status table row are updated to the closed state.
- The known-gap list gains the **medium** inherited `check-deps` failure, so it is not lost behind this milestone's closure.
