#!/usr/bin/env python3
"""Baseline comparison of assessment findings.

Demonstrates comparing two sets of findings to detect regressions,
new findings, and resolved issues.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/baseline_comparison.py
"""

from eggsec import (
    BaselineComparator,
    AffectedAsset,
    FindingType,
    VersionedFinding,
)


def main():
    baseline = [
        VersionedFinding(
            id="f1",
            title="XSS in search",
            description="Reflected XSS in search parameter",
            severity="high",
            finding_type=FindingType.Vulnerability,
            affected_asset=AffectedAsset("url", "https://x.example.com/search"),
            source_tool="fuzz",
            source_module="http",
        ),
        VersionedFinding(
            id="f2",
            title="Missing CSP header",
            description="Content-Security-Policy header not set",
            severity="medium",
            finding_type=FindingType.Misconfiguration,
            affected_asset=AffectedAsset("url", "https://example.com"),
            source_tool="recon",
            source_module="headers",
        ),
    ]

    current = [
        VersionedFinding(
            id="f3",
            title="XSS in search",
            description="Reflected XSS in search parameter",
            severity="high",
            finding_type=FindingType.Vulnerability,
            affected_asset=AffectedAsset("url", "https://x.example.com/search"),
            source_tool="fuzz",
            source_module="http",
        ),
        VersionedFinding(
            id="f4",
            title="New CSRF vulnerability",
            description="Cross-site request forgery on form endpoint",
            severity="critical",
            finding_type=FindingType.Vulnerability,
            affected_asset=AffectedAsset("url", "https://example.com/form"),
            source_tool="fuzz",
            source_module="auth",
        ),
    ]

    comparator = BaselineComparator()
    diff = comparator.compare(baseline, current)

    print(f"Baseline findings: {len(baseline)}")
    print(f"Current findings: {len(current)}")
    print(f"New findings: {diff.new_findings}")
    print(f"Resolved findings: {diff.resolved_findings}")
    print(f"Changed findings: {diff.changed_findings}")
    print(f"Unchanged findings: {diff.unchanged_findings}")
    print(f"Regression: {diff.is_regression}")
    print(f"Improvement: {diff.is_improvement}")


if __name__ == "__main__":
    main()
