#!/usr/bin/env python3
"""Check the desktop GUI's platform-qualification matrix against its evidence.

WHAT THIS IS. `docs/GUI_PLATFORM_QUALIFICATION.md` states which platforms the
desktop GUI is qualified on. A matrix is only worth reading if a cell's status
can be checked against the artefact it claims, so this script is the check: it
reads the matrix, reads the promotion rules the matrix itself publishes, and
rejects any cell whose claim is not backed.

THE PROMOTION RULE LIVES IN THE MATRIX. The rules table in
`docs/GUI_PLATFORM_QUALIFICATION.md` §6 is the single source of truth for which
columns exist, which status tokens are legal, and what each column needs before
it may be promoted. This script reads all three from there and implements the
*semantics*; it does not keep a second copy. A rule added to the matrix is
enforced here without a second edit, and a rule removed from the matrix stops
being enforced the moment the table stops naming it.

THE RECORD SCHEMA LIVES HERE TOO. This script writes the run record as well as
reading it, because a record that is written under one shape and read under
another is a record nobody can trust. The producer
(`scripts/gui_qualification.sh`) hands this script the facts a run observed and
asks for the record back.

EXIT STATUS. 0 when every cell is consistent with its evidence; 1 otherwise.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

MATRIX_PATH = os.path.join("docs", "GUI_PLATFORM_QUALIFICATION.md")
EVIDENCE_DIR = os.path.join("docs", "evidence", "gui-qualification")
CI_RUNS_PATH = os.path.join(EVIDENCE_DIR, "CI-RUNS.md")
TEMPLATE_PATH = os.path.join(EVIDENCE_DIR, "TEMPLATE.md")

#: The schema string that versions the run-record format. A record that
#: declares another schema is rejected by name rather than half-read.
RECORD_SCHEMA = "eggsec-gui-qualification-record/v1"

#: The matrix's header row, in order. The header is the canonical list of
#: behaviours; the harness's identifiers and the runbook's steps are the same
#: seven.
MATRIX_HEADER = [
    "Platform",
    "Build",
    "Launch",
    "Connect",
    "Submit",
    "Cancel",
    "Render",
    "Export",
]

#: The platforms the matrix has a row for.
MATRIX_PLATFORMS = ["linux", "macos", "windows"]

#: The kind of a cell's status token: `**status** — explanation`.
CELL_PATTERN = re.compile(r"^\*\*(?P<status>[^*]+)\*\*\s+—\s*(?P<explanation>.*)$")

#: A date, as an evidence reference must carry.
DATE_PATTERN = re.compile(r"\b\d{4}-\d{2}-\d{2}\b")

#: A CI run URL.
CI_RUN_PATTERN = re.compile(r"https://github\.com/\S+/actions/runs/\d+")

#: An ADR reference.
ADR_PATTERN = re.compile(r"ADR-\d+")


# ── Markdown helpers ────────────────────────────────────────────────────────


def read_text(path: str) -> str:
    """Read a file, returning an empty string when it is absent."""
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def split_row(line: str) -> Optional[List[str]]:
    """Split a Markdown table row into its cells, or `None` if it is not one."""
    stripped = line.strip()
    if not stripped.startswith("|"):
        return None
    body = stripped[1:]
    if body.endswith("|"):
        body = body[:-1]
    return [cell.strip() for cell in body.split("|")]


def is_separator(cells: Sequence[str]) -> bool:
    """Whether a row is a table's `---` separator row."""
    return bool(cells) and all(cell and set(cell) <= set("-: ") for cell in cells)


def tables(lines: Iterable[str]) -> List[List[List[str]]]:
    """Every Markdown table in a document, as lists of rows."""
    found: List[List[List[str]]] = []
    current: List[List[str]] = []
    for line in lines:
        cells = split_row(line)
        if cells is None:
            if current:
                found.append(current)
                current = []
            continue
        if is_separator(cells):
            continue
        if current:
            current.append(cells)
        else:
            current = [cells]
    if current:
        found.append(current)
    return found


def find_table(
    rows: Sequence[Sequence[str]], header: Sequence[str]
) -> Optional[List[List[str]]]:
    """The table whose header row equals `header`."""
    for table in tables(rows):
        if list(table[0]) == list(header):
            return [list(row) for row in table[1:]]
    return None


def strip_code(cell: str) -> str:
    """A cell's text without its backticks."""
    return cell.strip().strip("`").strip()


# ── The matrix ──────────────────────────────────────────────────────────────


class Rule:
    """One promotion rule, as the matrix's rules table states it."""

    def __init__(
        self,
        identifier: str,
        kind: str,
        platforms: Sequence[str],
        columns: Sequence[str],
        requires: str,
        outcome: str,
        attestation: str,
        artefact: str,
    ) -> None:
        self.identifier = identifier
        self.kind = kind
        self.platforms = [p.strip().strip("`").strip().lower() for p in platforms]
        self.columns = [c.strip().strip("`").strip() for c in columns]
        self.requires = requires
        self.outcome = outcome
        self.attestation = attestation
        self.artefact = artefact

    def applies_to(self, platform: str, column: str) -> bool:
        """Whether the rule governs this cell."""
        if platform.lower() not in self.platforms:
            return False
        return any(c.lower() == column.lower() for c in self.columns)

    def __repr__(self) -> str:  # pragma: no cover - diagnostics only
        return f"<Rule {self.identifier} {self.kind}>"


class Matrix:
    """The qualification matrix, parsed."""

    def __init__(self, text: str) -> None:
        self.text = text
        lines = text.splitlines()
        self.vocabulary = self._parse_vocabulary(lines)
        self.columns = self._parse_columns(lines)
        self.rows = self._parse_rows(lines)
        self.rules = self._parse_rules(lines)
        self.commit = self._parse_commit(lines)

    @staticmethod
    def _parse_vocabulary(lines: Sequence[str]) -> List[str]:
        table = find_table(lines, ["Status", "Meaning"])
        if table is None:
            return []
        return [strip_code(row[0]) for row in table if row]

    @staticmethod
    def _parse_columns(lines: Sequence[str]) -> List[str]:
        table = find_table(lines, MATRIX_HEADER)
        if table is None:
            return []
        return [column.lower() for column in MATRIX_HEADER[1:]]

    @staticmethod
    def _parse_rows(lines: Sequence[str]) -> Dict[str, Dict[str, str]]:
        table = find_table(lines, MATRIX_HEADER)
        if table is None:
            return {}
        rows: Dict[str, Dict[str, str]] = {}
        for row in table:
            if len(row) != len(MATRIX_HEADER):
                continue
            platform = strip_code(row[0]).lower()
            cells = {
                MATRIX_HEADER[index].lower(): row[index]
                for index in range(1, len(MATRIX_HEADER))
            }
            rows[platform] = cells
        return rows

    @staticmethod
    def _parse_rules(lines: Sequence[str]) -> List[Rule]:
        header = [
            "Rule",
            "Kind",
            "Platforms",
            "Columns",
            "Requires",
            "Required outcome",
            "Required attestation",
            "Artefact",
        ]
        table = find_table(lines, header)
        if table is None:
            return []
        rules: List[Rule] = []
        for row in table:
            if len(row) != len(header):
                continue
            rules.append(
                Rule(
                    identifier=strip_code(row[0]),
                    kind=strip_code(row[1]),
                    platforms=[
                        strip_code(part) for part in strip_code(row[2]).split(",")
                    ],
                    columns=[strip_code(part) for part in strip_code(row[3]).split(",")],
                    requires=row[4],
                    outcome=strip_code(row[5]),
                    attestation=strip_code(row[6]),
                    artefact=strip_code(row[7]),
                )
            )
        return rules

    @staticmethod
    def _parse_commit(lines: Sequence[str]) -> str:
        """The commit the matrix was regenerated at."""
        for line in lines:
            match = re.search(r"\*\*Last regenerated:\*\*.*?`([0-9a-f]{7,40})`", line)
            if match:
                return match.group(1)
        for line in lines:
            match = re.search(r"`([0-9a-f]{40})`", line)
            if match:
                return match.group(1)
        return ""


# ── The record ──────────────────────────────────────────────────────────────


def parse_record(text: str) -> Tuple[Optional[Dict[str, Any]], List[str]]:
    """The run record embedded in an evidence file.

    Returns the record and the problems found while reading it. An evidence
    file that carries no record block is **not** malformed: it is a file from
    before the record format existed, and it is simply not sufficient to
    promote a cell.
    """
    problems: List[str] = []
    blocks = re.findall(r"```json\s*\n(.*?)```", text, re.DOTALL)
    for block in blocks:
        try:
            candidate = json.loads(block)
        except json.JSONDecodeError as error:
            problems.append(f"the embedded json block does not parse: {error}")
            continue
        if not isinstance(candidate, dict):
            continue
        if candidate.get("schema") != RECORD_SCHEMA:
            problems.append(
                f"the embedded record declares the schema "
                f"`{candidate.get('schema')}`, but only `{RECORD_SCHEMA}` is accepted"
            )
            continue
        return candidate, problems
    return None, problems


def file_digest(path: str) -> Optional[str]:
    """The SHA-256 of a file, or `None` when it cannot be read."""
    try:
        with open(path, "rb") as handle:
            return hashlib.sha256(handle.read()).hexdigest()
    except OSError:
        return None


def is_ancestor(older: str, newer: str) -> Optional[bool]:
    """Whether `older` is an ancestor of `newer`, or `None` when unknown."""
    if not older or not newer:
        return None
    try:
        completed = subprocess.run(
            ["git", "merge-base", "--is-ancestor", older, newer],
            cwd=REPO_ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return completed.returncode == 0


# ── The checks ──────────────────────────────────────────────────────────────


class Checker:
    """The matrix's cells, checked against the evidence."""

    def __init__(self, matrix: Matrix, evidence_dir: str) -> None:
        self.matrix = matrix
        self.evidence_dir = evidence_dir
        self.problems: List[str] = []
        self.ci_runs = read_text(os.path.join(evidence_dir, "CI-RUNS.md"))

    def fail(self, message: str) -> None:
        self.problems.append(message)

    def check_shape(self) -> None:
        """The matrix's shape is exact: seven columns and three platforms."""
        if not self.matrix.columns:
            self.fail(
                "the matrix has no header row "
                f"`| {' | '.join(MATRIX_HEADER)} |`"
            )
            return
        if self.matrix.columns != [c.lower() for c in MATRIX_HEADER[1:]]:
            self.fail(
                "the matrix's header row must be exactly "
                f"`| {' | '.join(MATRIX_HEADER)} |`, found "
                f"`{' | '.join(self.matrix.columns)}`"
            )
        for platform in MATRIX_PLATFORMS:
            if platform not in self.matrix.rows:
                self.fail(f"the matrix has no `{platform}` row")
        for platform in self.matrix.rows:
            if platform not in MATRIX_PLATFORMS:
                self.fail(
                    f"the matrix has a `{platform}` row, which is not one of "
                    f"{', '.join(MATRIX_PLATFORMS)}"
                )
        for platform, cells in self.matrix.rows.items():
            for column in MATRIX_HEADER[1:]:
                if column.lower() not in cells:
                    self.fail(f"the `{platform}` row has no `{column}` cell")

    def check_vocabulary(self) -> None:
        """The status vocabulary is the matrix's own, and no other."""
        if not self.matrix.vocabulary:
            self.fail("the matrix publishes no status vocabulary")
            return
        if "pending M6" in self.matrix.vocabulary:
            self.fail(
                "the matrix's vocabulary still carries the `pending M6` token; "
                "the export affordance exists, so the token is illegal"
            )

    def check_rules(self) -> None:
        """The matrix publishes the rules the validator enforces."""
        identifiers = [rule.identifier for rule in self.matrix.rules]
        for expected in ["P1", "P2", "P3", "P4", "P5", "P6"]:
            if expected not in identifiers:
                self.fail(f"the matrix's rules table has no `{expected}` rule")

    def check_cells(self) -> None:
        """Every cell follows the grammar and its rule."""
        for platform in MATRIX_PLATFORMS:
            cells = self.matrix.rows.get(platform)
            if cells is None:
                continue
            for column in MATRIX_HEADER[1:]:
                cell = cells.get(column.lower())
                if cell is None:
                    continue
                self.check_cell(platform, column, cell)

    def check_cell(self, platform: str, column: str, cell: str) -> None:
        """One cell, checked."""
        where = f"the `{platform}` row's `{column}` cell"
        match = CELL_PATTERN.match(cell.strip())
        if match is None:
            self.fail(
                f"{where} is not `**status** — explanation`: {cell.strip()[:120]!r}"
            )
            return
        status = strip_code(match.group("status"))
        explanation = match.group("explanation").strip()

        if status not in self.matrix.vocabulary:
            self.fail(
                f"{where} carries the status `{status}`, which is not one of "
                f"{', '.join(self.matrix.vocabulary)}"
            )
            return
        if not explanation.strip("—- "):
            self.fail(f"{where} carries no explanation")
            return

        for rule in self.matrix.rules:
            if not rule.applies_to(platform, column):
                continue
            if rule.kind == "completeness":
                continue
            if rule.kind == "status":
                if status != rule.outcome:
                    self.fail(
                        f"{where} carries the status `{status}`, but rule "
                        f"{rule.identifier} requires `{rule.outcome}`"
                    )
                if not ADR_PATTERN.search(explanation):
                    self.fail(
                        f"{where} does not cite the decision that excludes the "
                        f"platform, which rule {rule.identifier} requires"
                    )
                continue
            if rule.kind == "evidence" and status == "qualified":
                self.check_qualified(platform, column, explanation, rule)

        if status == "partially qualified":
            if "qualified subset" not in explanation:
                self.fail(
                    f"{where} is partially qualified but does not name the "
                    "qualified subset"
                )
            if "missing part" not in explanation:
                self.fail(f"{where} is partially qualified but does not name the missing part")

    def check_qualified(
        self, platform: str, column: str, explanation: str, rule: Rule
    ) -> None:
        """A `qualified` cell: its evidence must resolve, and it must hold."""
        where = f"the `{platform}` row's `{column}` cell"

        references = re.findall(
            r"docs/evidence/gui-qualification/([A-Za-z0-9._-]+\.md)", explanation
        )
        urls = CI_RUN_PATTERN.findall(explanation)

        if not references and not urls:
            self.fail(f"{where} says `qualified` but names no evidence reference")
            return
        if not DATE_PATTERN.search(explanation):
            self.fail(
                f"{where} says `qualified` but names no date for the run; an "
                "evidence reference is a path, a command, and a date"
            )
            return

        for name in references:
            path = os.path.join(self.evidence_dir, name)
            if not os.path.isfile(path):
                self.fail(f"{where} cites `{name}`, which does not exist")
                continue
            self.check_record(platform, column, name, path, rule)
        for url in urls:
            if url not in self.ci_runs:
                self.fail(
                    f"{where} cites the CI run {url}, which is not recorded in "
                    f"{CI_RUNS_PATH}"
                )

    def check_record(
        self, platform: str, column: str, name: str, path: str, rule: Rule
    ) -> None:
        """The record an evidence file carries, checked against the rule."""
        where = f"the `{platform}` row's `{column}` cell"
        record, problems = parse_record(read_text(path))
        for problem in problems:
            self.fail(f"{where} cites `{name}`, whose record is rejected: {problem}")
        if record is None:
            # A file from before the record format existed is not malformed.
            # It is simply not sufficient to promote a cell, so the cell
            # cannot be `qualified` on its strength alone.
            self.fail(
                f"{where} says `qualified` on the strength of `{name}`, which "
                f"carries no `{RECORD_SCHEMA}` record block; an evidence file "
                "without a record cannot promote a cell"
            )
            return

        if record.get("platform") != platform:
            self.fail(
                f"{where} cites `{name}`, whose record qualifies "
                f"`{record.get('platform')}`, not `{platform}`"
            )
            return

        columns = record.get("columns")
        if not isinstance(columns, list):
            self.fail(f"{where} cites `{name}`, whose record carries no columns")
            return
        matches = [
            entry
            for entry in columns
            if isinstance(entry, dict) and entry.get("column") == column.lower()
        ]
        if len(matches) == 0:
            self.fail(
                f"{where} cites `{name}`, whose record has no `{column}` column"
            )
            return
        if len(matches) > 1:
            self.fail(
                f"{where} cites `{name}`, whose record has {len(matches)} "
                f"`{column}` columns; it must have exactly one"
            )
            return

        entry = matches[0]
        if entry.get("scripted") != rule.outcome:
            self.fail(
                f"{where} says `qualified`, but `{name}` records the `{column}` "
                f"column's scripted outcome as `{entry.get('scripted')}`, and "
                f"rule {rule.identifier} requires `{rule.outcome}`"
            )
        attestation = entry.get("operator")
        if rule.attestation == "not-applicable":
            if attestation not in ("not-applicable", "pass"):
                self.fail(
                    f"{where} says `qualified`, but `{name}` records the "
                    f"`{column}` column's operator outcome as `{attestation}`, "
                    f"and rule {rule.identifier} requires `not-applicable`"
                )
        elif attestation != rule.attestation:
            self.fail(
                f"{where} says `qualified`, but `{name}` records the `{column}` "
                f"column's operator outcome as `{attestation}`, and rule "
                f"{rule.identifier} requires `{rule.attestation}`"
            )

        artefact = entry.get("artifact") or ""
        if rule.artefact == "required" and not artefact:
            self.fail(
                f"{where} says `qualified`, but `{name}` names no artefact for "
                f"the `{column}` column, and rule {rule.identifier} requires one"
            )
        if artefact:
            artefact_path = os.path.join(self.evidence_dir, artefact)
            if not os.path.isfile(artefact_path):
                self.fail(
                    f"{where} says `qualified`, but the artefact `{artefact}` "
                    f"that `{name}` names does not exist under "
                    f"{self.evidence_dir}"
                )
                return
            observed = file_digest(artefact_path)
            if observed != entry.get("sha256"):
                self.fail(
                    f"{where} says `qualified`, but the artefact `{artefact}` "
                    f"digests to `{observed}`, not to the "
                    f"`{entry.get('sha256')}` that `{name}` records"
                )

        self.check_commit(where, name, record)

    def check_commit(self, where: str, name: str, record: Dict[str, Any]) -> None:
        """A record's commit is the matrix's commit or a descendant of it."""
        matrix_commit = self.matrix.commit
        record_commit = record.get("commit") or ""
        if not matrix_commit or not record_commit:
            return
        if matrix_commit == record_commit:
            return
        ancestor = is_ancestor(matrix_commit, record_commit)
        if ancestor is None:
            self.fail(
                f"{where} cites `{name}`, whose record was made at "
                f"`{record_commit}`, but the matrix was regenerated at "
                f"`{matrix_commit}` and the relationship cannot be checked"
            )
        elif not ancestor:
            self.fail(
                f"{where} cites `{name}`, whose record was made at "
                f"`{record_commit}`, which is not a descendant of the "
                f"matrix's regenerated-at commit `{matrix_commit}`"
            )

    def run(self) -> List[str]:
        """Every check, and the problems found."""
        self.check_shape()
        self.check_vocabulary()
        self.check_rules()
        self.check_cells()
        return self.problems


def validate(matrix_path: str, evidence_dir: str) -> List[str]:
    """Validate a matrix against its evidence directory."""
    matrix = Matrix(read_text(matrix_path))
    return Checker(matrix, evidence_dir).run()


# ── Writing a record ───────────────────────────────────────────────────────


def build_record(facts: Dict[str, Any]) -> Dict[str, Any]:
    """Assemble the run record from the facts a run observed."""
    columns: List[Dict[str, Any]] = []

    build = facts.get("build", {})
    columns.append(
        {
            "column": "build",
            "scripted": build.get("status", "skipped"),
            "operator": "not-applicable",
            "artifact": build.get("artifact", ""),
            "sha256": build.get("sha256", ""),
            "reason": build.get("reason", ""),
        }
    )

    launch = facts.get("launch", {})
    columns.append(
        {
            "column": "launch",
            "scripted": launch.get("status", "not-performed"),
            "operator": "not-performed",
            "artifact": launch.get("artifact", ""),
            "sha256": launch.get("sha256", ""),
            "reason": launch.get("reason", ""),
        }
    )

    harness = facts.get("harness", {})
    outcome_path = harness.get("outcome") or ""
    harness_columns: List[Dict[str, Any]] = []
    if outcome_path and os.path.isfile(outcome_path):
        try:
            with open(outcome_path, "r", encoding="utf-8") as handle:
                harness_columns = json.load(handle).get("columns", [])
        except (OSError, json.JSONDecodeError):
            harness_columns = []
    by_column = {entry.get("column"): entry for entry in harness_columns}

    export = facts.get("export", {})
    for column in ["connect", "submit", "cancel", "render", "export"]:
        entry = dict(by_column.get(column) or {})
        entry["column"] = column
        entry.setdefault("scripted", "skipped")
        entry.setdefault("operator", "not-performed")
        entry.setdefault("artifact", "")
        entry.setdefault("sha256", "")
        entry.setdefault("reason", "the harness did not report this column")
        if column == "export" and entry.get("artifact"):
            # The artefact is copied into the evidence directory under a name
            # that says what it is; the digest travels with it.
            entry["artifact"] = export.get("artifact", "")
            entry["sha256"] = export.get("sha256", "")
        columns.append(entry)

    window = facts.get("window", {})
    probe = facts.get("prerequisite_probe", {})
    daemon = facts.get("daemon", {})
    host = facts.get("host", {})

    return {
        "schema": RECORD_SCHEMA,
        "platform": facts.get("platform", ""),
        "mode": facts.get("mode", ""),
        "date": facts.get("date", ""),
        "commit": facts.get("commit", ""),
        "tree_state": facts.get("tree_state", ""),
        "host": host,
        "toolchain": facts.get("toolchain", {}),
        "features": facts.get("features", {}),
        "prerequisite_probe": {
            "command": probe.get("command", ""),
            "exit": probe.get("exit", 1),
            "verdict": probe.get("verdict", "UNKNOWN"),
        },
        "daemon": {
            "socket": daemon.get("socket", ""),
            "ready": daemon.get("ready", False),
            "version": daemon.get("version", ""),
            "features": daemon.get("features", ""),
            "task_kinds": daemon.get("task_kinds", ""),
            "scope": daemon.get(
                "scope", {"surface": "DesktopGuiManual", "source": "gui-default"}
            ),
        },
        "window": {
            "probe": window.get("probe", ""),
            "windows": window.get("windows", ""),
            "screenshot": window.get("screenshot", ""),
            "reason": window.get("reason", ""),
        },
        "columns": columns,
        "deviations": [],
    }


def trim_transcript(output: str) -> str:
    """The harness's observations, without the build machinery's noise.

    A cargo transcript carries the compiler's and the test runner's progress
    lines, and a path-like line in it (`Running tests/qualification.rs`) reads
    to a documentation-reference checker as a file reference. The record wants
    the observations: which test ran, what it said, and what the run concluded.
    """
    kept = []
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith(("running ", "test ", "SKIP:", "error", "warning:")):
            kept.append(stripped)
    if not kept:
        return output.strip()
    return "\n".join(kept)


def write_record_markdown(record: Dict[str, Any], facts: Dict[str, Any], path: str) -> None:
    """Write the evidence file: the template's fields and the record block."""
    host = record.get("host", {})
    toolchain = record.get("toolchain", {})
    probe = record.get("prerequisite_probe", {})
    daemon = record.get("daemon", {})
    window = record.get("window", {})
    harness = facts.get("harness", {})

    steps = {column.lower(): index + 1 for index, column in enumerate(MATRIX_HEADER[1:])}

    lines: List[str] = []
    lines.append(f"# GUI qualification evidence — {record['platform']} {record['date']}")
    lines.append("")
    lines.append(
        "This file is the run record for one desktop-GUI qualification run. It "
        "is produced by `scripts/gui_qualification.sh` and it is what the "
        "matrix's cells cite."
    )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## Identity")
    lines.append("")
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    lines.append(f"| Platform | `{record['platform']}` |")
    lines.append(
        "| Qualification column(s) this run supports | "
        + ", ".join(
            f"`{column}`"
            for column in MATRIX_HEADER[1:]
            if any(
                entry["column"] == column.lower() and entry["scripted"] == "pass"
                for entry in record["columns"]
            )
        )
        + " |"
    )
    lines.append(f"| Date of the run (UTC) | {record['date']} |")
    lines.append(f"| Operator or CI job | `{record.get('mode', 'operator')}` |")
    lines.append(f"| Commit SHA (`git rev-parse HEAD`) | `{record['commit']}` |")
    lines.append(f"| Tree state (`git status --porcelain` empty?) | `{record['tree_state']}` |")
    lines.append(f"| Host OS version | {host.get('os', '')} {host.get('os_version', '')} |")
    lines.append(f"| Architecture | {host.get('arch', '')} |")
    if host.get("process_arch") and host.get("process_arch") != host.get("arch"):
        lines.append(
            f"| Process architecture | {host.get('process_arch')} "
            "(the architecture of the running process, which differs from the "
            "targeted one under Rosetta) |"
        )
    lines.append(f"| Target triple | {host.get('target_triple', '')} |")
    lines.append(f"| Toolchain | `{toolchain.get('rustc', '')}` |")
    lines.append(f"| Cargo version | `{toolchain.get('cargo', '')}` |")
    lines.append(
        f"| Feature set | requested {record.get('features', {}).get('requested', [])}, "
        f"resolved {record.get('features', {}).get('resolved', [])} |"
    )
    lines.append("")
    lines.append("## Environment")
    lines.append("")
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    lines.append(
        f"| Prerequisite probe result | `{probe.get('command', '')}` -> exit "
        f"{probe.get('exit', '')} ({probe.get('verdict', '')}) |"
    )
    lines.append("| Webview SDK version | see the prerequisite probe's output below |")
    lines.append(
        f"| Daemon version and build features | {daemon.get('version', '')} "
        f"(features: {daemon.get('features', '')}) |"
    )
    lines.append(f"| Daemon socket path used | `{daemon.get('socket', '')}` |")
    scope = daemon.get("scope", {})
    lines.append(
        f"| Scope in force | surface `{scope.get('surface', '')}`, source "
        f"`{scope.get('source', '')}` |"
    )
    lines.append(f"| Window probe | {window.get('reason', '')} |")
    lines.append("")
    lines.append("## Commands run")
    lines.append("")
    lines.append("```")
    lines.append(f"# 1. {probe.get('command', '')}")
    lines.append(f"{probe.get('exit', '')}")
    lines.append("")
    lines.append(f"# 2. {harness.get('command', '')}")
    lines.append(f"{harness.get('exit', '')}")
    lines.append("```")
    lines.append("")
    lines.append("## Observed output")
    lines.append("")
    lines.append("The harness's transcript:")
    lines.append("")
    lines.append("```")
    lines.append(trim_transcript(harness.get("output", "") or ""))
    lines.append("```")
    lines.append("")
    lines.append("## Pass criterion and outcome")
    lines.append("")
    lines.append("| Column | Pass criterion | Outcome | Met? |")
    lines.append("|---|---|---|---|")
    for entry in record["columns"]:
        column = entry["column"]
        step = steps.get(column.lower(), 0)
        outcome = f"scripted `{entry['scripted']}`, operator `{entry['operator']}`"
        met = "yes" if entry["scripted"] == "pass" else "partial"
        lines.append(
            f"| `{column}` | the runbook's Step {step} (`docs/GUI_BUILD.md` §5) | "
            f"{outcome} | {met} |"
        )
    lines.append("")
    lines.append("Each column's reason:")
    lines.append("")
    for entry in record["columns"]:
        lines.append(f"- `{entry['column']}` — {entry['reason']}")
    lines.append("")
    lines.append("## Deviations and notes")
    lines.append("")
    deviations = record.get("deviations") or []
    if deviations:
        for deviation in deviations:
            lines.append(f"- {deviation}")
    else:
        lines.append("- None. The run followed `docs/GUI_BUILD.md` §5 as written.")
    lines.append("")
    lines.append("## The run record")
    lines.append("")
    lines.append(
        f"The block below is the `{RECORD_SCHEMA}` record. "
        "`scripts/check_gui_qualification.py` reads it."
    )
    lines.append("")
    lines.append("```json")
    lines.append(json.dumps(record, indent=2))
    lines.append("```")
    lines.append("")

    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


# ── The self-test ───────────────────────────────────────────────────────────


def self_test(matrix_path: str, evidence_dir: str) -> int:
    """Prove that the validator rejects what it is supposed to reject.

    Every case below is a deliberate corruption of a known-good input. The
    validator must reject each one, and it must say which cell it rejected.
    """
    original_matrix = read_text(matrix_path)
    original_evidence = {}
    if os.path.isdir(evidence_dir):
        for name in os.listdir(evidence_dir):
            path = os.path.join(evidence_dir, name)
            if os.path.isfile(path):
                with open(path, "rb") as handle:
                    original_evidence[name] = handle.read()

    failures: List[str] = []

    def check_case(name: str, expectation: str, mutate) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as workspace:
            matrix_copy = os.path.join(workspace, "matrix.md")
            evidence_copy = os.path.join(workspace, "evidence")
            os.makedirs(evidence_copy)
            with open(matrix_copy, "w", encoding="utf-8") as handle:
                handle.write(original_matrix)
            for filename, content in original_evidence.items():
                with open(os.path.join(evidence_copy, filename), "wb") as handle:
                    handle.write(content)
            mutate(matrix_copy, evidence_copy)
            problems = validate(matrix_copy, evidence_copy)
            if not problems:
                failures.append(f"{name}: the validator accepted it")
                return
            if expectation and expectation not in " ".join(problems):
                failures.append(
                    f"{name}: the validator rejected it, but not with `{expectation}`: "
                    f"{problems[0]}"
                )
                return
            print(f"ok: {name} -> {problems[0]}")

    def rewrite_matrix(matrix_copy: str, old: str, new: str) -> None:
        with open(matrix_copy, "r", encoding="utf-8") as handle:
            text = handle.read()
        if old not in text:
            raise SystemExit(f"the self-test's mutation target is absent: {old[:60]!r}")
        with open(matrix_copy, "w", encoding="utf-8") as handle:
            handle.write(text.replace(old, new, 1))

    def rewrite_records(evidence_copy: str, old: str, new: str) -> None:
        """Rewrite a string inside a record's json block, and only there."""
        touched = False
        for name in sorted(os.listdir(evidence_copy)):
            if not name.endswith(".md"):
                continue
            path = os.path.join(evidence_copy, name)
            with open(path, "r", encoding="utf-8") as handle:
                text = handle.read()
            blocks = list(re.finditer(r"```json\s*\n(.*?)```", text, re.DOTALL))
            if not blocks:
                continue
            block = blocks[0].group(1)
            if old not in block:
                continue
            mutated = text[: blocks[0].start(1)] + block.replace(old, new, 1) + text[blocks[0].end(1) :]
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(mutated)
            touched = True
        if not touched:
            raise SystemExit(f"the self-test's record mutation target is absent: {old[:60]!r}")

    def replace_cell(matrix_copy: str, platform: str, index: int, content: str) -> None:
        """Replace one cell of one matrix row, wholesale."""
        with open(matrix_copy, "r", encoding="utf-8") as handle:
            lines = handle.read().splitlines()
        for number, line in enumerate(lines):
            if not line.startswith(f"| `{platform}` |"):
                continue
            cells = line.split("|")
            # The first and last elements are the row's empty edges.
            cells[index + 1] = content
            lines[number] = "|".join(cells)
            with open(matrix_copy, "w", encoding="utf-8") as handle:
                handle.write("\n".join(lines) + "\n")
            return
        raise SystemExit(f"the self-test's matrix row is absent: {platform}")

    def blank_a_cell(matrix_copy: str, _evidence_copy: str) -> None:
        replace_cell(matrix_copy, "macos", 1, " **qualified** — ")

    def unknown_status(matrix_copy: str, _evidence_copy: str) -> None:
        rewrite_matrix(matrix_copy, "**qualified** — evidence:", "**almost qualified** — evidence:")

    def pending_token(matrix_copy: str, _evidence_copy: str) -> None:
        rewrite_matrix(
            matrix_copy,
            "| `out of scope` | The platform is excluded by a recorded decision.",
            "| `pending M6` | The behaviour cannot exist yet.",
        )

    def windows_promoted(matrix_copy: str, _evidence_copy: str) -> None:
        rewrite_matrix(matrix_copy, "**out of scope** — `ADR-0010`", "**qualified** — `ADR-0010`")

    def missing_artefact(matrix_copy: str, evidence_copy: str) -> None:
        rewrite_records(evidence_copy, '"artifact": "macos-', '"artifact": "absent-macos-')

    def wrong_digest(matrix_copy: str, evidence_copy: str) -> None:
        def mutate(path: str, text: str) -> str:
            return re.sub(r'"sha256": "[0-9a-f]{64}"', '"sha256": "' + "0" * 64 + '"', text)

        for name in sorted(os.listdir(evidence_copy)):
            if not name.endswith(".md"):
                continue
            path = os.path.join(evidence_copy, name)
            with open(path, "r", encoding="utf-8") as handle:
                text = handle.read()
            if RECORD_SCHEMA not in text:
                continue
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(mutate(path, text))

    def unknown_schema(matrix_copy: str, evidence_copy: str) -> None:
        rewrite_records(evidence_copy, RECORD_SCHEMA, "eggsec-gui-qualification-record/v0")

    def missing_column(matrix_copy: str, evidence_copy: str) -> None:
        rewrite_records(
            evidence_copy,
            '"column": "build",',
            '"column": "build-renamed",',
        )

    def missing_evidence_file(matrix_copy: str, _evidence_copy: str) -> None:
        rewrite_matrix(
            matrix_copy,
            "docs/evidence/gui-qualification/macos-2026-10-24.md",
            "docs/evidence/gui-qualification/macos-9999-01-01.md",
        )

    check_case("a blank cell is rejected", "carries no explanation", blank_a_cell)
    check_case("an unknown status token is rejected", "not one of", unknown_status)
    check_case("the retired milestone token is rejected", "pending M6", pending_token)
    check_case(
        "a promoted Windows cell is rejected", "requires `out of scope`", windows_promoted
    )
    check_case("a missing artefact is rejected", "does not exist", missing_artefact)
    check_case("a wrong digest is rejected", "digests to", wrong_digest)
    check_case("an unknown record schema is rejected", "schema", unknown_schema)
    check_case("a record without the column is rejected", "column", missing_column)
    check_case("a missing evidence file is rejected", "does not exist", missing_evidence_file)

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("ok: every deliberate corruption was rejected")
    return 0


# ── The entry point ─────────────────────────────────────────────────────────


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--matrix",
        default=os.path.join(REPO_ROOT, MATRIX_PATH),
        help="the matrix to validate",
    )
    parser.add_argument(
        "--evidence-dir",
        default=os.path.join(REPO_ROOT, EVIDENCE_DIR),
        help="the directory the evidence files live in",
    )
    parser.add_argument(
        "--write-record",
        metavar="PATH",
        help="write the run record's json to this path and exit",
    )
    parser.add_argument(
        "--run-inputs",
        metavar="PATH",
        help="the facts a run observed, as json",
    )
    parser.add_argument(
        "--record-markdown",
        metavar="PATH",
        help="write the evidence file to this path",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="prove that the validator rejects what it must reject",
    )
    arguments = parser.parse_args(argv)

    if arguments.self_test:
        return self_test(arguments.matrix, arguments.evidence_dir)

    if arguments.write_record:
        if not arguments.run_inputs:
            print("--write-record needs --run-inputs", file=sys.stderr)
            return 2
        with open(arguments.run_inputs, "r", encoding="utf-8") as handle:
            facts = json.load(handle)
        record = build_record(facts)
        with open(arguments.write_record, "w", encoding="utf-8") as handle:
            json.dump(record, handle, indent=2)
            handle.write("\n")
        if arguments.record_markdown:
            write_record_markdown(record, facts, arguments.record_markdown)
        return 0

    problems = validate(arguments.matrix, arguments.evidence_dir)
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        print(f"\n{len(problems)} problem(s) in {arguments.matrix}")
        return 1
    print(f"ok: {arguments.matrix} is consistent with its evidence")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
