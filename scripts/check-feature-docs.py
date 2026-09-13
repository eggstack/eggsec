#!/usr/bin/env python3
"""Feature documentation consistency validation (Phase B, workstream 5).

Mechanical oracle for source facts Cargo already provides:
- engine default feature set,
- declared main-crate features and their `full` membership,
- curated `full` semantics (pinned 28 members, not "everything"),
- domain-crate feature inventory presence.

Human docs may add explanations, risk notes, prerequisites, and surface
exposure, but membership/default claims must match machine-readable
declarations. Run locally or via architecture guards:

    python3 scripts/check-feature-docs.py
"""

import re
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FAILURES: list[str] = []


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"FAIL: {msg}")


def ok(msg: str) -> None:
    print(f"PASS: {msg}")


def read_toml(path: Path) -> dict:
    with open(path, "rb") as f:
        return tomllib.load(f)


def parse_registry_full_members(src: str) -> list[str]:
    m = re.search(r"pub static FULL_MEMBERS: &\[&str\] = &\[(.*?)\];", src, re.S)
    if not m:
        fail("feature_registry.rs: FULL_MEMBERS static not found")
        return []
    return re.findall(r'"([^"]+)"', m.group(1))


def parse_registry_excluded(src: str) -> dict[str, str]:
    m = re.search(
        r"pub static FULL_EXCLUDED_WITH_REASON:[^=]*=\s*&\[(.*?)\n\];", src, re.S
    )
    if not m:
        fail("feature_registry.rs: FULL_EXCLUDED_WITH_REASON static not found")
        return {}
    names = re.findall(r'\(\s*"([^"]+)",', m.group(1))
    return {n: "<reason>" for n in names}


def main() -> int:
    engine = read_toml(REPO / "crates/eggsec/Cargo.toml")["features"]
    default = engine.get("default", [])
    cargo_features = sorted(n for n in engine if n != "default")
    cargo_full = sorted(engine.get("full", []))

    # ── 1. Default feature set (Phase E WS2: empty library default) ──────
    if default == []:
        ok('engine default is [] (empty library default)')
    else:
        fail(f'engine default is {default}, expected []')

    matrix = (REPO / "docs/FEATURE_MATRIX.md").read_text()
    if 'default = []' in matrix:
        ok('docs/FEATURE_MATRIX.md states default = []')
    else:
        fail('docs/FEATURE_MATRIX.md must state default = []')
    if 'default = ["cli"]' in matrix:
        fail('docs/FEATURE_MATRIX.md must not claim default = ["cli"] (Phase E WS2)')

    # ── 2. Curated `full`: forbidden exhaustive claims ──────────────────
    forbidden = [
        "enables all non-default features",
        "enables everything",
        "Enables all non-default features",
        "Full build - all features",
        "includes all non-default features",
    ]
    # docs/extending/features.md uses `default = []` only inside a generic
    # "new feature" TOML template; check it separately for engine claims.
    scan_files = [
        "docs/FEATURE_MATRIX.md",
        "docs/BUILD.md",
        "docs/VERIFICATION.md",
        "README.md",
        "architecture/overview.md",
        "architecture/feature_matrix.md",
        "docs/extending/features.md",
        ".opencode/skills/eggsec-python/SKILL.md",
    ]
    for rel in scan_files:
        p = REPO / rel
        if not p.exists():
            continue  # symlinked skill dirs resolve to one real file
        text = p.read_text()
        for phrase in forbidden:
            if phrase in text:
                fail(f"{rel} contains exhaustive `full` claim: {phrase!r}")

    if '"full" aggregate includes all non-default features' in matrix:
        fail("docs/FEATURE_MATRIX.md Python section misstates `full` aggregate")
    if "curated" in matrix.lower() and "28" in matrix:
        ok("docs/FEATURE_MATRIX.md documents curated 28-member `full`")
    else:
        fail("docs/FEATURE_MATRIX.md must document `full` as curated with 28 members")

    # ── 3. Registry contract matches Cargo ──────────────────────────────
    registry_src = (REPO / "crates/eggsec/src/config/feature_registry.rs").read_text()
    full_members = parse_registry_full_members(registry_src)
    if sorted(full_members) == cargo_full:
        ok(f"FULL_MEMBERS matches Cargo `full` ({len(full_members)} members)")
    else:
        fail(
            "FULL_MEMBERS does not match Cargo `full`:\n"
            f"  registry-only: {sorted(set(full_members) - set(cargo_full))}\n"
            f"  cargo-only: {sorted(set(cargo_full) - set(full_members))}"
        )
    if len(full_members) == 28:
        ok("FULL_MEMBERS pins 28 curated members")
    else:
        fail(f"FULL_MEMBERS has {len(full_members)} entries, expected 28")

    excluded = parse_registry_excluded(registry_src)
    covered = set(full_members) | set(excluded) | {"full"}
    missing = [f for f in cargo_features if f not in covered]
    if not missing:
        ok("every engine feature is in FULL_MEMBERS or FULL_EXCLUDED_WITH_REASON")
    else:
        fail(f"features without full contract entry: {missing}")
    overlap = set(full_members) & set(excluded)
    if not overlap:
        ok("no feature is both member and excluded")
    else:
        fail(f"features both member and excluded: {sorted(overlap)}")
    for must_out in ("test-helpers", "insecure-tls"):
        if must_out in full_members:
            fail(f"'{must_out}' must never be in `full`")

    # ── 4. Feature inventory presence in docs ───────────────────────────
    absent = [f for f in cargo_features if f"`{f}`" not in matrix]
    if not absent:
        ok(f"docs/FEATURE_MATRIX.md names all {len(cargo_features)} engine features")
    else:
        fail(f"docs/FEATURE_MATRIX.md missing features: {absent}")

    # ── 5. Domain-crate inventory ───────────────────────────────────────
    domain_ok = True
    for crate, feat in [
        ("eggsec-nse", "`nse`"),
        ("eggsec-db-lab", "`db-drivers`"),
        ("eggsec-web-proxy", "`web-proxy`"),
        ("eggsec-mobile-lab", "`mobile-dynamic`"),
    ]:
        if crate in matrix and feat in matrix:
            continue
        domain_ok = False
        fail(f"docs/FEATURE_MATRIX.md domain inventory missing {crate} {feat}")
    if domain_ok:
        ok("docs/FEATURE_MATRIX.md domain-crate inventory present")

    build_md = (REPO / "docs/BUILD.md").read_text()
    if "curated" in build_md.lower() and "28" in build_md:
        ok("docs/BUILD.md documents curated 28-member `full`")
    else:
        fail("docs/BUILD.md must document `full` as curated with 28 members")

    arch_matrix = (REPO / "architecture/feature_matrix.md").read_text()
    if "| `full` | yes | yes | - | (all) | Deprecated |" in arch_matrix:
        fail("architecture/feature_matrix.md marks `full` Deprecated — use curated lab aggregate")
    else:
        ok("architecture/feature_matrix.md does not mark `full` Deprecated")

    print()
    if FAILURES:
        print(f"RESULT: FAIL ({len(FAILURES)} problem(s))")
        return 1
    print("RESULT: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
