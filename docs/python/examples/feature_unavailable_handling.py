#!/usr/bin/env python3
"""Feature-unavailable handling and experimental namespace inspection.

Demonstrates how to detect which features are compiled into the current
wheel, handle missing features gracefully, inspect the experimental
namespace, and use `domain_maturity()` to determine release boundaries.

Requirements:
    - eggsec (default features)

Usage:
    python3 docs/python/examples/feature_unavailable_handling.py
"""

import eggsec
from eggsec import (
    features,
    has_feature,
    build_info,
    wheel_profile,
    api_surface_version,
)


def demo_feature_detection():
    """Check which features are available in the current build."""
    available = features()
    print(f"Compiled features ({len(available)}):")
    for f in sorted(available):
        print(f"  + {f}")

    # Check specific features by name
    checks = [
        "websocket",
        "git-secrets",
        "sbom",
        "db-pentest",
        "web-proxy",
        "nse",
        "mobile",
        "container",
        "headless-browser",
        "packet-inspection",
        "stress-testing",
        "wireless",
        "evasion",
        "postex",
        "c2",
    ]
    print("\nFeature availability:")
    for name in checks:
        available_flag = has_feature(name)
        marker = "+" if available_flag else "-"
        print(f"  [{marker}] {name}")


def demo_build_diagnostics():
    """Show build metadata for issue reporting."""
    info = build_info()
    print("\nBuild info:")
    for key in sorted(info.keys()):
        val = info[key]
        if isinstance(val, list):
            val = ", ".join(str(v) for v in val)
        print(f"  {key}: {val}")

    profile = wheel_profile()
    print(f"\nWheel profile: {profile}")

    version = api_surface_version()
    print(f"API surface version:")
    print(f"  package:  {version.get('package_version', '?')}")
    print(f"  schema:   {version.get('schema_version', '?')}")
    print(f"  protocol: {version.get('protocol_version', '?')}")
    print(f"  abi:      {version.get('abi_version', '?')}")


def demo_feature_guard():
    """Use _feature_guard to detect unavailable features at runtime."""
    from eggsec._feature_guard import list_unavailable_features, FeatureUnavailableError

    unavailable = list_unavailable_features()
    if unavailable:
        print(f"\nUnavailable features ({len(unavailable)}):")
        for feat in unavailable:
            print(f"  - {feat['feature']} (maturity={feat['maturity']})")
            if feat.get("install_hint"):
                print(f"    install: {feat['install_hint']}")
            if feat.get("platform_prereqs"):
                print(f"    platform: {feat['platform_prereqs']}")
    else:
        print("\nAll features are available.")

    # Demonstrate catching FeatureUnavailableError
    print("\nFeatureUnavailableError demo:")
    try:
        # Try to import a feature-gated symbol that may not be available
        from eggsec.experimental import __all__ as exp_all
        if exp_all:
            print(f"  experimental namespace has {len(exp_all)} symbols")
        else:
            print("  experimental namespace is empty (no feature-gated symbols compiled)")
    except ImportError as e:
        print(f"  ImportError: {e}")


def demo_domain_maturity():
    """Inspect domain maturity for release boundary decisions."""
    try:
        maturity = eggsec.domain_maturity()
        print(f"\nDomain maturity ({len(maturity)} domains):")
        for domain_id, info in sorted(maturity.items()):
            status = info.get("status", "?") if isinstance(info, dict) else info
            feature = info.get("required_feature", "") if isinstance(info, dict) else ""
            feat_str = f" (feature: {feature})" if feature else ""
            print(f"  {domain_id}: {status}{feat_str}")
    except AttributeError:
        print("\ndomain_maturity() not available in this build")


def main():
    demo_feature_detection()
    demo_build_diagnostics()
    demo_feature_guard()
    demo_domain_maturity()


if __name__ == "__main__":
    main()
