"""Convert eggsec scan results to pandas DataFrames.

Demonstrates how to use to_rows() for easy conversion to pandas,
useful for analysis, visualization, and Jupyter notebooks.

Note: This example scans localhost. Ensure you have authorization
before scanning any target.
"""

import argparse
import sys

import eggsec


def main():
    parser = argparse.ArgumentParser(description="Eggsec scan to pandas")
    parser.add_argument("--target", default="127.0.0.1", help="Target host")
    parser.add_argument(
        "--local-fixture",
        action="store_true",
        help="Use localhost fixture (no network required)",
    )
    args = parser.parse_args()

    # Authorization: only scan targets you own or have written permission to test.
    scope = eggsec.Scope.allow_hosts([args.target])

    # --- Port scan ---
    print(f"[*] Running port scan on {args.target}...")
    ports = eggsec.scan_ports(
        target=args.target,
        ports=[22, 80, 443],
        scope=scope,
        timeout_ms=5000,
    )

    # Convert to rows (list of dicts)
    port_rows = ports.to_rows()
    print(f"    Port scan rows: {len(port_rows)}")
    for row in port_rows:
        print(f"      {row}")

    # --- Findings ---
    print("\n[*] Creating findings...")
    finding = eggsec.Finding(
        id="example-001",
        title="Test finding",
        severity=eggsec.Severity.Medium,
        target=args.target,
        category="example",
        description="A demonstration finding",
        evidence=eggsec.Evidence(
            kind="header",
            value="X-Test: true",
            source="scan",
        ),
    )

    finding_set = eggsec.FindingSet()
    finding_set.add_finding(finding)
    finding_set.add_finding(
        eggsec.Finding(
            id="example-002",
            title="Info disclosure",
            severity=eggsec.Severity.Low,
            target=args.target,
            category="info",
            description="Informational finding",
        )
    )

    finding_rows = finding_set.to_rows()
    print(f"    Finding rows: {len(finding_rows)}")
    for row in finding_rows:
        print(f"      {row}")

    finding_dicts = finding_set.to_dicts()
    print(f"\n    Finding dicts: {len(finding_dicts)}")

    # --- Pandas usage (if available) ---
    try:
        import pandas as pd

        df_ports = pd.DataFrame(port_rows)
        df_findings = pd.DataFrame(finding_rows)

        print(f"\n[*] pandas DataFrames:")
        print(f"    ports:      {df_ports.shape}")
        print(f"    findings:   {df_findings.shape}")
        print(f"\n[*] Findings by severity:")
        if not df_findings.empty and "severity" in df_findings.columns:
            print(df_findings.groupby("severity").size())
    except ImportError:
        print("\n    pandas not installed - install with: pip install pandas")

    # --- Report serialization ---
    report = eggsec.Report()
    report.add_result(ports)
    for f in finding_set.findings:
        report.add_finding(f)

    print(f"\n[*] Report has {len(report.findings)} total findings")
    print(f"    JSON size: {len(report.to_json())} bytes")


if __name__ == "__main__":
    main()
