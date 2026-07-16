#!/usr/bin/env python3
"""pytest post-processor: enforce skip/xfail budgets per profile.

Reads a profiles manifest and JUnit XML report, validates that each
profile stays within its skip and xfail budgets, and emits a structured
JSON verdict.

Usage (standalone):
    python scripts/python_skip_budget.py \
        --profile unit \
        --manifest tests/profiles.json \
        --junit-xml target/report.xml

Or as a pytest plugin (registered via conftest / entry_points).
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class SkipXfailRecord:
    node_id: str
    profile: str
    kind: str  # "skip" or "xfail"
    reason: str
    feature_gate: str | None = None
    source_file: str | None = None
    source_line: int | None = None


@dataclass
class ProfileBudget:
    name: str
    skip_budget: int = 0
    max_skips: int | None = None  # alias for skip_budget
    max_xfails: int | None = None
    required_min_tests: int = 1
    expected_features: list[str] = field(default_factory=list)
    allowed_skip_reasons: list[str] = field(default_factory=list)


@dataclass
class Verdict:
    profile: str
    total: int = 0
    passed: int = 0
    failed: int = 0
    skipped: int = 0
    xfailed: int = 0
    xpassed: int = 0
    error: int = 0
    skip_budget_status: str = "within"
    skip_count: int = 0
    xfail_count: int = 0
    unexpected_skips: list[dict[str, Any]] = field(default_factory=list)
    xfails_without_issue: list[dict[str, Any]] = field(default_factory=list)
    verdict: str = "PASS"
    failures: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Manifest parsing
# ---------------------------------------------------------------------------

def load_profiles(manifest_path: str) -> dict[str, ProfileBudget]:
    """Load profiles.json and return a dict keyed by profile name.

    Supports both formats:
    - ``{"profiles": [{...}, ...]}`` (canonical manifest)
    - ``{"profile_name": {...}, ...}`` (legacy flat dict)
    """
    with open(manifest_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)

    profiles: dict[str, ProfileBudget] = {}

    # Canonical format: {"profiles": [list of profile objects]}
    if "profiles" in data and isinstance(data["profiles"], list):
        for entry in data["profiles"]:
            if not isinstance(entry, dict):
                continue
            name = entry.get("name", "")
            if not name:
                continue
            profiles[name] = ProfileBudget(
                name=name,
                skip_budget=entry.get("skip_budget", entry.get("max_skips", 0)),
                max_xfails=entry.get("max_xfails"),
                required_min_tests=entry.get("required_min_tests", 1),
                expected_features=entry.get("expected_features", []),
                allowed_skip_reasons=entry.get("allowed_skip_reasons", []),
            )
    else:
        # Legacy flat dict format: {"profile_name": {...}}
        for name, entry in data.items():
            if not isinstance(entry, dict):
                continue
            profiles[name] = ProfileBudget(
                name=name,
                skip_budget=entry.get("skip_budget", entry.get("max_skips", 0)),
                max_xfails=entry.get("max_xfails"),
                required_min_tests=entry.get("required_min_tests", 1),
                expected_features=entry.get("expected_features", []),
                allowed_skip_reasons=entry.get("allowed_skip_reasons", []),
            )
    return profiles


# ---------------------------------------------------------------------------
# JUnit XML parsing
# ---------------------------------------------------------------------------

def _parse_source_ref(tc: ET.Element) -> tuple[str | None, int | None]:
    """Try to extract file/line from a <testcase> element.

    JUnit XML does not standardise this, but common reporters emit
    ``file`` / ``line`` attributes or nested ``<location>`` elements.
    """
    file_attr = tc.get("file") or tc.get("filepath") or tc.get("filename")
    line_attr = tc.get("line") or tc.get("lineno")

    loc = tc.find("location")
    if loc is not None:
        file_attr = file_attr or loc.get("file") or loc.get("path")
        line_attr = line_attr or loc.get("line")

    line_val: int | None = None
    if line_attr is not None:
        try:
            line_val = int(line_attr)
        except (ValueError, TypeError):
            pass

    return file_attr, line_val


def _extract_feature_gate(reason: str) -> str | None:
    """Heuristic: pull ``feature=xyz`` from skip/xfail reason text."""
    import re
    m = re.search(r"feature[=:]\s*(\S+)", reason, re.IGNORECASE)
    return m.group(1) if m else None


def parse_junit(
    junit_path: str,
    profile: str,
) -> tuple[dict[str, str], list[SkipXfailRecord]]:
    """Parse JUnit XML and return (status_map, records).

    ``status_map`` maps node_id -> outcome (passed/failed/skipped/xfailed/error).
    ``records`` contains skip and xfail details.
    """
    tree = ET.parse(junit_path)
    root = tree.getroot()

    status_map: dict[str, str] = {}
    records: list[SkipXfailRecord] = []

    # Support both <testsuites><testsuite>… and bare <testsuite>…
    testsuites = root.findall(".//testsuite") if root.tag != "testsuite" else [root]
    if not testsuites:
        testsuites = [root]

    for suite in testsuites:
        for tc in suite.findall("testcase"):
            node_id = tc.get("name") or tc.get("classname", "unknown")
            file_ref, line_ref = _parse_source_ref(tc)

            # Determine outcome
            skipped_el = tc.find("skipped")
            failure_el = tc.find("failure")
            error_el = tc.find("error")
            rerun_el = tc.find("rerun")

            if skipped_el is not None:
                reason = skipped_el.get("message", "") or skipped_el.text or ""
                status_map[node_id] = "skipped"
                records.append(SkipXfailRecord(
                    node_id=node_id,
                    profile=profile,
                    kind="skip",
                    reason=reason.strip(),
                    feature_gate=_extract_feature_gate(reason),
                    source_file=file_ref,
                    source_line=line_ref,
                ))
            elif failure_el is not None:
                status_map[node_id] = "failed"
            elif error_el is not None:
                status_map[node_id] = "error"
            elif rerun_el is not None:
                # Treat rerun as failed for budgeting purposes
                status_map[node_id] = "failed"
            else:
                # Check for xfail (pytest marks it as a <testcase> with no
                # failure but the xfail marker is reflected in the JUnit
                # output as ``outcome="skipped"`` or via ``<skipped>`` with
                # an ``xfail`` attribute in some reporters.
                xfail_attr = skipped_el.get("xfail") if skipped_el is not None else None
                if xfail_attr or (skipped_el is not None and "xfail" in (skipped_el.get("message") or "").lower()):
                    reason = (skipped_el.get("message", "") or "").strip() if skipped_el is not None else ""
                    status_map[node_id] = "xfailed"
                    records.append(SkipXfailRecord(
                        node_id=node_id,
                        profile=profile,
                        kind="xfail",
                        reason=reason,
                        feature_gate=_extract_feature_gate(reason),
                        source_file=file_ref,
                        source_line=line_ref,
                    ))
                else:
                    status_map[node_id] = "passed"

    return status_map, records


# ---------------------------------------------------------------------------
# Budget enforcement
# ---------------------------------------------------------------------------

def _is_unexpected_skip(record: SkipXfailRecord, budget: ProfileBudget) -> bool:
    """A skip is unexpected if the reason is not in the allowed list."""
    if not budget.allowed_skip_reasons:
        return False
    return record.reason not in budget.allowed_skip_reasons


def enforce_budget(
    status_map: dict[str, str],
    records: list[SkipXfailRecord],
    budget: ProfileBudget,
) -> Verdict:
    """Evaluate skip/xfail records against the profile budget."""
    v = Verdict(profile=budget.name)

    v.total = len(status_map)
    v.passed = sum(1 for s in status_map.values() if s == "passed")
    v.failed = sum(1 for s in status_map.values() if s == "failed")
    v.error = sum(1 for s in status_map.values() if s == "error")
    v.skipped = sum(1 for s in status_map.values() if s == "skipped")
    v.xfailed = sum(1 for s in status_map.values() if s == "xfailed")
    v.xpassed = sum(1 for s in status_map.values() if s == "xpassed")

    skips = [r for r in records if r.kind == "skip"]
    xfails = [r for r in records if r.kind == "xfail"]

    v.skip_count = len(skips)
    v.xfail_count = len(xfails)

    effective_budget = budget.max_skips if budget.max_skips is not None else budget.skip_budget

    # --- Rule 1: all selected tests skipped → FAIL ---
    if v.total > 0 and v.passed + v.failed + v.error + v.xpassed == 0 and v.skipped + v.xfailed > 0:
        v.failures.append(
            f"ALL {v.total} selected tests were skipped/xfailed — profile produced no real results"
        )

    # --- Rule 2: fewer than required_min_tests ran (non-skip/xfail) ---
    ran = v.passed + v.failed + v.error + v.xpassed
    if ran < budget.required_min_tests:
        v.failures.append(
            f"Only {ran} test(s) ran, but {budget.required_min_tests} required minimum"
        )

    # --- Rule 3: unexpected skips ---
    for rec in skips:
        if _is_unexpected_skip(rec, budget):
            v.unexpected_skips.append(asdict(rec))
            v.failures.append(
                f"Unexpected skip: {rec.node_id} — {rec.reason}"
            )

    # --- Rule 4: skip budget exceeded ---
    if v.skip_count > effective_budget:
        v.skip_budget_status = "over"
        v.failures.append(
            f"Skip budget exceeded: {v.skip_count} skips > budget of {effective_budget}"
        )
    else:
        v.skip_budget_status = "within"

    # --- Rule 5: xfails without issue references (informational → warn) ---
    for rec in xfails:
        # Heuristic: a proper xfail reason usually contains a GH/Jira ref
        import re
        has_ref = bool(re.search(r"(#\d+|GH-\d+|JIRA-\d+|CVE-\d{4}-\d+)", rec.reason, re.IGNORECASE))
        if not has_ref:
            v.xfails_without_issue.append(asdict(rec))

    # Max xfail budget
    if budget.max_xfails is not None and v.xfail_count > budget.max_xfails:
        v.failures.append(
            f"Xfail budget exceeded: {v.xfail_count} xfails > max of {budget.max_xfails}"
        )

    # --- Verdict ---
    v.verdict = "FAIL" if v.failures else "PASS"
    return v


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="Enforce skip/xfail budgets per profile using JUnit XML reports."
    )
    p.add_argument(
        "--profile", required=True,
        help="Profile name to enforce (must exist in the manifest)",
    )
    p.add_argument(
        "--manifest", required=True, metavar="PATH",
        help="Path to profiles.json manifest",
    )
    p.add_argument(
        "--junit-xml", required=True, metavar="PATH",
        help="Path to JUnit XML report",
    )
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)

    if not os.path.isfile(args.manifest):
        print(json.dumps({"error": f"Manifest not found: {args.manifest}"}))
        return 2
    if not os.path.isfile(args.junit_xml):
        print(json.dumps({"error": f"JUnit XML not found: {args.junit_xml}"}))
        return 2

    profiles = load_profiles(args.manifest)
    if args.profile not in profiles:
        available = sorted(profiles.keys())
        print(json.dumps({
            "error": f"Profile '{args.profile}' not found in manifest",
            "available_profiles": available,
        }))
        return 2

    budget = profiles[args.profile]
    status_map, records = parse_junit(args.junit_xml, args.profile)
    verdict = enforce_budget(status_map, records, budget)

    output = {
        "profile": verdict.profile,
        "total": verdict.total,
        "passed": verdict.passed,
        "failed": verdict.failed,
        "skipped": verdict.skipped,
        "xfailed": verdict.xfailed,
        "xpassed": verdict.xpassed,
        "error": verdict.error,
        "skip_budget_status": verdict.skip_budget_status,
        "skip_count": verdict.skip_count,
        "xfail_count": verdict.xfail_count,
        "unexpected_skips": verdict.unexpected_skips,
        "xfails_without_issue": verdict.xfails_without_issue,
        "verdict": verdict.verdict,
        "failures": verdict.failures,
    }

    print(json.dumps(output, indent=2))
    return 0 if verdict.verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
