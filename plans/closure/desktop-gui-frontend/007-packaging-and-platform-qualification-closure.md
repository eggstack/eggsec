# Desktop GUI Frontend Milestone 007 — Closure Status

Status: conditionally closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/007-packaging-and-platform-qualification.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md`

Applicable architecture decision:

- `plans/adrs/ADR-0010-gui-supported-platform-scope.md` — accepted 2026-10-23: the desktop GUI is supported on Linux and macOS; **Windows is out of scope**.

Repository baseline reviewed: `0c38c403`

Implementation commits or pull requests:

- Uncommitted working-tree change on branch `feat/auto-20261022-76692679`.

## 1. Executive finding

The milestone's **infrastructure** boundary is complete; its **operational** boundary is qualified only as far as this environment could honestly evidence.

What landed:

- **A machine-checked qualification matrix** (`docs/GUI_PLATFORM_QUALIFICATION.md`) with one row per platform and one cell per behaviour, every cell carrying a status token and either an evidence reference or a named root cause. No cell is blank and no cell carries an unqualified support claim.
- **A reproducible build recipe** (`scripts/build_gui_release.sh`, `make gui-release`) that produces the binary plus a build-metadata sidecar and performs **no** signing, bundling, installer, or publication.
- **A prerequisite probe** (`scripts/check_gui_prereqs.sh`, `make check-gui-prereqs`) that answers "buildable" or "not buildable" — never "skipped", because a qualification cell needs an answer.
- **A CI platform-matrix job** (`gui-platform` on `ubuntu-latest` and `macos-latest`) that is the evidence path for every cell the local environment could not close.
- **A handoff artifact** (`docs/GUI_DISTRIBUTION_HANDOFF.md`) with the seven required sections, including an explicit "what is NOT in the artefact" section and a numbered known-limitations list.
- **`bundle.active` stays `false`.** The milestone produces a binary and a recipe; the installers belong to the distribution workstream.

**Why the milestone is conditionally closed rather than closed.** The matrix is the deliverable, and it is complete and truthful. But the matrix reports that only **one** of its twenty-one behavioural cells is fully qualified (`macos` / Build), one is partially qualified (`macos` / Launch), and the remaining behavioural cells are `not qualified` because this environment has no Linux host, no window-automation driver, and — because milestone 006 has not closed — no report-export path and no daemon-lifecycle management in the GUI. The operational evidence that would promote those cells is named per cell and is owned by the `gui-platform` CI job and by milestone 006. Closing the milestone as *fully* qualified would state something the evidence does not support.

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| One matrix row per platform | `docs/GUI_PLATFORM_QUALIFICATION.md` §2 | pass | `linux`, `macos`, `windows` |
| One cell per required behaviour | The same, columns `Build \| Launch \| Connect \| Submit \| Cancel \| Render \| Export` | pass | Seven behavioural columns per row |
| Every cell states a status | The same | pass | Guard 168 rejects any cell that does not begin with an enumerated status token |
| No cell is blank and none carries an unqualified claim | The same | pass | Enforced by guard 168, negative-tested |
| Evidence references are reproducible | `docs/evidence/gui-qualification/macos-2026-10-23.md` | pass | Path + command + date + toolchain + tree state |
| The Windows row is `out of scope`, naming the decision | The `windows` row | pass | Every column names `ADR-0010`; guard 168 enforces it |
| The Windows root cause is measured, not asserted | The cross-compilation failure recorded in the plan and the matrix | pass | `cargo check -p eggsec-frontend-client --target x86_64-pc-windows-msvc` fails with E0432/E0433/E0599; the GUI's unconditional dependency imports `tokio::net::UnixStream` without a `cfg(unix)` gate |
| The supported-platform decision is recorded as an ADR | `plans/adrs/ADR-0010-gui-supported-platform-scope.md` | pass | Accepted 2026-10-23, with the re-entry path |
| The Linux system prerequisites are documented | `docs/GUI_BUILD.md` §2 | pass | Debian/Ubuntu, Fedora, Arch, macOS — as the Tauri v2 documentation states them |
| The platform floor is stated | The same | pass | `webkit2gtk-4.1` implies Ubuntu 22.04 / Debian 12 or newer (`ADR-0009`); the distributions below the floor are named |
| The workspace's own prerequisite subset is reconciled | The same | pass | The six packages Tauri lists that the workspace CI omits are named, with the reason the omission is safe on CI images but not on a bare distribution |
| The gate semantics are stated | The same | pass | A green `make check` on Linux does **not** prove the shell compiles; `make check-gui` is the ungated check |
| The shell's prerequisites are separated from the daemon's | The same | pass | The daemon's engine-feature prerequisites link to milestone 008's profile rather than restating it |
| The two stale prerequisite claims are not reintroduced | `docs/GUI_BUILD.md` | pass | `packet-inspection` needs no libpcap (pure-Rust `pnet`); `nse` vendors OpenSSL and needs a C toolchain |
| The prerequisite probe reports buildability | `scripts/check_gui_prereqs.sh` | pass | `VERDICT: BUILDABLE` on this host; never "skipped" |
| The build recipe is reproducible | `scripts/build_gui_release.sh` | pass | `cargo build --release -p eggsec-gui --features shell --locked`, with a metadata sidecar |
| The recipe performs no packaging | The script | pass | No signing, no bundling, no installer, no updater, no npm step |
| The build output is git-ignored | `.gitignore` | pass | `/dist/` |
| A CI platform-matrix job exists | `.github/workflows/deep-checks.yml` — `gui-platform` | pass | `ubuntu-latest` and `macos-latest`, `fail-fast: false` |
| The CI job exercises both GUI test profiles | The same | pass | `cargo test -p eggsec-gui --features shell --lib` and `--no-default-features --lib` |
| The CI job uploads the sidecar and checksum | The same | pass | So a cell can cite a CI run URL instead of a claim |
| The qualification procedure is a runbook | `docs/GUI_BUILD.md` §5 | pass | Precondition, command, pass criterion, evidence artefact per behaviour |
| The handoff artifact has the seven required sections | `docs/GUI_DISTRIBUTION_HANDOFF.md` | pass | In the required order |
| The handoff names what is NOT in the artefact | The same §3 | pass | No installer, signature, notarization, update channel, or bundled daemon |
| The known-limitations list is numbered and complete | The same §4 | pass | Includes the Windows exclusion, the socket-path duplication, the unwired `ApprovePolicy`, the Linux floor, and the `check-deps` advisories |
| The supply-chain posture is recorded without granting an exception | The same §5 | pass | The exception remains a maintainer decision (owner + review-by date); no `deny.toml` entry was added |
| `bundle.active` stays false | `crates/eggsec-gui/tauri.conf.json` | pass | Guard 168 enforces it |
| The GUI's Rust code is untouched by this milestone | The diff | pass | The only GUI changes in the branch are milestone 009's honesty corrections to the diagnostic text, which the agent was explicitly forbidden to make |
| No new dependency | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` unchanged | pass | — |

## 3. Production implementation evidence

**The qualification matrix** (`docs/GUI_PLATFORM_QUALIFICATION.md`). Seven sections: how to read a cell, the matrix, its provenance, the evidence mechanism, why Windows is out of scope, what promotes a cell, and the promotion table. The status vocabulary is fixed at five tokens — `qualified`, `partially qualified`, `not qualified`, `pending M6`, `out of scope` — and the document states the two absolute rules: no cell is blank, and no cell says `qualified` without a reproducible evidence reference.

The matrix as it stands:

| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |
|---|---|---|---|---|---|---|---|
| `linux` | not qualified | not qualified | not qualified | not qualified | not qualified | not qualified | pending M6 |
| `macos` | **qualified** | partially qualified | not qualified | not qualified | not qualified | not qualified | pending M6 |
| `windows` | out of scope | out of scope | out of scope | out of scope | out of scope | out of scope | out of scope |

Every `not qualified` cell names a **root cause**, not an excuse: the Linux cells say no Linux host was executed here and that the `gui-platform` job on `ubuntu-latest` is the evidence path; the macOS behavioural cells say the GUI connects only on an operator action and that no window-automation driver was available; the Submit, Cancel, and Render cells name their dependency on Connect and Submit.

**The evidence mechanism** (`docs/evidence/gui-qualification/`). A `TEMPLATE.md` fixing the fields a run must record — platform, host OS version and architecture, commit SHA, tree state, toolchain, feature set, the exact commands, the observed output, and the operator or CI job — plus the run file for this session's macOS run. The run file records the dirty tree state explicitly rather than hiding it, and points at the clean-tree build as the CI job's output.

**The build recipe** (`scripts/build_gui_release.sh`). Refuses a dirty tree unless `--allow-dirty` is passed and records the tree state either way; pins `cargo build --release -p eggsec-gui --features shell --locked`; writes `eggsec-gui.build.json` (commit SHA, dirty flag, `rustc -Vv`, cargo version, target triple, enabled cargo features, the resolved `Cargo.lock` hash, the binary's SHA-256, and its size) and `SHA256SUMS`; and performs no signing, no bundling, no installer, no publication, and no npm step.

**The prerequisite probe** (`scripts/check_gui_prereqs.sh`). Reports `BUILDABLE` or `NOT BUILDABLE` — never "skipped". It needs no toolchain install to give its answer.

**The CI job** (`gui-platform` in `.github/workflows/deep-checks.yml`). `fail-fast: false` over `ubuntu-latest` and `macos-latest`. The Linux leg installs the documented webview packages, then runs the probe, the **ungated** `make check-gui`, both GUI test profiles, and `make gui-release`, and uploads the sidecar and checksum. The Windows leg is excluded **by decision**, and the job's own comment records the measured reason (the GUI's unconditional `eggsec-frontend-client` dependency does not compile for Windows) and the re-entry path.

**The runbook** (`docs/GUI_BUILD.md` §5). One numbered step per behaviour column, each with a precondition, an exact command or action, an observable pass criterion, and the evidence artefact it produces. The GUI is daemon-only, so the procedure starts the daemon separately and points the GUI at it with `--socket`; the cancel step records the **terminal state observed** rather than the click, because cancellation is best-effort and a task that completes in between is not an error; the render step requires rendering a real result, not an empty dashboard.

**The handoff artifact** (`docs/GUI_DISTRIBUTION_HANDOFF.md`). Seven sections in the required order: artifact identity, runtime prerequisites, what is NOT in the artefact, the numbered known limitations, the supply-chain posture, the decisions the distribution workstream owns, and the decisions this milestone made. The supported-platform decision is explicitly **not** in the distribution workstream's column — `ADR-0010` already made it.

**Guard 168** pins the matrix's schema: the document must exist, must carry a row for Linux and macOS and an explicitly out-of-scope row for Windows naming `ADR-0010`, every status cell must be one of the five enumerated tokens, and `tauri.conf.json` must keep `bundle.active` false.

## 4. Verification executed

### Commands run

```bash
bash -n scripts/build_gui_release.sh
bash -n scripts/check_gui_prereqs.sh
bash scripts/check_gui_prereqs.sh
bash scripts/build_gui_release.sh --out /tmp/eggsec-gui-release-test --allow-dirty
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-gui --no-default-features --lib
python3 scripts/check_doc_references.py
cargo fmt --all --check
bash scripts/check-architecture-guards.sh
```

### Results

| Command | Result | Detail |
|---|---|---|
| `bash -n scripts/build_gui_release.sh` | pass | shell syntax clean |
| `bash -n scripts/check_gui_prereqs.sh` | pass | shell syntax clean |
| `bash scripts/check_gui_prereqs.sh` | pass | `VERDICT: BUILDABLE` on this host (exit 0) |
| `bash scripts/build_gui_release.sh --out /tmp/eggsec-gui-release-test --allow-dirty` | pass | release build finished in 3m 09s; `eggsec-gui`, `eggsec-gui.build.json`, and `SHA256SUMS` produced |
| `cargo test -p eggsec-gui --features shell --lib` | pass | **34 passed, 0 failed** |
| `cargo test -p eggsec-gui --no-default-features --lib` | pass | **30 passed, 0 failed** |
| `python3 scripts/check_doc_references.py` | pass | no new missing references beyond the baseline of 29 |
| `cargo fmt --all --check` | pass | clean |
| `bash scripts/check-architecture-guards.sh` | pass | **ALL PASSED** (guard 168 added) |

**Guard 168 was negative-tested** against three deliberate violations, each of which was caught with a named reason: downgrading the `windows` row's Build cell from `out of scope` to `not qualified`; removing the `linux` row entirely; and switching `bundle.active` to `true`. The tree was restored after each case and confirmed green.

**Not run:** `make check-python` (requires `.venv-ci/`). `make check` cannot complete because it stops at `make check-deps`; every step before it passes, and the steps after it were run directly and are green.

## 5. Invariant review

| Invariant | Evidence |
|---|---|
| The GUI links no engine, daemon, CLI, or TUI crate | Untouched by this milestone; guard 155 remains green. |
| The shell framework stays inside the GUI crate | Untouched; guard 156 remains green. |
| The GUI declares no second TLS stack | Untouched; guard 157 remains green. |
| The GUI's CSP forbids remote origins | Untouched; guard 158 remains green. |
| The GUI carries no per-operation form code | Untouched; guard 159 remains green. |
| The GUI retains no unbounded event log | Untouched; guard 160 remains green. |
| No signing, notarization, installer, updater, or npm step | The build recipe performs none; `bundle.active` stays false and guard 168 pins it. |
| Windows is out of scope for the desktop GUI | `ADR-0010`; the matrix's `windows` row is `out of scope` in every column and names it; the CI job excludes the leg by decision. |
| No new dependency | `deny.toml` and `docs/DEPENDENCY_EXCEPTIONS.md` gain no entry. |
| The GUI's Rust code is not modified by this milestone | The milestone's diff touches no file under `crates/eggsec-gui/`. The branch's only GUI changes are milestone 009's honesty corrections to the diagnostic text, which this milestone's agent was explicitly forbidden to make. |
| The daemon-only boundary is preserved | The runbook starts the daemon separately and points the GUI at it; the GUI still links no daemon crate. |
| The supply-chain exception remains a maintainer decision | The handoff records the advisories and names the decision; no exception was written. |

## 6. Failure and recovery review

- **Duplicate delivery.** Not applicable; the milestone produces a build recipe, not a service.
- **Cancellation races.** Recorded in the runbook: cancellation is best-effort, a task that completes between the click and the daemon's processing is not an error, and the **event stream** is authoritative for the terminal state. The step records the observed terminal state, not the click.
- **Daemon restart.** Not in this milestone's scope; the runbook points at an already-running daemon.
- **Partial persistence failure.** Not applicable.
- **Stale generation or lease.** Not applicable.
- **Contention and resource release.** The build recipe refuses a dirty tree so two operators cannot silently produce two different "the same" artefacts; the sidecar records the tree state either way, so a dirty build is identifiable after the fact.
- **Malformed or unauthorized input.** The prerequisite probe accepts only known argument shapes and fails closed on an unknown one. The matrix's cell grammar is machine-checked, so a malformed or blank cell is a build failure, not a silent degradation.
- **Bounded event and artifact behavior.** The build recipe writes one binary, one sidecar, and one checksum file. The probe installs nothing.

## 7. Migration and compatibility review

- **Schema migration.** None. The sidecar carries a `schema` field (`eggsec-gui-build-metadata/v1`).
- **Backward compatibility.** Nothing that existed before was removed or changed. `docs/PLATFORM.md` and `docs/FEATURE_MATRIX.md` were corrected in place by milestone 008, so their content changed, but no interface did.
- **Protocol negotiation.** Not applicable; the milestone produces no protocol.
- **Configuration behaviour.** The build recipe adds four build-time flags (`--out`, `--target`, `--allow-dirty`, `--help`). The GUI's runtime flags are unchanged.
- **Rollback limitations.** The milestone's output is documentation, scripts, and one CI job; reverting it removes them without affecting the runtime.
- **Legacy-path status.** The existing `make check-gui` path is unchanged and remains the ungated escape hatch. The new `gui-platform` job is **additional**, not a replacement.

## 8. Security review

- **Authorization enforcement.** Not applicable; the milestone produces no execution path.
- **Scope checks.** Not applicable.
- **Secret handling.** The sidecar records the commit SHA, the toolchain, the feature set, and the binary's hash — no credential. The build recipe reads and writes no secret.
- **Path validation.** The build recipe writes only under the caller-supplied `--out` directory, which defaults to the git-ignored `dist/`.
- **Privilege boundaries.** The prerequisite probe **reports** whether the host can build the shell; it never escalates and never installs. The GUI's own runtime privilege requirements are milestone 008's profile and the platform matrix's business, not this milestone's.
- **Transport checkpoints.** The GUI's transport remains the daemon's Unix domain socket; the milestone adds no network surface and no remote origin.
- **Denial-of-service bounds.** The CI job's two legs are bounded by the normal CI timeout; the release build is bounded by the toolchain.
- **Redaction.** Not applicable.
- **Audit behaviour.** Unchanged.

One supply-chain note is worth recording: the milestone's CI job installs the Linux webview packages **from the distribution's repositories**, exactly as the workspace's existing CI job does. That is a supply-chain surface the workspace already had; the milestone adds one more instance of it, not a new kind.

## 9. Documentation and operations

- **New:** `docs/GUI_PLATFORM_QUALIFICATION.md`, `docs/GUI_DISTRIBUTION_HANDOFF.md`, `docs/GUI_BUILD.md`, `docs/evidence/gui-qualification/TEMPLATE.md`, `docs/evidence/gui-qualification/macos-2026-10-23.md`, `scripts/build_gui_release.sh`, `scripts/check_gui_prereqs.sh`.
- **Updated:** `.github/workflows/deep-checks.yml` (the `gui-platform` job), `.gitignore` (`/dist/`), `docs/CI_ARCHITECTURE_GUARDS.md` (guard 168), `docs/VERIFICATION.md` (the `gui-release` and `check-gui-prereqs` targets, the prerequisite-skip semantics, and the daemon-profile note), `Makefile` (`gui-release`, `check-gui-prereqs`).
- **Operator diagnostics.** `make check-gui-prereqs` answers whether the shell can be built here; `make gui-release` produces the binary and the record; the matrix says what was actually qualified.
- **Static guards.** Guard 168, negative-tested against three violations.
- **Recovery instructions.** A failed release build reports cargo's own error; the recipe's metadata sidecar identifies the tree state, so a suspect artefact can be traced to its commit.

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | **The Linux row is entirely unqualified.** No Linux host was executed in this environment. | The desktop GUI's Linux support is documented and reproducible but not yet demonstrated. | The `gui-platform` CI job on `ubuntu-latest` is the named evidence path. When it runs green, the row's `Build` cell can be promoted; the behavioural cells need the operator steps in `docs/GUI_BUILD.md` §5, recorded per the evidence template. |
| medium | **The macOS behavioural cells (Connect, Submit, Cancel, Render) are unqualified.** The GUI connects only on an operator action, and no window-automation driver was available in this environment. | The GUI's end-to-end behaviour is unproven against a live daemon. | The same runbook steps, on a host with a live daemon and an operator or driver able to click. This is also gated on milestone 006, which owns the daemon lifecycle management the procedure otherwise has to do by hand. |
| medium | **The `ApprovePolicy` path is unwired.** The daemon answers it with `ErrorCode::Unsupported`, so the GUI can mark a task `awaiting_policy` but can offer no approval affordance. | A GUI operator cannot approve a policy-gated task. | A separate milestone in the daemon/GUI boundary. Recorded as a known limitation in the handoff artifact; it is **not** silently papered over. |
| medium | `make check-deps` fails on pre-existing advisories, including the entries reachable only through the GUI's Tauri closure (`glib 0.18.5` RUSTSEC-2024-0429; `glib-macros 0.18.5` → `proc-macro-error 1.0.4` RUSTSEC-2024-0370; five `unic-*` crates RUSTSEC-2025-0081/-0075/-0080/-0100/-0098). | The dependency-policy gate is red at the baseline. | **Inherited, not attributable to this milestone.** Recording the exceptions is a maintainer decision (owner + review-by date). No `deny.toml` entry was added here. |
| low | **The GUI's resident footprint was not measured to a published figure.** | The handoff names the measurement as outstanding rather than quoting an unverified number. | Measure it on a qualified host and record it in the handoff's artifact-identity section. |
| low | The socket-path default is duplicated in six production sites. | A future change to the default must touch all six. | Not this milestone's scope; recorded in the subsystem roadmap. |
| low | The GUI's Linux platform floor excludes distributions below Ubuntu 22.04 / Debian 12. | Operators on older distributions are unsupported. | Named in the handoff's known limitations, with the `ADR-0009` egui fallback as the documented alternative. |

## 11. Roadmap disposition

**Milestone conditionally closed with named operational evidence outstanding.** The production implementation is complete: the matrix, the recipe, the probe, the CI job, the runbook, and the handoff artifact all exist and are machine-checked. What remains outstanding is **operational evidence**, not implementation:

1. the `gui-platform` CI job must run green on `ubuntu-latest` to qualify the Linux row's `Build` cell;
2. the operator steps in `docs/GUI_BUILD.md` §5 must be executed and recorded to qualify the behavioural cells;
3. milestone 006 must close before the `Export` column can leave `pending M6`.

The risk of the conditional state is bounded and named: the matrix cannot overstate support, because guard 168 rejects an unqualified claim. The exact future evidence is recorded per cell in the matrix's promotion table.

## 12. Registry updates

- `plans/registry.md`: the milestone 007 entry changes from **planned 2026-10-23** to **conditionally closed 2026-10-23**, and names this closure record.
- `plans/subsystems/desktop-gui-frontend-roadmap.md`: the §4 current-state section, the milestone 7 entry, and the §12 status table row are updated to the conditionally-closed state, with the outstanding evidence named.
- The known-gap list gains the three **medium** findings above, so the unqualified rows and the unwired `ApprovePolicy` are not lost behind the closure.
