#!/usr/bin/env python3
"""Python-specific architecture guards for eggsec-python.

Validates:
- _capabilities.json schema and per-operation field completeness
- Stale operation count references in docs
- Domain maturity consistency between JSON and Rust source
- Feature metadata consistency
"""
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CAPS_JSON = REPO_ROOT / "crates" / "eggsec-python" / "python" / "eggsec" / "_capabilities.json"
DOMAINS_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "domains.rs"
REGISTRY_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "operation_registry.rs"

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
        print("SKIP: _capabilities.json not found.")

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
