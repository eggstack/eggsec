# GUI qualification evidence — TEMPLATE

Copy this file to `docs/evidence/gui-qualification/<platform>-<YYYY-MM-DD>.md`
and fill in every field. The qualification matrix
(`docs/GUI_PLATFORM_QUALIFICATION.md`) cites these files as the evidence for
its cells, so a cell that points at an evidence file must point at one whose
fields are all populated — an evidence reference that does not resolve to a
recorded run is not evidence.

**A run is only evidence if someone else can reproduce it.** Every field below
exists to let a second operator re-run the same commands and expect the same
observable outcome.

---

## Identity

| Field | Value |
|---|---|
| Platform | `linux` \| `macos` \| `windows` |
| Qualification column(s) this run supports | (e.g. `Build`, `Launch`) |
| Date of the run (UTC) | |
| Operator or CI job | (name, or the CI workflow + run URL) |
| Commit SHA (`git rev-parse HEAD`) | |
| Tree state (`git status --porcelain` empty?) | `clean` \| `dirty` — if dirty, list the paths and say why |
| Host OS version | (e.g. `macOS 15.6.0` / `Ubuntu 22.04.4 LTS`) |
| Architecture | (e.g. `x86_64`, `aarch64`) |
| Target triple | (e.g. `x86_64-unknown-linux-gnu`) |
| Toolchain | (`rustc -Vv` output) |
| Cargo version | (`cargo -V`) |
| Feature set | (the `--features` passed, and the resolved feature list) |

## Environment

| Field | Value |
|---|---|
| Prerequisite probe result | (paste `bash scripts/check_gui_prereqs.sh` output, or its exit status) |
| Webview SDK version | (`pkg-config --modversion webkit2gtk-4.1`, or `n/a` on macOS) |
| Daemon version and build features | (the `eggsec-daemon` commit and the cargo features it was built with) |
| Daemon socket path used | |
| Scope in force | |

## Commands run

One entry per command, in the order they were run. Paste the command
verbatim — a paraphrase is not reproducible.

```
# 1. <command>
<exit status>

# 2. <command>
<exit status>
```

## Observed output

Paste the output that the matrix cell's pass criterion is judged against. For
a behavioural column this is the operator-visible observation (the badge
state, the task status, the rendered result), not merely "the command
succeeded".

## Pass criterion and outcome

| Column | Pass criterion | Outcome | Met? |
|---|---|---|---|
| | | | yes / no / partial |

For a `partial` outcome, name the qualified subset and the missing part
separately — a partial that does not say what is missing is indistinguishable
from a pass.

## Deviations and notes

Anything that differed from the expected observation, including a deviation
from the runbook in `docs/GUI_BUILD.md`. A deviation is recorded here, not
smoothed over in the matrix.
