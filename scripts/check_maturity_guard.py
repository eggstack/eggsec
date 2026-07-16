"""Maturity guard: prevents docs from claiming higher maturity than evidence.

Workstream 13: Validates that documentation maturity claims are backed by
evidence from the profile test matrix. A promotion requires all required
stable profiles to pass; provisional status requires their evidence to be green.

Usage:
    python scripts/check_maturity_guard.py [--evidence-dir target/python-validation/<sha>]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


# Required evidence files for stable maturity
STABLE_REQUIRED_EVIDENCE = [
    "profile-manifest.json",
    "commit-info.json",
    "toolchain.json",
    "maturity-decision.json",
    "platform-details.json",
    "evidence-summary.json",
]

# Profiles required for stable maturity
STABLE_REQUIRED_PROFILES = [
    "default-wheel",
    "full-no-system",
    "websocket",
    "git-secrets",
    "sbom",
    "container",
    "mobile-static",
    "packet-parser",
]

# Profiles that remain provisional until their evidence is green
PROVISIONAL_UNTIL_GREEN = [
    "daemon-client",
    "headless-browser",
    "mobile-emulator",
    "web-proxy",
    "nse",
    "db-postgres",
    "db-mysql",
    "db-redis",
    "db-mongodb",
    "packet-live",
    "active-probes",
]


def load_json(path: Path) -> dict | list | None:
    """Load a JSON file, returning None on error."""
    try:
        return json.loads(path.read_text())
    except (json.JSONDecodeError, FileNotFoundError, OSError):
        return None


def check_evidence_files(evidence_dir: Path) -> list[str]:
    """Check that required evidence files exist."""
    errors = []
    for filename in STABLE_REQUIRED_EVIDENCE:
        path = evidence_dir / filename
        if not path.exists():
            errors.append(f"Missing required evidence file: {filename}")
        else:
            data = load_json(path)
            if data is None:
                errors.append(f"Invalid JSON in evidence file: {filename}")
    return errors


def check_profile_evidence(evidence_dir: Path) -> tuple[list[str], list[str]]:
    """Check that required profiles have passing evidence."""
    errors = []
    warnings = []

    maturity_path = evidence_dir / "maturity-decision.json"
    maturity = load_json(maturity_path)
    if maturity is None:
        errors.append("Cannot load maturity-decision.json")
        return errors, warnings

    domains = maturity.get("domains", {})
    overall = maturity.get("overall", "unknown")

    # Map profile names to their domain equivalents
    profile_to_domain = {
        "default-wheel": "stable-core",
        "full-no-system": "stable-core",
        "websocket": "stable-core",
        "git-secrets": "git-secrets",
        "sbom": "sbom",
        "container": "container",
        "mobile-static": "mobile",
        "packet-parser": "packet-inspection",
    }

    for profile in STABLE_REQUIRED_PROFILES:
        domain = profile_to_domain.get(profile, profile)
        decision = domains.get(domain)
        if decision is None:
            errors.append(f"Required stable profile '{profile}' (domain '{domain}') has no maturity decision")
        elif decision != "stable":
            errors.append(
                f"Required profile '{profile}' (domain '{domain}') is '{decision}' but must be 'stable' "
                f"for stable maturity"
            )

    provisional_domains = {
        "daemon-client": "daemon",
        "headless-browser": "browser",
        "mobile-emulator": "mobile",
        "web-proxy": "proxy",
        "nse": "nse",
        "db-postgres": "database",
        "db-mysql": "database",
        "db-redis": "database",
        "db-mongodb": "database",
        "packet-live": "packet-inspection",
        "active-probes": "packet-inspection",
    }

    for profile in PROVISIONAL_UNTIL_GREEN:
        domain = provisional_domains.get(profile, profile)
        decision = domains.get(domain)
        if decision == "stable":
            warnings.append(
                f"Provisional profile '{profile}' (domain '{domain}') is marked 'stable' — "
                f"verify this is intentional"
            )

    return errors, warnings


def check_commit_consistency(evidence_dir: Path) -> list[str]:
    """Check that evidence files reference the same commit."""
    errors = []
    commit_info = load_json(evidence_dir / "commit-info.json")
    if commit_info is None:
        return ["Cannot load commit-info.json"]

    evidence_commit = commit_info.get("commit_sha", "unknown")

    maturity = load_json(evidence_dir / "maturity-decision.json")
    if maturity is not None:
        maturity_commit = maturity.get("commit_sha", "unknown")
        if maturity_commit != evidence_commit:
            errors.append(
                f"Commit mismatch: commit-info={evidence_commit}, "
                f"maturity={maturity_commit}"
            )

    return errors


def check_docs_against_evidence(evidence_dir: Path, repo_root: Path) -> list[str]:
    """Check that documentation claims don't exceed evidence maturity."""
    errors = []

    maturity = load_json(evidence_dir / "maturity-decision.json")
    if maturity is None:
        return ["Cannot load maturity-decision.json for docs check"]

    decisions = maturity.get("decisions", {})
    overall = maturity.get("overall", "unknown")

    # Check domain-maturity.md doesn't claim higher than evidence
    domain_maturity_path = repo_root / "docs" / "python" / "domain-maturity.md"
    if domain_maturity_path.exists():
        content = domain_maturity_path.read_text()
        if overall == "provisional" and "stable" in content.lower():
            # Check if "stable" is used as a claim (not just a word)
            for line in content.split("\n"):
                stripped = line.strip()
                if stripped.startswith("#") and "stable" in stripped.lower():
                    errors.append(
                        f"domain-maturity.md heading claims 'stable' but evidence "
                        f"overall is '{overall}'"
                    )

    # Check README doesn't claim higher than evidence
    readme_path = repo_root / "crates" / "eggsec-python" / "README.md"
    if readme_path.exists():
        content = readme_path.read_text()
        if overall == "provisional":
            for line in content.split("\n"):
                stripped = line.strip()
                if "stable" in stripped.lower() and "provisional" not in stripped.lower():
                    if not stripped.startswith("#") and not stripped.startswith("<"):
                        continue
                    if stripped.startswith("#"):
                        errors.append(
                            f"README.md heading may claim 'stable' but evidence "
                            f"overall is '{overall}'"
                        )

    return errors


def main():
    parser = argparse.ArgumentParser(description="Maturity guard validation")
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        help="Path to evidence bundle directory",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).parent.parent,
        help="Repository root directory",
    )
    args = parser.parse_args()

    if args.evidence_dir is None:
        # Find the most recent evidence directory
        target_dir = args.repo_root / "target" / "python-validation"
        if target_dir.exists():
            dirs = sorted(target_dir.iterdir(), reverse=True)
            for d in dirs:
                if d.is_dir() and (d / "evidence-summary.json").exists():
                    args.evidence_dir = d
                    break

    if args.evidence_dir is None or not args.evidence_dir.exists():
        print("ERROR: No evidence directory found. Run build_python_release_evidence.py first.")
        sys.exit(1)

    print(f"Maturity guard: checking evidence in {args.evidence_dir}")

    all_errors = []
    all_warnings = []

    # Check evidence files exist
    errors = check_evidence_files(args.evidence_dir)
    all_errors.extend(errors)

    # Check commit consistency
    errors = check_commit_consistency(args.evidence_dir)
    all_errors.extend(errors)

    # Check profile evidence
    errors, warnings = check_profile_evidence(args.evidence_dir)
    all_errors.extend(errors)
    all_warnings.extend(warnings)

    # Check docs against evidence
    errors = check_docs_against_evidence(args.evidence_dir, args.repo_root)
    all_errors.extend(errors)

    # Report
    for w in all_warnings:
        print(f"  WARNING: {w}")

    if all_errors:
        print(f"\nFAILED: {len(all_errors)} maturity guard violation(s):")
        for e in all_errors:
            print(f"  - {e}")
        sys.exit(1)
    else:
        print("\nPASSED: Maturity guard check passed")
        sys.exit(0)


if __name__ == "__main__":
    main()
