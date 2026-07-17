#!/usr/bin/env python3
"""Stub parity checker for eggsec-python.

Compares runtime __all__ exports against .pyi stub declarations and
the machine-readable api_surface() to detect:
  - Symbols in __all__ but missing from stubs
  - Symbols in stubs but not in __all__ (extra)
  - Symbols in api_surface() but not in __all__
  - Key class method signatures mismatched

Usage:
    python scripts/check_python_stub_parity.py
    # Requires: eggsec installed via maturin develop or wheel
"""

import ast
import inspect
import os
import sys
from pathlib import Path

EGGSEC_DIR = Path(__file__).resolve().parent.parent / "crates" / "eggsec-python" / "python" / "eggsec"
STUB_DIR = EGGSEC_DIR

# Key classes and their expected methods/properties
EXPECTED_CLASS_MEMBERS = {
    "Engine": {
        "methods": ["run", "list_operations", "has_operation", "audit_events", "close", "plan"],
        "properties": ["scope", "mode", "concurrency", "timeout_ms"],
    },
    "AsyncEngine": {
        "methods": ["run", "list_operations", "has_operation", "audit_events", "close", "plan"],
        "properties": ["scope", "mode", "concurrency", "timeout_ms"],
    },
    "Scope": {
        "methods": ["allow_hosts", "allow_cidrs", "deny_all", "from_file", "is_target_allowed", "is_port_allowed"],
        "properties": [],
    },
    "Client": {
        "methods": ["close"],
        "properties": [],
    },
    "AsyncClient": {
        "methods": ["close"],
        "properties": [],
    },
}


def parse_init_all(init_path: Path) -> list[str]:
    """Parse __all__ from __init__.py AST."""
    tree = ast.parse(init_path.read_text())
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "__all__":
                    if isinstance(node.value, (ast.List, ast.Tuple)):
                        return [
                            elt.value
                            for elt in node.value.elts
                            if isinstance(elt, ast.Constant)
                        ]
    return []


def collect_stub_exports(stub_dir: Path) -> dict[str, set[str]]:
    """Collect all names exported by .pyi files, grouped by file."""
    exports: dict[str, set[str]] = {}
    for stub_file in stub_dir.glob("*.pyi"):
        names: set[str] = set()
        try:
            tree = ast.parse(stub_file.read_text())
            for node in ast.walk(tree):
                if isinstance(node, ast.ImportFrom):
                    for alias in node.names:
                        name = alias.asname if alias.asname else alias.name
                        if name != "*":
                            names.add(name)
                elif isinstance(node, ast.Assign):
                    for target in node.targets:
                        if isinstance(target, ast.Name):
                            names.add(target.id)
                elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    names.add(node.name)
                elif isinstance(node, ast.ClassDef):
                    names.add(node.name)
        except Exception as exc:
            print(f"  Warning: failed to parse {stub_file.name}: {exc}", file=sys.stderr)
        exports[stub_file.name] = names
    return exports


def collect_all_stub_names(exports: dict[str, set[str]]) -> set[str]:
    """Union of all stub-exported names."""
    result: set[str] = set()
    for names in exports.values():
        result |= names
    return result


def check_api_surface_vs_all(api_surface: dict, all_names: list[str]) -> tuple[list[str], list[str]]:
    """Return (in_surface_not_in_all, in_all_not_in_surface)."""
    surface_keys = set(api_surface.keys())
    all_set = set(all_names)
    return sorted(surface_keys - all_set), sorted(all_set - surface_keys)


def check_class_methods() -> list[str]:
    """Verify key classes have expected methods at runtime."""
    issues = []
    try:
        import eggsec
    except ImportError:
        return ["eggsec not importable"]

    for class_name, expected in EXPECTED_CLASS_MEMBERS.items():
        cls = getattr(eggsec, class_name, None)
        if cls is None:
            issues.append(f"{class_name}: class not found")
            continue
        for method in expected["methods"]:
            if not hasattr(cls, method):
                issues.append(f"{class_name}.{method}: not found")
        for prop in expected["properties"]:
            if not hasattr(cls, prop):
                issues.append(f"{class_name}.{prop}: property not found")
    return issues


def check_stub_method_signatures() -> list[str]:
    """Check that key .pyi stubs declare expected method signatures."""
    issues = []
    engine_stub = STUB_DIR / "engine.pyi"
    if engine_stub.exists():
        tree = ast.parse(engine_stub.read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name == "Engine":
                methods = {
                    n.name: n
                    for n in node.body
                    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
                }
                if "run" in methods:
                    run_method = methods["run"]
                    params = [arg.arg for arg in run_method.args.args]
                    if "request" not in params:
                        issues.append("engine.pyi: Engine.run missing 'request' parameter")
                else:
                    issues.append("engine.pyi: Engine.run not declared")
            if isinstance(node, ast.ClassDef) and node.name == "AsyncEngine":
                methods = {
                    n.name: n
                    for n in node.body
                    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
                }
                if "run" in methods:
                    run_method = methods["run"]
                    params = [arg.arg for arg in run_method.args.args]
                    if "request" not in params:
                        issues.append("engine.pyi: AsyncEngine.run missing 'request' parameter")
                else:
                    issues.append("engine.pyi: AsyncEngine.run not declared")

    scope_stub = STUB_DIR / "scope.pyi"
    if scope_stub.exists():
        tree = ast.parse(scope_stub.read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name == "Scope":
                methods = {
                    n.name: n
                    for n in node.body
                    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
                }
                for expected_method in ["allow_hosts", "allow_cidrs", "deny_all"]:
                    if expected_method not in methods:
                        issues.append(f"scope.pyi: Scope.{expected_method} not declared")
    return issues


def main() -> int:
    failures = 0

    print("=== eggsec-python Stub Parity Check ===")
    print()

    # Step 1: Load __all__
    init_path = EGGSEC_DIR / "__init__.py"
    all_names = parse_init_all(init_path)
    if not all_names:
        print("FAIL: Could not parse __all__ from __init__.py")
        return 1
    print(f"__all__ contains {len(all_names)} entries")

    # Step 2: Collect stub exports
    stub_exports_by_file = collect_stub_exports(STUB_DIR)
    all_stub_names = collect_all_stub_names(stub_exports_by_file)
    print(f"Stubs export {len(all_stub_names)} names across {len(stub_exports_by_file)} files")
    print()

    # Step 3: __all__ vs stubs
    print("--- __all__ vs stub declarations ---")
    in_all_not_in_stubs = sorted(set(all_names) - all_stub_names)
    in_stubs_not_in_all = sorted(all_stub_names - set(all_names))

    if in_all_not_in_stubs:
        # Filter out private names and known feature-gated names
        public_missing = [n for n in in_all_not_in_stubs if not n.startswith("_")]
        if public_missing:
            print(f"  WARN: {len(public_missing)} __all__ names not in any .pyi stub:")
            for name in public_missing[:20]:
                print(f"    - {name}")
            if len(public_missing) > 20:
                print(f"    ... and {len(public_missing) - 20} more")
            failures += 1
        else:
            print("  OK: All public __all__ names present in stubs")
    else:
        print("  OK: All __all__ names present in stubs")

    if in_stubs_not_in_all:
        public_extra = [n for n in in_stubs_not_in_all if not n.startswith("_")]
        if public_extra:
            print(f"  INFO: {len(public_extra)} stub names not in __all__ (extra in stubs):")
            for name in public_extra[:10]:
                print(f"    + {name}")
            if len(public_extra) > 10:
                print(f"    ... and {len(public_extra) - 10} more")
    print()

    # Step 4: api_surface() vs __all__
    print("--- api_surface() vs __all__ ---")
    try:
        import eggsec
        api_surface = eggsec.api_surface()
        in_surface_not_in_all, in_all_not_in_surface = check_api_surface_vs_all(api_surface, all_names)
        if in_surface_not_in_all:
            print(f"  INFO: {len(in_surface_not_in_all)} api_surface() names not in __all__:")
            for name in in_surface_not_in_all[:10]:
                print(f"    + {name}")
        if in_all_not_in_surface:
            print(f"  WARN: {len(in_all_not_in_surface)} __all__ names not in api_surface():")
            for name in in_all_not_in_surface[:10]:
                print(f"    - {name}")
        if not in_surface_not_in_all and not in_all_not_in_surface:
            print("  OK: api_surface() and __all__ are in sync")
    except ImportError:
        print("  FAIL: eggsec not importable — stub parity check requires installed package")
        failures += 1
    except Exception as e:
        print(f"  WARN: api_surface() check failed: {e}")
    print()

    # Step 5: Key class methods
    print("--- Key class method checks ---")
    method_issues = check_class_methods()
    if method_issues:
        print(f"  FAIL: {len(method_issues)} issues:")
        for issue in method_issues:
            print(f"    - {issue}")
        failures += 1
    else:
        print("  OK: All key class methods present")
    print()

    # Step 6: Stub method signature checks
    print("--- Stub method signature checks ---")
    sig_issues = check_stub_method_signatures()
    if sig_issues:
        print(f"  FAIL: {len(sig_issues)} signature issues:")
        for issue in sig_issues:
            print(f"    - {issue}")
        failures += 1
    else:
        print("  OK: All key stub signatures correct")
    print()

    # Summary
    print("=== Summary ===")
    if failures:
        print(f"RESULT: FAILED ({failures} issue(s))")
        return 1
    else:
        print("RESULT: PASSED")
        return 0


if __name__ == "__main__":
    sys.exit(main())
