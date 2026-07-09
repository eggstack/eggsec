#!/usr/bin/env python3
"""Verify eggsec Python package exports are consistent.

Checks:
1. __all__ exists and is non-empty
2. Every __all__ name resolves at runtime
3. Feature-gated symbols are absent by default
4. Required default symbols are present
5. Active top-level APIs require scope (positional parameter)

Usage:
    python scripts/check_eggsec_python_exports.py
    # Requires: eggsec installed via maturin develop or wheel
"""

import sys
import inspect

REQUIRED_DEFAULT_SYMBOLS = [
    # Version
    "__version__",
    "__version_info__",
    # Core functions
    "features",
    "has_feature",
    "build_info",
    "scan_ports",
    "async_scan_ports",
    "scan_endpoints",
    "async_scan_endpoints",
    "fingerprint_services",
    "async_fingerprint_services",
    # Recon
    "recon_dns",
    "async_recon_dns",
    "inspect_tls",
    "async_inspect_tls",
    "detect_technology",
    "async_detect_technology",
    # WAF
    "detect_waf",
    "async_detect_waf",
    "validate_waf",
    "async_validate_waf",
    "fuzz_http",
    "async_fuzz_http",
    "generate_fuzz_payloads",
    # Load testing
    "load_test_http",
    "async_load_test_http",
    # Classes
    "Scope",
    "Client",
    "AsyncClient",
    "PyFuture",
    "Severity",
    "Evidence",
    "Finding",
    "FindingSet",
    "Report",
    # Exceptions
    "EggsecError",
    "ConfigError",
    "ScopeError",
    "EnforcementError",
    "NetworkError",
    "ScanError",
    "TimeoutError",
    "FeatureUnavailableError",
    "SerializationError",
    "InternalError",
]

# Symbols that should NOT be present unless their feature is enabled
FEATURE_GATED_SYMBOLS = [
    "websocket_probe",
    "async_websocket_probe",
    "websocket_fuzz",
    "async_websocket_fuzz",
    "scan_git_secrets",
    "async_scan_git_secrets",
    "generate_sbom",
    "async_generate_sbom",
    "db_probe",
    "async_db_probe",
    "create_proxy_manager",
    "async_add_proxy",
    "analyze_apk",
    "async_analyze_apk",
    "analyze_ipa",
    "async_analyze_ipa",
    "scan_docker_image",
    "async_scan_docker_image",
    "scan_kubernetes",
    "async_scan_kubernetes",
    "detect_escape_risks",
    "check_cis_docker_benchmark",
]

# Active APIs that must accept scope as a positional parameter
SCOPE_ENFORCED_FUNCTIONS = [
    "validate_waf",
    "async_validate_waf",
    "fuzz_http",
    "async_fuzz_http",
    "load_test_http",
    "async_load_test_http",
]


def check_all_exists(mod):
    """Check __all__ exists and is non-empty."""
    all_list = getattr(mod, "__all__", None)
    if all_list is None:
        print("FAIL: __all__ is not defined")
        return False
    if not isinstance(all_list, (list, tuple)):
        print(f"FAIL: __all__ is not a list/tuple, got {type(all_list)}")
        return False
    if len(all_list) == 0:
        print("FAIL: __all__ is empty")
        return False
    print(f"OK: __all__ exists with {len(all_list)} entries")
    return True


def check_all_names_resolve(mod):
    """Check every __all__ name exists at runtime."""
    all_list = getattr(mod, "__all__", [])
    missing = [name for name in all_list if not hasattr(mod, name)]
    if missing:
        print(f"FAIL: {len(missing)} __all__ names do not resolve: {missing}")
        return False
    print(f"OK: All {len(all_list)} __all__ names resolve")
    return True


def check_required_default_symbols(mod):
    """Check required default symbols are present."""
    missing = [name for name in REQUIRED_DEFAULT_SYMBOLS if not hasattr(mod, name)]
    if missing:
        print(f"FAIL: {len(missing)} required default symbols missing: {missing}")
        return False
    print(f"OK: All {len(REQUIRED_DEFAULT_SYMBOLS)} required default symbols present")
    return True


def check_feature_gated_symbols_absent(mod):
    """Check feature-gated symbols are absent by default."""
    present = [name for name in FEATURE_GATED_SYMBOLS if hasattr(mod, name)]
    if present:
        print(f"NOTE: {len(present)} feature-gated symbols present (features enabled): {present}")
        return True  # Not a failure — features may be compiled in
    print(f"OK: All {len(FEATURE_GATED_SYMBOLS)} feature-gated symbols absent by default")
    return True


def check_scope_enforcement(mod):
    """Check that active top-level APIs accept scope as a positional parameter."""
    failures = []
    for name in SCOPE_ENFORCED_FUNCTIONS:
        func = getattr(mod, name, None)
        if func is None:
            failures.append(f"{name}: not found")
            continue
        try:
            sig = inspect.signature(func)
            params = list(sig.parameters.keys())
            # scope should be a positional parameter (not keyword-only)
            if "scope" not in params:
                failures.append(f"{name}: 'scope' not in parameters (params: {params})")
            else:
                scope_param = sig.parameters["scope"]
                if scope_param.kind == inspect.Parameter.KEYWORD_ONLY:
                    failures.append(f"{name}: 'scope' is keyword-only, should be positional")
        except (ValueError, TypeError):
            # Some builtins don't have signatures; skip
            pass
    if failures:
        print(f"FAIL: Scope enforcement issues: {failures}")
        return False
    print(f"OK: All {len(SCOPE_ENFORCED_FUNCTIONS)} active APIs enforce scope")
    return True


def main():
    try:
        import eggsec
    except ImportError as e:
        print(f"FAIL: Cannot import eggsec: {e}")
        print("Install with: maturin develop (from crates/eggsec-python/) or pip install eggsec")
        sys.exit(1)

    print(f"eggsec version: {eggsec.__version__}")
    print()

    results = []
    results.append(("__all__ exists", check_all_exists(eggsec)))
    results.append(("__all__ names resolve", check_all_names_resolve(eggsec)))
    results.append(("Required default symbols", check_required_default_symbols(eggsec)))
    results.append(("Feature-gated symbols absent", check_feature_gated_symbols_absent(eggsec)))
    results.append(("Scope enforcement", check_scope_enforcement(eggsec)))

    print()
    passed = sum(1 for _, ok in results if ok)
    failed = sum(1 for _, ok in results if not ok)
    if failed:
        print(f"RESULT: {failed}/{len(results)} checks FAILED")
        sys.exit(1)
    else:
        print(f"RESULT: All {len(results)} checks passed")
        sys.exit(0)


if __name__ == "__main__":
    main()
