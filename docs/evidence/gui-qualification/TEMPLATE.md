# GUI qualification evidence — TEMPLATE

Copy this file to `docs/evidence/gui-qualification/<platform>-<YYYY-MM-DD>.md`
and fill in every field. `scripts/gui_qualification.sh` writes a file in this
shape for you; this template is what it fills in, and what a hand-written run
must match. A second run on the same day adds a label — `--run-label operator`
names it `<platform>-<YYYY-MM-DD>-operator.md` — so a day with two runs keeps
two records rather than one overwriting the other.

The qualification matrix
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

One row per column the run supports. The **Pass criterion** is the criterion
the runbook's step states (`docs/GUI_BUILD.md` §5); the **Outcome** is what
was observed. `scripted` is what the machine measured; `operator` is what a
human attested.

| Column | Pass criterion | Outcome | Met? |
|---|---|---|---|
| `build` | | | |
| `launch` | | | |
| `connect` | | | |
| `submit` | | | |
| `cancel` | | | |
| `render` | | | |
| `export` | | | |

For a `partial` outcome, name the qualified subset and the missing part
separately — a partial that does not say what is missing is indistinguishable
from a pass.

## The operator's attestation

The scripted half of a run is what the machine measured. The operator's
attestation is the other half: what a person saw. It is a file of its own,
the operator writes it once the behaviours have been observed, and the
procedure merges it into the record.

Put it beside the evidence it belongs to, named
`<platform>-<YYYY-MM-DD>[-<run label>].attestation.json`:

```
docs/evidence/gui-qualification/macos-2026-10-24-operator.attestation.json
```

The schema string is `eggsec-gui-operator-attestation/v1`. An attestation that
declares another schema is rejected by name rather than half-read.

```json
{
  "schema": "eggsec-gui-operator-attestation/v1",
  "attested_by": "David Bowman",
  "attested_at": "2026-10-24",
  "platform": "macos",
  "commit": "0f5dbb26e7ab215613e5b6e2295d9c124374bf9f",
  "columns": {
    "connect": {
      "outcome": "pass",
      "observed": "the badge read CONNECTED and the sidebar listed the daemon's published operations"
    },
    "submit": {
      "outcome": "pass",
      "observed": "a task card appeared with a task id and a non-terminal status"
    }
  }
}
```

- `attested_by` names the human who observed the behaviour. It is not
  optional: an attestation nobody signed is not an attestation.
- `attested_at` is the date the behaviours were observed, as `YYYY-MM-DD`.
- `platform` is the platform they were observed on. It must be the platform of
  the run the attestation is merged into.
- `commit` is the commit the behaviours were observed against — the tree the
  operator actually ran. The run that carries the attestation must be that
  commit or a descendant of it, so that an attestation of one run cannot be
  read as an attestation of another. The abbreviated form git prints by
  default is accepted.
- `columns` maps a column identifier to the operator's outcome and to what
  they saw. `outcome` is one of `pass`, `fail`, `not-performed` or
  `not-applicable`; `observed` is what the operator saw, and it is never
  empty. A column the attestation does not name keeps the outcome the run
  gave it.

An attestation may not contradict the run: a `pass` for a column whose
scripted outcome is not `pass` is refused, because an operator cannot attest
to a behaviour the run did not observe.

Pass it to the procedure, and the record it writes carries both halves in one
write:

```
bash scripts/gui_qualification.sh --platform macos --mode operator \
  --attestation docs/evidence/gui-qualification/macos-2026-10-24-operator.attestation.json
```

A CI run cannot attest: it has nobody to observe with. `--mode ci` accepts an
attestation and merges it as given, but the attestation is still the human's
record, not the job's.

## The run record

The block below is the machine-readable record. It is what
`scripts/check_gui_qualification.py` reads when it checks a cell that claims
`qualified`, so a file without it cannot promote a cell — it is simply not
sufficient evidence.

The schema string is `eggsec-gui-qualification-record/v1`. A record that
declares another schema is rejected by name rather than half-read.

`attestation` and `observed` are optional. A run with no operator attestation
carries no `attestation` block and no `observed` field, and every column's
`operator` is what the run itself produced.

```json
{
  "schema": "eggsec-gui-qualification-record/v1",
  "platform": "<linux|macos>",
  "date": "<YYYY-MM-DD>",
  "commit": "<the commit the run was made at>",
  "tree_state": "<clean|dirty>",
  "attestation": {
    "schema": "eggsec-gui-operator-attestation/v1",
    "attested_by": "<the human who observed the behaviours>",
    "attested_at": "<YYYY-MM-DD>",
    "platform": "<the platform the behaviours were observed on>",
    "commit": "<the commit the behaviours were observed against>"
  },
  "host": {
    "os": "<linux|macos>",
    "os_version": "<uname -r>",
    "arch": "<uname -m>",
    "target_triple": "<rustc -vV host:>"
  },
  "toolchain": {
    "rustc": "<rustc -V>",
    "cargo": "<cargo -V>"
  },
  "features": {
    "requested": ["shell"],
    "resolved": ["default", "shell"]
  },
  "prerequisite_probe": {
    "command": "bash scripts/check_gui_prereqs.sh",
    "exit": 0,
    "verdict": "<BUILDABLE|NOT BUILDABLE>"
  },
  "daemon": {
    "socket": "<the socket the run used>",
    "ready": true,
    "version": "<the daemon's version line>",
    "features": "<the cargo features it was built with>",
    "scope": {
      "surface": "DesktopGuiManual",
      "source": "gui-default"
    }
  },
  "window": {
    "probe": "<xdotool|osascript|none>",
    "windows": "<the number of matching windows>",
    "screenshot": "<the screenshot's file name, or empty>",
    "reason": "<why the probe ran, or why it was skipped>"
  },
  "columns": [
    {
      "column": "<build|launch|connect|submit|cancel|render|export>",
      "scripted": "<pass|fail|skipped|not-performed|not-applicable>",
      "operator": "<pass|fail|not-performed|not-applicable>",
      "observed": "<what the operator saw, when there is an attestation>",
      "artifact": "<the artefact's file name under this directory, or empty>",
      "sha256": "<the artefact's SHA-256, or empty>",
      "reason": "<why the column landed where it did>"
    }
  ],
  "deviations": []
}
```

The `reason` is the scripted half's own account of the column; the `observed`
is the operator's. They are separate fields, and a column that carries a
`pass` must carry an `observed` to go with it: the validator refuses an
operator outcome of `pass` that no named human stands behind.

## Deviations and notes

Anything that differed from the expected observation, including a deviation
from the runbook in `docs/GUI_BUILD.md`. A deviation is recorded here, not
smoothed over in the matrix.
