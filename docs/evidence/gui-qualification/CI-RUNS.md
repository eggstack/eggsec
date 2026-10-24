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
| _(no CI run has been recorded yet)_ | | | | | |

## Adding a run

1. Wait for the `gui-platform` job to finish on the leg being recorded.
2. Download the `gui-qualification-<platform>` artefact.
3. Add a row above: the run URL, the platform, the workflow and job names, the
   artefact's name, and the date the row was added.
4. Point the matrix's cell at the URL in the §1 form — the URL, the command
   that reproduces it, and the date of the run.

A row that names a run whose artefact was not published is not evidence. The
artefact is the record; the URL only says where it came from.
