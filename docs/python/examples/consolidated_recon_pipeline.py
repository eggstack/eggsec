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

from eggsec import (
    ConsolidatedReconConfig,
    run_consolidated_recon,
)


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else "example.com"

    # Configure which recon modules to run
    config = ConsolidatedReconConfig(
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
        timeout_secs=60,
    )

    print(f"Running consolidated recon against {target}...")
    print(f"Modules: DNS, SSL, Tech Detect, CORS")

    try:
        report = run_consolidated_recon(target, config)
    except Exception as e:
        print(f"Recon failed: {e}")
        sys.exit(1)

    print(f"\nRecon complete for {target}")

    modules_executed = [m for m in report.modules if m.success]
    print(f"\nModules executed: {len(modules_executed)}")
    for module in report.modules:
        status = "OK" if module.success else f"FAIL: {module.error}"
        print(f"  {module.module}: {status}")


if __name__ == "__main__":
    main()
