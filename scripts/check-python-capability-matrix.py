#!/usr/bin/env python3
"""CI drift check: verify eggsec-python capability matrix consistency.

Reads _capabilities.json and parses operation_registry.rs to detect drift
between the machine-readable manifest and the Rust source of truth.
Exits non-zero on any mismatch.
"""
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CAPABILITIES_JSON = REPO_ROOT / "crates" / "eggsec-python" / "python" / "eggsec" / "_capabilities.json"
OPERATION_REGISTRY_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "operation_registry.rs"
DOMAINS_RS = REPO_ROOT / "crates" / "eggsec-python" / "src" / "domains.rs"

FAIL = 0


def fail(msg: str) -> None:
    global FAIL
    FAIL += 1
    print(f"FAIL: {msg}")


def pass_(msg: str) -> None:
    print(f"PASS: {msg}")


def parse_stable_operations_from_rust(path: Path) -> list[str]:
    """Extract operation IDs from OP_* constants in operation_registry.rs."""
    content = path.read_text()
    ops = []
    for m in re.finditer(r'pub const OP_(\w+):\s*&str\s*=\s*"([^"]+)";', content):
        ops.append(m.group(2))
    return ops


def parse_stable_operation_enum_ids(path: Path) -> list[str]:
    """Extract operation IDs from StableOperation::ALL match arms."""
    content = path.read_text()
    # Find the id() method body and extract the string literals
    ids = []
    in_id_method = False
    for line in content.splitlines():
        if 'pub const fn id(self)' in line:
            in_id_method = True
            continue
        if in_id_method:
            if line.strip().startswith('}'):
                break
            m = re.search(r'=>\s*([A-Z_]+)', line)
            if m:
                const_name = m.group(1)
                # Map const name to the OP_* value
                ids.append(const_name)
    return ids


def parse_domain_maturity_from_rust(path: Path) -> dict[str, str]:
    """Extract domain -> state mappings from domain_maturity() in domains.rs."""
    content = path.read_text()
    result = {}
    # Match tuples in the entries array — handles both single-line and multi-line formats:
    #   ("domain", "state", "description"),
    # or:
    #   (
    #       "domain",
    #       "state",
    #       "description",
    #   ),
    pattern = re.compile(
        r'\(\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]*)"\s*,?\s*\n?\s*\)',
        re.MULTILINE,
    )
    for m in pattern.finditer(content):
        domain, state, _desc = m.groups()
        result[domain] = state
    return result


def main() -> None:
    global FAIL

    print("=== Python Capability Matrix Drift Check ===")
    print()

    # 1. Load capabilities.json
    if not CAPABILITIES_JSON.exists():
        fail(f"Missing {CAPABILITIES_JSON}")
        sys.exit(1)

    with open(CAPABILITIES_JSON) as f:
        caps = json.load(f)

    json_ops = set(caps["stable_operations"])
    json_domains = caps["domains"]

    # 2. Parse operation_registry.rs
    rust_ops = set(parse_stable_operations_from_rust(OPERATION_REGISTRY_RS))

    # 3. Verify: every operation in JSON exists in registry
    print("--- Check 1: JSON operations exist in Rust registry ---")
    json_only = json_ops - rust_ops
    if json_only:
        for op in sorted(json_only):
            fail(f"Operation '{op}' in _capabilities.json but not in operation_registry.rs")
    else:
        pass_("All JSON operations found in Rust registry.")

    # 4. Verify: every registry operation appears in JSON
    print()
    print("--- Check 2: Rust registry operations appear in JSON ---")
    rust_only = rust_ops - json_ops
    if rust_only:
        for op in sorted(rust_only):
            fail(f"Operation '{op}' in operation_registry.rs but not in _capabilities.json")
    else:
        pass_("All Rust registry operations found in JSON.")

    # 5. Verify operation count matches
    print()
    print("--- Check 3: Operation count consistency ---")
    if len(json_ops) != len(rust_ops):
        fail(f"Operation count mismatch: JSON has {len(json_ops)}, Rust has {len(rust_ops)}")
    else:
        pass_(f"Operation count consistent: {len(json_ops)} operations.")

    # 6. Verify enum variant count matches constant count
    print()
    print("--- Check 4: Enum variant count matches constant count ---")
    content = OPERATION_REGISTRY_RS.read_text()
    enum_variants = len(re.findall(r'^\s+(?:ScanPorts|ScanEndpoints|FingerprintServices|ReconDns|InspectTls|DetectTechnology|DetectWaf|ValidateWaf|FuzzHttp|LoadTest|ScanGitSecrets|GenerateSbom|RunConsolidatedRecon|GraphqlTest|OauthTest|AuthTest|DbProbe|NseRun|ScanDockerImage|ScanKubernetes|AnalyzeApk|AnalyzeIpa),?\s*$', content, re.MULTILINE))
    constants = len(re.findall(r'pub const OP_\w+:', content))
    if enum_variants != constants:
        fail(f"Enum variant count ({enum_variants}) != constant count ({constants})")
    else:
        pass_(f"Enum variant count consistent: {enum_variants}.")

    # 7. Verify domain_maturity() in domains.rs matches JSON maturity
    print()
    print("--- Check 5: domain_maturity() matches JSON maturity ---")
    rust_maturity = parse_domain_maturity_from_rust(DOMAINS_RS)

    for domain_id, domain_info in json_domains.items():
        json_maturity = domain_info["maturity"]
        if domain_id in rust_maturity:
            rust_state = rust_maturity[domain_id]
            # Map rust state to maturity level
            if rust_state == "stable":
                rust_maturity_level = "stable"
            elif rust_state == "provisional":
                rust_maturity_level = "provisional"
            elif rust_state == "experimental":
                rust_maturity_level = "experimental"
            else:
                rust_maturity_level = rust_state

            if json_maturity != rust_maturity_level:
                fail(
                    f"Domain '{domain_id}': JSON maturity='{json_maturity}' "
                    f"but domain_maturity() state='{rust_state}'"
                )
        else:
            # Domain not in domain_maturity() — that's OK if it's a sub-domain
            # (e.g., git-secrets, sbom are sub-operations of stable-core)
            pass_(
                f"Domain '{domain_id}' not in domain_maturity() (sub-domain or implicit)."
            )

    # Check that all domain_maturity() entries appear in JSON
    for domain_id, state in rust_maturity.items():
        if domain_id not in json_domains:
            fail(
                f"Domain '{domain_id}' in domain_maturity() but not in _capabilities.json"
            )

    # 8. Verify: each operation in a domain's operations list is in stable_operations
    print()
    print("--- Check 6: Domain operations are in stable_operations ---")
    all_domain_ops = set()
    for domain_id, domain_info in json_domains.items():
        for op in domain_info.get("operations", []):
            all_domain_ops.add(op)
            if op not in json_ops:
                fail(f"Operation '{op}' in domain '{domain_id}' but not in stable_operations")

    # 9. Verify: every stable operation appears in at least one domain
    print()
    print("--- Check 7: Every stable operation is in a domain ---")
    orphan_ops = json_ops - all_domain_ops
    if orphan_ops:
        for op in sorted(orphan_ops):
            fail(f"Operation '{op}' in stable_operations but not in any domain's operations list")
    else:
        pass_("All stable operations assigned to at least one domain.")

    # 10. Verify StableOperation::ALL length matches
    print()
    print("--- Check 8: StableOperation::ALL length matches ---")
    all_match = re.search(r'pub const ALL:.*?=.*?\[(.*?)\]', content, re.DOTALL)
    if all_match:
        variants_in_all = len(re.findall(r'Self::\w+', all_match.group(1)))
        if variants_in_all != len(rust_ops):
            fail(f"StableOperation::ALL has {variants_in_all} variants, expected {len(rust_ops)}")
        else:
            pass_(f"StableOperation::ALL has {variants_in_all} variants (matches).")
    else:
        fail("Could not parse StableOperation::ALL")

    # Summary
    print()
    print("=" * 50)
    if FAIL:
        print(f"RESULT: {FAIL} drift issue(s) found.")
        sys.exit(1)
    else:
        print("RESULT: No drift detected.")
        sys.exit(0)


if __name__ == "__main__":
    main()
