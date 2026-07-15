#!/usr/bin/env python3
"""Static-to-dynamic mobile workflow.

Demonstrates building a DynamicAnalysisPlan from a StaticAnalysisSummary
and using it to configure a dynamic assessment session.

Requirements:
    - eggsec with mobile feature

Usage:
    python3 docs/python/examples/mobile_static_to_dynamic.py
"""

import sys

import eggsec
from eggsec import (
    StaticAnalysisSummary,
    DynamicAnalysisPlan,
)


def main():
    features = eggsec.features()
    if not features.get("mobile", False):
        print("Error: 'mobile' feature not compiled.")
        print("Build with: maturin develop --features mobile")
        sys.exit(1)

    static = StaticAnalysisSummary(
        package_id="com.example.vulnerable",
        permissions=["INTERNET", "READ_CONTACTS", "WRITE_EXTERNAL_STORAGE"],
        activities=["LoginActivity", "DataExportActivity"],
        services=["BackgroundSyncService"],
    )
    print(f"Package: {static.package_id}")
    print(f"Permissions: {static.permissions}")

    plan = static.to_dynamic_plan()
    print(f"Dynamic plan targets: {len(plan.targets)}")
    for target in plan.targets:
        print(f"  - {target.target_type}: {target.identifier}")

    print(f"Permissions to test: {plan.permissions_to_test}")
    print(f"Use Frida: {plan.use_frida}")
    print(f"Instrumentation focus: {plan.instrumentation_focus}")


if __name__ == "__main__":
    main()
