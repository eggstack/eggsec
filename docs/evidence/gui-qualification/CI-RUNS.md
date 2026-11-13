# GUI qualification — CI runs

A matrix cell in [`docs/GUI_PLATFORM_QUALIFICATION.md`](../../GUI_PLATFORM_QUALIFICATION.md)
may cite a CI run instead of a checked-in evidence file. The rule is the same
in both cases: **the reference must be reproducible by a documented command**,
and the run must be recorded here.

A cell that cites a URL below is checked by
[`scripts/check_gui_qualification.py`](../../../scripts/check_gui_qualification.py):
the URL must appear in this file, and the run's record must satisfy the
promotion rules the matrix publishes in its §6.

The `gui-platform` job in
[`.github/workflows/deep-checks.yml`](../../../.github/workflows/deep-checks.yml)
runs the procedure on `ubuntu-latest` and `macos-latest` and publishes the run
record as a job artefact named `gui-qualification-<platform>`. The procedure is:

```bash
bash scripts/gui_qualification.sh --platform <linux|macos> --mode ci
```

## Runs

| Run URL | Platform | Workflow | Job | Record artefact | Recorded on |
|---|---|---|---|---|---|
| https://github.com/eggstack/eggsec/actions/runs/40143483040 | `linux` | `Deep Checks` | `GUI platform (ubuntu-latest)` | `gui-qualification-ubuntu-latest` | 2026-10-24 |
| https://github.com/eggstack/eggsec/actions/runs/40143483040 | `macos` | `Deep Checks` | `GUI platform (macos-latest)` | `gui-qualification-macos-latest` | 2026-10-24 |

The first `Deep Checks` run of the `gui-platform` job, on branch
`feat/auto-20261022-76692679` at commit `d0cf5bc3`. Both legs published their
run record. The Linux record is committed beside this index as
`docs/evidence/gui-qualification/linux-2026-10-24.md`, with its build sidecar
and its exported report, so the Linux row's `Build` cell is checked rather
than merely cited.

**What a CI run cannot carry.** The `operator` half of a promotion. The job
has nobody to observe with, so every column it produces carries
`operator: not-performed`, and rule P1 asks for `pass`. A CI run therefore
measures the scripted half and cannot promote the behavioural columns; that
needs an operator at the platform's own desktop. The Linux `Launch` cell is
out of reach for a second reason: the job installs no `xdotool` and has no
display server, so the window probe records a named skip.

## Adding a run

1. Wait for the `gui-platform` job to finish on the leg being recorded.
2. Download the `gui-qualification-<platform>` artefact.
3. Add a row above: the run URL, the platform, the workflow and job names, the
   artefact's name, and the date the row was added.
4. Point the matrix's cell at the URL in the §1 form — the URL, the command
   that reproduces it, and the date of the run.

A row that names a run whose artefact was not published is not evidence. The
artefact is the record; the URL only says where it came from.
