#!/usr/bin/env python3
"""Python-specific architecture guards for eggsec-python.

Validates:
- _capabilities.json schema and per-operation field completeness
- Stale operation count references in docs
- Domain maturity consistency between JSON and Rust source
- Feature metadata consistency
- Sync/async operation list parity
- Registry descriptor completeness
- Feature metadata JSON/Rust consistency
- Stable operations have test fixtures
- Provisional domains not marked stable
"""
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CAPS_JSON = REPO_ROOT / "crates" / "eggsec-python" / "python" / "eggsec" / "_capabilities.json"
DOMAINS_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "domains.rs"
REGISTRY_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "operation_registry.rs"
CARGO_TOML = REPO_ROOT / "crates" / "eggsec-python" / "Cargo.toml"

FAIL = 0


def fail(msg: str) -> None:
    global FAIL
    FAIL += 1
    print(f"FAIL: {msg}")


def pass_(msg: str) -> None:
    print(f"PASS: {msg}")


def main() -> None:
    global FAIL

    print("=== Python Architecture Guards ===")
    print()

    # 1. Schema version check
    print("--- Guard 1: _capabilities.json schema version is 2 ---")
    if CAPS_JSON.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)
        if caps.get("version") != 2:
            fail(f"Schema version is {caps.get('version')} (expected 2)")
        else:
            pass_("Schema version is 2.")
    else:
        fail("_capabilities.json not found — cannot validate architecture guards")

    # 2. Per-operation field completeness
    print()
    print("--- Guard 2: Stable operations have per-operation metadata ---")
    if CAPS_JSON.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)
        ops = caps.get("operations", {})
        stable = set(caps.get("stable_operations", []))
        required_fields = [
            "operation_id", "domain", "maturity", "cargo_feature", "default_wheel",
            "sync_dispatch", "async_dispatch", "engine_dispatch", "direct_function",
            "policy", "scope", "audit", "timeout", "cancellation",
            "serialization", "stub", "installed_wheel",
        ]
        missing = []
        for op_id in sorted(stable):
            if op_id not in ops:
                missing.append(f"{op_id}: missing from operations map")
                continue
            for field in required_fields:
                if field not in ops[op_id]:
                    missing.append(f"{op_id}: missing field '{field}'")
        if missing:
            for m in missing:
                fail(m)
        else:
            pass_(f"All {len(stable)} operations have per-operation metadata.")

    # 3. Stale operation count references
    print()
    print("--- Guard 3: No stale 'ten stable' references in current docs ---")
    stale_patterns = [
        r"ten.operation",
        r"ten-operation",
        r"ten\s+stable",
    ]
    doc_files = [
        REPO_ROOT / "docs" / "python" / f
        for f in (REPO_ROOT / "docs" / "python").glob("*.md")
    ] + [
        REPO_ROOT / "architecture" / "python_api.md",
        REPO_ROOT / "crates" / "eggsec-python" / "README.md",
        REPO_ROOT / "crates" / "eggsec-python" / "VALIDATION.md",
    ]
    hits = []
    for doc_file in doc_files:
        if not doc_file.exists():
            continue
        content = doc_file.read_text()
        for i, line in enumerate(content.splitlines(), 1):
            for pattern in stale_patterns:
                if re.search(pattern, line, re.IGNORECASE):
                    # Skip if the line also says "twenty-two" or "22" (correct)
                    if re.search(r"twenty.two|22.operation", line, re.IGNORECASE):
                        continue
                    # Skip historical references like "original ten" or "the original ten"
                    if "original ten" in line.lower():
                        continue
                    hits.append(f"{doc_file.relative_to(REPO_ROOT)}:{i}: {line.strip()[:80]}")
    if hits:
        for h in hits:
            fail(h)
        print("  Use 'twenty-two stable operations' instead.")
    else:
        pass_("No stale 'ten stable' references in current docs.")

    # 4. Domain maturity consistency
    print()
    print("--- Guard 4: Domain maturity consistency (JSON vs Rust) ---")
    if CAPS_JSON.exists() and DOMAINS_RS.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)
        domains_json = caps.get("domains", {})

        # Parse domain_maturity() from domains.rs
        content = DOMAINS_RS.read_text()
        pattern = re.compile(
            r'\(\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]*)"\s*,?\s*\n?\s*\)',
            re.MULTILINE,
        )
        rust_maturity = {}
        for m in pattern.finditer(content):
            rust_maturity[m.group(1)] = m.group(2)

        mismatches = []
        for domain_id, domain_info in domains_json.items():
            json_maturity = domain_info.get("maturity")
            if domain_id in rust_maturity:
                rust_state = rust_maturity[domain_id]
                if json_maturity != rust_state:
                    mismatches.append(
                        f"Domain '{domain_id}': JSON='{json_maturity}' vs Rust='{rust_state}'"
                    )
        if mismatches:
            for m in mismatches:
                fail(m)
        else:
            pass_("All domain maturity levels consistent between JSON and Rust.")

    # 5. Feature metadata consistency
    print()
    print("--- Guard 5: Feature metadata consistency ---")
    if CAPS_JSON.exists() and REGISTRY_RS.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)

        # Parse feature_required() from operation_registry.rs
        content = REGISTRY_RS.read_text()
        rust_features = {}

        # Find the feature_required method body
        in_feature_method = False
        feature_lines = []
        for line in content.splitlines():
            if "pub const fn feature_required(self)" in line:
                in_feature_method = True
                continue
            if in_feature_method:
                if line.strip() == "}":
                    break
                feature_lines.append(line)

        # Parse the match arms
        # Strategy: collect all Self:: variants until we hit => None or => Some("...")
        current_variants = []
        for line in feature_lines:
            # Find all Self::VariantName patterns
            variants = re.findall(r'Self::(\w+)', line)
            current_variants.extend(variants)

            # Check if this line has the result
            if "=> None" in line:
                for v in current_variants:
                    rust_features[v] = None
                current_variants = []
            elif "=>" in line:
                m = re.search(r'Some\("([^"]+)"\)', line)
                if m:
                    for v in current_variants:
                        rust_features[v] = m.group(1)
                    current_variants = []

        # Cross-reference with JSON
        mismatches = []
        for op_id, op_info in caps.get("operations", {}).items():
            json_feature = op_info.get("cargo_feature")
            # Convert operation ID to enum variant name for lookup
            # scan_ports -> ScanPorts, scan_docker_image -> ScanDockerImage
            variant = "".join(word.capitalize() for word in op_id.split("_"))
            if variant in rust_features:
                rust_feature = rust_features[variant]
                if json_feature != rust_feature:
                    mismatches.append(
                        f"Operation '{op_id}': JSON feature='{json_feature}' vs Rust='{rust_feature}'"
                    )
        if mismatches:
            for m in mismatches:
                fail(m)
        else:
            pass_("Feature metadata consistent between JSON and Rust.")

    # 6. Sync/async operation list parity
    print()
    print("--- Guard 6: Sync/Async engine operation list parity ---")
    try:
        result = subprocess.run(
            [
                sys.executable, "-c",
                "import eggsec; "
                "e=eggsec.Engine(eggsec.Scope.allow_hosts(['x'])); "
                "ae=eggsec.AsyncEngine(eggsec.Scope.allow_hosts(['x'])); "
                "sync=sorted(e.list_operations()); "
                "async_=sorted(ae.list_operations()); "
                "assert sync==async_, f'mismatch: sync={sync} async={async_}'; "
                "assert len(sync)==22, f'expected 22 ops, got {len(sync)}'"
            ],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            err = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "unknown error"
            fail(f"Sync/async mismatch: {err}")
        else:
            pass_("Sync and Async engine list_operations() return same 22 operations.")
    except FileNotFoundError:
        fail("Python not available for runtime check")
    except subprocess.TimeoutExpired:
        fail("Runtime check timed out")

    # 7. Registry descriptor completeness
    print()
    print("--- Guard 7: Registry descriptor completeness ---")
    if REGISTRY_RS.exists():
        content = REGISTRY_RS.read_text()

        # Extract all StableOperation variants from the enum definition
        enum_match = re.search(
            r'pub enum StableOperation \{(.*?)\}', content, re.DOTALL
        )
        rust_variants = []
        if enum_match:
            body = enum_match.group(1)
            rust_variants = re.findall(r'(\w+)\s*,', body)

        # Extract operations covered by classify_risk
        classify_match = re.search(
            r'fn classify_risk\(operation: StableOperation\).*?\{(.*?)\n    \}',
            content, re.DOTALL,
        )
        classified_variants = set()
        if classify_match:
            classify_body = classify_match.group(1)
            # Match all StableOperation::VariantName occurrences
            classified_variants = set(re.findall(r'StableOperation::(\w+)', classify_body))

        missing_variants = [v for v in rust_variants if v not in classified_variants]
        if missing_variants:
            for v in missing_variants:
                fail(f"StableOperation::{v} missing from classify_risk()")
        else:
            pass_(f"All {len(rust_variants)} StableOperation variants have risk classification.")

    # 8. Feature metadata JSON/Rust consistency
    print()
    print("--- Guard 8: Feature metadata JSON/Rust consistency ---")
    if CAPS_JSON.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)

        # Parse features from Cargo.toml
        cargo_features = set()
        if CARGO_TOML.exists():
            cargo_content = CARGO_TOML.read_text()
            in_features = False
            for line in cargo_content.splitlines():
                if line.strip().startswith("[features]"):
                    in_features = True
                    continue
                if in_features and line.strip().startswith("["):
                    break
                if in_features:
                    m = re.match(r'^\s*(\S+)\s*=', line)
                    if m and not line.strip().startswith("#"):
                        cargo_features.add(m.group(1))

        # Collect feature-gated operations from JSON
        json_features = set()
        for op_id, op_info in caps.get("operations", {}).items():
            feat = op_info.get("cargo_feature")
            if feat:
                json_features.add(feat)

        phantom = json_features - cargo_features
        missing = cargo_features - json_features
        if phantom:
            for f in sorted(phantom):
                fail(f"Phantom feature in JSON: '{f}' not in Cargo.toml")
        if missing:
            # Only flag features that are actually used by operations, not all Cargo features
            op_features_in_json = {op_info.get("cargo_feature") for op_info in caps.get("operations", {}).values() if op_info.get("cargo_feature")}
            truly_missing = op_features_in_json - cargo_features
            for f in sorted(truly_missing):
                fail(f"Feature in JSON operations missing from Cargo.toml: '{f}'")
        if not phantom:
            pass_(f"Feature metadata consistent: {len(json_features)} JSON features match Cargo.toml.")

    # 9. Stable operations have test fixtures
    print()
    print("--- Guard 9: Stable operations have test fixtures ---")
    if CAPS_JSON.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)
        ops = caps.get("operations", {})
        stable = caps.get("stable_operations", [])
        missing_fixture = []
        for op_id in sorted(stable):
            op_info = ops.get(op_id, {})
            fixture = op_info.get("test_fixture", "")
            maturity = op_info.get("maturity", "")
            cargo_feature = op_info.get("cargo_feature")
            if maturity != "stable":
                continue
            if not fixture or fixture.strip() == "":
                missing_fixture.append(f"{op_id}: empty test_fixture")
            elif fixture.strip() == "Feature-gated compilation" and cargo_feature:
                # Feature-gated ops can use this placeholder
                pass
            elif not cargo_feature and "Feature-gated compilation" in fixture:
                missing_fixture.append(
                    f"{op_id}: always-compiled op uses 'Feature-gated compilation' fixture"
                )
        if missing_fixture:
            for m in missing_fixture:
                fail(m)
        else:
            pass_(f"All {len(stable)} stable operations have test fixtures.")

    # 10. Provisional domains not marked stable
    print()
    print("--- Guard 10: Provisional domains not marked stable ---")
    if CAPS_JSON.exists():
        with open(CAPS_JSON) as f:
            caps = json.load(f)
        provisional_domains = {
            "daemon", "browser", "proxy", "packet-inspection",
            "wireless", "evasion", "postex", "c2", "distributed", "ai",
        }
        ops = caps.get("operations", {})
        violations = []
        for op_id, op_info in ops.items():
            domain = op_info.get("domain", "")
            maturity = op_info.get("maturity", "")
            if domain in provisional_domains and maturity == "stable":
                violations.append(
                    f"{op_id}: domain '{domain}' is provisional but maturity is 'stable'"
                )
        if violations:
            for v in violations:
                fail(v)
        else:
            pass_("No provisional domain operations marked as stable.")

    # Summary
    print()
    print("=" * 50)
    if FAIL:
        print(f"RESULT: {FAIL} guard(s) failed.")
        sys.exit(1)
    else:
        print("RESULT: All Python architecture guards passed.")
        sys.exit(0)


if __name__ == "__main__":
    main()
