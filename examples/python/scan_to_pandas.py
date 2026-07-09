"""Convert eggsec scan results to pandas DataFrames.

Demonstrates how to use to_rows() for easy conversion to pandas,
useful for analysis, visualization, and Jupyter notebooks.

Note: Scan targets use scanme.nmap.org (Nmap's official test target).
"""

import eggsec


def main():
    # Use Nmap's official test target (authorized for scanning)
    target = "scanme.nmap.org"
    scope = eggsec.Scope.allow_hosts([target])

    # --- Port scan ---
    print(f"[*] Running port scan on {target}...")
    ports = eggsec.scan_ports(
        target=target,
        ports=[22, 80, 443, 9929, 31337],
        scope=scope,
        timeout_ms=5000,
    )

    # Convert to rows (list of dicts)
    port_rows = ports.to_rows()
    print(f"    Port scan rows: {len(port_rows)}")
    for row in port_rows:
        print(f"      {row}")

    # --- Endpoint scan ---
    print(f"\n[*] Running endpoint scan on {target}...")
    endpoints = eggsec.scan_endpoints(
        base_url=f"http://{target}",
        endpoints=["/", "/nonexistent"],
        scope=scope,
        timeout_ms=5000,
    )

    endpoint_rows = endpoints.to_rows()
    print(f"    Endpoint scan rows: {len(endpoint_rows)}")
    for row in endpoint_rows:
        print(f"      {row}")

    # --- Findings ---
    print("\n[*] Creating findings...")
    finding = eggsec.Finding(
        title="Test finding",
        description="A demonstration finding",
        severity=eggsec.Severity.MEDIUM,
        evidence=[
            eggsec.Evidence(kind="header", value="X-Test: true"),
            eggsec.Evidence(kind="body", value="sample output"),
        ],
    )

    finding_set = eggsec.FindingSet()
    finding_set.add(finding)
    finding_set.add(eggsec.Finding(
        title="Info disclosure",
        severity=eggsec.Severity.LOW,
    ))

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
        df_endpoints = pd.DataFrame(endpoint_rows)
        df_findings = pd.DataFrame(finding_rows)

        print(f"\n[*] pandas DataFrames:")
        print(f"    ports:      {df_ports.shape}")
        print(f"    endpoints:  {df_endpoints.shape}")
        print(f"    findings:   {df_findings.shape}")
        print(f"\n[*] Findings by severity:")
        if not df_findings.empty and "severity" in df_findings.columns:
            print(df_findings.groupby("severity").size())
    except ImportError:
        print("\n    pandas not installed - install with: pip install pandas")


if __name__ == "__main__":
    main()
