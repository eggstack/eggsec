"""Reconnaissance scan with structured reporting.

Combines DNS enumeration, TLS inspection, and technology detection
into a single Report that can be exported as JSON.

Note: Targets use scanme.nmap.org (Nmap's official test target).
"""

import eggsec


def main():
    target = "scanme.nmap.org"

    # --- DNS enumeration ---
    print(f"[*] Enumerating DNS records for {target}...")
    dns = eggsec.recon_dns(target)
    print(f"    A records:     {dns.a_records}")
    print(f"    AAAA records:  {dns.aaaa_records}")
    print(f"    MX records:    {[f'{mx.exchange} (pref {mx.preference})' for mx in dns.mx_records]}")
    print(f"    NS records:    {dns.ns_records}")
    print(f"    TXT records:   {dns.txt_records}")

    # --- TLS inspection ---
    print(f"\n[*] Inspecting TLS for {target}...")
    tls = eggsec.inspect_tls(target)
    if tls.has_ssl:
        cert = tls.certificate
        print(f"    Subject:       {cert.subject}")
        print(f"    Issuer:        {cert.issuer}")
        print(f"    Valid until:   {cert.valid_until}")
        print(f"    Days left:     {cert.days_until_expiry}")
        print(f"    Key size:      {cert.key_size} bit")
        for issue in tls.issues:
            print(f"    ISSUE [{issue.severity}]: {issue.description}")
    else:
        print("    No TLS detected")

    # --- Technology detection ---
    print(f"\n[*] Detecting technologies for http://{target}...")
    tech = eggsec.detect_technology(f"http://{target}")
    stack = tech.tech_stack
    print(f"    Servers:   {stack.servers}")
    print(f"    Languages: {stack.languages}")
    print(f"    CDNs:      {stack.cdns}")
    print(f"    CMS:       {stack.cms}")

    # --- Build a report ---
    print("\n[*] Building report...")
    report = eggsec.Report({
        "title": f"Recon Report: {target}",
        "target": target,
        "operator": "recon_report.py",
    })

    # Add DNS findings
    for a_ip in dns.a_records:
        finding = eggsec.Finding(
            title=f"A record: {a_ip}",
            description=f"DNS A record for {target} resolves to {a_ip}",
            severity=eggsec.Severity.INFO,
            location=f"dns://{target}",
        )
        report.add_finding(finding)

    # Add TLS findings
    if tls.has_ssl and tls.certificate.is_expired:
        finding = eggsec.Finding(
            title="Expired TLS certificate",
            description=f"Certificate expired on {tls.certificate.valid_until}",
            severity=eggsec.Severity.HIGH,
            location=f"https://{target}",
        )
        report.add_finding(finding)

    # Export as JSON
    print("\n[*] Report as JSON:")
    print(report.to_json()[:500], "...")

    report.write_json("recon_report.json")
    report.write_markdown("recon_report.md")
    print("\n[+] Wrote recon_report.json and recon_report.md")


if __name__ == "__main__":
    main()
