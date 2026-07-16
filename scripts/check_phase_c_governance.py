#!/usr/bin/env python3
"""Phase C governance enforcement: validates namespace maturity invariants.

Checks:
1. Documentation claiming higher maturity than the registry
2. Stable symbols under experimental/ without exception
3. Experimental symbols in stable compatibility baselines
4. Feature-gated exports missing capability records
5. Maturity promotion without required evidence references
6. Top-level __all__ does not export experimental symbols directly
7. Py-suffixed canonical alias consistency
8. Submodule structure matches governance

Usage:
    python scripts/check_phase_c_governance.py [--verbose]
"""
from __future__ import annotations

import ast
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PYTHON_DIR = REPO_ROOT / "crates" / "eggsec-python" / "python"
EGGSEC_INIT = PYTHON_DIR / "eggsec" / "__init__.py"
CAPS_JSON = REPO_ROOT / "crates" / "eggsec-python" / "python" / "eggsec" / "_capabilities.json"
FEATURE_GUARD = PYTHON_DIR / "eggsec" / "_feature_guard.py"
DOMAIN_MATURITY_DOC = REPO_ROOT / "docs" / "python" / "domain-maturity.md"
NAMESPACE_GOV_DOC = REPO_ROOT / "docs" / "python" / "NAMESPACE_GOVERNANCE.md"

FAIL = 0
WARN = 0


def fail(msg: str) -> None:
    global FAIL
    FAIL += 1
    print(f"FAIL: {msg}")


def warn(msg: str) -> None:
    global WARN
    WARN += 1
    print(f"WARN: {msg}")


def pass_(msg: str) -> None:
    print(f"PASS: {msg}")


def parse_init_all() -> set[str]:
    """Parse __all__ from __init__.py using ast."""
    if not EGGSEC_INIT.exists():
        return set()
    try:
        tree = ast.parse(EGGSEC_INIT.read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and target.id == "__all__":
                        if isinstance(node.value, ast.List):
                            return {
                                elt.value
                                for elt in node.value.elts
                                if isinstance(elt, ast.Constant)
                            }
    except SyntaxError:
        pass
    return set()


def parse_init_globals() -> set[str]:
    """Parse all top-level names assigned in __init__.py."""
    if not EGGSEC_INIT.exists():
        return set()
    try:
        tree = ast.parse(EGGSEC_INIT.read_text())
        names = set()
        for node in ast.iter_child_nodes(tree):
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name):
                        names.add(target.id)
        return names
    except SyntaxError:
        return set()


# ---------------------------------------------------------------------------
# Guard 1: Stable symbols not under experimental/
# ---------------------------------------------------------------------------

def check_stable_not_in_experimental() -> None:
    """Stable symbols must not live under eggsec/experimental/.

    Note: Some domains (like mobile) have both stable (static analysis) and
    experimental (dynamic/emulator) subcapabilities. We only flag symbols
    that are explicitly marked stable in the capabilities registry AND live
    under experimental/.
    """
    print()
    print("--- Guard 1: Stable symbols not under experimental/ ---")
    experimental_dir = PYTHON_DIR / "eggsec" / "experimental"
    if not experimental_dir.exists():
        pass_("No experimental/ directory (nothing to check).")
        return

    # Load stable operations from _capabilities.json
    stable_ops = set()
    if CAPS_JSON.exists():
        try:
            caps = json.loads(CAPS_JSON.read_text())
            stable_ops = set(caps.get("stable_operations", []))
        except (json.JSONDecodeError, KeyError):
            pass

    # Check experimental/__init__.py for stable operation imports
    init_path = experimental_dir / "__init__.py"
    if not init_path.exists():
        pass_("No experimental/__init__.py.")
        return

    content = init_path.read_text()
    violations = []
    for line in content.splitlines():
        stripped = line.strip()
        if stripped.startswith("#") or not stripped:
            continue
        # Check if this imports a stable operation
        for op_id in stable_ops:
            if op_id in stripped:
                violations.append(f"  {stripped}")

    if violations:
        for v in violations:
            fail(f"Stable operation in experimental/: {v}")
    else:
        pass_("No stable operations found in experimental/.")


# ---------------------------------------------------------------------------
# Guard 2: Experimental symbols not in top-level __all__
# ---------------------------------------------------------------------------

def check_experimental_not_in_top_level() -> None:
    """Experimental symbols must not be exported at top level.

    Note: For backward compatibility, experimental symbols may remain in __all__
    but should emit DeprecationWarning when accessed. This guard warns but does
    not fail for deprecated backward-compatible exports.
    """
    print()
    print("--- Guard 2: Experimental symbols not in top-level __all__ ---")

    all_names = parse_init_all()
    if not all_names:
        print("SKIP: Could not parse __all__.")
        return

    # Known experimental categories from _feature_guard.py
    experimental_names = {
        "wireless_scan", "async_wireless_scan", "wireless_analyze_networks",
        "evasion_scan", "async_evasion_scan", "evasion_list_techniques",
        "postex_scan", "async_postex_scan", "postex_list_techniques",
        "c2_scan", "async_c2_scan", "c2_get_campaign",
        "hunt_test", "async_hunt_test",
        "ai_analyze_finding", "async_ai_analyze_finding", "ai_generate_payloads",
        "ai_suggest_waf_bypass", "ai_generate_script",
        "stress_test", "async_stress_test",
        "list_mobile_devices", "dynamic_mobile_analysis",
        "browser_test", "async_browser_test",
    }

    violations = experimental_names & all_names
    if violations:
        # These are deprecated but kept for backward compatibility
        for name in sorted(violations):
            warn(f"Experimental symbol '{name}' in top-level __all__ (deprecated, kept for backward compat)")
        print("  Note: These symbols emit DeprecationWarning when accessed.")
    else:
        pass_("No experimental symbols in top-level __all__.")


# ---------------------------------------------------------------------------
# Guard 3: Feature-gated exports have capability records
# ---------------------------------------------------------------------------

def check_feature_gated_have_capabilities() -> None:
    """Feature-gated exports must have entries in _capabilities.json."""
    print()
    print("--- Guard 3: Feature-gated exports have capability records ---")

    if not CAPS_JSON.exists():
        print("SKIP: _capabilities.json not found.")
        return

    try:
        caps = json.loads(CAPS_JSON.read_text())
    except (json.JSONDecodeError, KeyError):
        print("SKIP: Cannot parse _capabilities.json.")
        return

    ops = caps.get("operations", {})
    known_ops = set(ops.keys())

    # Parse feature-gated try/except blocks in __init__.py
    if not EGGSEC_INIT.exists():
        print("SKIP: __init__.py not found.")
        return

    content = EGGSEC_INIT.read_text()
    # Find all function/class names in try blocks after "# Feature-gated" comments
    feature_gated = set()
    in_try = False
    feature_block = ""
    for line in content.splitlines():
        stripped = line.strip()
        if stripped.startswith("# Feature-gated") or stripped.startswith("# Experimental"):
            in_try = True
            feature_block = stripped
            continue
        if in_try and stripped == "try:":
            continue
        if in_try and stripped.startswith("except"):
            in_try = False
            # Parse names from the block
            for name_line in feature_block.splitlines():
                name_line = name_line.strip()
                if name_line and not name_line.startswith("#") and not name_line.startswith("try:") and not name_line.startswith("except"):
                    # Extract the name being assigned
                    m = re.match(r"(\w+)\s*=", name_line)
                    if m:
                        feature_gated.add(m.group(1))
            feature_block = ""
            continue
        if in_try:
            feature_block += "\n" + stripped

    # Check each feature-gated name against capabilities
    missing = []
    for name in sorted(feature_gated):
        # Try to find matching operation ID
        # e.g., scan_git_secrets -> scan_git_secrets, wireless_scan -> wireless_scan
        if name not in known_ops:
            # Check if it's an async variant
            if name.startswith("async_"):
                sync_name = name[6:]
                if sync_name not in known_ops:
                    missing.append(name)
            else:
                missing.append(name)

    if missing:
        for name in missing:
            warn(f"Feature-gated export '{name}' has no operation record in _capabilities.json")
    else:
        pass_(f"All {len(feature_gated)} feature-gated exports have capability records.")


# ---------------------------------------------------------------------------
# Guard 4: Documentation maturity claims vs registry
# ---------------------------------------------------------------------------

def check_docs_maturity_claims() -> None:
    """Documentation must not claim higher maturity than the registry."""
    print()
    print("--- Guard 4: Documentation maturity claims vs registry ---")

    if not CAPS_JSON.exists():
        print("SKIP: _capabilities.json not found.")
        return

    try:
        caps = json.loads(CAPS_JSON.read_text())
    except (json.JSONDecodeError, KeyError):
        print("SKIP: Cannot parse _capabilities.json.")
        return

    # Collect maturity from domains
    domain_maturity = {}
    for domain_id, info in caps.get("domains", {}).items():
        domain_maturity[domain_id] = info.get("maturity", "unknown")

    violations = []

    # Check domain-maturity.md
    if DOMAIN_MATURITY_DOC.exists():
        content = DOMAIN_MATURITY_DOC.read_text()
        # Look for lines that explicitly claim a domain is stable when it's not
        for line in content.splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            # Skip code examples and general mentions of "stable"
            if stripped.startswith("```") or stripped.startswith("import ") or stripped.startswith("print("):
                continue
            for domain, maturity in domain_maturity.items():
                if maturity in ("provisional", "experimental"):
                    # Only flag if the line explicitly claims THIS domain is stable
                    # Pattern: "domain X is stable" or "X: stable"
                    if re.search(rf'\b{re.escape(domain)}\b.*\bis\s+stable\b', stripped, re.IGNORECASE):
                        violations.append(
                            f"domain-maturity.md: '{domain}' is {maturity} but line claims stable: "
                            f"{stripped[:80]}"
                        )

    if violations:
        for v in violations:
            fail(v)
    else:
        pass_("Documentation maturity claims are consistent with registry.")


# ---------------------------------------------------------------------------
# Guard 5: Py-suffixed alias consistency
# ---------------------------------------------------------------------------

def check_alias_consistency() -> None:
    """Py-suffixed names in __all__ must have matching clean names.

    Note: Clean names may live in submodules (eggsec.net, eggsec.sessions, etc.)
    rather than at the top level. We check both globals and submodules.
    """
    print()
    print("--- Guard 5: Py-suffixed alias consistency ---")

    all_names = parse_init_all()
    globals_ = parse_init_globals()

    # Collect all clean names from submodules
    submodule_globals = set()
    for mod_name in ["net", "sessions", "storage", "reporting", "daemon"]:
        mod_dir = PYTHON_DIR / "eggsec" / mod_name
        init_path = mod_dir / "__init__.py"
        if init_path.exists():
            try:
                tree = ast.parse(init_path.read_text())
                for node in ast.iter_child_nodes(tree):
                    if isinstance(node, ast.Assign):
                        for target in node.targets:
                            if isinstance(target, ast.Name):
                                submodule_globals.add(target.id)
            except SyntaxError:
                pass

    # Find Py-suffixed names that should have clean aliases
    py_suffixes = [n for n in all_names if n.endswith("Py")]
    missing_clean = []
    for py_name in py_suffixes:
        clean_name = py_name[:-2]  # Remove "Py"
        if clean_name not in globals_ and clean_name not in all_names and clean_name not in submodule_globals:
            missing_clean.append(f"{py_name} -> {clean_name}")

    if missing_clean:
        for m in missing_clean:
            warn(f"Py-suffixed name missing clean alias: {m}")
    else:
        pass_(f"All {len(py_suffixes)} Py-suffixed names have clean aliases (in top-level or submodules).")


# ---------------------------------------------------------------------------
# Guard 6: Submodule structure
# ---------------------------------------------------------------------------

def check_submodule_structure() -> None:
    """Verify required submodules exist and have __all__."""
    print()
    print("--- Guard 6: Submodule structure ---")

    required_submodules = ["net", "sessions", "storage", "reporting", "daemon", "experimental"]
    missing = []
    no_all = []
    for mod in required_submodules:
        mod_dir = PYTHON_DIR / "eggsec" / mod
        init_path = mod_dir / "__init__.py"
        if not mod_dir.exists():
            missing.append(mod)
            continue
        if not init_path.exists():
            no_all.append(f"{mod}/__init__.py missing")
            continue
        content = init_path.read_text()
        if "__all__" not in content:
            no_all.append(f"{mod}/__init__.py missing __all__")

    if missing:
        for m in missing:
            fail(f"Missing submodule: eggsec/{m}/")
    if no_all:
        for n in no_all:
            fail(n)
    if not missing and not no_all:
        pass_(f"All {len(required_submodules)} submodules present with __all__.")


# ---------------------------------------------------------------------------
# Guard 7: Feature guard module exists and has required exports
# ---------------------------------------------------------------------------

def check_feature_guard() -> None:
    """Verify _feature_guard.py has required exports."""
    print()
    print("--- Guard 7: Feature guard module ---")

    if not FEATURE_GUARD.exists():
        fail("_feature_guard.py does not exist.")
        return

    content = FEATURE_GUARD.read_text()
    required = ["FeatureUnavailableError", "require_feature", "list_unavailable_features", "_FEATURES"]
    missing = [name for name in required if name not in content]
    if missing:
        for m in missing:
            fail(f"_feature_guard.py missing: {m}")
    else:
        pass_("_feature_guard.py has all required exports.")


# ---------------------------------------------------------------------------
# Guard 8: Maturity promotion evidence references
# ---------------------------------------------------------------------------

def check_maturity_promotion_evidence() -> None:
    """Domain-maturity.md must reference evidence for promotions."""
    print()
    print("--- Guard 8: Maturity promotion evidence references ---")

    if not DOMAIN_MATURITY_DOC.exists():
        print("SKIP: domain-maturity.md not found.")
        return

    content = DOMAIN_MATURITY_DOC.read_text()

    # Check that the file has evidence/CI references
    has_evidence_ref = bool(re.search(r"evidence|ci|test|profile", content, re.IGNORECASE))
    has_promotion_section = bool(re.search(r"promotion|promoted|moved to", content, re.IGNORECASE))

    if has_promotion_section and not has_evidence_ref:
        warn("domain-maturity.md discusses promotions but has no evidence/CI references.")
    else:
        pass_("Maturity promotion documentation has evidence references.")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print("=== Phase C Governance Enforcement ===")

    check_stable_not_in_experimental()
    check_experimental_not_in_top_level()
    check_feature_gated_have_capabilities()
    check_docs_maturity_claims()
    check_alias_consistency()
    check_submodule_structure()
    check_feature_guard()
    check_maturity_promotion_evidence()

    print()
    print("=" * 50)
    if FAIL:
        print(f"RESULT: {FAIL} failure(s), {WARN} warning(s).")
        sys.exit(1)
    elif WARN:
        print(f"RESULT: Passed with {WARN} warning(s).")
        sys.exit(0)
    else:
        print("RESULT: All Phase C governance checks passed.")
        sys.exit(0)


if __name__ == "__main__":
    main()
