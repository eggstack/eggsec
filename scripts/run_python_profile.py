#!/usr/bin/env python3
"""Canonical interface for running eggsec-python validation profiles.

Loads a profile from profiles.json, checks prerequisites, builds the wheel,
runs the profile's test selector, enforces skip budgets, and produces a
structured summary with pass/fail/skip/xfail counts.

Usage:
    python scripts/run_python_profile.py --profile default-wheel
    python scripts/run_python_profile.py --profile nse --manifest path/to/profiles.json
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent
PYTHON_CRATE = REPO_ROOT / "crates" / "eggsec-python"
DEFAULT_MANIFEST = PYTHON_CRATE / "validation" / "profiles.json"
CARGO_TOML = PYTHON_CRATE / "Cargo.toml"
WHEEL_DIR = REPO_ROOT / "target" / "python-wheels"
TEST_DIRS = [
    PYTHON_CRATE / "tests",
    PYTHON_CRATE / "python" / "tests",
]


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class Profile:
    name: str
    description: str = ""
    cargo_features: list[str] = field(default_factory=list)
    system_packages: list[str] = field(default_factory=list)
    binaries: list[str] = field(default_factory=list)
    services: list[str] = field(default_factory=list)
    fixture_setup: str | None = None
    readiness_probe: str | None = None
    test_selector: str = ""
    required_min_tests: int = 1
    max_skips: int = 0
    max_xfails: int = 0
    blocking: bool = True
    schedule: str = "push"
    supported_os: list[str] = field(default_factory=list)
    expected_artifacts: list[str] = field(default_factory=list)
    timeout_seconds: int = 600
    memory_mb: int = 512
    requires_privileges: bool = False
    skip_budget: dict[str, int] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Profile:
        return cls(
            name=data["name"],
            description=data.get("description", ""),
            cargo_features=data.get("cargo_features", []),
            system_packages=data.get("system_packages", []),
            binaries=data.get("binaries", []),
            services=data.get("services", []),
            fixture_setup=data.get("fixture_setup"),
            readiness_probe=data.get("readiness_probe"),
            test_selector=data.get("test_selector", ""),
            required_min_tests=data.get("required_min_tests", 1),
            max_skips=data.get("max_skips", 0),
            max_xfails=data.get("max_xfails", 0),
            blocking=data.get("blocking", True),
            schedule=data.get("schedule", "push"),
            supported_os=data.get("supported_os", []),
            expected_artifacts=data.get("expected_artifacts", []),
            timeout_seconds=data.get("timeout_seconds", 600),
            memory_mb=data.get("memory_mb", 512),
            requires_privileges=data.get("requires_privileges", False),
            skip_budget=data.get("skip_budget", {}),
        )


@dataclass
class TestCounts:
    total: int = 0
    passed: int = 0
    failed: int = 0
    skipped: int = 0
    xfailed: int = 0
    xpassed: int = 0
    error: int = 0


@dataclass
class ProfileResult:
    profile: str
    success: bool = False
    phase: str = "init"
    counts: TestCounts = field(default_factory=TestCounts)
    budget_verdict: str = "PASS"
    budget_failures: list[str] = field(default_factory=list)
    junit_path: str = ""
    wheel_path: str = ""
    evidence_path: str = ""
    errors: list[str] = field(default_factory=list)
    skipped_prereqs: list[str] = field(default_factory=list)
    duration_seconds: float = 0.0


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def log(msg: str) -> None:
    print(f"[profile] {msg}", flush=True)


def log_err(msg: str) -> None:
    print(f"[profile] ERROR: {msg}", file=sys.stderr, flush=True)


def run_cmd(
    cmd: list[str],
    *,
    timeout: int = 300,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    capture: bool = True,
) -> subprocess.CompletedProcess[str]:
    merged_env = dict(os.environ)
    if env:
        merged_env.update(env)
    return subprocess.run(
        cmd,
        capture_output=capture,
        text=True,
        timeout=timeout,
        cwd=cwd or REPO_ROOT,
        env=merged_env,
    )


def tool_version(cmd: str) -> str:
    try:
        result = run_cmd([cmd, "--version"], timeout=10)
        return result.stdout.strip() if result.returncode == 0 else "unknown"
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return "not found"


# ---------------------------------------------------------------------------
# Manifest loading
# ---------------------------------------------------------------------------

def load_manifest(path: Path) -> dict[str, Profile]:
    with open(path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
    profiles_raw = data.get("profiles", [])
    profiles: dict[str, Profile] = {}
    for entry in profiles_raw:
        p = Profile.from_dict(entry)
        profiles[p.name] = p
    return profiles


# ---------------------------------------------------------------------------
# Prerequisite checking
# ---------------------------------------------------------------------------

def check_system_packages(packages: list[str]) -> list[str]:
    missing = []
    for pkg in packages:
        result = run_cmd(["dpkg", "-s", pkg], timeout=10)
        if result.returncode != 0:
            missing.append(pkg)
    return missing


def check_binaries(binaries: list[str]) -> list[str]:
    missing = []
    for b in binaries:
        if shutil.which(b) is None:
            missing.append(b)
    return missing


def check_services(services: list[str]) -> list[str]:
    not_running = []
    for svc in services:
        result = run_cmd(["systemctl", "is-active", svc], timeout=10)
        if result.returncode != 0:
            not_running.append(svc)
    return not_running


def check_os_support(supported_os: list[str]) -> bool:
    if not supported_os:
        return True
    current = platform.system().lower()
    os_map = {"linux": "linux", "darwin": "macos", "windows": "windows"}
    normalized = os_map.get(current, current)
    return normalized in supported_os


def check_privileges(requires_priv: bool) -> bool:
    if not requires_priv:
        return True
    return os.geteuid() == 0


def verify_prerequisites(
    profile: Profile,
) -> tuple[bool, list[str]]:
    """Check all prerequisites. Returns (ok, list_of_failure_reasons)."""
    failures: list[str] = []

    if not check_os_support(profile.supported_os):
        current = platform.system().lower()
        failures.append(
            f"OS not supported: current={current}, required={profile.supported_os}"
        )

    if not check_privileges(profile.requires_privileges):
        failures.append("Profile requires root/privileges but running as non-root")

    missing_pkgs = check_system_packages(profile.system_packages)
    if missing_pkgs:
        failures.append(f"Missing system packages: {', '.join(missing_pkgs)}")

    missing_bins = check_binaries(profile.binaries)
    if missing_bins:
        failures.append(f"Missing binaries: {', '.join(missing_bins)}")

    not_running = check_services(profile.services)
    if not_running:
        failures.append(f"Services not running: {', '.join(not_running)}")

    return len(failures) == 0, failures


# ---------------------------------------------------------------------------
# Fixture setup / service management
# ---------------------------------------------------------------------------

def run_fixture_setup(setup_path: str | None) -> bool:
    if not setup_path:
        return True
    script = REPO_ROOT / setup_path
    if not script.exists():
        log_err(f"Fixture setup script not found: {script}")
        return False
    log(f"Running fixture setup: {setup_path}")
    result = run_cmd(["bash", str(script)], timeout=120)
    if result.returncode != 0:
        log_err(f"Fixture setup failed (rc={result.returncode})")
        if result.stderr:
            log_err(result.stderr.strip()[-500:])
        return False
    return True


def wait_for_readiness(probe_cmd: str | None, timeout: int = 30) -> bool:
    if not probe_cmd:
        return True
    log(f"Waiting for readiness: {probe_cmd}")
    deadline = time.time() + timeout
    while time.time() < deadline:
        result = run_cmd(["bash", "-c", probe_cmd], timeout=10)
        if result.returncode == 0:
            log("Service ready")
            return True
        time.sleep(1)
    log_err(f"Readiness probe timed out after {timeout}s")
    return False


# ---------------------------------------------------------------------------
# Wheel building
# ---------------------------------------------------------------------------

def build_wheel(profile: Profile) -> tuple[bool, str]:
    """Build the wheel with the profile's cargo features. Returns (ok, wheel_path)."""
    WHEEL_DIR.mkdir(parents=True, exist_ok=True)

    cmd = [
        "maturin", "build", "--release",
        "--manifest-path", str(CARGO_TOML),
        "--out", str(WHEEL_DIR),
    ]

    if profile.cargo_features:
        cmd.extend(["--features", ",".join(profile.cargo_features)])

    log(f"Building wheel with features: {profile.cargo_features or ['(default)']}")
    result = run_cmd(cmd, timeout=profile.timeout_seconds)
    if result.returncode != 0:
        log_err(f"maturin build failed (rc={result.returncode})")
        if result.stderr:
            log_err(result.stderr.strip()[-1000:])
        return False, ""

    wheels = sorted(WHEEL_DIR.glob("*.whl"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not wheels:
        log_err("No wheel produced in output directory")
        return False, ""

    wheel_path = str(wheels[0])
    log(f"Built wheel: {wheels[0].name}")
    return True, wheel_path


# ---------------------------------------------------------------------------
# Wheel installation
# ---------------------------------------------------------------------------

def install_wheel(wheel_path: str) -> bool:
    log(f"Installing wheel: {Path(wheel_path).name}")
    result = run_cmd(
        [sys.executable, "-m", "pip", "install", "--force-reinstall", wheel_path],
        timeout=120,
    )
    if result.returncode != 0:
        log_err(f"pip install failed (rc={result.returncode})")
        if result.stderr:
            log_err(result.stderr.strip()[-500:])
        return False
    return True


# ---------------------------------------------------------------------------
# Test execution
# ---------------------------------------------------------------------------

def run_tests(
    profile: Profile,
    output_dir: Path,
) -> tuple[bool, TestCounts, str]:
    """Run the profile's test selector and capture JUnit XML. Returns (ok, counts, junit_path)."""
    junit_path = output_dir / "test-results.xml"

    test_dirs_exist = [d for d in TEST_DIRS if d.exists()]
    if not test_dirs_exist:
        log_err("No test directories found")
        return False, TestCounts(), ""

    cmd_parts = profile.test_selector.split()
    if not cmd_parts:
        log_err("Empty test_selector")
        return False, TestCounts(), ""

    cmd = cmd_parts + [
        f"--junitxml={junit_path}",
        "--tb=short",
        "--strict-markers",
        "-q",
    ]
    cmd.extend(str(d) for d in test_dirs_exist)

    log(f"Running tests: {' '.join(cmd_parts)}")
    log(f"JUnit XML output: {junit_path}")

    env = {
        "PYTHONDONTWRITEBYTECODE": "1",
    }

    try:
        result = run_cmd(
            cmd,
            timeout=profile.timeout_seconds,
            env=env,
            capture=False,
        )
    except subprocess.TimeoutExpired:
        log_err(f"Test run timed out after {profile.timeout_seconds}s")
        return False, TestCounts(), str(junit_path)

    counts = parse_junit(junit_path)
    ok = result.returncode == 0
    return ok, counts, str(junit_path)


def parse_junit(path: Path) -> TestCounts:
    counts = TestCounts()
    if not path.exists():
        return counts

    try:
        tree = ET.parse(path)
    except ET.ParseError:
        return counts

    root = tree.getroot()
    if root.tag == "testsuites":
        suites = root.findall("testsuite")
    elif root.tag == "testsuite":
        suites = [root]
    else:
        suites = []

    for suite in suites:
        for tc in suite.findall("testcase"):
            counts.total += 1
            skipped_el = tc.find("skipped")
            failure_el = tc.find("failure")
            error_el = tc.find("error")
            xfail_el = tc.find("xfail")

            if skipped_el is not None:
                xfail_attr = skipped_el.get("xfail")
                if xfail_attr or "xfail" in (skipped_el.get("message") or "").lower():
                    counts.xfailed += 1
                else:
                    counts.skipped += 1
            elif failure_el is not None:
                counts.failed += 1
            elif error_el is not None:
                counts.error += 1
            elif xfail_el is not None:
                counts.xfailed += 1
            else:
                counts.passed += 1

    return counts


# ---------------------------------------------------------------------------
# Skip budget enforcement
# ---------------------------------------------------------------------------

def enforce_skip_budget(
    profile: Profile,
    counts: TestCounts,
) -> tuple[str, list[str]]:
    """Evaluate skip/xfail counts against profile budget. Returns (verdict, failures)."""
    failures: list[str] = []
    total_ran = counts.passed + counts.failed + counts.error + counts.xpassed

    if counts.total > 0 and total_ran == 0 and (counts.skipped + counts.xfailed) > 0:
        failures.append(
            f"ALL {counts.total} selected tests were skipped/xfailed — "
            "profile produced no real results"
        )

    if total_ran < profile.required_min_tests:
        failures.append(
            f"Only {total_ran} test(s) ran, but {profile.required_min_tests} "
            "required minimum"
        )

    feature_gate_budget = profile.skip_budget.get("feature_gate", 10)
    if counts.skipped > feature_gate_budget:
        failures.append(
            f"Skip budget exceeded: {counts.skipped} skips > feature_gate budget of {feature_gate_budget}"
        )

    if profile.max_xfails is not None and counts.xfailed > profile.max_xfails:
        failures.append(
            f"Xfail budget exceeded: {counts.xfailed} xfails > max of {profile.max_xfails}"
        )

    verdict = "FAIL" if failures else "PASS"
    return verdict, failures


# ---------------------------------------------------------------------------
# Evidence generation
# ---------------------------------------------------------------------------

def generate_evidence(
    profile: Profile,
    counts: TestCounts,
    budget_verdict: str,
    budget_failures: list[str],
    wheel_path: str,
    junit_path: str,
    output_dir: Path,
) -> str:
    """Generate profile-specific evidence JSON. Returns path to evidence file."""
    evidence = {
        "profile": profile.name,
        "description": profile.description,
        "generated_at": _utc_now_iso(),
        "platform": {
            "system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
            "python_implementation": platform.python_implementation(),
        },
        "toolchain": {
            "rustc": tool_version("rustc"),
            "cargo": tool_version("cargo"),
        },
        "cargo_features": profile.cargo_features,
        "wheel": {
            "path": wheel_path,
            "filename": Path(wheel_path).name if wheel_path else None,
            "sha256": _sha256(wheel_path) if wheel_path else None,
        },
        "test_results": {
            "junit_xml": junit_path,
            "total": counts.total,
            "passed": counts.passed,
            "failed": counts.failed,
            "skipped": counts.skipped,
            "xfailed": counts.xfailed,
            "xpassed": counts.xpassed,
            "error": counts.error,
        },
        "skip_budget": {
            "verdict": budget_verdict,
            "failures": budget_failures,
            "budget_config": profile.skip_budget,
        },
        "profile_config": {
            "timeout_seconds": profile.timeout_seconds,
            "required_min_tests": profile.required_min_tests,
            "blocking": profile.blocking,
            "schedule": profile.schedule,
        },
    }

    dest = output_dir / "evidence.json"
    dest.write_text(json.dumps(evidence, indent=2) + "\n")
    return str(dest)


def _utc_now_iso() -> str:
    import datetime
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def _sha256(path: str) -> str | None:
    import hashlib
    if not path or not Path(path).exists():
        return None
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Summary output
# ---------------------------------------------------------------------------

def print_summary(result: ProfileResult) -> None:
    c = result.counts
    status = "PASS" if result.success else "FAIL"

    print()
    print("=" * 72)
    print(f"  Profile: {result.profile}")
    print(f"  Status:  {status}")
    print("=" * 72)
    print(f"  Total:      {c.total}")
    print(f"  Passed:     {c.passed}")
    print(f"  Failed:     {c.failed}")
    print(f"  Skipped:    {c.skipped}")
    print(f"  XFailed:    {c.xfailed}")
    print(f"  XPassed:    {c.xpassed}")
    print(f"  Errors:     {c.error}")
    print("-" * 72)
    print(f"  Budget:     {result.budget_verdict}")
    print(f"  Duration:   {result.duration_seconds:.1f}s")

    if result.wheel_path:
        print(f"  Wheel:      {Path(result.wheel_path).name}")
    if result.junit_path:
        print(f"  JUnit XML:  {result.junit_path}")
    if result.evidence_path:
        print(f"  Evidence:   {result.evidence_path}")

    if result.budget_failures:
        print("-" * 72)
        print("  Budget failures:")
        for f in result.budget_failures:
            print(f"    - {f}")

    if result.errors:
        print("-" * 72)
        print("  Errors:")
        for e in result.errors:
            print(f"    - {e}")

    print("=" * 72)


# ---------------------------------------------------------------------------
# Main orchestration
# ---------------------------------------------------------------------------

def run_profile(
    profile: Profile,
    output_base: Path,
) -> ProfileResult:
    result = ProfileResult(profile=profile.name)
    start = time.monotonic()

    output_dir = output_base / profile.name
    output_dir.mkdir(parents=True, exist_ok=True)

    # Phase 1: Prerequisites
    result.phase = "prerequisites"
    log(f"Checking prerequisites for profile: {profile.name}")
    ok, failures = verify_prerequisites(profile)
    if not ok:
        result.errors.extend(failures)
        result.success = False
        result.duration_seconds = time.monotonic() - start
        return result

    # Phase 2: Fixture setup / service startup
    result.phase = "fixture_setup"
    if not run_fixture_setup(profile.fixture_setup):
        result.errors.append("Fixture setup failed")
        result.success = False
        result.duration_seconds = time.monotonic() - start
        return result

    if not wait_for_readiness(profile.readiness_probe):
        result.errors.append("Service readiness probe failed")
        result.success = False
        result.duration_seconds = time.monotonic() - start
        return result

    # Phase 3: Build wheel
    result.phase = "build"
    ok, wheel_path = build_wheel(profile)
    if not ok:
        result.errors.append("Wheel build failed")
        result.success = False
        result.duration_seconds = time.monotonic() - start
        return result
    result.wheel_path = wheel_path

    # Phase 4: Install wheel
    result.phase = "install"
    if not install_wheel(wheel_path):
        result.errors.append("Wheel installation failed")
        result.success = False
        result.duration_seconds = time.monotonic() - start
        return result

    # Phase 5: Run tests
    result.phase = "test"
    junit_dir = output_dir
    ok, counts, junit_path = run_tests(profile, junit_dir)
    result.counts = counts
    result.junit_path = junit_path

    # Phase 6: Skip budget enforcement
    result.phase = "budget"
    verdict, budget_failures = enforce_skip_budget(profile, counts)
    result.budget_verdict = verdict
    result.budget_failures = budget_failures

    # Phase 7: Evidence generation
    result.phase = "evidence"
    result.evidence_path = generate_evidence(
        profile, counts, verdict, budget_failures,
        wheel_path, junit_path, output_dir,
    )

    # Determine overall success
    result.success = ok and verdict == "PASS"
    result.duration_seconds = time.monotonic() - start
    return result


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run an eggsec-python validation profile.",
    )
    parser.add_argument(
        "--profile",
        required=True,
        help="Profile name from profiles.json (e.g. default-wheel, nse, db-postgres)",
    )
    parser.add_argument(
        "--manifest",
        default=str(DEFAULT_MANIFEST),
        help=f"Path to profiles.json (default: {DEFAULT_MANIFEST})",
    )
    parser.add_argument(
        "--output-dir",
        default=None,
        help="Directory for JUnit XML and evidence output (default: target/python-validation/<profile>)",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="Skip wheel build and install (use existing wheel)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Check prerequisites only, don't build or test",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    manifest_path = Path(args.manifest)
    if not manifest_path.exists():
        log_err(f"Manifest not found: {manifest_path}")
        return 2

    profiles = load_manifest(manifest_path)
    if args.profile not in profiles:
        available = sorted(profiles.keys())
        log_err(f"Profile '{args.profile}' not found. Available: {', '.join(available)}")
        return 2

    profile = profiles[args.profile]
    log(f"Profile: {profile.name} — {profile.description}")

    if args.output_dir:
        output_base = Path(args.output_dir)
    else:
        output_base = REPO_ROOT / "target" / "python-validation"
    output_base.mkdir(parents=True, exist_ok=True)

    if args.dry_run:
        ok, failures = verify_prerequisites(profile)
        if ok:
            log("All prerequisites met")
            return 0
        else:
            for f in failures:
                log_err(f)
            return 1

    result = run_profile(profile, output_base)
    print_summary(result)

    return 0 if result.success else 1


if __name__ == "__main__":
    sys.exit(main())
