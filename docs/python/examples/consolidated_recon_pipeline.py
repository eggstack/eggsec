#!/usr/bin/env python3
"""
Consolidated Recon Pipeline.

Demonstrates running a multi-module reconnaissance pipeline using
`run_consolidated_recon` with various module toggles.

Requirements:
    - eggsec installed (consolidated-recon is in the default wheel)
    - Network access to the target

Usage:
    python consolidated_recon_pipeline.py [target]
"""

import sys

import eggsec
from eggsec import Engine, Scope, ConsolidatedReconConfig


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "example.com"

    # Create engine
    scope = Scope.allow_hosts([target])
    engine = Engine(scope)

    # Configure which recon modules to run
    config = ConsolidatedReconConfig(
        target=target,
        run_dns=True,
        run_ssl=True,
        run_tech_detect=True,
        run_subdomain=False,
        run_whois=False,
        run_cors=True,
        run_wayback=False,
        run_js_analysis=False,
        run_content=False,
        run_email=False,
        timeout_ms=60000,
    )

    print(f"Running consolidated recon against {target}...")
    print(f"Modules: DNS, SSL, Tech Detect, CORS")

    result = engine.run_consolidated_recon(config)

    if result.status.name() == "Completed":
        report = result.payload
        print(f"\nRecon complete for {target}")

        # DNS results
        if report.dns:
            print(f"\nDNS Records:")
            for record in report.dns.records:
                print(f"  {record.record_type}: {record.value}")

        # SSL/TLS results
        if report.ssl:
            print(f"\nTLS Certificate:")
            cert = report.ssl.certificate
            print(f"  Subject: {cert.subject}")
            print(f"  Issuer: {cert.issuer}")
            print(f"  Expires: {cert.not_after}")

        # Technology detection
        if report.tech_detect:
            print(f"\nTechnologies detected:")
            for tech in report.tech_detect.technologies:
                print(f"  {tech.name} ({tech.category})")

        # CORS
        if report.cors:
            print(f"\nCORS Configuration:")
            print(f"  Allow Origin: {report.cors.allow_origin}")
            print(f"  Allow Credentials: {report.cors.allow_credentials}")

        # Summary
        print(f"\nModules executed: {report.modules_executed}")
        print(f"Total findings: {report.total_findings}")

    elif result.status.name() == "Failed":
        error = result.error
        print(f"Recon failed ({error.kind}): {error.message}")
        sys.exit(1)
    else:
        print(f"Unexpected status: {result.status.name()}")
        sys.exit(1)


if __name__ == "__main__":
    main()
