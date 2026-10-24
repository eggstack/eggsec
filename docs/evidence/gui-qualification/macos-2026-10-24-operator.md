# GUI qualification evidence — macos 2026-10-24

This file is the run record for one desktop-GUI qualification run. It is produced by `scripts/gui_qualification.sh` and it is what the matrix's cells cite.

---

## Identity

| Field | Value |
|---|---|
| Platform | `macos` |
| Qualification column(s) this run supports | `Build`, `Launch`, `Connect`, `Submit`, `Cancel`, `Render`, `Export` |
| Date of the run (UTC) | 2026-10-24 |
| Run label | `operator` |
| Operator or CI job | `operator` |
| Commit SHA (`git rev-parse HEAD`) | `309d77d1362fef645927117efe78e9c19e53cbe2` |
| Tree state (`git status --porcelain` empty?) | `clean` |
| Host OS version | macos 25.6.0 |
| Architecture | aarch64 |
| Process architecture | arm64 (the architecture of the running process, which differs from the targeted one under Rosetta) |
| Target triple | aarch64-apple-darwin |
| Toolchain | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Cargo version | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| Feature set | requested ['shell'], resolved ['default', 'shell'] |

## Environment

| Field | Value |
|---|---|
| Prerequisite probe result | `bash scripts/check_gui_prereqs.sh` -> exit 0 (BUILDABLE) |
| Webview SDK version | see the prerequisite probe's output below |
| Daemon version and build features | attached to an operator-started daemon (features: unknown) |
| Daemon socket path used | `/tmp/eggsec-op-34860.sock` |
| Scope in force | surface `DesktopGuiManual`, source `gui-default` |
| Window probe | the window probe ran on this host |

## Commands run

```
# 1. bash scripts/check_gui_prereqs.sh
0

# 2. cargo test -p eggsec-gui --no-default-features --test qualification
0
```

## Observed output

The harness's transcript:

```
running 5 tests
test the_column_identifiers_are_the_seven_the_matrix_publishes ... ok
test the_outcome_file_round_trips_and_names_what_is_wrong_with_it ... ok
test the_export_refuses_a_non_regular_destination_and_backs_up_a_real_one ... ok
test the_live_daemon_half_records_its_outcomes ... ok
test every_report_format_renders_writes_and_digests ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

## Pass criterion and outcome

| Column | Pass criterion | Outcome | Met? |
|---|---|---|---|
| `build` | the runbook's Step 1 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-applicable` | yes |
| `launch` | the runbook's Step 2 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |
| `connect` | the runbook's Step 3 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |
| `submit` | the runbook's Step 4 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |
| `cancel` | the runbook's Step 5 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |
| `render` | the runbook's Step 6 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |
| `export` | the runbook's Step 7 (`docs/GUI_BUILD.md` §5) | scripted `pass`, operator `not-performed` | yes |

Each column's reason:

- `build` — the release build produced `eggsec-gui.build.json` and its SHA-256 is recorded
- `launch` — the probe found exactly one window titled `Eggsec`
- `connect` — the GUI's own client connected to the daemon and read its catalogue
- `submit` — the GUI's own form engine assembled the `PortScan` request and the daemon accepted it as task task:beaf9cfd-62ee-41a8-8bfb-5534b67510a0
- `cancel` — the daemon accepted the cancellation and the event stream reported `cancelled`
- `render` — the daemon's report for the task rendered as 0 findings and 1 open ports against `127.0.0.1`
- `export` — the GUI's own export path wrote the daemon's report in 6 formats, and each written file's bytes equal the renderer's output

## Deviations and notes

- None. The run followed `docs/GUI_BUILD.md` §5 as written.

## The run record

The block below is the `eggsec-gui-qualification-record/v1` record. `scripts/check_gui_qualification.py` reads it.

```json
{
  "schema": "eggsec-gui-qualification-record/v1",
  "platform": "macos",
  "mode": "operator",
  "date": "2026-10-24",
  "run_label": "operator",
  "commit": "309d77d1362fef645927117efe78e9c19e53cbe2",
  "tree_state": "clean",
  "host": {
    "os": "macos",
    "os_version": "25.6.0",
    "arch": "aarch64",
    "process_arch": "arm64",
    "target_triple": "aarch64-apple-darwin"
  },
  "toolchain": {
    "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
    "cargo": "cargo 1.99.1 (5f94df478 2026-08-27)"
  },
  "features": {
    "requested": [
      "shell"
    ],
    "resolved": [
      "default",
      "shell"
    ]
  },
  "prerequisite_probe": {
    "command": "bash scripts/check_gui_prereqs.sh",
    "exit": 0,
    "verdict": "BUILDABLE"
  },
  "daemon": {
    "socket": "/tmp/eggsec-op-34860.sock",
    "ready": true,
    "version": "attached to an operator-started daemon",
    "features": "unknown",
    "task_kinds": "30",
    "scope": {
      "surface": "DesktopGuiManual",
      "source": "gui-default"
    }
  },
  "window": {
    "probe": "osascript",
    "windows": "1",
    "screenshot": "macos-2026-10-24-operator.launch.png",
    "reason": "the window probe ran on this host"
  },
  "columns": [
    {
      "column": "build",
      "scripted": "pass",
      "operator": "not-applicable",
      "artifact": "macos-2026-10-24-operator.build.json",
      "sha256": "94c94f31e37eeff833e60a63a77e9f2bc6e1ee269b63f210c9de5116cf7f62ef",
      "reason": "the release build produced `eggsec-gui.build.json` and its SHA-256 is recorded"
    },
    {
      "column": "launch",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "macos-2026-10-24-operator.launch.png",
      "sha256": "82c29e41dd1a37df3cfb18792794059918348517435954f49992fcbe27ca8090",
      "reason": "the probe found exactly one window titled `Eggsec`"
    },
    {
      "column": "connect",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "",
      "sha256": "",
      "reason": "the GUI's own client connected to the daemon and read its catalogue"
    },
    {
      "column": "submit",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "",
      "sha256": "",
      "reason": "the GUI's own form engine assembled the `PortScan` request and the daemon accepted it as task task:beaf9cfd-62ee-41a8-8bfb-5534b67510a0"
    },
    {
      "column": "cancel",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "",
      "sha256": "",
      "reason": "the daemon accepted the cancellation and the event stream reported `cancelled`"
    },
    {
      "column": "render",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "",
      "sha256": "",
      "reason": "the daemon's report for the task rendered as 0 findings and 1 open ports against `127.0.0.1`"
    },
    {
      "column": "export",
      "scripted": "pass",
      "operator": "not-performed",
      "artifact": "macos-2026-10-24-operator.export.json",
      "sha256": "c77cee1715754a644aee6995114d430564763b4977b221717396201ade51c5ab",
      "reason": "the GUI's own export path wrote the daemon's report in 6 formats, and each written file's bytes equal the renderer's output"
    }
  ],
  "deviations": []
}
```
