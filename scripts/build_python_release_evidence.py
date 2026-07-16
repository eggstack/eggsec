#!/usr/bin/env python3
"""Generate a commit-bound evidence bundle for Python release validation.

Creates a directory under target/python-validation/<commit-sha>/ containing
JSON evidence files that pin the exact toolchain, feature set, test results,
and guard outcomes for a given commit.  An evidence-summary.json with SHA-256
checksums over every generated file provides tamper detection.

Exit 0 when all required evidence files are produced; exit 1 when any
required file is missing.
"""
import argparse
import datetime
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
PYTHON_CRATE = REPO_ROOT / "crates" / "eggsec-python"
PROFILES_JSON = PYTHON_CRATE / "validation" / "profiles.json"
CARGO_TOML = PYTHON_CRATE / "Cargo.toml"
WHEEL_DIR = REPO_ROOT / "target" / "python-wheels"
JUNIT_XML = PYTHON_CRATE / "python" / "test-results.xml"
GUARD_SCRIPT = REPO_ROOT / "scripts" / "check-architecture-guards.sh"
CAPABILITY_MATRIX_SCRIPT = REPO_ROOT / "scripts" / "check-python-capability-matrix.py"
STUB_PARITY_SCRIPT = REPO_ROOT / "scripts" / "check_python_stub_parity.py"
DOMAINS_RS = PYTHON_CRATE / "src" / "domains.rs"

REQUIRED_EVIDENCE = {
    "profile-manifest.json",
    "commit-info.json",
    "toolchain.json",
    "cargo-features.json",
    "platform-details.json",
    "maturity-decision.json",
    "evidence-summary.json",
}


def sha256(path: Path) -> str:
    import hashlib

    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=30,
    )
    return result.stdout.strip()


def tool_version(cmd: str) -> str:
    result = subprocess.run(
        [cmd, "--version"],
        capture_output=True,
        text=True,
        timeout=15,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def python_version() -> str:
    return sys.version.split()[0]


# ---------------------------------------------------------------------------
# Evidence collectors
# ---------------------------------------------------------------------------

def collect_profile_manifest(out_dir: Path) -> Path:
    dest = out_dir / "profile-manifest.json"
    if PROFILES_JSON.exists():
        import shutil

        shutil.copy2(PROFILES_JSON, dest)
    else:
        dest.write_text(json.dumps({"profiles": [], "_note": "profiles.json not found"}, indent=2))
    return dest


def collect_commit_info(commit: str, out_dir: Path) -> Path:
    dest = out_dir / "commit-info.json"
    dirty = subprocess.run(
        ["git", "diff", "--quiet", "HEAD"],
        cwd=REPO_ROOT,
        capture_output=True,
    )
    dirty_tree = dirty.returncode != 0
    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    message = git("log", "-1", "--pretty=%s")
    author = git("log", "-1", "--pretty=%an")
    date = git("log", "-1", "--pretty=%aI")
    data = {
        "sha": commit,
        "short_sha": commit[:12],
        "dirty_tree": dirty_tree,
        "branch": branch,
        "commit_message": message,
        "author": author,
        "commit_date": date,
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_toolchain(out_dir: Path) -> Path:
    dest = out_dir / "toolchain.json"
    data = {
        "rustc": tool_version("rustc"),
        "cargo": tool_version("cargo"),
        "python": python_version(),
        "python_implementation": platform.python_implementation(),
        "platform": platform.platform(),
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_cargo_features(out_dir: Path) -> Path:
    dest = out_dir / "cargo-features.json"
    features: dict[str, Any] = {}
    if CARGO_TOML.exists():
        text = CARGO_TOML.read_text()
        in_features = False
        for line in text.splitlines():
            if line.strip().startswith("[features]"):
                in_features = True
                continue
            if in_features and line.strip().startswith("["):
                break
            if in_features:
                m = re.match(r'^\s*(\S+)\s*=\s*\[(.*)\]', line)
                if m and not line.strip().startswith("#"):
                    name = m.group(1)
                    deps_raw = m.group(2).strip()
                    deps = [d.strip().strip('"') for d in deps_raw.split(",") if d.strip().strip('"')]
                    features[name] = deps
    data = {"features": features, "feature_count": len(features)}
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_wheel_info(out_dir: Path) -> Path:
    dest = out_dir / "wheel-info.json"
    wheels = sorted(WHEEL_DIR.glob("*.whl")) if WHEEL_DIR.exists() else []
    if not wheels:
        data = {"wheel_exists": False, "wheels": []}
    else:
        wheel_entries = []
        for w in wheels:
            import hashlib

            h = hashlib.sha256()
            with open(w, "rb") as f:
                for chunk in iter(lambda: f.read(65536), b""):
                    h.update(chunk)
            wheel_entries.append({
                "filename": w.name,
                "sha256": h.hexdigest(),
                "size_bytes": w.stat().st_size,
                "path": str(w),
            })
        data = {"wheel_exists": True, "wheel_count": len(wheel_entries), "wheels": wheel_entries}
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def _parse_junit(path: Path) -> dict[str, Any]:
    """Parse JUnit XML for pass/fail/skip/xfail counts and detailed records."""
    if not path.exists():
        return {"exists": False}

    import xml.etree.ElementTree as ET

    try:
        tree = ET.parse(path)
    except ET.ParseError:
        return {"exists": True, "parse_error": True}

    root = tree.getroot()
    # Support both <testsuites> and direct <testsuite>
    if root.tag == "testsuites":
        suites = root.findall("testsuite")
    elif root.tag == "testsuite":
        suites = [root]
    else:
        suites = []

    pass_count = 0
    fail_count = 0
    skip_count = 0
    xfail_count = 0
    error_count = 0
    total = 0
    skips: list[dict[str, str]] = []
    xfails: list[dict[str, str]] = []

    for suite in suites:
        for tc in suite.findall("testcase"):
            total += 1
            # Check for <skipped>
            skipped = tc.find("skipped")
            if skipped is not None:
                skip_count += 1
                skips.append({
                    "name": tc.get("name", ""),
                    "classname": tc.get("classname", ""),
                    "message": skipped.get("message", ""),
                })
                continue
            # Check for <failure>
            failure = tc.find("failure")
            if failure is not None:
                fail_count += 1
                continue
            # Check for <error>
            error = tc.find("error")
            if error is not None:
                error_count += 1
                continue
            # Check for xfail (pytest marks xfail via <failure> with type="xfail"
            # or as a custom attribute — pytest-junitxml uses the "xfail" tag)
            xfail_tag = tc.find("xfail")
            if xfail_tag is not None:
                xfail_count += 1
                xfails.append({
                    "name": tc.get("name", ""),
                    "classname": tc.get("classname", ""),
                    "message": xfail_tag.get("message", xfail_tag.text or ""),
                })
                continue
            pass_count += 1

    return {
        "exists": True,
        "parse_error": False,
        "total": total,
        "passed": pass_count,
        "failed": fail_count,
        "skipped": skip_count,
        "xfail": xfail_count,
        "errors": error_count,
    }


def collect_test_counts(out_dir: Path) -> Path:
    dest = out_dir / "test-counts.json"
    counts = _parse_junit(JUNIT_XML)
    dest.write_text(json.dumps(counts, indent=2) + "\n")
    return dest


def collect_skip_xfail_report(out_dir: Path) -> Path:
    dest = out_dir / "skip-xfail-report.json"
    counts = _parse_junit(JUNIT_XML)

    skips = []
    xfails = []

    if counts.get("exists") and not counts.get("parse_error"):
        import xml.etree.ElementTree as ET

        try:
            tree = ET.parse(JUNIT_XML)
        except ET.ParseError:
            pass
        else:
            root = tree.getroot()
            suites = root.findall("testsuite") if root.tag == "testsuites" else (
                [root] if root.tag == "testsuite" else []
            )
            for suite in suites:
                for tc in suite.findall("testcase"):
                    skipped = tc.find("skipped")
                    if skipped is not None:
                        skips.append({
                            "test_name": tc.get("name", ""),
                            "classname": tc.get("classname", ""),
                            "reason": skipped.get("message", ""),
                        })
                    xfail_tag = tc.find("xfail")
                    if xfail_tag is not None:
                        xfails.append({
                            "test_name": tc.get("name", ""),
                            "classname": tc.get("classname", ""),
                            "reason": xfail_tag.get("message", xfail_tag.text or ""),
                        })

    data = {
        "source": str(JUNIT_XML.relative_to(REPO_ROOT)) if JUNIT_XML.exists() else None,
        "skip_count": len(skips),
        "xfail_count": len(xfails),
        "skips": skips,
        "xfails": xfails,
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def _run_guard(script: Path) -> dict[str, Any]:
    """Run a guard script and capture pass/fail result."""
    name = script.stem
    result = {
        "script": str(script.relative_to(REPO_ROOT)),
        "passed": False,
        "return_code": None,
        "output_preview": "",
    }
    if not script.exists():
        result["output_preview"] = "script not found"
        return result
    try:
        proc = subprocess.run(
            [sys.executable, str(script)] if script.suffix == ".py" else ["bash", str(script)],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=REPO_ROOT,
        )
        result["return_code"] = proc.returncode
        result["passed"] = proc.returncode == 0
        # Keep last 20 lines for context
        lines = (proc.stdout + proc.stderr).strip().splitlines()
        result["output_preview"] = "\n".join(lines[-20:]) if lines else ""
    except subprocess.TimeoutExpired:
        result["output_preview"] = "timeout after 120s"
    except Exception as e:
        result["output_preview"] = str(e)
    return result


def collect_guard_results(out_dir: Path) -> Path:
    dest = out_dir / "guard-results.json"
    guards = {
        "architecture_guards": _run_guard(GUARD_SCRIPT),
        "capability_matrix": _run_guard(CAPABILITY_MATRIX_SCRIPT),
        "stub_parity": _run_guard(STUB_PARITY_SCRIPT),
    }
    all_passed = all(g["passed"] for g in guards.values())
    data = {
        "all_passed": all_passed,
        "guards": guards,
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_type_check_results(out_dir: Path) -> Path:
    dest = out_dir / "type-check-results.json"
    results: dict[str, Any] = {}

    # mypy
    mypy_result = subprocess.run(
        [sys.executable, "-m", "mypy", "--version"],
        capture_output=True, text=True, timeout=10,
    )
    if mypy_result.returncode == 0:
        proc = subprocess.run(
            [sys.executable, "-m", "mypy", str(PYTHON_CRATE / "python" / "eggsec"),
             "--ignore-missing-imports", "--no-error-summary"],
            capture_output=True, text=True, timeout=120, cwd=REPO_ROOT,
        )
        results["mypy"] = {
            "available": True,
            "passed": proc.returncode == 0,
            "return_code": proc.returncode,
            "output": proc.stdout.strip()[-2000:] if proc.stdout else "",
        }
    else:
        results["mypy"] = {"available": False, "reason": "mypy not installed"}

    # pyright
    pyright_result = subprocess.run(
        ["pyright", "--version"],
        capture_output=True, text=True, timeout=10,
    )
    if pyright_result.returncode == 0:
        proc = subprocess.run(
            ["pyright", str(PYTHON_CRATE / "python" / "eggsec")],
            capture_output=True, text=True, timeout=120, cwd=REPO_ROOT,
        )
        results["pyright"] = {
            "available": True,
            "passed": proc.returncode == 0,
            "return_code": proc.returncode,
            "output": proc.stdout.strip()[-2000:] if proc.stdout else "",
        }
    else:
        results["pyright"] = {"available": False, "reason": "pyright not installed"}

    results["overall_passed"] = all(
        r.get("passed", True) for r in results.values() if isinstance(r, dict) and "passed" in r
    )
    dest.write_text(json.dumps(results, indent=2) + "\n")
    return dest


def collect_binary_size_report(out_dir: Path) -> Path:
    dest = out_dir / "binary-size-report.json"
    # Look for any existing binary size report in target/python-validation/
    existing = REPO_ROOT / "target" / "python-validation"
    candidates = list(existing.glob("*size*")) + list(existing.glob("*binary*")) if existing.exists() else []
    if candidates:
        import shutil

        shutil.copy2(candidates[0], dest)
    else:
        # Try to compute wheel sizes as a fallback
        wheels = sorted(WHEEL_DIR.glob("*.whl")) if WHEEL_DIR.exists() else []
        entries = []
        for w in wheels:
            entries.append({
                "filename": w.name,
                "size_bytes": w.stat().st_size,
            })
        data = {
            "source": "computed" if entries else "no-data",
            "wheels": entries,
        }
        dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def _parse_domain_maturity() -> dict[str, str]:
    """Extract domain -> maturity from domains.rs."""
    if not DOMAINS_RS.exists():
        return {}
    content = DOMAINS_RS.read_text()
    pattern = re.compile(
        r'\(\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]+)"\s*,\s*\n?\s*"([^"]*)"\s*,?\s*\n?\s*\)',
        re.MULTILINE,
    )
    result = {}
    for m in pattern.finditer(content):
        result[m.group(1)] = m.group(2)
    return result


def collect_maturity_decision(out_dir: Path) -> Path:
    dest = out_dir / "maturity-decision.json"
    domains = _parse_domain_maturity()
    data = {
        "domains": domains,
        "domain_count": len(domains),
        "source": str(DOMAINS_RS.relative_to(REPO_ROOT)) if DOMAINS_RS.exists() else None,
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_platform_details(out_dir: Path) -> Path:
    dest = out_dir / "platform-details.json"
    uname = os.uname()
    data = {
        "system": uname.sysname,
        "node": uname.nodename,
        "release": uname.release,
        "version": uname.version,
        "machine": uname.machine,
        "architecture": platform.machine(),
        "processor": platform.processor() or "unknown",
        "python_build": platform.python_build(),
        "python_compiler": platform.python_compiler(),
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


def collect_junit_xml(out_dir: Path) -> Path:
    """Copy JUnit XML test results if available."""
    dest = out_dir / "junit-results.xml"
    if JUNIT_XML.exists():
        import shutil
        shutil.copy2(JUNIT_XML, dest)
    else:
        dest.write_text(
            '<?xml version="1.0" encoding="UTF-8"?>\n'
            '<testsuites _note="JUnit XML not found">\n'
            '  <testsuite name="eggsec-python" tests="0"/>\n'
            "</testsuites>\n"
        )
    return dest


def collect_performance_report(out_dir: Path) -> Path:
    """Collect performance and leak test results."""
    dest = out_dir / "performance-report.json"
    perf_data: dict[str, Any] = {}

    perf_markers = ["test_performance", "test_stress_leak", "test_leak"]
    test_dir = PYTHON_CRATE / "tests"
    if test_dir.exists():
        perf_files = []
        for f in test_dir.glob("test_*.py"):
            content = f.read_text()
            for marker in perf_markers:
                if marker in content:
                    perf_files.append(f.name)
                    break
        perf_data["performance_test_files"] = perf_files

    perf_data["baseline"] = {
        "description": "Baseline metrics collected during CI run",
        "note": "Populated by test_stress_leak.py and test_performance_report.py",
    }

    dest.write_text(json.dumps(perf_data, indent=2) + "\n")
    return dest


def collect_fixture_versions(out_dir: Path) -> Path:
    """Collect subsystem fixture versions and dependencies."""
    dest = out_dir / "fixture-versions.json"
    fixture_data: dict[str, Any] = {}

    cert_path = PYTHON_CRATE / "tests" / "fixtures" / "fixture-cert.pem"
    if cert_path.exists():
        import hashlib
        h = hashlib.sha256(cert_path.read_bytes()).hexdigest()[:16]
        fixture_data["tls_fixture_cert"] = {
            "path": str(cert_path.relative_to(REPO_ROOT)),
            "sha256_prefix": h,
            "size_bytes": cert_path.stat().st_size,
        }

    nse_fixture = PYTHON_CRATE / "tests" / "fixtures" / "nse_loopback.py"
    if nse_fixture.exists():
        import hashlib
        h = hashlib.sha256(nse_fixture.read_bytes()).hexdigest()[:16]
        fixture_data["nse_loopback_fixture"] = {
            "path": str(nse_fixture.relative_to(REPO_ROOT)),
            "sha256_prefix": h,
            "size_bytes": nse_fixture.stat().st_size,
        }

    stable_core = PYTHON_CRATE / "tests" / "fixtures" / "stable_core.py"
    if stable_core.exists():
        import hashlib
        h = hashlib.sha256(stable_core.read_bytes()).hexdigest()[:16]
        fixture_data["stable_core_fixture"] = {
            "path": str(stable_core.relative_to(REPO_ROOT)),
            "sha256_prefix": h,
            "size_bytes": stable_core.stat().st_size,
        }

    dest.write_text(json.dumps(fixture_data, indent=2) + "\n")
    return dest


# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

def build_summary(commit: str, out_dir: Path, file_paths: list[Path]) -> Path:
    dest = out_dir / "evidence-summary.json"
    checksums = {}
    for p in file_paths:
        rel = p.name
        if p.exists():
            checksums[rel] = {
                "sha256": sha256(p),
                "size_bytes": p.stat().st_size,
            }
        else:
            checksums[rel] = {"sha256": None, "size_bytes": 0}

    # Determine if all required files are present with valid checksums
    required_present = all(
        name in checksums and checksums[name]["sha256"] is not None
        for name in REQUIRED_EVIDENCE
    )

    data = {
        "commit_sha": commit,
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "generator": "scripts/build_python_release_evidence.py",
        "files": checksums,
        "required_files_present": required_present,
        "required_files": sorted(REQUIRED_EVIDENCE),
        "total_files": len(checksums),
    }
    dest.write_text(json.dumps(data, indent=2) + "\n")
    return dest


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate commit-bound evidence bundle for Python release validation.",
    )
    parser.add_argument(
        "--commit",
        required=True,
        help="Git commit SHA to bind to the evidence bundle (required).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    commit = args.commit

    out_dir = REPO_ROOT / "target" / "python-validation" / commit
    out_dir.mkdir(parents=True, exist_ok=True)

    collectors = [
        ("profile-manifest.json", lambda: collect_profile_manifest(out_dir)),
        ("commit-info.json", lambda: collect_commit_info(commit, out_dir)),
        ("toolchain.json", lambda: collect_toolchain(out_dir)),
        ("cargo-features.json", lambda: collect_cargo_features(out_dir)),
        ("wheel-info.json", lambda: collect_wheel_info(out_dir)),
        ("test-counts.json", lambda: collect_test_counts(out_dir)),
        ("skip-xfail-report.json", lambda: collect_skip_xfail_report(out_dir)),
        ("guard-results.json", lambda: collect_guard_results(out_dir)),
        ("type-check-results.json", lambda: collect_type_check_results(out_dir)),
        ("binary-size-report.json", lambda: collect_binary_size_report(out_dir)),
        ("maturity-decision.json", lambda: collect_maturity_decision(out_dir)),
        ("platform-details.json", lambda: collect_platform_details(out_dir)),
        ("junit-results.xml", lambda: collect_junit_xml(out_dir)),
        ("performance-report.json", lambda: collect_performance_report(out_dir)),
        ("fixture-versions.json", lambda: collect_fixture_versions(out_dir)),
    ]

    generated: list[Path] = []
    for name, collector in collectors:
        try:
            path = collector()
            generated.append(path)
            print(f"  [ok] {name}")
        except Exception as exc:
            print(f"  [FAIL] {name}: {exc}", file=sys.stderr)
            # Create a stub so the summary can reference it
            stub = out_dir / name
            stub.write_text(json.dumps({"error": str(exc)}, indent=2) + "\n")
            generated.append(stub)

    summary_path = build_summary(commit, out_dir, generated)
    print(f"  [ok] evidence-summary.json")

    print(f"\nEvidence bundle: {out_dir}")
    print(f"Files: {len(generated) + 1}")

    # Check required files (evidence-summary.json is the manifest itself;
    # verify it exists and is non-empty but skip the self-referencing checksum check)
    summary_data = json.loads(summary_path.read_text())
    missing = []
    for name in sorted(REQUIRED_EVIDENCE):
        p = out_dir / name
        if not p.exists() or p.stat().st_size == 0:
            missing.append(name)
        elif name != "evidence-summary.json":
            entry = summary_data["files"].get(name, {})
            if entry.get("sha256") is None:
                missing.append(name)

    if missing:
        print(f"\nMISSING required files: {', '.join(missing)}", file=sys.stderr)
        return 1

    print("\nAll required evidence files generated.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
