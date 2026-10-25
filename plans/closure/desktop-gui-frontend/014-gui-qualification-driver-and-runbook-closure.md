# Desktop GUI Frontend Milestone 014 — Closure Status

Status: conditionally closed

Source implementation plan:

- `plans/implementation/desktop-gui-frontend/014-gui-qualification-driver-and-runbook.md`

Source subsystem roadmap:

- `plans/subsystems/desktop-gui-frontend-roadmap.md#milestone-14--the-guis-qualification-driver-and-runbook`

Repository baseline reviewed: `0f5dbb26e7ab215613e5b6e2295d9c124374bf9f`

Implementation commits or pull requests:

- `6de6340e` — feat(desktop-gui): implement the desktop GUI milestones 012 to 015

## 1. Executive finding

**The qualification procedure, the assertion harness, the promotion-rule
validator, and the nine static guards all landed, and the matrix's behavioural
and export cells are now *promotable*.** The milestone's own outcome — which
the plan explicitly permits — is that **no behavioural cell was promoted**,
because promoting one requires an **operator attestation** that no run in this
environment could supply, and because the Linux row has no recorded run at all.

What changed, concretely:

- The matrix's **root causes were corrected**. The claim that the GUI has "no
  `export` reference anywhere in `crates/eggsec-gui/frontend/app.js`" was
  false — the frontend carries ~50, including `"export_report"` at
  `crates/eggsec-gui/frontend/app.js:1624` and `"report_for"` at `:556`. The
  export columns now name the *real* blocker (an attestation, not a feature),
  and the retired milestone-006 blocking token is gone from the document
  entirely.
- The matrix's **promotion rule is now machine-enforced** and lives in exactly
  one place: the matrix's own §6 rules table (P1–P6).
  `scripts/check_gui_qualification.py` reads the rules from there, so a rule
  added to the matrix is enforced without a second edit.
- The **export procedure is written** (`docs/GUI_BUILD.md` §5.7): the three
  `*Not yet defined.*` placeholders are gone, replaced by a step that names
  the panel's control, the `export_report` command, the six formats, the byte
  and digest checks, the `.bak` backup, and the two refusal cases.
- The **harness measures the GUI's real paths** against a live daemon:
  connect, submit, cancel, render, export — with the loopback fixture as the
  only target — and records a per-column outcome with a named reason.
- **One cell changed status**: the macOS `Build` cell, which now cites a
  `eggsec-gui-qualification-record/v1` run record whose build sidecar's
  SHA-256 the validator re-checks. Its status was already `qualified`; what
  changed is that its evidence is now a *record* rather than a prose
  reference. The macOS `Launch` cell stays `partially qualified`; the
  behavioural cells stay `not qualified`; the Windows row is byte-identical.

The milestone is **conditionally closed**: the infrastructure is complete and
verified, and the named outstanding evidence is an operator-attested run
(§10).

## 2. Requirement-to-evidence matrix

| Requirement | Evidence | Result | Notes |
|---|---|---|---|
| §13.1 — `docs/GUI_BUILD.md` §5.7 contains a complete export procedure (four filled rows, no `Not yet defined`, naming the control, the `export_report` command, the six formats, the digest and parse checks, the `.bak` backup, the refusal cases) | `docs/GUI_BUILD.md:480-513`; guard 181 (passes, and fails on a negative fixture) | **pass** | The four rows are Precondition, Action, Pass criterion, Evidence artefact; the six formats are named `json`, `csv`, `html`, `md`, `sarif`, `xml`. |
| §13.2 — `scripts/gui_qualification.sh` exists, is hermetic, and writes a run record with all seven columns | `scripts/gui_qualification.sh`; the recorded run `docs/evidence/gui-qualification/macos-2026-10-24.md`; guard 187 | **pass** | The record carries all seven columns. Hermeticity: no `sudo`, no recursive remove, no absolute path literal outside the temporary root. |
| §13.3 — `crates/eggsec-gui/tests/qualification.rs` exists, adds **no** dependency to `crates/eggsec-gui`, SKIPs with a named reason when its environment is absent, and asserts the seven columns | `crates/eggsec-gui/tests/qualification.rs`; `cargo test -p eggsec-gui --no-default-features --test qualification` (5 passed); `git diff crates/eggsec-gui/Cargo.toml` (empty) | **pass** | The SKIP line is printed: `SKIP: neither EGGSEC_QUALIFY_SOCKET nor EGGSEC_QUALIFY_DAEMON_BIN is set, so no daemon was reachable and the live half did not run`. |
| §13.4 — `scripts/check_gui_qualification.py` exists and enforces rules P1–P6 | `scripts/check_gui_qualification.py`; `python3 scripts/check_gui_qualification.py` (exit 0); `--self-test` (9 deliberate corruptions, all rejected) | **pass** | The rules are read from the matrix's §6 table, not from a second copy in the script. |
| §13.5 — the `gui-platform` CI job runs the procedure on both legs and publishes the run record as a job artefact | `.github/workflows/deep-checks.yml:144-176`; guard 189 | **pass** (not yet run) | The job builds the `service`-profile daemon, runs the procedure, and uploads `gui-qualification-${{ matrix.os }}`. The job has **not run in CI yet** — no run URL exists (§10). |
| §13.6 — the matrix contains no `pending M6` token, no blank cell, and no `qualified` cell without a resolving evidence reference | `docs/GUI_PLATFORM_QUALIFICATION.md`; guards 182, 183, 184; the validator (exit 0) | **pass** | The token is absent from the document; the vocabulary is four tokens; all 21 cells are non-blank; the one `qualified` cell resolves. |
| §13.7 — every behavioural cell's root cause names the **current** blocker; the macOS `Connect` cell no longer says "the repository ships no such driver" | `docs/GUI_PLATFORM_QUALIFICATION.md` (the macOS row) | **pass** | The behavioural cells now name the *operator attestation* that rules P1/P3/P4 require, and cite the record that carries the machine half. |
| §13.8 — guards 181–189 exist, are numbered after 176, and each fails when its invariant is broken | `/tmp/eggsec-guardwork/m014-guards.sh`; §4.1 | **pass** | Nine checks, 181–189. Each was demonstrated to FAIL against a deliberately broken input and the tree was restored from `cp` backups. The block is **not yet merged** into `scripts/check-architecture-guards.sh` (the owner merges it). |
| §13.9 — no cell is promoted without a recorded run satisfying P1–P6 | the matrix; the validator | **pass** | The one `qualified` cell cites `macos-2026-10-24.md`, whose record satisfies P2 (scripted `pass`, operator `not-applicable`, artefact present, digest matching). No behavioural cell was promoted. |
| §13.10 — the Windows row is byte-identical to its baseline state | `docs/GUI_PLATFORM_QUALIFICATION.md` | **pass** | The row's text is unchanged; the re-anchoring was applied to §5's `server.rs` list, not to the row. |
| §10 — the run-record schema round-trips and a missing column is rejected by name | `the_outcome_file_round_trips_and_names_what_is_wrong_with_it` | **pass** | The deserialiser rejects an unknown `schema` by name; `validate()` names the missing column. |
| §10 — the seven identifiers are the seven columns | `the_column_identifiers_are_the_seven_the_matrix_publishes`; guard 186 | **pass** | |
| §10 — the format tables agree with the record | `crates/eggsec-gui/src/export.rs`'s new `the_format_table_agrees_with_the_views_and_the_written_files`; `every_report_format_renders_writes_and_digests`; the export column's artefact | **pass** | `ReportFormat::parse` accepts every name the frontend's `format_views()` offers, and the file the writer produces carries the suffix the table names, for all six formats; and all six render, write, and digest to the value the record carries. |
| §10 — Tier A export refuses a non-regular destination and backs up an existing file | `the_export_refuses_a_non_regular_destination_and_backs_up_a_real_one` | **pass** | A directory is refused with the named discipline; an existing file survives at `<path>.bak`. |
| §10 — Tier B connect / submit / cancel / render / export against a live daemon | `the_live_daemon_half_records_its_outcomes`, run against a `service`-profile daemon | **pass** | Every one of the five columns recorded `scripted: pass`; see the record's reasons. |
| §10 — the daemon restarts between the task and the export | the harness's report-fetch path | **partial** | The path is implemented and reachable (the daemon's own refusal is carried verbatim and the export column becomes `fail`), but the restart itself was **not exercised** in a test — no test can restart the daemon the procedure started. Recorded in §10. |
| §10 — the script's cleanup leaves nothing behind | the procedure's `cleanup` trap; the daemon log | **pass** | The trap stops the daemon and removes its socket on EXIT, INT and TERM. The run's daemon log shows `Daemon stopped.`; the socket is gone. |
| §10 — two concurrent harness runs do not interfere | the procedure's `$$`-derived socket path | **partial** | The socket path is unique per run, and two runs were made against two distinct sockets during this milestone's work (one macOS, one `--platform linux`). A simultaneous two-run demonstration was **not** performed. |
| §10 — the validator round-trips (green on a good matrix, non-zero with a named cell on a tampered one) | `python3 scripts/check_gui_qualification.py --self-test` | **pass** | Nine deliberate corruptions, each rejected with the named cell. |
| §10 — no enforcement bypass; the harness adds no network reach; no credentials are logged | §8 below | **pass** | |
| §11 — the matrix's status vocabulary is unchanged and `pending M6` is rejected | the validator; guard 182 | **partial** | The **matrix's** vocabulary is now four tokens: `pending M6` is removed by guard 182 once the affordance exists, exactly as the plan's §9 specifies. The *guard 168* list in `scripts/check-architecture-guards.sh` still carries `pending M6`; that file is shared and outside this milestone's range, and the divergence is recorded in the guard-doc delta. |
| §11 — the old evidence file still validates but is not sufficient to promote a cell | the validator's `check_record` | **pass** | `macos-2026-10-23.md` carries no record block; it is not rejected as malformed, and a `qualified` cell resting on it alone is rejected as insufficient. The macOS `Build` cell therefore cites the **new** record. |
| §11 — `make check` | not run | **not run** | The plan assigns the `check-deps` advisory backlog to milestone 015, and the session owner runs `make check` at integration. |
| §11 — `make check-deps` | not run | **not run** | Same reason. |
| §11 — `make check-python` | not run | **not run** | M014 changes no Python binding, stub, or script. |

## 3. Production implementation evidence

### 3.1 The procedure — `scripts/gui_qualification.sh` (new)

The operator-facing procedure. It:

- takes `--platform`, `--mode`, `--socket`, `--daemon-bin`, `--out`, `--date`,
  and `--skip-window`;
- runs `scripts/check_gui_prereqs.sh` and records its exit status and verdict;
- builds the shell through `scripts/build_gui_release.sh` and copies the build
  sidecar into the evidence directory under
  `<platform>-<date>.build.json`, with its SHA-256;
- starts a `service`-profile daemon on a `$$`-derived socket with
  `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` in its environment, or attaches to one the
  operator started;
- runs the harness and reads its per-column outcome file;
- probes for the window (`xdotool` on Linux, `osascript` on macOS) and takes a
  screenshot **only when a GUI process is running** — a full-screen capture of
  a host with no GUI is a picture of the operator's desktop, not evidence;
- writes the run record and the artefacts into `--out`;
- stops the daemon and removes its socket on every exit path (EXIT, INT, TERM).

**A run qualifies the platform it was made on.** If the requested platform is
not the host's, the procedure records a named skip for every column rather
than putting the host's observations under another platform's label. That is
why `bash scripts/gui_qualification.sh --platform linux --mode ci` on a macOS
host produces seven named skips and exits 0.

### 3.2 The harness — `crates/eggsec-gui/tests/qualification.rs` (new)

Five tests, no new dependency, no new file under `crates/eggsec-gui/src/`:

| Test | What it proves |
|---|---|
| `the_column_identifiers_are_the_seven_the_matrix_publishes` | The harness's identifier set is the matrix's seven, once each. |
| `the_outcome_file_round_trips_and_names_what_is_wrong_with_it` | A record round-trips; an unknown `schema` is rejected by name; a record missing a column is rejected **by the column's name**. |
| `every_report_format_renders_writes_and_digests` | All six formats render, write through the GUI's own writer, and the written file's SHA-256 equals the rendered bytes' digest. |
| `the_export_refuses_a_non_regular_destination_and_backs_up_a_real_one` | The write discipline: a directory is refused with a named error and nothing is written; an existing regular file survives at `<path>.bak`. |
| `the_live_daemon_half_records_its_outcomes` | The live half, or a named SKIP. |

The live half: connects through the GUI's **own** client and identity
(`gui_identity()`), reads the daemon's catalogue and capabilities, opens a
`DesktopGuiManual` session with the GUI's own scope, assembles the request
through the GUI's **own** form engine (`form::assemble_request`), submits,
cancels, observes the terminal state **in the daemon's event stream**, fetches
the report, renders it, and writes it in all six formats, comparing each
written file's bytes with the renderer's output.

Two design points worth naming:

- **The cancellation's outcome is the terminal state the event stream
  reported**, never the cancel request's acknowledgement. A task that
  completes before the cancellation is processed is recorded as a *completed
  task*, not as a failed cancellation — the plan's §8.2 rule.
- **The target is the loopback fixture** (`127.0.0.1`), and the harness
  supplies a target only to a task kind whose published schema declares one.
  The engine admits a loopback target only when
  `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` is set in the daemon's environment
  (`crates/eggsec/src/utils/parsing.rs:77`), which the procedure sets. The
  run therefore reaches nothing outside the host.

### 3.3 The validator — `scripts/check_gui_qualification.py` (new)

- **Reads** the matrix's status vocabulary (§1), the matrix's column set
  (the header row), and the promotion rules (§6's rules table: P1–P6, each
  with a kind, its platforms, its columns, its required outcome, its required
  attestation, and its artefact policy). It keeps **no** second copy.
- **Rejects** a cell that is not `**status** — explanation`; an unknown status
  token; a blank explanation; a `partially qualified` cell that does not name
  both the qualified subset and the missing part; a `qualified` cell that
  names no evidence, no date, or an evidence file that does not exist, or a CI
  run URL that `CI-RUNS.md` does not carry; a `qualified` cell whose record
  lacks the column, carries the wrong scripted or operator outcome, names an
  artefact that does not exist, or records a digest that does not match; a
  record whose `schema` is not `v1`; a Windows cell that is not
  `out of scope` or does not cite the ADR.
- **Writes** the run record as well as reading it, because it is the thing
  that reads the record: the record's shape therefore has exactly one
  definition in the repository. The producer script hands it a bag of facts
  and asks for the record back.
- **Proves itself** with `--self-test`: nine deliberate corruptions of a
  known-good matrix and record, each of which it must reject.

### 3.4 The run record

`docs/evidence/gui-qualification/macos-2026-10-24.md` is the milestone's
recorded run, produced by the procedure on this host on 2026-10-24. Its
per-column outcomes:

| Column | `scripted` | `operator` | Artefact |
|---|---|---|---|
| `build` | `pass` | `not-applicable` | `macos-2026-10-24.build.json` (SHA-256 recorded) |
| `launch` | `not-performed` | `not-performed` | — |
| `connect` | `pass` | `not-performed` | — |
| `submit` | `pass` | `not-performed` | — |
| `cancel` | `pass` | `not-performed` | — |
| `render` | `pass` | `not-performed` | — |
| `export` | `pass` | `not-performed` | `macos-2026-10-24.export.json` (SHA-256 recorded) |

The five other exported formats (`csv`, `html`, `md`, `sarif`, `xml`) are
committed beside it. The `launch` column is `not-performed` because **no
`eggsec-gui` process was running** when the probe ran: the launch step is an
operator action, and the procedure does not pretend to have taken it.

### 3.5 The matrix

`docs/GUI_PLATFORM_QUALIFICATION.md`:

- the retired milestone-006 blocking token is **gone** (guard 182 makes its
  return an error);
- the **Last regenerated** header names `2026-10-24` and the baseline commit
  `0f5dbb26e7ab215613e5b6e2295d9c124374bf9f`;
- §2.1 now says what is true: the Linux row has no recorded run; the macOS row
  is qualified only where there is an artefact; the behavioural columns are
  **measured** and are missing an operator attestation; the export column is
  blocked on an attestation, not on a feature;
- §3's regeneration procedure names the script and the validator;
- §4.1 lists the artefacts and `CI-RUNS.md`;
- §5's `server.rs` anchor is re-pointed from `:95` to `:147`;
- §6 is **the promotion rules table** P1–P6, in a form the validator parses.

### 3.6 The runbook — `docs/GUI_BUILD.md`

- §2.2 gains the note that the repository's own procedure does **not** use the
  webdriver tooling it names, and why (`xdotool` *is* used, as the probe);
- §5.0's daemon-lifecycle paragraph is corrected: the GUI never manages the
  daemon's lifecycle, and that is an architectural boundary, not an open
  milestone;
- §5.0's anchors are re-anchored: `config.rs:56` (the `--socket` flag),
  `config.rs:19` (the default), the **six** production sites spelled out
  (the webview spells it out twice), and `server.rs:524-527` (the scope);
- §5.3, §5.4 and §5.5's frontend anchors are re-anchored to the current
  `app.js` (`:1682`, `:1006`, `:1525`, `:1538`) and the event-stream
  authority to `commands.rs:572-573`;
- §5.7 is the export procedure (four filled rows);
- §6 names the export artefact and the validator.

### 3.7 The CI job — `.github/workflows/deep-checks.yml`

The `gui-platform` job gains a `platform` matrix label, a step that builds the
`service`-profile daemon, a step that runs the procedure, and a step that
publishes the run record as the `gui-qualification-${{ matrix.os }}` artefact.
The upload step runs `if: always()`, because **a failed run's record is the
evidence of the failure**.

### 3.8 The guards

`/tmp/eggsec-guardwork/m014-guards.sh` holds the nine checks, 181–189, in
the shared script's house style. They are **not yet merged** into
`scripts/check-architecture-guards.sh`: that file is shared by four
concurrent agents, and the session owner merges the block before the summary
block.

### 3.9 Measured counts

| Fact | Value | Measured by |
|---|---|---|
| Columns in the matrix | 7 | guard 184 |
| Cells in the matrix | 21 | guard 184 |
| Platform rows | 3 | guard 184 |
| Status tokens in the matrix's vocabulary | 4 | `docs/GUI_PLATFORM_QUALIFICATION.md` §1 |
| Export formats the GUI offers | 6 | `every_report_format_renders_writes_and_digests` |
| Production sites for the socket default | 6 | `docs/GUI_BUILD.md` §5.0 |
| Guards in this milestone | 9 (181–189) | the guard block |
| New dependencies | 0 | `git diff crates/eggsec-gui/Cargo.toml` is empty |

### 3.10 One file outside this milestone's scope was touched by a formatter

`cargo fmt -p eggsec-gui` formats the whole package, so it also visited the
files milestone 012 and milestone 013 are editing in this tree. It changed
**two lines** in `crates/eggsec-gui/src/config.rs` — a re-indentation of two
`assert_eq!` lines inside milestone 012's own new test — and nothing else.
`git diff -w` against `git diff` shows the difference is exactly those two
lines and no behaviour change. The tree moved **towards** the required
`cargo fmt --all --check` state, but it is an edit to a file this milestone
does not own, and it is recorded here rather than left for the owner to
discover. No other file outside this milestone's ownership was written.

## 4. Verification executed

### Commands run

```bash
cargo test -p eggsec-gui --features shell --lib
cargo test -p eggsec-gui --no-default-features --lib
cargo test -p eggsec-gui --no-default-features --test qualification
cargo test -p eggsec-gui --no-default-features --test qualification -- --nocapture
cargo clippy -p eggsec-gui --no-default-features --tests -- -D warnings
cargo clippy -p eggsec-gui --features shell --tests -- -D warnings
cargo fmt -p eggsec-gui -- --check
bash scripts/gui_qualification.sh --platform macos --mode ci --daemon-bin <service-profile daemon> --out docs/evidence/gui-qualification
bash scripts/gui_qualification.sh --platform linux --mode ci --out /tmp/eggsec-qualify-linux
python3 scripts/check_gui_qualification.py
python3 scripts/check_gui_qualification.py --self-test
python3 scripts/check_doc_references.py
make check-gui
make check-gui-prereqs
make gui-release
bash /tmp/eggsec-guardwork/run-m014.sh          # this milestone's guard block
bash /tmp/eggsec-guardwork/m014-negative.sh     # the nine negative demonstrations
```

### Results

| Command | Exit | Observed |
|---|---|---|
| `cargo test -p eggsec-gui --features shell --lib` | 0 | **93 passed; 0 failed** |
| `cargo test -p eggsec-gui --no-default-features --lib` | 0 | **83 passed; 0 failed** |
| `cargo test -p eggsec-gui --no-default-features --test qualification` | 0 | **5 passed; 0 failed** |
| the same, `-- --nocapture`, no daemon | 0 | `SKIP: neither EGGSEC_QUALIFY_SOCKET nor EGGSEC_QUALIFY_DAEMON_BIN is set, so no daemon was reachable and the live half did not run` |
| `cargo clippy -p eggsec-gui --no-default-features --tests -- -D warnings` | 0 | clean |
| `cargo clippy -p eggsec-gui --features shell --tests -- -D warnings` | 0 | clean |
| `cargo fmt -p eggsec-gui -- --check` | 0 | clean |
| `bash scripts/gui_qualification.sh --platform macos --mode ci …` | 0 | the record was written; every column's scripted outcome is `pass`, `skipped`, or `not-applicable` |
| `bash scripts/gui_qualification.sh --platform linux --mode ci …` | 0 | seven named skips (the host is macOS, not linux); the daemon half and the harness did not run |
| `python3 scripts/check_gui_qualification.py` | 0 | `ok: …/docs/GUI_PLATFORM_QUALIFICATION.md is consistent with its evidence` |
| `python3 scripts/check_gui_qualification.py --self-test` | 0 | nine deliberate corruptions, each rejected with the named cell; `ok: every deliberate corruption was rejected` |
| `python3 scripts/check_doc_references.py` | 1 | **29 missing references — all pre-existing**, in `architecture/nse_integration.md`, `architecture/overview.md`, `architecture/proxy.md`, `architecture/transport_eggfetch.md`, `architecture/capability_segregation.md`, `architecture/fuzzer.md`, `architecture/knowledge_corpus.md`, `architecture/network_dependency_baseline.md`, `architecture/nse_capability_inventory.md`, `docs/CAPABILITIES.md`, `docs/CI_ARCHITECTURE_GUARDS.md`, `docs/NSE_COMPATIBILITY.md`. None is in a file this milestone touched. The first run of this checker during the milestone reported **30**: the extra one was the milestone's own record, whose embedded cargo transcript contained the line `Running tests/qualification.rs`, which the checker read as a file reference. The record writer now trims the transcript to the observations, and the count is back to the pre-milestone baseline of 29. |
| `make check-gui` | 0 | `cargo check -p eggsec-gui --features shell` and `cargo clippy -p eggsec-gui --features shell -- -D warnings` both clean |
| `make check-gui-prereqs` | 0 | `VERDICT: BUILDABLE - the desktop GUI shell profile can be built on this host.` |
| `make gui-release` | **2** | **Failed for a pre-existing reason**: `FAIL: the working tree is dirty; a release build must come from a pinned commit.` The tree carries this milestone's changes and three other agents' concurrent changes, and the session owner commits. The `--allow-dirty` path (which the procedure uses) succeeds. |
| `make check` | **not run** | The plan assigns the inherited `check-deps` advisory backlog to milestone 015, and the session owner runs `make check` at integration. |
| `make check-deps` | **not run** | Same reason. |
| `make check-python` | **not run** | M014 changes no Python surface. |

### 4.1 Guard negative tests

Each check was demonstrated to **FAIL** against a deliberately broken input.
The tree was restored from `cp` backups taken immediately before each break —
**never** with `git checkout --`. The transcript is
`/tmp/eggsec-guardwork/m014-negative.sh`; the exact observed output:

```
--- Check 181: the export procedure is written ---
FAIL: docs/GUI_BUILD.md §5.7 still says 'Not yet defined'.
6:| **Action** | *Not yet defined.* | The frontend invokes the `export_report` IPC command …

--- Check 182: the retired milestone token stays retired ---
FAIL: docs/GUI_PLATFORM_QUALIFICATION.md still carries the retired 'pending M6' token, but the export affordance exists.
33:| `pending M6` | The behaviour cannot exist yet. | The cell names the root cause. |

--- Check 183: a qualified cell names resolvable evidence ---
FAIL: docs/GUI_PLATFORM_QUALIFICATION.md:67 cites `macos-1999-01-01.md`, which does not exist.

--- Check 184: the matrix shape is exact ---
FAIL: docs/GUI_PLATFORM_QUALIFICATION.md the `windows` row has 6 cells; it must have 7.

--- Check 185: the runbook's steps are the matrix's columns ---
FAIL: docs/GUI_BUILD.md §5's steps are ['build', 'launch', 'connect', 'submit', 'cancel', 'render', 'exfiltration']; the matrix's columns are ['build', 'launch', 'connect', 'submit', 'cancel', 'render', 'export'].

--- Check 186: the harness's identifiers and record schema are pinned ---
FAIL: crates/eggsec-gui/tests/qualification.rs's identifiers are ['build', 'launch', 'connect', 'submit', 'cancel', 'render', 'exfiltrate']; the matrix's columns are ['build', 'launch', 'connect', 'submit', 'cancel', 'render', 'export'].

--- Check 187: the qualification procedure is hermetic ---
FAIL: scripts/gui_qualification.sh elevates its privileges.
33:sudo -n true 2>/dev/null || true

--- Check 188: the promotion validator passes ---
FAIL: the `windows` row's `Build` cell carries the status `qualified`, but rule P5 requires `out of scope`

--- Check 189: the CI job publishes the qualification record ---
FAIL: the gui-platform job does not publish a gui-qualification artefact.
```

And after the restoration:

```
The tree is restored; re-running the block:
ALL NINE GUARDS PASS AGAIN
```

The nine checks, green:

```
--- Check 181: the export procedure is written ---
PASS: the export procedure names its action, its criterion, its artefact, and its six formats.
--- Check 182: the retired milestone token stays retired ---
PASS: the matrix carries no retired blocking token.
--- Check 183: a qualified cell names resolvable evidence ---
PASS: every qualified cell names evidence that resolves.
--- Check 184: the matrix shape is exact ---
PASS: the matrix has the exact header, three rows, and twenty-one cells.
--- Check 185: the runbook's steps are the matrix's columns ---
PASS: the runbook's seven steps are the matrix's seven columns, in order.
--- Check 186: the harness's identifiers and record schema are pinned ---
PASS: the harness's identifiers are the matrix's columns, and every writer and reader carries the same record schema.
--- Check 187: the qualification procedure is hermetic ---
PASS: the procedure needs no privileges and writes only under its output and the temporary root.
--- Check 188: the promotion validator passes ---
PASS: the promotion validator is green.
--- Check 189: the CI job publishes the qualification record ---
PASS: the gui-platform job runs the procedure and publishes the run record.
```

### 4.2 The end-to-end proof

`bash scripts/gui_qualification.sh --platform macos --mode ci --daemon-bin <service-profile daemon> --out docs/evidence/gui-qualification`
exited 0 and produced a record whose `connect`, `submit`, `cancel`, `render`
and `export` columns all carry `scripted: pass` against a **real
`service`-profile daemon**, with the reasons recorded verbatim:

- `connect` — "the GUI's own client connected to the daemon and read its catalogue";
- `submit` — "the GUI's own form engine assembled the `PortScan` request and the daemon accepted it as task task:…";
- `cancel` — "the daemon accepted the cancellation and the event stream reported `cancelled`";
- `render` — "the daemon's report for the task rendered as 0 findings and 1 open ports against `127.0.0.1`";
- `export` — "the GUI's own export path wrote the daemon's report in 6 formats, and each written file's bytes equal the renderer's output".



### Integration verification (2026-10-24, session owner)

The milestone's own commands were run in isolation; the owner's integration
step then ran the repository's mandatory contract over the whole tree, with
all four milestones landed together.

| Command | Result |
|---|---|
| `make check` | **exit 0** — `cargo fmt --all --check`, the no-default-features and per-crate checks, `cargo deny --workspace --all-features check` (all five sub-checks), clippy, the full test suite (**4315 tests across 77 test binaries, 0 failures**), and `scripts/check-architecture-guards.sh` (**190 guards, "ALL PASSED: No architecture drift detected"**) |
| `make check-msrv` | **exit 0** — all five steps green on Rust 1.89 |
| `python3 scripts/check_doc_references.py` | 29 missing references — the unchanged pre-existing baseline, none of them in a file this milestone touched |

This is the **first time `make check` has completed on this branch**: it had
always stopped at the inherited `check-deps` advisory backlog, which milestone
015 cleared. Three integration tests that milestone 010 had left stale were
found by that first complete run and corrected in `dcfcd4e2`; the corrections
are recorded as a dated addendum on milestone 010's closure record rather
than by rewriting it.

## 5. Invariant review

| Invariant | Evidence |
|---|---|
| `EnforcementContext::evaluate()` remains the only pre-dispatch gate | The harness **changes no** enforcement, authorisation, scope, or dispatch path. It submits through the daemon's own `submit` path and reads the daemon's own refusal when the daemon refuses. The one observed refusal during development — the daemon answering `unsupported task kind for this runtime` — was the daemon's own admission control, surfaced unchanged. |
| Strict surfaces fail closed | No strict surface was touched. The daemon opened the session under `RuntimeSurface::DesktopGuiManual` with the daemon's own `gui-default` scope; the harness neither widened nor bypassed it. |
| Scope is the daemon's, not the GUI's | The record carries `daemon.scope = {surface: DesktopGuiManual, source: gui-default}`, taken from the GUI's own `gui_session_scope()`. The harness sends no scope of its own. |
| One canonical source per fact | The **promotion rule** lives only in the matrix's §6 rules table, and the validator reads it from there. The **record schema** string lives in one definition (the validator's writer) and the four copies are pinned by guard 186. The **column set** lives in the matrix's header row; the harness's identifiers and the runbook's steps are checked against it by guards 185 and 186. The **status vocabulary** lives in the matrix's §1 and the validator reads it from there. |
| No `let _ =`, no `filter_map(|e| e.ok())` | The Rust harness uses pattern matching throughout; the shell script uses `|| true` only on commands whose exit status is checked separately, and every fallible command's status is captured. |
| Ring-only TLS | No TLS was added; no dependency was added. |
| Spawned tokio tasks carry 30–300 s timeouts | The harness's waits are bounded: `CONNECT_WAIT` 30 s, `TERMINAL_WAIT` 60 s — both inside the band. The harness spawns no detached task. The daemon the procedure starts is terminated by the cleanup trap, which SIGTERMs, waits five seconds, and then SIGKILLs. |
| Workspace Tokio is `default-features = false` with per-crate features | The harness uses the GUI crate's existing Tokio features (`rt`, `time`, `macros`); it adds no Tokio feature and no dependency. |
| DTO crates carry no Tokio | No DTO crate was touched. |
| The GUI links no engine/daemon/CLI/TUI crate (guard 155) | `git diff crates/eggsec-gui/Cargo.toml` is **empty**: the dependency closure is unchanged. The harness uses `eggsec-frontend-client`, `eggsec-daemon-protocol`, `eggsec-runtime`, and `eggsec-output` — all of which the GUI crate already depended on. |
| New guards start at 177; this milestone's range is 181–189 | The nine checks are numbered 181–189, and no number outside the range is used. |
| Never run the suite as root; platform scripts stay hermetic | Every command was run as the ordinary user. The procedure's hermeticity is guard 187. |

## 6. Failure and recovery review

| Case | Behaviour | Evidence |
|---|---|---|
| The daemon is a no-op executor (built without the `service` feature) | The harness **detects** it (the daemon publishes no task kinds) and records a **named skip** naming the cause, rather than reporting a failed submission. The procedure also warns and names the fix. | Observed during development: the daemon's own warning (`this binary was built WITHOUT the 'service' cargo feature`) and the harness's skip reason. |
| The daemon does not answer | The connect attempt is bounded at 30 s and the live columns record a **named skip**. | The harness's `CONNECT_WAIT` path. |
| The cancel loses the race | The terminal state is read from the **event stream**. A task that completes first is recorded as a *completed task*, not a failed cancellation — the plan's §8.2 rule. | The harness's cancel match arms. |
| The daemon restarts between the task and the export | The daemon's own refusal is carried **verbatim** into the render and export columns' reasons, and the export column becomes `fail`. There is no silent success. | The harness's report-fetch path. The restart itself was **not** exercised in a test (§2). |
| The host is not the platform being qualified | The procedure records a **named skip** for every column and starts no daemon. | `bash scripts/gui_qualification.sh --platform linux --mode ci` on this macOS host: seven skips. |
| The prerequisite probe reports NOT BUILDABLE | The build column records a named skip naming the probe's verdict; the procedure continues. | The script's build branch. |
| The window probe's tool is absent | The launch column records a **named skip** naming the absent tool. | The script's probe branch. |
| Two concurrent runs | The socket path is `$$`-derived, so two runs on one host use two sockets and two records. | The script's `OWNED_SOCKET`. Two simultaneous runs were **not** demonstrated (§2). |
| The script is interrupted | The cleanup trap stops the daemon and removes the socket on EXIT, INT and TERM. | The script's `cleanup` trap and `trap cleanup EXIT INT TERM`. |
| A cell's artefact is missing or its digest is wrong | The validator rejects the cell **by name**. | `--self-test`: "a missing artefact is rejected", "a wrong digest is rejected". |

## 7. Migration and compatibility review

| Aspect | Status |
|---|---|
| Schema migration | None. The milestone adds a **new** record format (`eggsec-gui-qualification-record/v1`) and changes no persisted representation. No milestone in this roadmap changes one (`plans/subsystems/desktop-gui-frontend-roadmap.md:1085-1086`). |
| Backward compatibility | The pre-existing evidence file `macos-2026-10-23.md` carries **no** record block. The validator does not reject it as malformed; it simply is not sufficient to promote a cell, and the macOS `Build` cell now cites the new record. |
| Protocol negotiation | Unchanged. The harness speaks the **existing** daemon protocol; no protocol, wire, or DTO changed. `crates/eggsec-daemon-protocol/src/protocol.rs` is not touched by this milestone. |
| Configuration behaviour | The GUI's flags are unchanged; the procedure passes the same `--socket` and `--daemon-bin` flags the operator already uses. |
| Rollback limitations | The milestone's artefacts are additive: a new script, a new test target, a new evidence directory entry, and doc edits. Reverting the commit removes them without touching anything else. The matrix's cells fall back to their previous text only by reverting the doc. |
| Legacy-path status | The retired milestone-006 blocking token is removed from the matrix's vocabulary. Guard 168's **guard-side** token list in the shared script still carries it; that file is outside this milestone's range, and the divergence is documented in the guard-doc delta. |

## 8. Security review

| Aspect | Status |
|---|---|
| Authorisation enforcement | Unchanged and not bypassed. The harness submits through the daemon; a task the daemon refuses is refused for the harness too, and the refusal is recorded as an observation. |
| Scope checks | The harness sends no scope. The daemon opened the session with its own `DesktopGuiManual` / `gui-default` scope. The harness's target is the loopback fixture, which the engine admits only behind `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`. |
| Secret handling | The harness reads no credential and logs none. The record's fields carry no credential material; the procedure records the daemon **binary's** SHA-256 and its path, which are not secrets. |
| Path validation | The harness's temporary directories are built under `${TMPDIR:-/tmp}` with the process id and a nanosecond timestamp, and the export path's own discipline (refuse a non-regular file, back up an existing file) is exercised by a test. |
| Privilege boundaries | The procedure never elevates its privileges and never asks for root (guard 187). It starts and stops its own daemon as the ordinary user. |
| Transport checkpoints | The harness opens exactly **one** socket, a Unix socket, and constructs **no** HTTP client. |
| Denial-of-service bounds | The harness's waits are bounded (30 s connect, 60 s terminal), and the procedure's daemon readiness wait is bounded at 60 s. The port range the harness scans is `1-32` with a 200 ms per-port timeout. |
| Redaction | The record carries the harness's per-column reasons verbatim. No token, credential, or private key appears in any of them. |
| Audit behaviour | Unchanged. The daemon's own audit trail records the dispatches the harness made; the harness does not and cannot write to it. |
| Screenshot privacy | The procedure takes a screenshot **only when a GUI process is running**, so the repository never receives a picture of an operator's desktop with no GUI on it. The first version of the script did capture unconditionally; that was corrected before the record was committed. |

## 9. Documentation and operations

### 9.1 Files this milestone owns and changed

| File | Change |
|---|---|
| `scripts/gui_qualification.sh` | **new** — the operator procedure and the run-record producer. |
| `scripts/check_gui_qualification.py` | **new** — the promotion-rule validator and the record writer. |
| `crates/eggsec-gui/tests/qualification.rs` | **new** — the assertion harness. |
| `crates/eggsec-gui/src/export.rs` | one extended test — `the_format_table_agrees_with_the_views_and_the_written_files` — asserting that `ReportFormat::parse` accepts every name the frontend offers and that the written file carries the suffix the table names, for all six formats. No production code changed. |
| `docs/evidence/gui-qualification/TEMPLATE.md` | the record block, the seven-row outcome table, and the `export` row. |
| `docs/evidence/gui-qualification/CI-RUNS.md` | **new** — the index of CI runs a cell may cite. |
| `docs/evidence/gui-qualification/macos-2026-10-24.md` | **new** — the milestone's recorded run. |
| `docs/evidence/gui-qualification/macos-2026-10-24.*` | **new** — the run's artefacts (the build sidecar and the six exported reports). |
| `docs/GUI_BUILD.md` | §2.2's webdriver note; §5.0's corrections and re-anchoring; §5.3–§5.6's re-anchoring; §5.7's export procedure; §6's artefact names. |
| `docs/GUI_PLATFORM_QUALIFICATION.md` | §1's vocabulary and absolute rules; §2's regeneration header; §2.1; the three rows; §3; §4.1; §5's anchor; §6's rules table P1–P6. |
| `docs/GUI_DISTRIBUTION_HANDOFF.md` | §1's export-artefact note; §4's daemon-lifecycle limitation; the `config.rs:17` → `:19` anchor. |
| `.github/workflows/deep-checks.yml` | the `gui-platform` job's `platform` label, the service-daemon build, the procedure step, and the run-record artefact. |

### 9.2 Shared-document deltas (written to scratch, not applied)

| File | Contents |
|---|---|
| `/tmp/eggsec-guardwork/m014-guards.sh` | the nine checks, 181–189, in the shared script's house style, to be appended **before** the summary block. |
| `/tmp/eggsec-guardwork/m014-guards-doc.md` | the `docs/CI_ARCHITECTURE_GUARDS.md` entries for 181–189, plus the companion edit that drops `pending M6` from the Check 168 bullet's enumeration, and the note about guard 168's guard-side list. |
| `/tmp/eggsec-guardwork/m014-skill.md` | the `.opencode/skills/eggsec-gui/SKILL.md` delta: a **Platform qualification** section, a **Known gaps** bullet, and **Resources** entries. It restates **no** counted set. |
| `/tmp/eggsec-guardwork/m014-desktop_gui.md` | the `architecture/desktop_gui.md` delta: the replacement for the **qualification matrix** section. |

### 9.3 Operator diagnostics

- The procedure prints what it is doing and what it skipped, and it names the
  reason for every skip.
- A daemon that is a no-op executor is detected and named, with the fix:
  `scripts/build-daemon-profile.sh --profile service --out <dir>`.
- The validator names the cell it rejected and the rule it applied.

### 9.4 Recovery instructions

- A failed run's record is still a record: the CI job uploads it with
  `if: always()`.
- To re-run a qualification: `bash scripts/gui_qualification.sh --platform
  <linux|macos> --mode ci --daemon-bin <path>`; the daemon it starts is
  stopped and its socket removed on every exit path.
- To check a matrix change: `python3 scripts/check_gui_qualification.py`
  (and `--self-test` for the validator's own negative cases).

## 10. Unresolved findings

| Severity | Finding | Impact | Required action |
|---|---|---|---|
| medium | **No behavioural cell can be promoted yet**: rules P1/P3/P4 require an `operator` attestation, and no operator-attested run exists. | The `Connect`, `Submit`, `Cancel`, `Render` and `Export` columns stay `not qualified` on macOS and on Linux. The cells are *promotable*; they are not *promoted*. | An operator runs `docs/GUI_BUILD.md` §5 on the platform, witnesses the behaviours, and records the attestation in the run record's `operator` fields. |
| medium | **The Linux row has no recorded run.** The `gui-platform` job's Linux leg has never run. | The Linux row's seven cells stay `not qualified`, and the Linux behavioural cells cannot be promoted by anything this milestone shipped. | Run the `gui-platform` job on `ubuntu-latest` and record the run URL in `docs/evidence/gui-qualification/CI-RUNS.md`. |
| medium | **The Windows row is excluded by decision and was not touched** (`ADR-0010`). | The Windows cells stay `out of scope`. | None for this milestone. Re-entry is gated on a Windows daemon transport with its own ADR. |
| medium | **OQ-2 is unresolved: whether the GUI paints a window under a virtual display on `ubuntu-latest` is unmeasured.** The environment has no `xvfb` (`xvfb-run` is absent) and no Linux host, and the `gui-platform` job installs no `xvfb` and no `xdotool`. The plan forbids weakening a cell's criterion to make it green, so the criterion stands and the Linux `Launch` cell stays unpromoted. | The Linux `Launch` cell cannot be promoted by CI alone as the job is currently configured. | Either add `xvfb` and `xdotool` to the Linux leg and measure, or accept that the Linux `Launch` cell is operator-run only. |
| low | **OQ-3 is unresolved: whether a CI macOS runner has an attachable GUI session is unmeasured.** This host's session type is `Aqua` (`launchctl managername` → `Aqua`; `who` shows a console login), which is a desktop session, not a CI runner's. | The macOS `Launch` cell's operator attestation cannot be obtained from CI as the job stands. | Measure on the runner; until then the macOS `Launch` cell stays `partially qualified`. |
| low | **OQ-4 is partially resolved.** The `osascript` accessibility probe **does** reach System Events on this host and answers — it failed with `Can't get process 1 whose name = "eggsec-gui". Invalid index. (-1719)`, a *process-lookup* error, not an assistive-access permission error — and it exits **1** when no such process is running. The procedure therefore treats a non-zero exit as "zero windows", not as "the probe is broken". Whether the same holds on a CI runner, where the Accessibility permission is almost certainly not granted, is unmeasured. | A CI macOS leg's window probe would report zero windows either way, which is indistinguishable from "the window never painted". | Measure the probe's behaviour on a runner and, if it is permission-blocked, record the probe's own error in the run record so the two cases are distinguishable. |
| low | **The daemon-restart case was not exercised in a test.** The harness's path for it is implemented (the daemon's own refusal is carried verbatim and the export column becomes `fail`), but no test restarts the daemon the procedure started. | The path is asserted by construction, not by execution. | A test that stops the daemon between the submit and the export, once a test harness can own the daemon's lifecycle. |
| low | **Two concurrent procedure runs were not demonstrated.** The socket path is `$$`-derived, and two runs against two distinct sockets were made during this milestone, but not simultaneously. | The non-interference claim rests on the socket path's uniqueness. | A simultaneous two-run demonstration. |
| low | **`make gui-release` failed** in this environment because the working tree is dirty (this milestone's changes plus three other agents' concurrent changes). The `--allow-dirty` path succeeds, and the procedure uses it. | The release-grade build is unverified from a pinned commit in this milestone. | The session owner runs `make gui-release` after committing, on a clean tree. |
| low | **Guard 168's guard-side token list still carries `pending M6`**, while the matrix's vocabulary no longer does. The two lists therefore differ. The token is illegal either way (guard 182 forbids it outright), so no cell can carry it. | A reader comparing the two lists would find a divergence. | Drop the token from guard 168's list when the shared script is next edited. Check 168 is outside this milestone's reserved range. |
| low | **The frontend has no JavaScript test infrastructure.** The webview's own rendering is covered by the operator's attestation and the screenshot alone; the harness exercises the Rust side of the export path. | A frontend regression in the export control would not be caught by the harness. | Out of scope for this milestone: a JavaScript test runner needs an ADR under the zero-npm posture. |

### 10.1 The open questions, measured

| ID | Outcome | How it was measured |
|---|---|---|
| **OQ-1** — Does a Tauri-v2 WebDriver driver exist for the macOS leg that does not require a Node client? | **Answered: no.** `tauri-driver` is a published crate (2.0.5 … 2.1.0, 3.0.0-alpha.1), but its **own README** lists the supported platforms as "Linux via `WebKitWebDriver`", "Windows via Microsoft Edge Driver", and "**[Todo]** macOS via Appium Mac2 Driver (probably)", with the note that the "(probably)" items "haven't been proof-of-concept'd yet". There is therefore **no macOS driver** upstream. The crate is also a WebDriver **server** that a separate WebDriver **client** connects to; the ecosystem's client is WebdriverIO, a Node tool. Nothing in this repository pins or installs either: `Cargo.lock` carries no driver crate, and `tauri-driver` is absent from this host's PATH (`safaridriver`, macOS's own WebDriver, is present at `/System/Cryptexes/App/usr/bin/safaridriver`). | `https://index.crates.io/ta/ur/tauri-driver` and `https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-driver/README.md`; `rg -n "tauri-driver\|webdriver\|wdio\|safaridriver\|geckodriver" Cargo.lock` (no hits); `command -v` for each driver. |
| **OQ-2** — Can the GUI paint a window under `xvfb` on `ubuntu-latest`? | **Unresolved.** No `xvfb` and no `xdotool` on this host (`xvfb-run` and `xdotool` are absent), there is no Linux host, and the `gui-platform` job installs neither. The criterion is left standing. | `command -v xvfb-run xdotool`; the `gui-platform` job's install step. |
| **OQ-3** — Does the CI macOS runner have an attachable GUI session? | **Unresolved.** This host's session is `Aqua` (`launchctl managername` → `Aqua`; `who` shows a console login), which is a desktop session. A CI runner's session type could not be measured from here. | `launchctl managername`; `who`. |
| **OQ-4** — Does the macOS `osascript` accessibility probe work in a CI context? | **Partially answered.** On this host the probe **reaches System Events and answers**: it failed with `Can't get process 1 whose name = "eggsec-gui". Invalid index. (-1719)` — a *process-lookup* error, not an assistive-access permission error — and it **exits 1** when the process does not exist. The procedure therefore reads a non-zero exit as "zero windows". Whether the probe works on a CI runner, where the Accessibility permission is almost certainly not granted, is **unresolved**. | `osascript -e 'tell application "System Events" to tell (first process whose name is "eggsec-gui") to get name of every window'` → exit 1 with the error above. |

**OQ-1's answer does not trigger the plan's §14 stop condition.** That stop
condition fires when a macOS driver *turns out to exist* and is usable
without a Node client. It does not exist upstream, so the milestone's
deliverable boundary — a maintained operator procedure rather than a
window-automation driver — is unchanged and correct.

## 11. Roadmap disposition

**Milestone conditionally closed with named operational evidence outstanding.**

The capability this milestone set out to build — the qualification
procedure, the harness, the validator, the guards, the export procedure, and
the corrected matrix — **is complete and verified**. The matrix's behavioural
and export cells are **promotable**: the rules that promote them exist, are
machine-enforced, and the machinery to produce a promotable record works end
to end against a live daemon.

The milestone does **not** promote a cell, and it does not claim to. Two
named pieces of evidence are outstanding:

1. an **operator attestation** for the behavioural columns (rules P1, P3, P4),
   which only a person who watched the behaviour can supply; and
2. a **recorded Linux run** from the `gui-platform` job.

The plan explicitly permits this outcome: *"the cells are now promotable but
not promoted because no recorded run exists in this environment"*. Nothing
in the matrix was weakened to reach it: every criterion stands as written,
every skip is named, and the one cell that is `qualified` satisfies its rule.

Milestone 015 (the dependency advisory backlog) is unaffected and remains the
owner of the inherited `check-deps` advisories. The next dependency —
milestone 015's own work — may proceed.

## 12. Registry updates

The session owner owns these files; this milestone does not edit them.

- `plans/registry.md` — the milestone's entry: **conditionally closed**, with
  the two outstanding evidence items named (§10).
- `plans/subsystems/desktop-gui-frontend-roadmap.md` — the milestone's status
  row, and the §4 gap-table row that carries the qualification-matrix gap.

### 12.1 Merges the owner must apply

1. **`scripts/check-architecture-guards.sh`** — append
   `/tmp/eggsec-guardwork/m014-guards.sh` **before** the `=== Summary ===`
   block. The block uses `FAIL` and `SECTION_FAIL`, the script's own
   variables, and `rg`, exactly as the existing checks do.
2. **`docs/CI_ARCHITECTURE_GUARDS.md`** — apply
   `/tmp/eggsec-guardwork/m014-guards-doc.md`.
3. **`.opencode/skills/eggsec-gui/SKILL.md`** — apply
   `/tmp/eggsec-guardwork/m014-skill.md` (shared with milestone 013).
4. **`architecture/desktop_gui.md`** — apply
   `/tmp/eggsec-guardwork/m014-desktop_gui.md` (shared with milestone 013).

## Addendum 2026-10-24 — the operator's attestation, and the macOS promotion

This milestone's own record said the behavioural cells were **promotable, not
promoted**, because rule P1 requires an `operator` attestation and nothing
existed to record one. That gap is now closed, and the macOS cells are
promoted on the evidence of a run the operator attested to.

### What was missing, and what it was

The harness emitted `operator: not-performed` at every one of its sites, and
`OperatorOutcome::Pass` occurred zero times. The validator's writer hardcoded
the operator outcome, and the only documented route to a promotion was a
**hand edit of the record's json block** — which a later run silently
overwrites. So the promotion path existed on paper and in no tooling: the
cells were promotable by a human with a text editor, and by nothing else.

### What was added

- An **attestation file** of the operator's own, schema
  `eggsec-gui-operator-attestation/v1`: who observed the behaviours, when, on
  which platform, against which commit, and what they saw for each column.
- `--attestation` on `scripts/gui_qualification.sh`, which validates the file
  before the run and merges it into the record **in the same write that
  produces the record** — so the record has one writer and a later run cannot
  erase the attestation.
- The validator now refuses an operator outcome of `pass` that **no named
  human signed**, that carries **no observation**, or whose attestation names
  another platform or a commit the run does not descend from; and it refuses
  an attestation that claims a column passed whose scripted outcome was not
  `pass`.
- `--run-label`, so a day with two runs keeps two records instead of one
  overwriting the other.
- **Guard 191**, which pins the three facts that make the machinery real (the
  procedure accepts an attestation and passes it to the writer; the validator
  knows the schema and refuses an unattributed `pass`; no code path defaults
  an operator outcome to `pass`) and the two that keep it honest (the harness
  never self-attests; the matrix's own rule P1 still requires the operator's
  `pass`). It was negative-tested against five injected violations — the
  procedure losing the flag, the flag not reaching the writer, P1 weakened to
  `not-applicable`, the harness emitting a pass, and the validator defaulting
  an outcome to `pass` — and it caught every one of them.

### Three defects the promotion found

Running the procedure for real on a host with a live GUI window found three
defects that the milestone's own green runs could not have found, because
they all need a window that is actually there.

1. **The launch column could never pass.** The status chain tested
   `LAUNCH_STATUS == "not-performed"` first, and the status starts life as
   `not-performed`, so every run took the failure branch however many windows
   the probe had found. A record that said "the probe found 1 window(s) titled
   `Eggsec`" was still filed as `fail`. The count decides now.
2. **The launch screenshot was a photograph of the operator's desktop.** On
   macOS the procedure captured the whole screen, so the artefact it proposed
   to commit was whatever else was open. The capture is now of the GUI's own
   window rectangle, taken from the window's accessibility geometry.
3. **The Linux prerequisite probe could not see the appindicator module.**
   Debian and Ubuntu name the pkg-config module `ayatana-appindicator3-0.1`,
   without the `lib` prefix the library keeps everywhere. The probe knew only
   the other spelling, so it reported a distribution that has the library as
   one that does not, and the Linux leg failed its own probe before it could
   produce a record.

### What was observed, and what is promoted

The operator ran the procedure in operator mode against a live `service`-profile
daemon on this macOS host, with the GUI's window open, and attested to what
they saw: the window painted and titled `Eggsec`; the badge reading
`CONNECTED` with the daemon's published catalogue (OPERATIONS 35, TASK KINDS
30, CAPABILITIES 30) and a session id; the schema-driven form assembling a
request the daemon accepted; the cancel control reaching the daemon and the
daemon's answer surfaced verbatim; the report rendering as data; and the
export writing the daemon's report to the chosen destination. The run's
record is `docs/evidence/gui-qualification/macos-2026-10-24-operator.md`, the
attestation is `docs/evidence/gui-qualification/macos-2026-10-24-operator.attestation.json`,
and the matrix's macOS `Launch`, `Connect`, `Submit`, `Cancel`, `Render` and
`Export` cells are now `qualified`.

### What is still not promoted, and why

- **The Linux behavioural cells stay `not qualified`.** A CI run cannot
  attest: the `gui-platform` job has nobody to observe with, so every column
  it produces carries `operator: not-performed`, and rule P1 asks for `pass`.
  Promoting the Linux behavioural cells needs an operator at a Linux desktop.
  The `gui-platform` job has now run on both legs (run
  `https://github.com/eggstack/eggsec/actions/runs/40143483040`, at commit
  `d0cf5bc3`), and the Linux record carries the scripted outcome `pass` for
  every behavioural column — the harness measured them on Linux. The Linux
  **Build** cell is qualified on that run, with its build sidecar committed
  beside the record so its digest is re-checked rather than merely cited.
- **OQ-2 and OQ-3 remain open** as they were: whether the GUI paints a window
  under a virtual display on `ubuntu-latest` is still unmeasured, and whether
  a CI macOS runner has an attachable GUI session is still unmeasured. The
  macOS `Launch` cell no longer waits on OQ-3, because the attestation it
  needed was obtained on a desktop session.
- **OQ-4 is now partly answered by observation.** The `osascript` probe
  reaches System Events on a desktop session and answers, and it had to be
  made to bring the application forward first: a window that is not on the
  current Space reads as absent. Whether the probe behaves the same way on a
  runner, where the Accessibility permission is almost certainly not granted,
  is still unmeasured.
- **The Windows row is excluded by decision** (`ADR-0010`) and was not
  touched.
