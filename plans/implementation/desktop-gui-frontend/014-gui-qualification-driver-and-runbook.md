# Desktop GUI Frontend Milestone 014 — The GUI's Qualification Driver and Runbook

Status: ready for handoff

Repository baseline: branch `feat/auto-20261022-76692679` at commit `821c8189`
("docs(plans): correct the roadmap's remaining stale approval claims"), with
milestones 001–011 closed, conditionally closed, or implemented, and
`ADR-0007`, `ADR-0009`, `ADR-0010` accepted.

Source roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-14--the-guis-qualification-driver-and-runbook`

Applicable ADRs:

- `plans/adrs/ADR-0007-desktop-gui-frontend-boundary.md` (status: accepted)
- `plans/adrs/ADR-0009-gui-shell-technology-selection.md` (status: accepted)
- `plans/adrs/ADR-0010-gui-supported-platform-scope.md` (status: accepted)

Related long-term requirements:

- `plans/000-long-term-specification.md#4-enforcement-invariants`
- `plans/000-long-term-specification.md#10-frontend-rules`
- `plans/001-terminology-and-domain-model.md#6-frontend-and-session-terms`

Primary class: capability

## 1. Objective

One bounded outcome: **make the behavioural and export cells of the desktop
GUI's platform qualification matrix promotable, and write the export
procedure the matrix leaves undefined.**

Concretely, it delivers five things and nothing else:

1. **The export procedure**, written into `docs/GUI_BUILD.md` §5.7, replacing
   the three `*Not yet defined.*` placeholders that sit there today
   (`docs/GUI_BUILD.md:466`, `:467`, `:468`).
2. **A maintained qualification procedure** — a version-controlled,
   machine-checkable script (`scripts/gui_qualification.sh`) plus a Rust
   assertion harness (`crates/eggsec-gui/tests/qualification.rs`) that
   produces the record a matrix cell cites.
3. **A promotion rule** — a written, checkable rule set stating what a cell
   must carry before it may read `qualified`, and a validator
   (`scripts/check_gui_qualification.py`) that enforces it.
4. **The static guards** that keep the matrix, the runbook, and the harness
   from drifting apart (guards 181–189).
5. **A regenerated matrix** whose stated blockers are true — see §1.1.

**This milestone does not promote any cell by itself.** It makes promotion
*possible* and records the procedure that makes it reproducible. A cell's
status changes only when a recorded run exists and that run satisfies the
promotion rule. The roadmap's objective wording is deliberate — "make the
behavioural cells … **promotable**"
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1022-1023`), not "promote
the behavioural cells".

### 1.1 Why the scope is what it is: the matrix's own blockers are stale

This is a **finding**, not a preference, and it is the reason the milestone's
scope is larger than "write a driver".

The matrix's header says "**Last regenerated:** 2026-10-23 — commit
`7332baeafe5391e228a0b91fec48cf68999729f1`"
(`docs/GUI_PLATFORM_QUALIFICATION.md:54-55`), and the matrix is *regenerated*,
not hand-maintained (`:99-120`). Three of its stated blockers are false
against the current baseline, so **the exit condition "the CI job's artefacts
are recorded against the matrix" cannot be met against a matrix whose blockers
are stale**:

| The matrix says | Where | Measured truth |
|---|---|---|
| "The GUI frontend has no export affordance today — there is no `export` reference anywhere in `crates/eggsec-gui/frontend/app.js`" | `docs/GUI_PLATFORM_QUALIFICATION.md:239-243`, and the same sentence inside **both** `pending M6` cells (`:59`, `:60`) | False. `grep -c "export" crates/eggsec-gui/frontend/app.js` returns **50**; `"export_report"` is at `:1624`, `"report_for"` at `:556`, `state.daemon.report_formats` at `:354`. The Rust side has `export_report` at `crates/eggsec-gui/src/commands.rs:670` and `report_for` at `:718`. Milestone 006 is implemented. |
| "the daemon's engine-feature profile and obtainability recipe (milestones 008 and 009) had not closed" | `docs/GUI_PLATFORM_QUALIFICATION.md:60` (macos Submit cell) | False. Milestones 008 and 009 are both closed (2026-10-23). |

The **full** drift inventory — these two plus every stale `path:line` anchor
in the three documents — is §3.5, and §7's first work package applies all of
it.

**Therefore regenerating the matrix is in scope for M014**, and §7's
matrix-update work package is not optional polish: until the matrix's
blockers are true, the CI artefacts cannot be recorded against it.

The plan's promotion design hangs off the matrix's **existing** evidence
mechanism rather than inventing a second one: §4.1 fixes the layout
(`docs/evidence/gui-qualification/TEMPLATE.md` plus one file per run named
`<platform>-<YYYY-MM-DD>.md`, `:126-136`), §4.2 fixes the fields a run must
carry — including *tree state* and *deviations* (`:138-156`), and §4.3 fixes
the standard that the runbook "gives each step an explicit pass criterion and
names the artefact the step produces" (`:158-164`). M014's export procedure
therefore belongs in `docs/GUI_BUILD.md` §5, in the same shape as §5.1–§5.6,
and it must produce a named artefact.

### 1.2 Corrections to the brief

Three statements in the task briefing are corrected here, with the repository
evidence:

| Briefing said | Repository evidence | Correction |
|---|---|---|
| The plan file for milestone 010 is `010-full-operation-coverage-and-report-export.md`. | The directory holds `006-full-operation-coverage-and-report-export.md` and `010-daemon-full-manual-capability-policy.md`. | The house-voice references for this plan are `010-daemon-full-manual-capability-policy.md` and `011-daemon-mediated-policy-approval.md`. |
| The export cells are blocked "on a procedure that was never written", and the matrix says the GUI has no export affordance. | The export affordance **exists**: `crates/eggsec-gui/frontend/index.html:77-111` (the export panel), `crates/eggsec-gui/frontend/app.js:1612-1624` (the submit handler invoking `export_report`), `crates/eggsec-gui/src/commands.rs:670` (the command), `crates/eggsec-gui/src/export.rs:59-76` (the six formats). | The export cells are blocked on **two** things: the unwritten procedure *and* a matrix that still asserts the feature is absent. Both are M014's to fix. |
| "Eight behavioural cells across two platforms are blocked on the absence of a driver." | The roadmap says this (`plans/subsystems/desktop-gui-frontend-roadmap.md:1032-1034`), but the matrix's own cell text gives **two different** root causes: the macOS `Connect` cell says "no windowed operator session or window-automation driver was available in this environment; the repository ships no such driver" (`docs/GUI_PLATFORM_QUALIFICATION.md:60`), while every Linux cell says "no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path" (`docs/GUI_PLATFORM_QUALIFICATION.md:59`). | The brief's phrasing is the roadmap's shorthand. The plan separates the two root causes, because they need different remedies: a driver-shaped gap on macOS and a **no-host / no-display** gap on Linux. See §3.3. |

## 2. Why this milestone is ready

### 2.1 Hard dependency: M7 — conditionally closed, with its gate now met

`plans/closure/desktop-gui-frontend/007-packaging-and-platform-qualification-closure.md`
closes milestone 007 **conditionally** and names three outstanding conditions
(`:207-211`): the `gui-platform` CI job must run green on `ubuntu-latest`; the
operator steps in `docs/GUI_BUILD.md` §5 must be executed and recorded; and
milestone 006 must close before the `Export` column can leave `pending M6`.

Condition 3 is now **met** — "Milestone 7's dependency on milestone 6 is now
**met**: the export path …"
(`plans/closure/desktop-gui-frontend/006-full-operation-coverage-and-report-export-closure.md:445`).
Conditions 1 and 2 are the *substance* of this milestone: M014 builds the
mechanism by which both are discharged, and states honestly that the
**recording** of a green run is an operational event that may follow the merge.

M7's production surface, all present at the baseline:

| Artefact | Location |
|---|---|
| The qualification matrix | `docs/GUI_PLATFORM_QUALIFICATION.md` (243 lines) |
| The build recipe | `scripts/build_gui_release.sh` (310 lines), invoked by `make gui-release` (`Makefile:213-214`) |
| The prerequisite probe | `scripts/check_gui_prereqs.sh` (157 lines), invoked by `make check-gui-prereqs` (`Makefile:206-207`) |
| The CI job | `.github/workflows/deep-checks.yml:91-143` (`gui-platform`, matrix `ubuntu-latest` + `macos-latest` at `:97`) |
| The runbook | `docs/GUI_BUILD.md` (495 lines) |
| The distribution handoff | `docs/GUI_DISTRIBUTION_HANDOFF.md` (358 lines) |
| The evidence template | `docs/evidence/gui-qualification/TEMPLATE.md` |

### 2.2 Soft dependency: M6 — implemented

The export path exists and is unit-tested. The six formats are
`crates/eggsec-gui/src/export.rs:59-76`; `ReportFormat::parse` is
`crates/eggsec-gui/src/export.rs:107-116`; `export::render` is
`crates/eggsec-gui/src/export.rs:312`; `export::write_export` is
`crates/eggsec-gui/src/export.rs:353`; `backup_existing_export` is
`crates/eggsec-gui/src/export.rs:370`. The IPC command is
`crates/eggsec-gui/src/commands.rs:670-705`.

### 2.3 Other closed dependencies that unblock specific cells

- **M8** (daemon feature profile) and **M9** (daemon obtainability) are
  closed. The macOS `Submit` cell's root cause — "the daemon's engine-feature
  profile and obtainability recipe (milestones 008 and 009) had not closed"
  (`docs/GUI_PLATFORM_QUALIFICATION.md:60`) — is **stale** and must be rewritten
  even if the cell's status does not change.
- **M11** (`ADR-0012`) wired `ApprovePolicy`; the GUI's IPC surface is
  **thirteen** commands (`crates/eggsec-gui/src/shell.rs:33-47`), and the
  roadmap records the count as informational (gap 12,
  `plans/subsystems/desktop-gui-frontend-roadmap.md:348`).

### 2.4 Stable interface dependencies this milestone builds on

The daemon protocol (`crates/eggsec-daemon-protocol/src/protocol.rs`) — unchanged
by this milestone; the frontend-neutral client contract, used by the harness
exactly as the GUI uses it (`crates/eggsec-gui/src/connection.rs:30-35` defines
`gui_identity()`); the GUI's report renderers, reached through `eggsec-output`;
the build recipe; the CI job.

## 3. Current implementation evidence

### 3.1 The matrix, measured

`docs/GUI_PLATFORM_QUALIFICATION.md:57-61` is one table: a header row and
**three platform rows** (`linux`, `macos`, `windows`) against **seven
columns** (`Build`, `Launch`, `Connect`, `Submit`, `Cancel`, `Render`,
`Export`) — **21 cells**.

**Measurement rule.** The status tokens are the bolded words at the start of
each cell; each is counted with
`grep -o "\*\*$t\*\*" docs/GUI_PLATFORM_QUALIFICATION.md | wc -l` for each of
the five tokens.

**Result at the baseline:**

| Status token | Count | Where |
|---|---|---|
| `qualified` | 1 | macos / Build |
| `partially qualified` | 1 | macos / Launch |
| `not qualified` | 10 | linux / Build, Launch, Connect, Submit, Cancel, Render (6); macos / Connect, Submit, Cancel, Render (4) |
| `pending M6` | 2 | linux / Export, macos / Export |
| `out of scope` | 7 | windows / all seven columns |
| **Total** | **21** | |

The vocabulary is fixed by `docs/GUI_PLATFORM_QUALIFICATION.md:29-35`. The two
absolute rules are stated at `:37-48`:

- **No cell is blank**, and no cell carries an unqualified support claim
  (enforced by check 168, `scripts/check-architecture-guards.sh:5457`).
- **No cell says `qualified` without an evidence reference** — "a path plus
  the command that reproduces it plus the date of the run", or a CI run URL.

**Neither rule is fully enforced today.** Check 168 (read at
`scripts/check-architecture-guards.sh:5457-5600`) enforces the *grammar* of a
cell (a status token, a dash, an explanation), the *vocabulary* of the status
tokens, and the Windows row's `out of scope` status. It does **not** check
that a `qualified` cell's evidence reference resolves, and it does **not**
check the cell *count*. M014 closes both gaps (guards 183 and 184).

### 3.2 The eight behavioural cells and the two export cells

The behavioural columns are `Connect`, `Submit`, `Cancel`, `Render` — four
columns × two in-scope platforms = **eight behavioural cells**. The `Export`
column contributes **two** more. The table below is also the answer to *how
each cell becomes promotable*: it names the current root cause, what M014
adds, the evidence reference the cell ends up carrying, and the rule that
gates the promotion.

| Cell | Current status and root cause (as the matrix states it) | What M014 adds | The evidence reference the cell carries | Rule |
|---|---|---|---|---|
| **linux** Connect / Submit / Cancel / Render (4 cells) | `not qualified` — "no Linux host was executed in this environment; the `gui-platform` CI job on `ubuntu-latest` is the evidence path" (`:59`) | The CI job gains the harness step and publishes the run record; the runbook gains the operator procedure | A CI run URL (`docs/evidence/gui-qualification/CI-RUNS.md`) **plus**, for the window-observable half, a `docs/evidence/gui-qualification/linux-<date>.md` record with the operator's screenshot | P1 (and the Linux row's own rule at `:234`: a green CI run **plus** an operator run) |
| **macos** Connect (1 cell) | `not qualified` — "no windowed operator session or window-automation driver was available in this environment; the repository ships no such driver" (`:60`) | The procedure and the harness; the window-existence probe (scripted) beside the operator's observation (attested) | `docs/evidence/gui-qualification/macos-<date>.md`, whose `connect` entry carries the screenshot and the probe's count | P1 |
| **macos** Submit | `not qualified` — "submitting requires a connected session … and the daemon's engine-feature profile and obtainability recipe (milestones 008 and 009) had not closed" (`:60`) — **stale** | The root cause is rewritten; the harness submits against a `service`-profile daemon | The same record, whose `submit` entry carries the task id and the daemon's outcome (accepted **or** refused — both are legitimate observations) | P1 |
| **macos** Cancel | `not qualified` — "a cancellation can only be observed against a live task … the event stream's authority (`crates/eggsec-gui/src/commands.rs:432`) is implemented and unit-tested but was not exercised against a live daemon" (`:60`) — **the cited line is wrong** | The harness exercises the event stream against a live daemon; the citation is re-anchored to `commands.rs:572` | The same record, whose `cancel` entry carries the observed terminal state | P1 |
| **macos** Render | `not qualified` — "rendering a real result requires a completed task carrying a result payload, which requires the Connect and Submit cells" (`:60`) | The harness drives a task to `completed` and asserts the payload is non-empty | The same record, whose `render` entry carries the rendered-result screenshot | P1 |
| **linux** Export | `pending M6` — "milestone 006 owns the export path; the GUI frontend has no export affordance today" (`:59`) — **stale** | The export procedure (§6) and the harness's export assertions; the token is removed by guard 182 | The same record, whose `export` entry carries the exported file's path and SHA-256 | P4 |
| **macos** Export | `pending M6` — same text (`:60`) — **stale** | Same | Same | P4 |

The `windows` row is not in this table: it is `out of scope` in all seven
columns and has no promotion path (rule P5). The accounting closes: 8
behavioural + 2 export + 7 Windows + 3 Build/Launch = 21.

**Two facts the export procedure must respect.** The report is fetched from
the daemon's **in-memory** store, bounded at
`MAX_RETAINED_REPORTS: usize = 64` (`crates/eggsec-runtime/src/report.rs:29`),
oldest-first eviction; a daemon restart empties it, so the procedure must say
*when* to export relative to the task, not merely *that* export is possible.
And **not every task carries a report**: the report bridge is
`crates/eggsec/src/runtime_bridge/report.rs:54` (`report_for_result`), whose
remaining coverage is milestone 013's; a task kind it does not cover yields
`None`, and `export_report` refuses with "the daemon has no report for task
…, so there is nothing to export"
(`crates/eggsec-gui/src/commands.rs:689-691`).

**The two groups are blocked for different reasons, and the plan does not
conflate them.** The eight behavioural cells need *an executed and recorded
run*; the two export cells need *a procedure*; the macOS `Launch` cell needs
*one narrower observation* (a painted window), not a driver
(`docs/GUI_PLATFORM_QUALIFICATION.md:60`). The linux leg's promotion path
runs through a green CI run, because that is what the matrix names
(`:59`); the macos leg's runs through a driver **or** an operator procedure
(`:60`). One procedure serves both legs; only the linux leg additionally
needs the CI half.

### 3.3 What is actually available to drive a window

This is the question the roadmap delegates to M014
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1028-1030`). The measured
situation:

**The GUI is a buildless, zero-npm, CSP-constrained Tauri v2 shell.**

`crates/eggsec-gui/frontend/` holds exactly three plain files — `app.js`
(62 086 bytes), `index.html` (5 299 bytes), `styles.css` (20 088 bytes) — and
no bundler. The repository-root `package.json` is `{}`: no `dependencies` key,
no scripts. `docs/ARCHITECTURE.md:59` calls it a "buildless frontend with zero
npm dependencies"; `docs/GUI_BUILD.md:293` and
`docs/GUI_DISTRIBUTION_HANDOFF.md:345` both say "no npm step". The window is
declared in config, not in code (`crates/eggsec-gui/tauri.conf.json:11-21`:
label `main`, title `Eggsec`, 1200×800); the CSP forbids remote origins and
inline script (`:22-24`); the bundler is off (`:26-28`,
`bundle.active: false`).

**The repository's own documented driver path is the Tauri webdriver
tooling, and the documentation describes it as a Linux-shaped dependency.**
`docs/GUI_BUILD.md:75-78` says "`libxdo-dev` (Debian/Ubuntu), `libxdo-devel`
(Fedora) and `xdotool` (Arch) are the same package under three names: they
provide the XTest library the Tauri webdriver tooling uses. They are needed
to *drive* the shell, not to compile it"; `:139` classifies `libxdo-dev` as
"only needed to *drive* the shell" and "needed for webdriver-based UI
testing"; and the macOS section (`:80-89`) documents **no** driver tooling at
all, because the webview ships as WKWebView.

**What is not established by repository evidence, and is therefore an open
question (OQ-1, §14.1):** whether a Tauri-v2 WebDriver driver exists for the
**macOS** leg. Nothing in this repository answers it, and the driver option
has to work on *both* in-scope platforms to promote a two-platform matrix.

**What is established:** the webdriver route needs a WebDriver *client*
process alongside the driver, and the repository contains none; the standard
Tauri pairing is a Node client, which contradicts the zero-npm posture
recorded in `docs/ARCHITECTURE.md:59` and the empty root `package.json`.

**And CI has no display on either leg.** The `gui-platform` job
(`.github/workflows/deep-checks.yml:91-143`) is, step for step: checkout
(`:101`), toolchain (`:102`), rust-cache (`:103-105`), the Linux webview
prerequisites via apt (`:112-114`), `bash scripts/check_gui_prereqs.sh`
(`:117-118`), `make check-gui` (`:122-123`), the two GUI test profiles
(`:128-131`), `make gui-release` (`:132-133`), and the artefact upload of
`dist/gui/eggsec-gui.build.json` + `dist/gui/SHA256SUMS` as
`gui-release-${{ matrix.os }}` with `if-no-files-found: error` (`:136-143`).

**There is no daemon, no window, and no driver in that job.** It compiles,
unit-tests, builds, and uploads a checksum, so on its own it can never promote
a behavioural cell: it satisfies only the *"A green `gui-platform` CI run on
`ubuntu-latest` whose artefacts are recorded"* half of the Linux row's
promotion rule (`docs/GUI_PLATFORM_QUALIFICATION.md:234`); the other half —
*"plus an operator run of the behavioural steps if the cell is a behavioural
one"* — is exactly what M014 delivers. Any proposal promising CI-driven
promotion must therefore either install and drive a virtual display (Xvfb)
plus a window-automation tool, or route the cells through the **operator
procedure** while CI covers Build and the sidecar. M014 chooses the second.

### 3.4 The macOS `Launch` cell's missing part

`docs/GUI_PLATFORM_QUALIFICATION.md:60` records the macOS `Launch` cell as
`partially qualified` because "the painted window was never observed (a
full-screen capture showed no `eggsec-gui` window and the accessibility query
returned zero windows)". The run recorded the two probes it used
(`docs/evidence/gui-qualification/macos-2026-10-23.md:45-46`) — `screencapture
-x -o <file>` and `osascript -e 'tell application "System Events" to tell
process "eggsec-gui" to get count of windows'`. Both are **reusable as
assertions**, and that is the seed of §6's design: on macOS a window's
existence is scriptable, and on Linux it is scriptable too (`xdotool
search --name`, a tool the repository already names at
`docs/GUI_BUILD.md:75-78`).

### 3.5 The stale claims this milestone must correct

The matrix and the runbook both assert facts that milestone 006 made false.
`scripts/check_doc_references.py` checks **path existence only**
(`scripts/check_doc_references.py:6` — the docstring says "checks that each
referenced path exists relative to the workspace root"); it never checks a
`path:line` anchor. So nothing in the toolchain notices this drift. The
following are measured, not asserted:

| Stale claim | Location | Actual |
|---|---|---|
| "The GUI frontend has no export affordance today — there is no `export` reference anywhere in `crates/eggsec-gui/frontend/app.js`" | `docs/GUI_BUILD.md:466` | False. `crates/eggsec-gui/frontend/app.js:1624` invokes `export_report`. |
| "The GUI frontend has no export affordance today — there is no `export` reference anywhere in `crates/eggsec-gui/frontend/app.js`" | `docs/GUI_PLATFORM_QUALIFICATION.md:239-243` (and inside both `pending M6` cells, `:59`, `:60`) | False. `grep -c "export" crates/eggsec-gui/frontend/app.js` returns **50**; `"export_report"` is at `:1624`, `"report_for"` at `:556`, `state.daemon.report_formats` at `:354`. |
| "the daemon's engine-feature profile and obtainability recipe (milestones 008 and 009) had not closed" | `docs/GUI_PLATFORM_QUALIFICATION.md:60` | False. Milestones 008 and 009 are both closed (2026-10-23). |
| `crates/eggsec-daemon/src/server.rs:95` is `UnixListener::bind(socket_path)?` | `docs/GUI_PLATFORM_QUALIFICATION.md:210-212`, `docs/GUI_DISTRIBUTION_HANDOFF.md:142` | Line 95 is the `refused_command_response` function's signature (its doc comment is at `:93-94`). The bind is at **`crates/eggsec-daemon/src/server.rs:147`**; the `use tokio::net::UnixListener;` import at `:5` is still accurate. |
| "The daemon's lifecycle is milestone 006's. The GUI does not start, supervise, or stop the daemon. Until M6 closes, the operator starts the daemon by hand" | `docs/GUI_BUILD.md:375-377` | False. `--daemon-bin` exists (`crates/eggsec-gui/src/config.rs:64`) and the GUI supervises and stops what it starts (`crates/eggsec-gui/src/shell.rs:21-23`, `:53-56`). |
| "The GUI does not manage the daemon's lifecycle. It does not start, supervise, or stop the daemon … M6 has not closed." | `docs/GUI_DISTRIBUTION_HANDOFF.md:181-186` | False, same evidence. |
| `crates/eggsec-gui/frontend/app.js:1284-1288` is the connect handler; `:604-609`, submit handler at `:626`; `:896-904` (`cancel_task`), `:1128` (`cancel_active`) | `docs/GUI_BUILD.md:421`, `:430`, `:439`; `docs/GUI_PLATFORM_QUALIFICATION.md:60` | The connect invoke is `crates/eggsec-gui/frontend/app.js:1682`; the submit invoke is `:1006`; `cancel_task` is `:1525`; `cancel_active` is `:1538`. |
| `crates/eggsec-gui/src/config.rs:47` (the `--socket` flag), `:17` (the default) | `docs/GUI_BUILD.md:380-381`, `docs/GUI_DISTRIBUTION_HANDOFF.md:173` | The `--socket` match arm is `crates/eggsec-gui/src/config.rs:56`; the default constant is `:19`. |
| `crates/eggsec-gui/src/commands.rs:432` is the event stream's authority | `docs/GUI_BUILD.md:447`, `docs/GUI_PLATFORM_QUALIFICATION.md:60` | `commands.rs:432` is a `tauri::State` parameter. The authority statement is `crates/eggsec-gui/src/commands.rs:572`; the forwarder is `:857`. |
| `crates/eggsec-daemon/src/server.rs:476-483` constructs the GUI's scope | `docs/GUI_BUILD.md:389` | That range is inside a `#[cfg(test)]` module. The production construction is `crates/eggsec-daemon/src/server.rs:520-530`. |
| `crates/eggsec-daemon/src/main.rs:40-46` is the `--full-executor` fallback | `docs/GUI_BUILD.md:370` | The flag is at `crates/eggsec-daemon/src/main.rs:46`; the fallback branch is at `:112-130`. |

### 3.6 The socket default's six production sites

`docs/GUI_BUILD.md:383` and `docs/GUI_DISTRIBUTION_HANDOFF.md:170-176` both
say the default is spelled out in six production sites. Measured with
`grep -rn "/tmp/eggsec-daemon.sock" --include="*.rs" --include="*.js" .`
excluding `target/`, `plans/`, and `docs/`, there are **six** production
sites — `crates/eggsec-gui/frontend/app.js:23`,
`crates/eggsec-gui/frontend/index.html:25`,
`crates/eggsec-gui/src/config.rs:19`, `crates/eggsec-daemon/src/config.rs:22`,
`crates/eggsec-daemon/src/main.rs:20`, `crates/eggsec/src/cli/mod.rs:249` —
plus two test assertions (`crates/eggsec-gui/src/config.rs:134`,
`crates/eggsec-daemon/src/config.rs:38`). The count is right; M014 only
re-anchors the line numbers.

### 3.7 The daemon fixture the harness needs

The behavioural cells need a daemon that publishes a **non-empty** catalogue
and can complete a task. A no-op daemon cannot:

- `crates/eggsec-daemon/src/host.rs:183-190` — `DaemonHost::new_noop` installs
  `RuntimeCapabilities::noop()`.
- `crates/eggsec-daemon/src/host.rs:4071-4075` — the test
  `a_noop_host_still_publishes_an_empty_list` asserts the published task-kind
  list is **empty**.

So the harness's daemon fixture must be a `service`-profile daemon:
`scripts/build-daemon-profile.sh --profile service --out <dir>`
(`Makefile:224`, `:233`; the profile's own contract is
`docs/DAEMON_FEATURE_PROFILE.md:148`). A `service` daemon performs **real**
work, so the procedure must target a loopback fixture, which the engine
gates behind `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`
(`crates/eggsec/src/utils/parsing.rs:77`, `crates/eggsec-udp-scan/src/lib.rs:425`).

The daemon's `--full-executor` flag is a no-op on a binary built without the
`service` feature, with a warning
(`crates/eggsec-daemon/src/main.rs:112-130`).

### 3.8 The guards this milestone adds to

`scripts/check-architecture-guards.sh` carries **78** distinct check numbers,
ranging from **006** to **176**, the highest being
`scripts/check-architecture-guards.sh:5866` (the comment
`# 176. The daemon records the answer it accepts.`). The count is measured, not
estimated — the union of the `# NNN.` comment headers and the
`echo "--- Check NNN:"` headers, sorted and counted:

```bash
{ grep -oE "^# [0-9]{3}\." scripts/check-architecture-guards.sh | grep -oE "[0-9]{3}";
  grep -oE '^echo "--- Check [0-9]{3}:' scripts/check-architecture-guards.sh | grep -oE "[0-9]{3}"; } | sort -u | wc -l
```

That is 78 of the 171 numbers in the 006–176 range; 93 are skipped. Number
**006** is the one entry with a `# NNN.` comment header but no matching
`echo "--- Check 006:"` line; the other 77 have both.

**The numbers 181–189 are reserved for M014's nine guards**, in the execution
order of the four planned milestones: milestone 012 takes 177–179, milestone
013 takes 180, this milestone takes **181–189**, and milestone 015 takes 190.
The reservation follows the precedent of milestone 007's plan, which took 168
because the guards of milestones 006, 008, and 009 landed first. An
implementer who takes this milestone out of milestone order must re-derive the
numbers from the script's then-current highest, and must not reuse a number
another planned milestone has reserved. M014's guards therefore
start at **181**. The house style, read from the tail of the script: a
`# NNN. <one-sentence claim>` comment, then the check body, then a
`pass`/`fail` line in the script's own idiom.

## 4. Invariants that must not regress

1. **`EnforcementContext::evaluate()` remains the mandatory pre-dispatch
   gate.** The harness submits tasks through the daemon exactly as an operator
   would; it introduces no bypass, no operator override of its own, and no
   second authorization path. The GUI has no enforcement surface
   (`plans/000-long-term-specification.md:25`).
2. **The GUI stays daemon-only.** It links no engine, daemon, CLI, or TUI
   crate and performs no outbound HTTP (guard 155). The harness lives inside
   `crates/eggsec-gui` and uses the GUI's existing dependencies only.
3. **The frontend stays buildless and zero-npm.** The root `package.json`
   stays `{}`; no `package.json` appears under `crates/eggsec-gui/`.
4. **The CSP and the report-display rule are unchanged** (guard 162): the
   report reaches the webview as structured data, and no remote origin is
   introduced.
5. **No parallel lookup tables.** The seven column names and the seven
   assertion identifiers are the same seven concepts. M014 introduces them in
   exactly two places — the matrix header and the harness — and guards 185 and
   186 pin their agreement.
6. **No `let _ =` and no `filter_map(|e| e.ok())`.** Every failure is logged
   through `tracing` and surfaced as a named outcome, never discarded.
7. **TLS stays ring-only.** The harness opens a Unix socket and nothing else;
   it introduces no TLS stack and no HTTP client.
8. **No new dependency without a `deny.toml` decision.** M014's design adds
   **zero** dependencies: the harness uses `tokio` (already a GUI dependency
   with the `rt`, `sync`, `time`, `macros`, `net`, `process` and `signal`
   features — `crates/eggsec-gui/Cargo.toml:54`) and the GUI's existing
   workspace dependencies. If the implementer finds a dependency unavoidable,
   it needs an explicit `deny.toml` entry plus
   `docs/DEPENDENCY_EXCEPTIONS.md` with an owner and a review-by date.
9. **Spawned Tokio tasks carry 30–300 s timeout wrappers.** The harness's
   daemon supervisor and its socket waits are wrapped.
10. **Platform scripts stay hermetic.** No root, no hardware, no network
    beyond loopback. Absent prerequisites produce a **SKIP with the reason
    named**, never a silent pass and never a red job.
11. **The matrix's two absolute rules hold**: no cell is blank; no cell says
    `qualified` without an evidence reference.
12. **`make check` remains the mandatory contract.** M014 adds to it; it does
    not replace it.

## 5. Scope

### In scope

- The **export procedure**, written into `docs/GUI_BUILD.md` §5.7, and the
  §5.0/§5.1–§5.6 corrections the export procedure's neighbours need.
- The **qualification procedure**: `scripts/gui_qualification.sh` (the
  operator-facing driver and record producer) and
  `crates/eggsec-gui/tests/qualification.rs` (the assertion harness), plus the
  run-record format and its place in
  `docs/evidence/gui-qualification/TEMPLATE.md`.
- The **promotion rule** and its validator,
  `scripts/check_gui_qualification.py`.
- The **matrix update**: the stale root causes corrected, the `Last
  regenerated` header refreshed, and the cells promoted **only** where a
  recorded run already exists at the baseline or lands before closure.
- The **CI job extension**: the `gui-platform` job gains the harness step and
  publishes the run record as a job artefact.
- **Guards 181–189** in `scripts/check-architecture-guards.sh`, plus the
  matching entries in `docs/CI_ARCHITECTURE_GUARDS.md`.
- The **stale-claim corrections** enumerated in §3.5, in the three documents
  that carry them.

### Explicitly out of scope

- **The Windows row.** `ADR-0010` excludes Windows from the desktop GUI
  (`plans/adrs/ADR-0010-gui-supported-platform-scope.md`), and the matrix keeps
  all seven Windows cells `out of scope`
  (`docs/GUI_PLATFORM_QUALIFICATION.md:61`). M014 changes nothing about
  Windows: no Windows cell, no Windows CI leg, no Windows transport. The
  re-entry path is a daemon-subsystem change requiring its own ADR
  (`docs/GUI_PLATFORM_QUALIFICATION.md:214-226`).
- **A window-automation driver.** See §6 (Frontend or operator surface) for the
  decision and its justification. M014 does not add one.
- **The daemon's socket-path contract** — milestone 012's subject
  (`plans/subsystems/desktop-gui-frontend-roadmap.md:344`).
- **The report bridge's remaining coverage** — milestone 013's subject (`:347`).
  M014's export procedure describes the export of the reports that exist today
  and names the ones that do not.
- **The dependency advisory backlog** — milestone 015's subject (`:1012`).
  `make check` currently stops at the inherited `check-deps` advisory backlog;
  that backlog is *not* absorbed here (see §11.1).
- **The daemon's executor surface.** M014 does not add a fake or stub executor
  to the daemon binary; the harness uses the `service` profile.
- **The frontend's JavaScript test infrastructure.** M014 introduces no
  JavaScript test runner and no `package.json` under `crates/eggsec-gui/`.
- **The bundler, installers, signing, notarisation, and publication** — the
  distribution workstream's (`docs/GUI_DISTRIBUTION_HANDOFF.md:302-326`).

## 6. Required production changes

### Core/domain

M014 adds no production domain type, no engine concept, and no operation. Its
one new *domain* artefact is the qualification run record — the seven column
identifiers the matrix and the harness share. They are exactly the matrix's
seven columns:

`build`, `launch`, `connect`, `submit`, `cancel`, `render`, `export`.

They are the harness's assertion identifiers, the run record's `column`
values, and the promotion rule's subject. Guard 186 pins the set, and
guard 185 makes the runbook's step headings *derive* from the matrix
rather than restate it — so the seven concepts have exactly one owner
(the matrix header) and every other place is checked against it.

### Storage and migrations

**None.** M014 persists nothing: the run record is a file artefact under
`docs/evidence/gui-qualification/`, written by the procedure script and
committed like any other evidence file — not a table, not a daemon store, not
a GUI-side cache. The daemon's own report store is unchanged and stays
in-memory and bounded at 64 documents
(`crates/eggsec-runtime/src/report.rs:29`). No persisted representation
changes, so no data migration is required.

### Protocol and DTOs

**None on the wire.** M014 adds no `ClientCommand`, no `ServerMessage`,
and no field to either; the daemon protocol is untouched. The record block
is a *file* artefact, not a wire DTO: it is parsed by
`scripts/check_gui_qualification.py` out of a Markdown file, and it is
versioned by its own `schema` string
(`eggsec-gui-qualification-record/v1`).

The GUI's IPC surface stays at thirteen commands
(`crates/eggsec-gui/src/shell.rs:33-47`). M014 adds no command, changes
no command's signature, and does not widen what the webview may request.

### Runtime and concurrency

- **The harness's own concurrency.** One operation at a time against the
  daemon; the GUI's event forwarder (`crates/eggsec-gui/src/commands.rs:857`)
  is the only concurrent consumer, and it "never spawns a task per event"
  (`:843`).
- **Timeout wrappers.** Every awaited socket wait and every spawned task in
  the harness and the script carries a 30–300 s timeout wrapper.
- **The event stream is the authority** for a terminal state, not the cancel
  request's own acknowledgement
  (`crates/eggsec-gui/src/commands.rs:572`); the cancel race is expected and
  recorded, not treated as an error (`docs/GUI_BUILD.md:443-450`).
- **The report-retention window** bounds when an export may happen
  (`crates/eggsec-runtime/src/report.rs:29`).
- **The daemon's lifecycle.** The script starts the daemon it owns and stops
  it on every exit path; the GUI supervises and stops a daemon it started
  itself (`crates/eggsec-gui/src/shell.rs:21-23`, `:53-56`).

### Frontend or operator surface

- The **export panel** is unchanged. M014 documents it; it does not change
  it. The panel's controls are at `crates/eggsec-gui/frontend/index.html:77-111`.
- The **operator surface** gains one thing: the runbook's per-step checklist
  names the exact expected string for each operator observation, so the
  operator's attestation is specific rather than impressionistic.
- The **frontend's JavaScript is unchanged.** M014 adds no JavaScript.

`docs/GUI_BUILD.md` §5.7 "Step 7 — Export" currently reads (`:461-472`):

```
| **Precondition** | Milestone 006 closed. |
| **Action** | *Not yet defined.* … |
| **Pass criterion** | *Not yet defined.* |
| **Evidence artefact** | *Not yet defined.* |
```

M014 replaces it with a step matching the shape of §5.1–§5.6 (Precondition /
Action / Pass criterion / Evidence artefact):

- **Precondition.** A task that reached `completed` **and** whose result the
  report bridge covers (`crates/eggsec/src/runtime_bridge/report.rs:54`). The
  export must happen inside the daemon's report-retention window
  (`MAX_RETAINED_REPORTS = 64`, `crates/eggsec-runtime/src/report.rs:29`): a
  report evicted, or lost to a daemon restart, is gone.
- **Action.** The export panel (`crates/eggsec-gui/frontend/index.html:77-111`):
  the task id in `#export-task`, the format in `#export-format`, an absolute
  destination path in `#export-path`, then the button at `:106`. The handler is
  `crates/eggsec-gui/frontend/app.js:1612`; it invokes `export_report`
  (`:1624`); the command is `crates/eggsec-gui/src/commands.rs:670`.
- **Pass criterion.** The command returns `Ok`; the destination file exists;
  its SHA-256 equals the digest the record carries; its bytes parse as the
  chosen format; and — where the destination pre-existed — the prior content
  is preserved at `<path>.bak` (`crates/eggsec-gui/src/export.rs:353`, `:370`).
  A **refusal** is a legitimate observation: a non-regular-file destination is
  refused, and a task with no report is refused with a named reason
  (`crates/eggsec-gui/src/commands.rs:689-691`).
- **Evidence artefact.** The exported file, its SHA-256, the sidecar, and the
  run record's `export` entry.

The step must also name the **six** formats and their extensions
(`crates/eggsec-gui/src/export.rs:59-76`, `:92-99`), and must state that the
GUI writes **nowhere else**: "the destination is the operator's decision
alone" is the command's own contract (`crates/eggsec-gui/src/commands.rs:667`).

**Decision: M014 delivers a maintained, scripted operator procedure plus a
machine-checkable assertion harness. It does not deliver a window-automation
driver.**

The reasoning, in the order the evidence forces it:

0. **The matrix's own promotion rule already assumes the procedure route.**
   `docs/GUI_PLATFORM_QUALIFICATION.md:235` promotes the behavioural columns
   when "Milestones 006, 008, and 009 closed, then **the runbook steps in
   `docs/GUI_BUILD.md` §5 executed and recorded**" — not when a driver runs.
   The Linux row's rule (`:234`) adds "**plus an operator run of the
   behavioural steps** if the cell is a behavioural one". Both rules are
   already written against an operator procedure; a driver is an *addition* to
   them, not a substitute. All three closing milestones (006, 008, 009) are
   closed, so the remaining gate on every behavioural cell is the execution
   and recording of §5.
1. **The roadmap allows either** ("a window-automation driver, **or** a
   maintained operator procedure", `plans/subsystems/desktop-gui-frontend-roadmap.md:1028-1030`).
2. **A driver must work on both in-scope platforms** to discharge a
   two-platform matrix. The repository documents driver tooling for Linux only
   (`docs/GUI_BUILD.md:75-78`, `:139`) and documents **no** driver tooling for
   macOS (`:80-89`). Whether a macOS Tauri-v2 WebDriver driver exists is
   **not established by any evidence in this repository** (OQ-1, §14.1).
3. **The standard driver pairing needs a Node client**, which contradicts the
   zero-npm posture the empty root `package.json` and `docs/ARCHITECTURE.md:59`
   encode.
4. **CI has no display on either leg.** The `gui-platform` job installs no
   `xvfb` and no `xdotool` (`.github/workflows/deep-checks.yml:114`).
5. **But a *scripted* procedure is achievable**, and the repository already
   contains its two primitives: the window-existence probe the macOS evidence
   file used (`docs/evidence/gui-qualification/macos-2026-10-23.md:45-46`), and
   the XTest tooling the build doc already names (`docs/GUI_BUILD.md:75-78`).

So the procedure is **scripted wherever scripting is possible, and
operator-attested only where it is not**:

| Layer | What it proves | Where it runs |
|---|---|---|
| **Tier A — in-process** | The GUI's own library logic: the format/extension/parse tables, `export::render`, `export::write_export`, the backup discipline, form derivation, coverage derivation. | `cargo test`, any host, no daemon, no display. |
| **Tier B — live daemon** | The GUI's transport and session lifecycle against a real `service`-profile daemon: connect, catalogue, task kinds, submit, cancel, snapshot, report retrieval, export write. | The harness, against a socket. Hermetic; loopback-only target. |
| **Tier C — window** | The painted window: its existence, its title, and the operator-visible content (badge, populated sidebar, rendered card, export panel). | A desktop session. Scripted for existence; operator-attested for content; screenshot as the artefact. |

**The derivation that makes Tier B sufficient for the *cause* of a Tier C
observation.** The frontend sets its connection badge from the state it
receives: `crates/eggsec-gui/frontend/app.js:1682-1684` sets `state.connected
= true` from the `connect_daemon` result. Tier B asserts that `connect_daemon`
**succeeds**; the badge therefore reads "Connected". Tier B proves the
*cause*, the Tier C screenshot proves the *effect*, and **both** are required
before a cell may be promoted (rule P1).

**What the two tiers still do not prove:** that the webview paints its
pixels, that the DOM renders without a console error, or that the report
document renders faithfully in the window. Those are what the Tier C
screenshot and the operator's attestation carry, and the plan does not pretend
otherwise.

A hermetic, exit-code-carrying script, following the repository's existing
precedent `scripts/smoke-daemon-local.sh` (builds binaries into a temp
directory at `:50-52`, uses a `cleanup` trap at `:52-64`, exits 0/1 at
`:330`/`:327`).

| Aspect | Requirement |
|---|---|
| Interface | `--platform <linux\|macos>`, `--socket <path>`, `--daemon-bin <path>`, `--out <dir>`, `--mode <operator\|ci>`, `--skip-window`. |
| Daemon lifecycle | Starts the daemon on the given socket, waits for a **real readiness signal** (the CLI's start path implements — guard 166, `scripts/check-architecture-guards.sh:5334`), and stops it on exit. In `--mode operator` it may instead attach to a daemon the operator started. |
| Build | Delegates to `scripts/build_gui_release.sh --out <dir>` (the M007 recipe) and records the sidecar's fields. |

| Window probe | Linux: `xdotool search --name '^Eggsec$'` (XTest, named at `docs/GUI_BUILD.md:75-78`). macOS: `osascript -e 'tell application "System Events" to tell process "eggsec-gui" to get count of windows'` (the probe the macOS evidence file used at `:46`). Both are **recorded**, not asserted, when the prerequisite is absent — a named SKIP. |
| Screenshot | Linux: `import -window <id>` (ImageMagick) or `xwd`. macOS: `screencapture -x -o <file>` (the command the macOS evidence file used at `:45`). |
| Record | Writes `docs/evidence/gui-qualification/<platform>-<YYYY-MM-DD>.md` containing the template's fields **and** one fenced `json` block named `eggsec-gui-qualification-record` — the machine-readable run record (below). |
| Hermeticity | Writes only under `--out` and `${TMPDIR:-/tmp}`. No `sudo`, no network beyond loopback; absent prerequisites are named SKIPs. |

**One file, not two.** The evidence file is `<platform>-<YYYY-MM-DD>.md`,
following `docs/evidence/gui-qualification/TEMPLATE.md`, and it carries a
single fenced block:

````markdown
```json
{
  "schema": "eggsec-gui-qualification-record/v1",
  "platform": "linux",
  "date": "2026-10-25",
  "commit": "<40-hex>",
  "tree_state": "clean",
  "host": { "os": "…", "arch": "…" },
  "toolchain": { "rustc": "…", "cargo": "…" },
  "features": { "requested": ["shell"], "resolved": ["default", "shell"] },
  "prereq_probe": { "command": "bash scripts/check_gui_prereqs.sh", "exit": 0, "verdict": "BUILDABLE" },
  "daemon": { "version": "…", "features": ["service"], "socket": "/tmp/eggsec-gui-qualify.sock",
              "scope": { "surface": "DesktopGuiManual", "source": "gui-default" } },
  "columns": [
    { "column": "build",   "scripted": "pass", "operator": "not-applicable", "artifact": "…", "sha256": "…" },
    { "column": "<each of the other six columns, in matrix order>", "scripted": "pass",
      "operator": "pass", "artifact": "…", "sha256": "…" }
  ],
  "deviations": []
}
```
````

**Why one file.** Two artefacts (a `.md` and a `.json`) would be two sources
of truth for the same run, and they would drift. One file with one embedded
record removes the duplication; the validator reads the block and the
operator reads the prose.

**The seven `column` values are exactly the seven matrix columns.** They are
the harness's assertion identifiers, and guard 186 pins the set.

An **integration test** in the GUI crate. Rationale:

- It lives in the crate that owns the behaviour, so it links only what the GUI
  already links: `eggsec-frontend-client`, `eggsec-runtime`,
  `eggsec-daemon-protocol`, `eggsec-ui-model`, `eggsec-output`, `serde`,
  `serde_json`, `tokio`, `tracing` (`crates/eggsec-gui/Cargo.toml:37-56`).
  **No new dependency.**
- It uses the GUI's **own** client identity, `gui_identity()`
  (`crates/eggsec-gui/src/connection.rs:30-35`), so the daemon records the
  session exactly as it would for the window.
- It exercises the GUI's **ungated** library modules — `form`, `coverage`,
  `export`, `tasks`, `availability`, `connection`
  (`crates/eggsec-gui/src/lib.rs:11-18`) — because the Tauri-gated modules
  (`commands`, `shell`, `crates/eggsec-gui/src/lib.rs:20-23`) require a
  `tauri::State` and a live webview; that glue stays covered by the crate's
  existing unit tests, which the CI job already runs in both profiles
  (`.github/workflows/deep-checks.yml:128-131`).

**The SKIP discipline.** The test reads `EGGSEC_QUALIFY_SOCKET` and
`EGGSEC_QUALIFY_DAEMON_BIN`. When they are absent, it prints a **SKIP line
naming the missing prerequisite** and returns — the same discipline the
workspace's feature gate uses (`Makefile:24-33`) and the probe uses
(`scripts/check_gui_prereqs.sh:25-26`). A skipped harness is not a passed
harness; the CI job's step is what makes it run.

**The seven assertions**, one per column:

| ID | Tier | Assertion | Anchor |
|---|---|---|---|
| `build` | A | The release recipe emits the binary, the sidecar, and `SHA256SUMS`; the sidecar's `binary_sha256` equals the binary's SHA-256. | `scripts/build_gui_release.sh`; `docs/GUI_BUILD.md:400` |
| `launch` | C | A window titled `Eggsec` exists (scripted probe) and the operator confirms the painted contents. | `crates/eggsec-gui/tauri.conf.json:11-21`; `docs/GUI_BUILD.md:409-414` |
| `connect` | B | `connect_daemon` succeeds against the daemon; the catalogue is non-empty; the no-op badge stays hidden. | `crates/eggsec-gui/frontend/app.js:1682`; `docs/GUI_BUILD.md:422` |
| `submit` | B | The form derives from the task kind's descriptor; the submit is accepted (or refused with a named reason — both are legitimate observations); the task appears with a non-terminal status. | `crates/eggsec-gui/src/form.rs`; `crates/eggsec-gui/frontend/app.js:1006`; `docs/GUI_BUILD.md:431` |
| `cancel` | B | A cancel request against a live task reaches a terminal state, observed through the event stream. | `crates/eggsec-gui/src/commands.rs:572`, `:857`; `docs/GUI_BUILD.md:440-450` |
| `render` | B | A completed task carries a result payload, and the GUI renders it from that payload rather than from an empty dashboard. | `crates/eggsec-gui/frontend/app.js`; `docs/GUI_BUILD.md:458` |
| `export` | A + B | The export writes the chosen format to the chosen path, refuses a non-regular destination, and backs up an existing file to `<path>.bak`. | `crates/eggsec-gui/src/export.rs:312`, `:353`, `:370`; `crates/eggsec-gui/src/commands.rs:670` |

### Security and authorization

- The harness submits through the daemon, so the daemon's enforcement applies
  unchanged. **No bypass, no override, no second path.**
- The harness runs against a **local** socket and a **loopback** target
  (`EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`,
  `crates/eggsec/src/utils/parsing.rs:77`). It performs no outbound HTTP.
- The harness writes **only** under its `--out` directory and
  `${TMPDIR:-/tmp}` — never into the repository outside the evidence
  directory, never into a system path.
- The harness reads no credential, stores no credential, and logs no secret
  (`plans/subsystems/desktop-gui-frontend-roadmap.md`, "The GUI MUST NOT
  persist credentials in its own state or logs").
- The exported report is written **only** to the operator's chosen path, per
  `crates/eggsec-gui/src/commands.rs:667`.

### Documentation and static guards

**The promotion rule** (this is the answer to "how do the eight behavioural
cells become promotable"). Six rules, each mechanically checkable:

| Rule | Statement |
|---|---|
| **P1 — behavioural** | A behavioural cell moves `not qualified` → `qualified` iff a run record exists for that platform whose `commit` is the matrix's regenerated-at commit or a descendant recorded in the record; the record's entry for that column has `scripted == "pass"` **and** `operator == "pass"`; the entry's `artifact` exists under `docs/evidence/gui-qualification/` and its recorded `sha256` equals the file's actual SHA-256; **and** the cell names that record in the form the matrix §1 prescribes (path + reproducing command + run date). |
| **P2 — build** | As P1, except `operator` may be `not-applicable`: the Build criterion is machine-checkable end to end. |
| **P3 — launch** | `partially qualified` → `qualified` requires the **missing part** — the painted window — to be observed: the record's `launch` entry must carry a screenshot whose window title is `Eggsec`, and the scripted window-existence probe must have found exactly one window. A probe that found zero windows leaves the cell `partially qualified`, exactly as the matrix does today. |
| **P4 — export** | `pending M6` → `qualified` requires milestone 006 closed (it is), the export procedure present (guard 181), and an `export` entry with `scripted == "pass"` whose artefact is an exported file whose bytes equal the renderer's output for that report, and — where the destination pre-existed — the prior content preserved at `<path>.bak`. |
| **P5 — windows** | No promotion path. All seven Windows cells stay `out of scope` (`ADR-0010`; `docs/GUI_PLATFORM_QUALIFICATION.md:176-226`). |
| **P6 — no blank cell** | Every cell keeps a status token and a non-empty explanation. Check 168 owns the grammar; guard 183 owns the resolution. |

**The validator** `scripts/check_gui_qualification.py`:

1. Parses `docs/GUI_PLATFORM_QUALIFICATION.md`'s matrix into 21 cells.
2. Rejects a cell with no status token, no `—`, or an empty explanation
   (mirroring check 168, so the tool and the guard agree), and rejects a
   Windows cell that is not `out of scope`.
3. Rejects a `qualified` cell whose evidence reference does not resolve:
   neither an existing `docs/evidence/gui-qualification/<name>.md` nor a CI
   run URL recorded in `docs/evidence/gui-qualification/CI-RUNS.md`.
4. For a `qualified` cell that cites a run record: parses the embedded
   `eggsec-gui-qualification-record` block, requires the column exactly once,
   requires `scripted == "pass"` and `operator == "pass"` (unless the column
   is `build`), and requires the artefact's SHA-256 to match.
5. Rejects `pending M6` **anywhere** in the matrix while the export affordance
   exists in `crates/eggsec-gui/frontend/app.js` (guard 182 makes the same
   check static; the validator makes it semantic).

It exits 0 only if every check holds; otherwise it prints the offending cell
and exits non-zero.

The validator is the same shape as the repository's existing Python guards
(`scripts/check-python-architecture-guards.py`), and it is a *semantic*
companion to the *static* guards 181–189.

`.github/workflows/deep-checks.yml`'s `gui-platform` job (`:91-143`) gains a
step that builds the daemon's `service` profile via the canonical recipe
(`make build-daemon-profile`, `Makefile:224`); a step that runs
`bash scripts/gui_qualification.sh --platform <os> --mode ci`; and an upload
step publishing the run record and the exported artefact as job artefacts named
`gui-qualification-${{ matrix.os }}`, alongside the existing
`gui-release-${{ matrix.os }}` (`:136-143`).

The job's matrix stays `[ubuntu-latest, macos-latest]` (`:97`) — the Windows
leg stays excluded, per `ADR-0010` and the comment already in the workflow at
`:85-90`.

**Cost note:** building the `service` profile links the engine, so the
`gui-platform` job becomes materially slower. It lives in the `deep-checks`
workflow, whose trigger is a schedule
(`.github/workflows/deep-checks.yml:9`), not per-PR CI — so the cost lands
where a deep check belongs.

Nine guards, 181–189. Each is specified precisely enough to implement; M014
does **not** write them (that is the implementer's work).

| Guard | Invariant it checks | How it is checked |
|---|---|---|
| **181** | The export procedure is written. `docs/GUI_BUILD.md` §5.7 contains no `Not yet defined`; the Step 7 table's Action, Pass criterion, and Evidence artefact rows are each non-empty; the Action names `export_report` and the six format names. | Bash: extract the §5.7 block, assert the absence of the placeholder, assert three non-empty rows, assert the named strings. |
| **182** | `pending M6` is self-expiring. If `crates/eggsec-gui/frontend/app.js` contains the literal `export_report`, then `docs/GUI_PLATFORM_QUALIFICATION.md` contains no `pending M6`. | Bash: two greps, one conditional. |
| **183** | `qualified` ⇒ resolvable evidence. Every matrix cell whose token is `qualified` names either an existing `docs/evidence/gui-qualification/<file>.md` or a CI run URL that also appears in `docs/evidence/gui-qualification/CI-RUNS.md`. | Bash: extract the qualified cells, resolve each reference, assert existence. |
| **184** | The matrix shape is exact. The header row is exactly `\| Platform \| Build \| Launch \| Connect \| Submit \| Cancel \| Render \| Export \|`; there are exactly three platform rows (`linux`, `macos`, `windows`); the cell count is 21. | Bash: count the table rows and the pipes per row. |
| **185** | The runbook derives its steps from the matrix. `docs/GUI_BUILD.md` §5's step headings are exactly `Step 1 — Build` … `Step 7 — Export`, one per matrix column, in the matrix's order. | Bash: extract the §5.x headings, compare to the matrix header. |
| **186** | The harness's assertion identifiers are exactly the seven columns. The harness source defines no identifier outside `{build, launch, connect, submit, cancel, render, export}`, and defines each of the seven. | Bash: extract the identifier set from the harness's assertion table and compare. |
| **187** | The procedure is hermetic. `scripts/gui_qualification.sh` contains no `sudo` and no `rm -rf /`; it writes only under `--out` and `${TMPDIR:-/tmp}`; and it never requires root. | Bash: forbidden-token scan plus a write-path scan. |
| **188** | The promotion validator is green. `python3 scripts/check_gui_qualification.py` exits 0. | Bash: run the validator, propagate its exit code. |
| **189** | The CI job publishes the qualification record. The `gui-platform` job runs the qualification script and uploads a run record as a job artefact. | Bash: assert the workflow's `gui-platform` block contains the script invocation and the artefact upload path. |

Each guard's body follows the tail-of-file house style
(`scripts/check-architecture-guards.sh:5866` onward): a `# NNN.` claim
comment, the check, and the script's own pass/fail idiom.

## 7. Ordered work packages

Each package names its **intent**, the **changes** (pointing at §6, which
specifies them), and the **acceptance evidence** that closes it.

### Work package A — Correct the stale claims

**Intent.** Before anything else cites them, the matrix and the runbook must
stop asserting facts that milestone 006 made false. The drift is invisible to
the toolchain: `scripts/check_doc_references.py:6` checks path existence
only, never a `path:line` anchor.

**Required changes.** The whole table in §3.5: the two "no export affordance"
claims (`docs/GUI_BUILD.md:466`, `docs/GUI_PLATFORM_QUALIFICATION.md:239-243`),
the two stale daemon-lifecycle claims (`docs/GUI_BUILD.md:375-377`,
`docs/GUI_DISTRIBUTION_HANDOFF.md:181-186`), every stale `path:line` anchor,
and the two stale root causes in the macOS row
(`docs/GUI_PLATFORM_QUALIFICATION.md:60`).

**Acceptance evidence.** No `path:line` anchor points at a line that does not
carry the cited symbol; the "no export affordance" claims are gone;
`scripts/check_doc_references.py` reports the documents' paths as existing.

### Work package B — Write the export procedure

**Intent.** Close the single gap the matrix names: "the export procedure the
matrix leaves undefined"
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1030`).

**Required changes.** `docs/GUI_BUILD.md` §5.7 per §6 (Frontend or operator
surface); §6 (Evidence, `:488-495`) gains the export artefact's place in the
record; `docs/evidence/gui-qualification/TEMPLATE.md` gains an `export` row and
the `eggsec-gui-qualification-record` block.

**Acceptance evidence.** §5.7 has four filled rows and no `Not yet defined`;
the Action names the panel's three controls, the IPC command, and the six
formats; the Pass criterion names the digest, the parse, and the `.bak`
backup; the Evidence artefact names the file, the digest, and the record.

### Work package C — The run-record format and the validator

**Intent.** Give the matrix a machine-checkable notion of "promotable".

**Required changes.** (1) `docs/evidence/gui-qualification/TEMPLATE.md` — the
`eggsec-gui-qualification-record` fenced JSON block with the seven `column`
entries. (2) `docs/evidence/gui-qualification/CI-RUNS.md` — a new index of CI
runs a cell may cite: run URL, workflow, job, matrix leg, commit, artefact
names, date; the matrix §4.4 rule
(`docs/GUI_PLATFORM_QUALIFICATION.md:166-172`) needs a home for the URL it
cites. (3) `scripts/check_gui_qualification.py` — the validator. (4)
`docs/GUI_PLATFORM_QUALIFICATION.md` §6 — the promotion table gains rules
P1–P6, replacing the prose-only table at `:232-237`.

**Acceptance evidence.** The validator exits 0 on the matrix as it stands
after WP-A; it rejects a `qualified` cell with an unresolvable reference; the
promotion table names every rule the validator enforces.

### Work package D — The assertion harness

**Intent.** Make the GUI's own behaviour assertable without a window.

**Required changes.** `crates/eggsec-gui/tests/qualification.rs`, per §6
(Frontend or operator surface): the seven assertions, the SKIP discipline,
`tracing`-only failure logging, and the 30–300 s timeout wrappers.

**Acceptance evidence.** The seven assertion identifiers exist and no others
(guard 186); the test SKIPs with a named reason when the environment is
absent; the Tier A assertions run without a daemon.

### Work package E — The procedure script

**Intent.** Turn the runbook into something a second operator can follow
without inventing steps.

**Required changes.** `scripts/gui_qualification.sh`, per §6 (Frontend or
operator surface): the interface, the daemon lifecycle with its cleanup trap,
the window probe and screenshot with named SKIPs, the record writer, and the
hermeticity rules of guard 187.

**Acceptance evidence.** The script runs end to end on a host with the
prerequisites and writes a record; on a host without them it writes a record
with named SKIPs; it exits non-zero when an assertion fails; it leaves nothing
outside `--out` and `${TMPDIR:-/tmp}`.

### Work package F — The CI job, the guards, and the docs

**Intent.** Give the Linux row the evidence path the matrix names ("the
`gui-platform` CI job on `ubuntu-latest` is the evidence path",
`docs/GUI_PLATFORM_QUALIFICATION.md:59`), pin the invariants, and make the
new artefacts discoverable.

**Required changes.** (1) `.github/workflows/deep-checks.yml`'s `gui-platform`
job — build the `service` daemon profile, run the procedure in `--mode ci`,
upload the run record and the exported artefact as
`gui-qualification-${{ matrix.os }}`; the matrix stays
`[ubuntu-latest, macos-latest]` (`:97`). (2) Guards 181–189 in
`scripts/check-architecture-guards.sh`, each per §6 (Documentation and static
guards), plus the matching entries in `docs/CI_ARCHITECTURE_GUARDS.md`. (3)
`architecture/desktop_gui.md`'s "The qualification matrix" section
(`:669-678`) names the procedure and the validator; `docs/GUI_BUILD.md` §2
gains a note that the webdriver tooling the document mentions
(`docs/GUI_BUILD.md:75-78`) is **not** used by the repository's own procedure;
`.opencode/skills/eggsec-gui/SKILL.md` gains the procedure's and the
validator's entry points; `plans/registry.md` gains the milestone's entry;
the roadmap's §4 gap table row 9
(`plans/subsystems/desktop-gui-frontend-roadmap.md:345`) and its §12 status
row are updated.

**Acceptance evidence.** The job's YAML defines both steps and the upload and
its artefact list includes the record; the guard script reaches the new checks
and each fails when its invariant is violated; every new artefact is reachable
from the matrix, the runbook, the deep dive, and the skill.

### Work package G — The matrix update

**Intent.** Move the matrix from "blocked on an absent capability" to
"promotable", and promote only what a recorded run already supports.

**Required changes.** (1) Refresh the `Last regenerated` header
(`docs/GUI_PLATFORM_QUALIFICATION.md:54-55`). (2) Rewrite every cell's root
cause so it names the **current** blocker: the procedure now exists, so the
blocker is the absence of a recorded run (and, on Linux, the absence of a
display in CI). (3) Promote a cell **only** where a record already exists at
the baseline or lands before closure, and only under rules P1–P6. (4) Leave
the Windows row untouched.

**Acceptance evidence.** The validator exits 0 against the updated matrix; no
cell is blank; every `qualified` cell resolves.

## 8. Failure, cancellation, restart, and contention semantics

### 8.1 For a CI run

| Situation | Behaviour |
|---|---|
| The webview SDK is absent on the Linux leg | The probe reports `NOT BUILDABLE` with the missing packages named (`scripts/check_gui_prereqs.sh:25-26`); the harness records a **named SKIP**; the job continues. The job never reports green-without-owner. |
| The `service` daemon build fails, or the daemon never becomes ready | The job fails. A missing fixture is not a pass. The readiness wait is bounded by a 30–300 s timeout wrapper; the failure is logged through `tracing` and the daemon's last log lines are attached. |
| The harness's assertions fail | The job fails, and the record is still uploaded with the failing column's outcome recorded as `fail` — a red job **with** its evidence. |
| A column's prerequisite is absent (no `xdotool` on the leg) | The column's `scripted` outcome is `skipped` with the reason named; the column **cannot** be promoted (rule P1 requires `pass`), and the skip alone does not fail the job. |
| The CI job is cancelled mid-run | The cleanup trap stops the daemon and unlinks the socket; the partial record is **not** uploaded, because a cancelled run is not a run. |

### 8.2 For a manual qualification run

| Situation | Behaviour |
|---|---|
| The operator starts the daemon by hand | The script attaches to the given `--socket` instead of starting one; the record's `daemon.socket` field carries the path actually used. |
| The operator uses the GUI's own `--daemon-bin` | The GUI supervises and stops the daemon it started (`crates/eggsec-gui/src/shell.rs:21-23`, `:53-56`); the record names which lifecycle was used. |
| The operator's daemon is restarted mid-run | The daemon's report store is in-memory (`crates/eggsec-runtime/src/report.rs:29-40`), so every report published before the restart is gone. The `export` column is recorded as `fail` with the restart named as the cause; the run is **not** silently continued as if the export had succeeded. |
| A cancel races the task's completion | The event stream is authoritative (`crates/eggsec-gui/src/commands.rs:572`). A `completed` terminal state after a cancel click is recorded as a completed task, not a failed cancellation (`docs/GUI_BUILD.md:443-450`). |
| The submit is refused by the daemon | A refusal is a legitimate observation and is recorded as such (`docs/GUI_BUILD.md:431`). The `submit` column is `pass` when the GUI surfaced the refusal correctly — which is what the cell says the GUI's job is. |
| Two operators run concurrently | Each writes its own record, `<platform>-<date>.md`. A same-day collision is resolved by a distinct suffix — the validator requires the matrix's cited file to exist, not a unique name. |
| The operator aborts | The cleanup trap stops the daemon it started and removes the socket; the evidence directory keeps what was captured, with the incomplete columns recorded as `not-performed`. |

### 8.3 Contention and concurrency inside the harness

The harness issues **one** operation at a time against the daemon; the GUI's
event forwarder is the only concurrent consumer
(`crates/eggsec-gui/src/commands.rs:857-948`), and it "never spawns a task per
event" (`crates/eggsec-gui/src/commands.rs:843`). The cancel assertion
observes the terminal state through the event stream, not through the cancel
request's own acknowledgement. The harness never retains an unbounded set of
payloads; each task's result is fetched on demand and dropped.

## 9. Compatibility and migration

**No wire change.** M014 adds no `ClientCommand`, no `ServerMessage`, and no
field to either; the daemon protocol is unchanged.

**No persisted-representation change and no migration.** No milestone in this
roadmap changes an existing persisted representation
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1085-1086`); M014 adds no
storage.

**No configuration change.** The GUI's flags are unchanged
(`crates/eggsec-gui/src/config.rs:46-93`); the procedure passes the same
`--socket` and `--daemon-bin` flags the operator already uses.

**The evidence format is new** (`eggsec-gui-qualification-record/v1`): the
validator accepts **only** `v1` and rejects an unknown `schema` with a named
error, so a future `v2` cannot silently pass under a `v1` validator.

**The matrix's status vocabulary is unchanged**
(`docs/GUI_PLATFORM_QUALIFICATION.md:29-35`), but the `pending M6` token is
**removed** by guard 182 once the export affordance exists — so its meaning
changes from "blocked on a milestone" to "illegal". That is deliberate: a
token that can outlive its cause is a token that lies. The removal criterion is
exactly the affordance's existence in
`crates/eggsec-gui/frontend/app.js`, and M014 delivers that removal as part of
the matrix update.

## 10. Required tests

### Focused unit tests

| Test | Where | Assertion |
|---|---|---|
| The run-record schema round-trips | `crates/eggsec-gui/tests/qualification.rs` | A record with all seven columns parses and re-serialises unchanged; a record missing a column is rejected by name. |
| The seven identifiers are the seven columns | `crates/eggsec-gui/tests/qualification.rs` | The harness's identifier set equals `{build, launch, connect, submit, cancel, render, export}` — the same set as the matrix header. |
| The format tables agree with the record | `crates/eggsec-gui/src/export.rs`'s existing tests, extended | `ReportFormat::parse` accepts every name the frontend offers and `extension()` agrees with the written file's suffix, for all six formats. |

### Integration tests

| Test | Where | Assertion |
|---|---|---|
| Tier A export, all six formats | `crates/eggsec-gui/tests/qualification.rs` | For each of the six formats, `export::render` produces bytes, `export::write_export` writes them, and the file's SHA-256 equals the recorded digest. |
| Tier A export refuses a non-regular destination | `crates/eggsec-gui/tests/qualification.rs` | Writing to a directory or a device is refused with a named error; nothing is written. |
| Tier A export backs up an existing file | `crates/eggsec-gui/tests/qualification.rs` | Writing over an existing regular file leaves the prior content at `<path>.bak`. |
| Tier B connect | `crates/eggsec-gui/tests/qualification.rs` | `connect_daemon` (via the GUI's own client and identity) succeeds against a live daemon; the catalogue is non-empty. |
| Tier B submit and cancel | `crates/eggsec-gui/tests/qualification.rs` | A submitted task reaches a terminal state observed through the event stream; a cancel against a live task reaches a terminal state. |
| Tier B render | `crates/eggsec-gui/tests/qualification.rs` | A completed task's payload is non-empty and the GUI's rendering path derives fields from it. |
| Tier B export | `crates/eggsec-gui/tests/qualification.rs` | The exported file's bytes equal `export::render`'s output for the report the daemon published. |
| The script writes a record | `scripts/gui_qualification.sh` (end-to-end, hermetic) | A run against a locally built daemon writes `docs/evidence/gui-qualification/<platform>-<date>.md` containing the record block with all seven columns present. |
| The validator round-trips | `scripts/check_gui_qualification.py` | The validator exits 0 on a well-formed matrix and record pair, and non-zero with a named cell on a tampered one. |

### Restart and recovery tests

| Test | Where | Assertion |
|---|---|---|
| The daemon restarts between the task and the export | `crates/eggsec-gui/tests/qualification.rs` | The export is refused with the daemon's own reason ("the daemon has no report for task …"), and the record's `export` outcome is `fail` with the restart named. The harness does **not** report a silent success. |
| The script's cleanup leaves nothing behind | `scripts/gui_qualification.sh` | After the script exits — normally, on failure, and on `SIGINT` — no daemon process and no socket file remain. |

### Contention and cancellation tests

| Test | Where | Assertion |
|---|---|---|
| Cancel loses the race | `crates/eggsec-gui/tests/qualification.rs` | When the task completes before the cancel is processed, the terminal state observed through the event stream is `completed`, and the harness records it as a completed task rather than a failed cancellation. |
| The event stream is the authority | `crates/eggsec-gui/tests/qualification.rs` | The harness's terminal-state observation comes from the event stream, never from the cancel request's acknowledgement. |
| Two concurrent harness runs | `scripts/gui_qualification.sh` | Two runs against two distinct sockets do not interfere; each writes its own record. |

### Security and negative tests

| Test | Where | Assertion |
|---|---|---|
| No enforcement bypass | `crates/eggsec-gui/tests/qualification.rs` | The harness's submit path goes through the daemon and is subject to the daemon's enforcement; a task the daemon refuses is refused for the harness too. |
| The harness adds no network reach | `crates/eggsec-gui/tests/qualification.rs` | The harness opens exactly one socket, a Unix socket; no HTTP client is constructed. |
| No credentials are logged | `crates/eggsec-gui/tests/qualification.rs` | The record's fields contain no credential material; the harness logs no token. |
| The harness writes only where it says | `scripts/gui_qualification.sh` | The script's write paths are confined to `--out` and `${TMPDIR:-/tmp}`; a sentinel file elsewhere is untouched. |
| The matrix cannot overstate | `scripts/check_gui_qualification.py` | A `qualified` cell whose artefact is missing, whose digest does not match, or whose record lacks the column is rejected. |
| The Windows row cannot be promoted | `scripts/check_gui_qualification.py` | A Windows cell that is not `out of scope` is rejected. |

### Migration and compatibility tests

| Test | Where | Assertion |
|---|---|---|
| The status vocabulary is unchanged | `scripts/check_gui_qualification.py` | The five tokens at `docs/GUI_PLATFORM_QUALIFICATION.md:29-35` are the only ones accepted; the validator's list and guard 168's list are the same list. |
| `pending M6` is rejected once the affordance exists | `scripts/check_gui_qualification.py` | A matrix containing `pending M6` is rejected while `crates/eggsec-gui/frontend/app.js` contains `export_report`. |
| The matrix shape is unchanged | `scripts/check_gui_qualification.py` | A matrix with 20 cells, 22 cells, or a renamed column is rejected. |
| The record schema version is enforced | `scripts/check_gui_qualification.py` | A record whose `schema` is not `eggsec-gui-qualification-record/v1` is rejected by name. |
| The guards fail when their invariant is broken | `scripts/check-architecture-guards.sh` | Each of guards 181–189 is demonstrated to fail against a deliberately broken input — the negative test the closure record must record. |
| The old evidence file still validates | `scripts/check_gui_qualification.py` | `docs/evidence/gui-qualification/macos-2026-10-23.md` — a v0 file with no record block — is **not** rejected as malformed; it is simply not sufficient to promote a cell. |

## 11. Required verification commands

```bash
make check                  # the mandatory Rust contract (Makefile:161)
make check-deps             # dependency policy only (Makefile:152)

make check-gui              # the ungated shell-profile check (Makefile:198-200)
make check-gui-prereqs      # the prerequisite probe (Makefile:206-207)
make gui-release            # the release build and its sidecar (Makefile:213-214)

cargo test -p eggsec-gui --features shell --lib        # the shell profile
cargo test -p eggsec-gui --no-default-features --lib    # the no-shell profile
cargo test -p eggsec-gui --no-default-features --test qualification
bash scripts/gui_qualification.sh --platform linux --mode ci
python3 scripts/check_gui_qualification.py

bash scripts/check-architecture-guards.sh   # needs ripgrep (rg)
python3 scripts/check_doc_references.py

make check-features-individual   # Makefile:248 — deep check
make check-msrv                  # Makefile:135 — the GUI crate is included
make check-full                   # Makefile:241 — deep check
```

### 11.1 The contract, and the backlog that is not M014's

`make check` currently stops at the inherited `check-deps` advisory backlog.
**That backlog belongs to milestone 015**
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1042`), not to M014, and
M014 does not absorb it. The implementer records the advisory count observed
at the time of the run and does **not** attempt to clear it.

`make check-python` (`Makefile:290`) is **not** required: M014 changes no
Python binding, stub, or script.

### 11.2 The CI job

The `gui-platform` job in `.github/workflows/deep-checks.yml` (`:91-143`), on
both matrix legs. Its artefacts are `gui-release-${{ matrix.os }}` (the
existing pair) and `gui-qualification-${{ matrix.os }}` (the new one).

## 12. Documentation updates

| Document | Change |
|---|---|
| `docs/GUI_BUILD.md` | §5.7 rewritten (the export procedure); §5.0's daemon-lifecycle paragraph corrected; §5.1–§5.6's stale anchors re-anchored and each step's operator observation made specific; §2 gains a note that the webdriver tooling the document names is **not** used by the repository's own procedure; §6 names the export artefact. |
| `docs/GUI_PLATFORM_QUALIFICATION.md` | The two "no export affordance" claims corrected; the stale root causes rewritten; §6's promotion table replaced by rules P1–P6; the `Last regenerated` header refreshed. |
| `docs/GUI_DISTRIBUTION_HANDOFF.md` | §4's daemon-lifecycle limitation corrected; the §1 artefact-identity table's stale `config.rs:17` anchor re-anchored to `:19`; the handoff gains a line stating what the export procedure produces. |
| `docs/evidence/gui-qualification/TEMPLATE.md` and the new `CI-RUNS.md` | The `eggsec-gui-qualification-record` block, the `export` row, and the index of CI runs a cell may cite. The handoff's stale `server.rs:95` anchor (§3.5) is re-pointed at `:147`. |
| `architecture/desktop_gui.md` | The "qualification matrix" section (`:669-678`) names the procedure, the harness, and the validator. |
| `docs/CI_ARCHITECTURE_GUARDS.md` | Guards 181–189. |
| `.opencode/skills/eggsec-gui/SKILL.md` | The procedure's and the validator's entry points; the counted-set claims checked. |
| `plans/registry.md` and the subsystem roadmap | The milestone's entry; the roadmap's §4 gap table row 9 (`:345`) and its §12 status row. |

## 13. Acceptance criteria

1. `docs/GUI_BUILD.md` §5.7 contains a complete export procedure — four filled
   rows, no `Not yet defined` — naming the panel's controls, the
   `export_report` command, the six formats, the digest and parse checks, the
   `.bak` backup, and the refusal cases.
2. `scripts/gui_qualification.sh` exists in the repository, is hermetic, and
   writes a run record with all seven columns.
3. `crates/eggsec-gui/tests/qualification.rs` exists, adds **no** dependency
   to `crates/eggsec-gui`, SKIPs with a named reason when its environment is
   absent, and asserts the seven columns.
4. `scripts/check_gui_qualification.py` exists and enforces rules P1–P6.
5. The `gui-platform` CI job runs the procedure on both legs and publishes the
   run record as a job artefact.
6. The matrix contains no `pending M6` token, no blank cell, and no
   `qualified` cell without a resolving evidence reference.
7. Every behavioural cell's root cause names the **current** blocker; the
   macOS `Connect` cell no longer says "the repository ships no such driver",
   because the repository now ships a procedure.
8. Guards 181–189 exist, are numbered after 176, and each fails when its
   invariant is broken.
9. No cell is promoted without a recorded run satisfying P1–P6.
10. The Windows row is byte-identical to its baseline state.

## 14. Stop conditions

The agent must stop and report rather than improvise when:

- **A Tauri-v2 WebDriver driver for the macOS leg turns out to exist** and to
  be usable without a Node client. That would change the deliverable from "a
  maintained operator procedure" to "a window-automation driver" — a change to
  the roadmap's deliverable boundary, and therefore a planning decision, not
  an implementation decision (OQ-1).
- **A driver is adopted.** It would add a dependency or a Node toolchain and
  would break the zero-npm posture. That needs an ADR.
- **The webview does not paint under a virtual display on `ubuntu-latest`.**
  Then the Linux behavioural cells cannot be promoted by CI at all, and the
  plan's honest outcome is "operator-run only". The implementer records that
  and does **not** weaken the cell's criterion to make it green (OQ-2).
- **The `service` daemon profile cannot be built on a CI leg.** Then the
  harness's daemon half cannot run there and the leg records a named SKIP. The
  implementer does not add a fake executor to the daemon binary to work around
  it — that is a daemon-subsystem change outside this milestone.
- **The report bridge's coverage** (milestone 013) turns out to be a
  precondition for the export column. The export procedure then documents the
  limitation rather than waiting for 013.
- **`make check`'s advisory backlog** grows during the milestone. It stays
  milestone 015's.
- **The matrix's promotion rule is judged insufficient** — for example, if a
  reviewer holds that a harness assertion plus a screenshot cannot promote a
  cell. That is a change to the rule this milestone writes; if the rule is
  wrong, the milestone stops and revises it deliberately.
- **Any change to the daemon protocol, the enforcement gate, or the GUI's
  dependency closure** turns out to be necessary. All three are out of scope.

### 14.1 Open questions carried into the implementation

| ID | Question | Why it is open |
|---|---|---|
| **OQ-1** | Does a Tauri-v2 WebDriver driver exist for the macOS leg that does not require a Node client? | Nothing in this repository answers it. `docs/GUI_BUILD.md:80-89` documents no macOS driver tooling. It changes the deliverable, so §14 makes it a stop condition rather than an assumption. |
| **OQ-2** | Can the WKWebView-backed GUI paint a window under `xvfb` on `ubuntu-latest`? | Unmeasured. The `gui-platform` job installs no `xvfb` (`.github/workflows/deep-checks.yml:114`). The windowed CI leg in WP-F depends on the answer. |
| **OQ-3** | Does the CI macOS runner have an attachable GUI session? | Unmeasured. The macOS evidence file records a display (`screencapture` produced a 3840×1080 capture on the operator's host, `docs/evidence/gui-qualification/macos-2026-10-23.md:143-145`) but says nothing about a CI runner's session. |
| **OQ-4** | Does the macOS `osascript` accessibility probe work in a CI context? | Unmeasured; the macOS probe needs the Accessibility permission for the *scripting* process. On the operator's host it ran and returned `0` rather than erroring (`docs/evidence/gui-qualification/macos-2026-10-23.md:146-148`), which suggests the permission was present there and the window genuinely was not there. |

## 15. Closure evidence required

The closure record must contain:

1. **The export procedure, verbatim** — `docs/GUI_BUILD.md` §5.7 after the
   change, with the four rows filled and the six formats named.
2. **The procedure script** — `scripts/gui_qualification.sh` in the
   repository, with a transcript of one run per platform (or an explicit
   statement that a platform had no host).
3. **A run record** — a `docs/evidence/gui-qualification/<platform>-<date>.md`
   file containing the `eggsec-gui-qualification-record` block, with the
   artefact paths and digests it claims.
4. **The harness's output** — the
   `cargo test -p eggsec-gui --no-default-features --test qualification`
   transcript, including the SKIP line when the daemon was absent.
5. **The validator's output** — `python3 scripts/check_gui_qualification.py`
   with its exit code, and **one demonstrated rejection** of a tampered matrix.
6. **The guards' evidence** — `bash scripts/check-architecture-guards.sh` with
   the new checks visible, and **one demonstrated failure per new guard**
   against a deliberately broken input.
7. **The CI run** — the `gui-platform` run URL for each leg, the artefact
   names, and the entries added to
   `docs/evidence/gui-qualification/CI-RUNS.md`.
8. **The matrix diff** — the before and after, with every promotion tied to a
   specific record and every non-promotion tied to a specific missing
   precondition.
9. **The dependency statement** — that `crates/eggsec-gui`'s dependency closure
   is unchanged, or the `deny.toml` entry and the
   `docs/DEPENDENCY_EXCEPTIONS.md` record with owner and review-by date.
10. **The `make check` transcript** — including the advisory count observed,
    attributed to milestone 015 rather than absorbed.

## 16. Handoff notes

### 16.1 Known hazards

- **The procedure's evidence is an attestation plus artefacts.** The validator
  checks that the artefacts exist, that their digests match, and that the
  record is internally consistent. It **cannot** check that a human observed
  what the record says they observed. That is the honest cost of choosing the
  procedure route over a driver; the matrix's own standard is
  reproducibility-by-command (`docs/GUI_PLATFORM_QUALIFICATION.md:43-48`), and
  M014 strengthens it by requiring the artefacts themselves to be committed
  and digested.
- **The harness proves the cause, not the effect.** The screenshot proves the
  effect. A cell needs both (rule P1); a promotion resting on the harness
  alone is not a promotion.
- **The `service` daemon profile is heavy.** It links the engine, so the
  `gui-platform` job becomes materially slower. That job lives in the
  scheduled `deep-checks` workflow, not per-PR CI.
- **The matrix and the runbook are load-bearing for guards 181–189.** Editing
  their structure without updating the guards produces a guard failure that
  looks like a defect in the matrix.
- **The frontend has no JavaScript test infrastructure at all.** The webview's
  own rendering is covered by the Tier C screenshot and the operator's
  attestation only; closing that gap needs a JavaScript test runner, which the
  zero-npm posture forbids without an ADR.
- **The record format is a new contract.** Anything that writes a record — the
  script, a future CI step, a human — must write the same shape; guard 186 and
  the validator's schema check keep that honest.

### 16.2 Environment requirements

- A host with the Tauri v2 webview SDK to build the shell
  (`docs/GUI_BUILD.md` §2), or a leg that installs it
  (`.github/workflows/deep-checks.yml:112-114`).
- A `service`-profile daemon (`scripts/build-daemon-profile.sh --profile
  service --out <dir>`).
- `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` in the run's environment, so the engine's
  loopback guard admits the fixture target
  (`crates/eggsec/src/utils/parsing.rs:77`).
- `xdotool` (Linux) or `osascript` (macOS) for the window probe; `import` /
  `xwd` (Linux) or `screencapture` (macOS) for the screenshot. Both are
  **optional**: absence is a named SKIP, not a failure.

### 16.3 Resource constraints, serial-test requirements, and preserved changes

The harness's socket waits and spawned tasks carry 30–300 s timeout wrappers.
The daemon's report store retains at most 64 reports
(`crates/eggsec-runtime/src/report.rs:29`); the harness exports promptly and
does not accumulate payloads. The script's cleanup trap removes its socket and
stops its daemon on every exit path, including `SIGINT` and `SIGTERM`.

The harness binds a socket, so **one harness run per socket**. The script uses
a unique socket path per run (a `$$`-derived temporary path, as
`scripts/smoke-daemon-local.sh:44-46` does), so concurrent runs on the same
host do not collide.

The working tree at the baseline carries **no uncommitted changes**
(`git status --porcelain` is empty at commit `821c8189`), so nothing needs
preserving. If the implementer finds the tree dirty on arrival, the dirty
state must be recorded in the run record's `tree_state` field and in the
evidence file's "Tree state" field, as
`docs/GUI_PLATFORM_QUALIFICATION.md:151-153` requires.

### 16.4 Volatile anchors

The subsystem roadmap is being edited in parallel by the milestones that
precede and follow this one, so **its** `path:line` anchors shift. The
milestone-14 section's anchors in this plan were measured against the roadmap
at the baseline commit; re-verify them against the
`### Milestone 14 — The GUI's qualification driver and runbook` heading
(`plans/subsystems/desktop-gui-frontend-roadmap.md:1018` at the baseline) when
implementing. The anchors into `docs/GUI_PLATFORM_QUALIFICATION.md`,
`docs/GUI_BUILD.md`, `docs/GUI_DISTRIBUTION_HANDOFF.md`, and the crates are
stable for this milestone's duration, because M014 owns the first two and does
not edit the third.

### 16.5 What this milestone deliberately does not do

It does not promote a cell without a recorded run; add a window-automation
driver (§6, Frontend or operator surface); touch the Windows row
(`ADR-0010`); add a dependency to any crate; absorb the `check-deps` advisory
backlog (milestone 015); run `make check-python` (no Python surface changes);
clear the socket-path duplication (milestone 012); extend the report bridge
(milestone 013); or commit anything.
