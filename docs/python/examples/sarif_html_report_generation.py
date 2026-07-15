#!/usr/bin/env python3
"""SARIF and HTML report generation.

Demonstrates generating security reports in SARIF and HTML formats
using the streaming reporter.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/sarif_html_report_generation.py
"""

import json
import tempfile
import os

from eggsec import StreamingReportConfig, StreamingReporter


def main():
    findings = [
        json.dumps({
            "id": "f1",
            "title": "SQL Injection",
            "severity": "high",
            "finding_type": "vulnerability",
            "affected_asset": {"asset_type": "url", "identifier": "https://app.example.com/api"},
        }),
        json.dumps({
            "id": "f2",
            "title": "Missing X-Frame-Options",
            "severity": "low",
            "finding_type": "misconfiguration",
            "affected_asset": {"asset_type": "url", "identifier": "https://example.com"},
        }),
    ]

    # SARIF report
    sarif_path = os.path.join(tempfile.gettempdir(), "report.sarif")
    sarif_config = StreamingReportConfig(format="sarif", output_path=sarif_path)
    with StreamingReporter(sarif_config) as reporter:
        reporter.start()
        for f in findings:
            reporter.write_finding(f)
        summary = reporter.finish()
    print(f"SARIF: {summary.total_findings} findings, {summary.output_size_bytes} bytes")
    print(f"  Written to: {sarif_path}")

    # HTML report
    html_path = os.path.join(tempfile.gettempdir(), "report.html")
    html_config = StreamingReportConfig(format="html", output_path=html_path)
    with StreamingReporter(html_config) as reporter:
        reporter.start()
        for f in findings:
            reporter.write_finding(f)
        summary = reporter.finish()
    print(f"HTML: {summary.output_size_bytes} bytes")
    print(f"  Written to: {html_path}")

    # JSON report
    json_path = os.path.join(tempfile.gettempdir(), "report.json")
    json_config = StreamingReportConfig(format="json", output_path=json_path)
    with StreamingReporter(json_config) as reporter:
        reporter.start()
        for f in findings:
            reporter.write_finding(f)
        summary = reporter.finish()
    print(f"JSON: {summary.output_size_bytes} bytes")

    # Cleanup
    for p in [sarif_path, html_path, json_path]:
        if os.path.exists(p):
            os.unlink(p)


if __name__ == "__main__":
    main()
