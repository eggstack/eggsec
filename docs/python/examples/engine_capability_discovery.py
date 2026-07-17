#!/usr/bin/env python3
"""Engine capability discovery at runtime.

Demonstrates querying the installed wheel for available features, build
metadata, API surface version, and domain maturity levels. All calls
are pure introspection -- no network or file I/O required.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/engine_capability_discovery.py
"""

import eggsec


def main():
    # Feature flags -- what is compiled into this wheel
    features = eggsec.features()
    print(f"Total features: {len(features)}")
    enabled = [k for k, v in features.items() if v]
    print(f"Enabled: {', '.join(enabled[:10])}{'...' if len(enabled) > 10 else ''}")

    # Individual feature check
    print(f"\nHas 'nse': {eggsec.has_feature('nse')}")
    print(f"Has 'db-pentest': {eggsec.has_feature('db-pentest')}")

    # Build info
    info = eggsec.build_info()
    print(f"\nBuild info:")
    for key, value in info.items():
        print(f"  {key}: {value}")

    # API surface version
    version = eggsec.api_surface_version()
    print(f"\nAPI surface version: {version}")

    # Domain maturity
    maturity = eggsec.domain_maturity()
    print(f"\nDomain maturity entries: {len(maturity)}")
    for domain, level in list(maturity.items())[:8]:
        print(f"  {domain}: {level}")
    if len(maturity) > 8:
        print(f"  ... and {len(maturity) - 8} more")


if __name__ == "__main__":
    main()
