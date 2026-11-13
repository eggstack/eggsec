# Desktop GUI platform qualification

This document is the authoritative statement of **which platforms the Eggsec
desktop GUI is qualified on, and for which behaviours**. It exists because
"it compiles on my machine" is not a qualification, and because the two
failure modes that matter for a shipped desktop binary are invisible in a
green CI log: a platform that was never exercised, and a platform that was
exercised but whose result was never recorded.

The desktop GUI is a **daemon-only** frontend (`ADR-0007`): it reaches the
engine only through the daemon's Unix-domain-socket transport, and it links
no engine crate. A GUI that cannot connect cannot do anything, so a
qualification that stopped at "the binary builds" would say almost nothing.

- The build recipe, the platform prerequisites, and the operator procedure
  live in [`docs/GUI_BUILD.md`](GUI_BUILD.md).
- The artefact handed to the distribution workstream is described in
  [`docs/GUI_DISTRIBUTION_HANDOFF.md`](GUI_DISTRIBUTION_HANDOFF.md).
- The supported-platform decision is
  [`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md).

---

## 1. How to read a cell

Every cell in the matrix below states exactly one status, followed by either
the evidence that supports it or the root cause that prevents it.

| Status | Meaning |
|---|---|
| `qualified` | The behaviour was observed on this platform and a reproducible evidence artefact exists. The cell names the artefact. |
| `partially qualified` | Part of the behaviour was observed. The cell names **both** the qualified subset and the missing part. |
| `not qualified` | The behaviour was not observed, or it failed. The cell names the root cause. |
| `out of scope` | The platform is excluded by a recorded decision. The cell names the decision and its root cause. |

The milestone-006 blocking token has been **retired**. The export affordance
exists in the frontend (`crates/eggsec-gui/frontend/app.js:1624` invokes the
`export_report` command), so a cell can no longer be blocked on the milestone
that owned it, and the token is no longer part of the vocabulary. A token that
can outlive its cause is a token that lies.

Three rules are absolute:
- **No cell is blank**, and no cell carries an unqualified support claim. A
  cell that says "works" without a status token, an evidence reference, or a
  root cause is a defect in this document, and the static guard
  (`scripts/check-architecture-guards.sh`, check 168) rejects it.
- **No cell says `qualified` without an evidence reference.** An evidence
  reference is a path plus the command that reproduces it plus the date of
  the run, for example
  `docs/evidence/gui-qualification/macos-2026-10-23.md` (run
  2026-10-23 with `bash scripts/build_gui_release.sh --out /tmp/eggsec-gui-release-test --allow-dirty`),
  or a CI run URL.
- **No cell says `qualified` without a run record that satisfies the promotion
  rules in §6.** The validator (`scripts/check_gui_qualification.py`) enforces
  them, and it reads the rules from §6 rather than carrying its own copy.

---

## 2. The qualification matrix

**Last regenerated:** 2026-10-24 — commit
`d0cf5bc3b71ce4d575a3888cce390cbbc310d12b`.

| Platform | Build | Launch | Connect | Submit | Cancel | Render | Export |
|---|---|---|---|---|---|---|---|
| `linux` | **qualified** — evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`); the record's `build` column carries the scripted outcome `pass`, and the build sidecar's SHA-256 is recorded beside it | **not qualified** — the procedure's window probe needs `xdotool` and an attached display server, and the `gui-platform` job installs neither, so the Linux leg records a named skip (`xdotool` is not installed on this host) and no painted window has been observed; evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24) | **not qualified** — the run measured it: the record's `connect` column carries the scripted outcome `pass` (the GUI's own client connected to the daemon and read its catalogue); evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`). The missing part is the **operator attestation** that rule P1 requires, and a CI run has nobody to observe with — the operator's half needs an operator at a Linux desktop | **not qualified** — the run measured it: the record's `submit` column carries the scripted outcome `pass` (the GUI's own form engine assembled the request and the daemon accepted it); evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`). The missing part is the **operator attestation** that rule P1 requires, and a CI run has nobody to observe with — the operator's half needs an operator at a Linux desktop | **not qualified** — the run measured it: the record's `cancel` column carries the scripted outcome `pass` (the daemon accepted the cancellation and the event stream reported `cancelled`); evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`). The missing part is the **operator attestation** that rule P1 requires, and a CI run has nobody to observe with — the operator's half needs an operator at a Linux desktop | **not qualified** — the run measured it: the record's `render` column carries the scripted outcome `pass` (the daemon's report for the task rendered through the GUI's own rendering path); evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`). The missing part is the **operator attestation** that rule P1 requires, and a CI run has nobody to observe with — the operator's half needs an operator at a Linux desktop | **not qualified** — the run measured it: the record's `export` column carries the scripted outcome `pass` (the GUI's own export path wrote the daemon's report); evidence: `docs/evidence/gui-qualification/linux-2026-10-24.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform linux --mode ci`). The missing part is the **operator attestation** that rule P1 requires, and a CI run has nobody to observe with — the operator's half needs an operator at a Linux desktop
| `macos` | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator`); the record's `build` column carries the scripted outcome `pass`, and the build sidecar's SHA-256 is recorded beside it | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `launch` column carries the scripted outcome `pass` (the probe found exactly one window titled `Eggsec`) and the operator outcome `pass`, and the attestation names the human who observed it, and the artefact `macos-2026-10-24-operator.launch.png` is the GUI's own window rather than a photograph of the operator's desktop | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `connect` column carries the scripted outcome `pass` (the GUI's own client connected to the daemon and read its catalogue) and the operator outcome `pass`, and the attestation names the human who observed it | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `submit` column carries the scripted outcome `pass` (the GUI's own form engine assembled the `PortScan` request and the daemon accepted it) and the operator outcome `pass`, and the attestation names the human who observed it | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `cancel` column carries the scripted outcome `pass` (the daemon accepted the cancellation and the event stream reported `cancelled`) and the operator outcome `pass`, and the attestation names the human who observed it | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `render` column carries the scripted outcome `pass` (the daemon's report for the task rendered through the GUI's own rendering path) and the operator outcome `pass`, and the attestation names the human who observed it | **qualified** — evidence: `docs/evidence/gui-qualification/macos-2026-10-24-operator.md` (run 2026-10-24, `bash scripts/gui_qualification.sh --platform macos --mode operator --run-label operator --attestation <attestation file>`); the record's `export` column carries the scripted outcome `pass` (the GUI's own export path wrote the daemon's report in the six formats) and the operator outcome `pass`, and the attestation names the human who observed it
| `windows` | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows | **out of scope** — `ADR-0010` excludes Windows from the desktop GUI's supported platforms; the GUI's unconditional dependency `eggsec-frontend-client` imports `tokio::net::UnixStream` without a `cfg(unix)` gate, so the GUI does not compile for Windows |

### 2.1 Reading the matrix honestly

The matrix is mostly `not qualified`, and that is the accurate result of a
milestone whose job is to *measure* rather than to assert. Three things are
worth saying plainly:

1. **The Linux row is empty because no Linux run has been recorded.** It is
   not empty because the GUI failed to build on Linux. The `gui-platform` CI
   job runs the procedure on `ubuntu-latest` and publishes the run record as
   a job artefact; until that job has a green run and its artefact is
   recorded in `docs/evidence/gui-qualification/CI-RUNS.md`, the row stays
   `not qualified`.
2. **The macOS row is qualified only where there is an artefact.** `Build` has
   one — a run record whose build column carries the scripted outcome `pass`
   and whose build sidecar's digest matches the file. `Launch` has one too:
   the run record carries the scripted outcome `pass` — the probe found exactly
   one window titled `Eggsec` — and the artefact is a screenshot of the
   painted window. That artefact is the GUI's *own window rectangle*, not a
   photograph of the operator's desktop: a full-screen capture on a
   developer's machine is a picture of everything else that was open, which is
   neither evidence of the GUI nor anybody else's business to commit.
3. **The behavioural columns are measured, and the measurement is not the
   whole of the promotion.** The harness drives the GUI's own client, form
   engine, and export path against a live daemon, and the macOS run record
   carries the outcomes. What rule P1 additionally requires is the
   **operator's attestation**: a person who saw the behaviour. The harness
   proves the cause; only the operator proves the effect. The macOS
   behavioural cells are promoted on an operator-attested run — the attestation
   is a file of its own, merged into the record by the procedure that writes
   it, and the validator refuses a `pass` that nobody signed.
4. **The export column is blocked on an attestation, not on a feature.** The
   export affordance exists — the frontend invokes `export_report`
   (`crates/eggsec-gui/frontend/app.js:1624`) — and the harness writes the
   daemon's report in the six formats, byte for byte as the renderer
   produced it. The earlier claim that the GUI had no export affordance at
   all was false, and it has been corrected rather than repeated.

### 2.2 The bundler decision

`bundle.active` stays `false` in `crates/eggsec-gui/tauri.conf.json`. The
reasoning is in [`docs/GUI_BUILD.md`](GUI_BUILD.md) §3.3: the roadmap and
`ADR-0007` place installers, signing, and notarization in the separate
distribution workstream, and enabling the bundler would make a plain
`make check` emit installers as a side effect.

If a later decision re-opens it, this document must name the superseding
ADR on the same line as the `bundle.active` mention; the static guard
(check 168) rejects a `bundle.active: true` configuration that does not.

---

## 3. Provenance

This matrix is **regenerated**, not hand-maintained. The regeneration
procedure is:

1. Run the qualification procedure in
   [`docs/GUI_BUILD.md`](GUI_BUILD.md) §5 on the platform being qualified.
   The procedure is `scripts/gui_qualification.sh`; it runs the steps, runs
   the harness, and writes the run's evidence file under
   `docs/evidence/gui-qualification/`, following
   `docs/evidence/gui-qualification/TEMPLATE.md`.
2. For each column, set the cell to the status the evidence supports and
   point it at the evidence file (or name the root cause). A cell may say
   `qualified` only when the run record satisfies the rule that covers it
   (§6).
3. Update the **Last regenerated** line above with the new date and the
   commit the matrix was regenerated at.
4. Re-run `python3 scripts/check_gui_qualification.py` (the promotion
   validator), `python3 scripts/check_doc_references.py`, and
   `bash scripts/check-architecture-guards.sh` (check 168 enforces the cell
   grammar; checks 181–189 enforce the procedure, the harness, and the
   promotion rules).

The matrix is a claim about a specific commit. A matrix row that was
qualified against an older commit is stale, not merely historical — which is
why the regeneration header carries the commit SHA.

---

## 4. The evidence mechanism

### 4.1 Where evidence lives

```
docs/evidence/gui-qualification/
  TEMPLATE.md                        # the fields every run must fill
  CI-RUNS.md                         # the index of CI runs a cell may cite
  macos-2026-10-23.md                # one file per qualification run
  macos-2026-10-24.md                #   ... and its artefacts beside it
  macos-2026-10-24.build.json        #   the build sidecar
  macos-2026-10-24.export.json       #   the exported report
```

One file per run, named `<platform>-<YYYY-MM-DD>.md`. A second run on the same
day adds a label: `--run-label operator` names it
`<platform>-<YYYY-MM-DD>-<label>.md`, so a day with two runs keeps two records
rather than one overwriting the other. The filename is part of
the evidence reference, so a cell that cites an evidence file cites it by
name. The artefacts a run produced — the build sidecar, the exported report —
are committed beside the record under the same prefix, because a cell's
artefact reference has to resolve to a file whose digest the validator can
re-check.

`CI-RUNS.md` is the index of the CI runs a cell may cite instead of a
checked-in file. A cell that cites a run URL the index does not carry is
rejected.

### 4.2 What an evidence file must contain

`docs/evidence/gui-qualification/TEMPLATE.md` fixes the fields. In short:
platform; the columns the run supports; the date; the operator or CI job;
the commit SHA; the tree state; the host OS version and architecture; the
target triple; the toolchain and cargo version; the requested and resolved
feature set; the prerequisite-probe result; the daemon version and build
features; the socket path and scope in force; the exact commands run with
their exit statuses; the observed output; the pass criterion and its
outcome; and any deviation from the runbook.

Two of those fields exist because they are the ones that get forgotten:

- **The tree state.** A build from a dirty tree is a diagnostic build, not a
  release build, and the difference has to be visible in the record rather
  than inferred later.
- **The deviations.** A deviation is recorded as a deviation. A runbook that
  was followed approximately is a different runbook, and the matrix cell has
  to say so.

### 4.3 Reproducibility

An evidence file is only useful if a second operator can re-run its commands
and expect the same observable outcome. That is why the template records the
*commands* rather than a description of them, and why the runbook in
`docs/GUI_BUILD.md` §5 gives each step an explicit pass criterion and names
the artefact the step produces.

### 4.4 CI as an evidence source

The `gui-platform` job in `.github/workflows/deep-checks.yml` runs on
`ubuntu-latest` and `macos-latest` and uploads the build metadata sidecar and
the binary's checksum as job artefacts. A matrix cell may therefore cite a CI
run URL instead of a checked-in evidence file. The rule is the same in both
cases: the reference must be reproducible by a documented command.

---

## 5. Why Windows is out of scope

The supported-platform decision is
[`ADR-0010`](../plans/adrs/ADR-0010-gui-supported-platform-scope.md):
**the desktop GUI is supported on Linux and macOS; Windows is out of scope.**
This milestone implements that decision; it does not make it.

The technical root cause is measured, not inferred. Cross-compiling the
GUI's unconditional dependency for Windows fails at the Unix-domain-socket
transport:

```
$ cargo check -p eggsec-frontend-client --target x86_64-pc-windows-msvc
error[E0432]: unresolved import `tokio::net::UnixStream`
   --> crates/eggsec-frontend-client/src/daemon.rs:9:5
error[E0433]: cannot find `unix` in `net`
   --> crates/eggsec-frontend-client/src/daemon.rs:56:31
error[E0599]: no method named `into_split` found for type `!`
   --> crates/eggsec-frontend-client/src/daemon.rs:85:46
error: could not compile `eggsec-frontend-client` (lib) due to 3 previous errors
```

The same three errors reproduce through the GUI crate, because
`eggsec-frontend-client` is an **unconditional** dependency of
`crates/eggsec-gui` (`crates/eggsec-gui/Cargo.toml:38`). The two
unconditional Unix-only import sites are:

- `crates/eggsec-frontend-client/src/daemon.rs:9` —
  `use tokio::net::UnixStream;`
- `crates/eggsec-frontend-client/src/daemon.rs:56` —
  `writer: Mutex<tokio::net::unix::OwnedWriteHalf>`

and the daemon side is equally unconditional:

- `crates/eggsec-daemon/src/server.rs:5` —
  `use tokio::net::UnixListener;`
- `crates/eggsec-daemon/src/server.rs:147` — `UnixListener::bind(socket_path)?`

### 5.1 The re-entry path

Adding a Windows leg to the CI matrix is gated on the daemon-transport
milestone `ADR-0010` names. A Windows daemon transport — named pipes or an
equivalent local IPC — is a **daemon-subsystem** change, not a GUI change: it
changes the *kind* of transport rather than adding a read path, so it
requires an ADR of its own. Milestone 007 names that path; it does not
implement it, and it deliberately does **not** add a Windows transport in
order to make a matrix cell green.

Until that ADR lands and the transport is implemented, the Windows row stays
`out of scope` in every column, and the CI matrix stays
`[ubuntu-latest, macos-latest]`.

---

## 6. What promotes a cell

The rules below are the whole promotion rule. `scripts/check_gui_qualification.py`
reads **this table** and enforces it: the validator does not carry a second
copy of the rule, so a rule added here is enforced without a second edit, and
a rule removed here stops being enforced the moment this table stops naming
it.

The **Kind** column says what kind of rule a row is:

- `evidence` — a cell may say `qualified` only when a run record satisfies
  the row's requirements.
- `status` — every cell the row covers carries the status the row names.
- `completeness` — every cell the row covers carries a legal status and a
  non-empty explanation.

The **Required outcome** and **Required attestation** columns are what the run
record's per-column fields must carry. `scripted` is what the machine
measured; `operator` is what a human attested. A `not-applicable` attestation
means the column does not call for a human, so the record may carry
`not-applicable` or `pass` for it.

| Rule | Kind | Platforms | Columns | Requires | Required outcome | Required attestation | Artefact |
|---|---|---|---|---|---|---|---|
| P1 | `evidence` | `linux`, `macos` | Connect, Submit, Cancel, Render | A run record for this platform whose `commit` is the matrix's regenerated-at commit or a descendant of it, whose `columns` carry this column exactly once, and whose `scripted` and `operator` outcomes are the ones this row names. The cell names that record in the §1 form: the path, the command that reproduces the run, and the date. | `pass` | `pass` | `optional` |
| P2 | `evidence` | `linux`, `macos` | Build | A run record for this platform, as P1 requires, whose `build` column carries the scripted outcome `pass` and whose artefact is the build sidecar, which must exist under `docs/evidence/gui-qualification/` and whose recorded SHA-256 must match the file. The build criterion is machine-checkable end to end, so no operator attestation is required. | `pass` | `not-applicable` | `required` |
| P3 | `evidence` | `linux`, `macos` | Launch | A run record for this platform, as P1 requires, whose `launch` column carries the scripted outcome `pass` and whose artefact is a screenshot of the painted window. The scripted half proves the window exists; the operator's attestation proves that a person saw it. | `pass` | `pass` | `required` |
| P4 | `evidence` | `linux`, `macos` | Export | A run record for this platform, as P1 requires, whose `export` column carries the scripted outcome `pass` and whose artefact is an exported file whose bytes equal the renderer's output for the report the daemon published. | `pass` | `pass` | `required` |
| P5 | `status` | `windows` | Build, Launch, Connect, Submit, Cancel, Render, Export | Every cell on the `windows` row carries the status this row names, and its explanation cites the decision that excludes the platform. | `out of scope` | — | — |
| P6 | `completeness` | `linux`, `macos`, `windows` | Build, Launch, Connect, Submit, Cancel, Render, Export | Every cell carries a status token from §1 and a non-empty explanation. | — | — | — |

**The two halves.** `scripted` proves the cause and `operator` proves the
effect. A promotion resting on the scripted half alone is not a promotion: the
harness can prove that the export path writes six formats, and it cannot prove
that a person saw the window.

**How the operator half is recorded.** The operator writes an **attestation
file** of their own — `docs/evidence/gui-qualification/<platform>-<YYYY-MM-DD>.attestation.json`,
schema `eggsec-gui-operator-attestation/v1` — naming who observed the
behaviours, when, on which platform, against which commit, and what they saw
for each column. The procedure merges it into the record in the same write
that produces the record (`bash scripts/gui_qualification.sh … --attestation
<path>`), so the record has one writer and a later run cannot erase the
attestation. The validator refuses an unattributed `pass`: a record whose
operator outcome is `pass` with no attestation block, an empty attester, or
no observation is rejected, and an attestation that names a platform, or a
commit the run does not descend from, is rejected. The commit binding is
decided against the repository's history, so the checkout has to carry it:
a checkout that cannot answer says so rather than accusing the attestation,
and an undecidable binding is still a rejection. The runbook's procedure is
[`docs/GUI_BUILD.md`](GUI_BUILD.md) §5.8, and the file's fields are in
[`docs/evidence/gui-qualification/TEMPLATE.md`](evidence/gui-qualification/TEMPLATE.md).

**What the validator cannot check.** It checks that the artefacts exist, that
their digests match, and that the record is internally consistent. It cannot
check that a human observed what the record says they observed. That is the
honest cost of the procedure route, and the matrix's own standard is
reproducibility-by-command (§4.3).
