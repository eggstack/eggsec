#!/usr/bin/env python3
"""Validate the eggsec-python validation/profiles.json manifest.

Checks:
1. Required fields present (name, cargo_features, test_selector, blocking, max_skips)
2. No duplicate profile names
3. cargo_features are known (whitelist from Cargo.toml features)
4. test_selector is a non-empty string containing 'pytest'
5. skip_budget is a dict with non-negative integer values
6. Blocking profiles: max_skips >= 0, required_min_tests >= 1
7. Profiles requiring privileges have schedule != 'always'
8. Output summary table, exit 0 on success, 1 on failure

Usage:
    python scripts/validate_python_profiles.py
    python scripts/validate_python_profiles.py --manifest path/to/profiles.json
    python scripts/validate_python_profiles.py --strict
"""

import argparse
import json
import sys
import textwrap
from pathlib import Path

# Valid cargo features for eggsec-python (from Cargo.toml [features])
VALID_CARGO_FEATURES = {
    "default",
    "websocket",
    "git-secrets",
    "sbom",
    "db-pentest",
    "db-pentest-mongodb",
    "db-pentest-redis",
    "web-proxy",
    "mobile",
    "mobile-dynamic",
    "packet-inspection",
    "stress-testing",
    "nse",
    "container",
    "daemon-client",
    "headless-browser",
    "advanced-hunting",
    "compliance",
    "wireless",
    "evasion",
    "postex",
    "c2",
    "ai-integration",
    "full-no-system",
}

REQUIRED_FIELDS = [
    "name", "cargo_features", "test_selector", "blocking", "max_skips",
    "description", "system_packages", "binaries", "services",
    "fixture_setup", "readiness_probe", "required_min_tests", "max_xfails",
    "schedule", "supported_os", "expected_artifacts",
    "timeout_seconds", "memory_mb",
]

VALID_SCHEDULES = {"always", "push", "manual", "weekly", "nightly"}


class ValidationResult:
    def __init__(self, profile_name: str):
        self.profile_name = profile_name
        self.errors: list[str] = []
        self.warnings: list[str] = []

    @property
    def ok(self) -> bool:
        return len(self.errors) == 0

    def add_error(self, msg: str):
        self.errors.append(msg)

    def add_warning(self, msg: str):
        self.warnings.append(msg)


def validate_profile(profile: dict, index: int, all_names: set[str]) -> ValidationResult:
    name = profile.get("name", f"<unnamed profile #{index}>")
    result = ValidationResult(name)

    # Required fields
    for field in REQUIRED_FIELDS:
        if field not in profile:
            result.add_error(f"missing required field: {field}")

    if not result.ok:
        return result

    # name must be a non-empty string
    if not isinstance(profile["name"], str) or not profile["name"].strip():
        result.add_error("name must be a non-empty string")

    # Duplicate detection is handled at the caller level

    # cargo_features: must be list of known strings
    features = profile.get("cargo_features")
    if not isinstance(features, list):
        result.add_error("cargo_features must be a list")
    else:
        for feat in features:
            if not isinstance(feat, str):
                result.add_error(f"cargo_features entry is not a string: {feat!r}")
            elif feat not in VALID_CARGO_FEATURES:
                result.add_error(f"unknown cargo_feature: {feat!r}")

    # test_selector: must be non-empty string containing 'pytest'
    selector = profile.get("test_selector")
    if not isinstance(selector, str) or not selector.strip():
        result.add_error("test_selector must be a non-empty string")
    elif "pytest" not in selector:
        result.add_error("test_selector does not contain 'pytest'")

    # blocking: must be bool
    blocking = profile.get("blocking")
    if not isinstance(blocking, bool):
        result.add_error(f"blocking must be a bool, got {type(blocking).__name__}")

    # max_skips: must be int >= 0
    max_skips = profile.get("max_skips")
    if not isinstance(max_skips, int) or max_skips < 0:
        result.add_error(f"max_skips must be a non-negative integer, got {max_skips!r}")

    # skip_budget: must be dict with non-negative integer values
    skip_budget = profile.get("skip_budget")
    if skip_budget is not None:
        if not isinstance(skip_budget, dict):
            result.add_error(f"skip_budget must be a dict, got {type(skip_budget).__name__}")
        else:
            for k, v in skip_budget.items():
                if not isinstance(v, int) or v < 0:
                    result.add_error(f"skip_budget.{k} must be a non-negative integer, got {v!r}")

    # Blocking profile constraints
    if blocking is True:
        if max_skips is not None and isinstance(max_skips, int) and max_skips < 0:
            result.add_error("blocking profile has negative max_skips")
        required_min = profile.get("required_min_tests")
        if required_min is not None:
            if not isinstance(required_min, int) or required_min < 1:
                result.add_error(
                    f"blocking profile requires required_min_tests >= 1, got {required_min!r}"
                )

    # requires_privileges + schedule != 'always'
    requires_priv = profile.get("requires_privileges", False)
    schedule = profile.get("schedule")
    if requires_priv is True and schedule == "always":
        result.add_error(
            "profile requires_privileges but schedule is 'always' — "
            "privileged profiles must not run unconditionally in CI"
        )

    # Warnings for non-blocking profiles
    if blocking is False:
        result.add_warning("non-blocking profile — failures will not gate CI")

    # Warnings for unknown schedules (non-fatal)
    if schedule is not None and schedule not in VALID_SCHEDULES:
        result.add_warning(f"unrecognized schedule value: {schedule!r}")

    # Warnings for optional fields with unexpected types
    for field in ("system_packages", "binaries", "services", "supported_os", "expected_artifacts"):
        val = profile.get(field)
        if val is not None and not isinstance(val, list):
            result.add_warning(f"{field} should be a list, got {type(val).__name__}")

    for field in ("timeout_seconds", "memory_mb", "required_min_tests"):
        val = profile.get(field)
        if val is not None and (not isinstance(val, int) or val < 0):
            result.add_warning(f"{field} should be a non-negative integer, got {val!r}")

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Validate eggsec-python validation/profiles.json manifest."
    )
    parser.add_argument(
        "--manifest",
        default="crates/eggsec-python/validation/profiles.json",
        help="Path to profiles.json (default: crates/eggsec-python/validation/profiles.json)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Treat warnings as errors (exit 1 on any warning)",
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    if not manifest_path.exists():
        print(f"ERROR: manifest not found: {manifest_path}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(manifest_path, "r") as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"ERROR: invalid JSON in {manifest_path}: {e}", file=sys.stderr)
        sys.exit(1)

    profiles = data.get("profiles")
    if not isinstance(profiles, list):
        print("ERROR: 'profiles' key must be a list", file=sys.stderr)
        sys.exit(1)

    # Duplicate name detection
    seen_names: dict[str, int] = {}
    all_results: list[ValidationResult] = []
    has_errors = False
    has_warnings = False

    for idx, profile in enumerate(profiles):
        name = profile.get("name", f"<unnamed #{idx}>")
        if name in seen_names:
            result = ValidationResult(name)
            result.add_error(
                f"duplicate profile name at indices {seen_names[name]} and {idx}"
            )
            all_results.append(result)
            has_errors = True
            continue
        seen_names[name] = idx

        result = validate_profile(profile, idx, set(seen_names.keys()))
        all_results.append(result)
        if not result.ok:
            has_errors = True
        if result.warnings:
            has_warnings = True

    # Summary
    blocking_count = sum(1 for p in profiles if p.get("blocking") is True)
    non_blocking_count = len(profiles) - blocking_count

    print("=" * 72)
    print("  eggsec-python profiles.json validation report")
    print("=" * 72)
    print(f"  Manifest:       {manifest_path}")
    print(f"  Profile count:  {len(profiles)}")
    print(f"  Blocking:       {blocking_count}")
    print(f"  Non-blocking:   {non_blocking_count}")
    print("-" * 72)

    for res in all_results:
        status = "PASS" if res.ok else "FAIL"
        print(f"  [{status}] {res.profile_name}")
        for err in res.errors:
            print(f"         ERROR: {err}")
        for warn in res.warnings:
            print(f"         WARN:  {warn}")

    print("-" * 72)
    error_count = sum(1 for r in all_results if not r.ok)
    warn_count = sum(1 for r in all_results if r.warnings)
    print(f"  Results: {error_count} error(s), {warn_count} warning(s)")
    print("=" * 72)

    if has_errors:
        sys.exit(1)
    if args.strict and has_warnings:
        print("  --strict: treating warnings as errors", file=sys.stderr)
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()
