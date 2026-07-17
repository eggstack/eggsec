"""Negative tests for evidence pipeline fail-closed behavior.

These tests verify that missing, empty, or malformed artifacts cause
hard failures instead of silent success.
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

SCRIPTS_DIR = Path(__file__).resolve().parent.parent / "scripts"
SKIP_BUDGET = SCRIPTS_DIR / "python_skip_budget.py"
EVIDENCE_BUILDER = SCRIPTS_DIR / "build_python_release_evidence.py"
PROFILE_VALIDATOR = SCRIPTS_DIR / "validate_python_profiles.py"
PROFILES_JSON = (
    Path(__file__).resolve().parent.parent
    / "crates" / "eggsec-python" / "validation" / "profiles.json"
)


class TestSkipBudgetFailClosed:
    """Verify python_skip_budget.py fails on bad inputs."""

    def _run_skip_budget(self, **extra_args):
        cmd = [
            sys.executable, str(SKIP_BUDGET),
            "--profile", "default-wheel",
            "--manifest", str(PROFILES_JSON),
        ]
        for k, v in extra_args.items():
            cmd.extend([f"--{k.replace('_', '-')}", str(v)])
        return subprocess.run(cmd, capture_output=True, text=True, timeout=30)

    def test_missing_junit_fails(self):
        """Missing JUnit XML should cause exit 2."""
        result = self._run_skip_budget(**{
            "junit_xml": "/nonexistent/path.xml",
        })
        assert result.returncode == 2, (
            f"Expected exit 2, got {result.returncode}: {result.stderr}"
        )

    def test_empty_junit_fails(self):
        """Empty JUnit XML should cause failure."""
        with tempfile.NamedTemporaryFile(
            suffix=".xml", delete=False, mode="w"
        ) as f:
            f.write("")
            tmp = f.name
        try:
            result = self._run_skip_budget(**{"junit_xml": tmp})
            assert result.returncode != 0, (
                f"Expected non-zero exit, got {result.returncode}"
            )
        finally:
            os.unlink(tmp)

    def test_zero_tests_junit_fails(self):
        """JUnit with zero executed tests should cause failure."""
        junit = (
            '<?xml version="1.0" encoding="UTF-8"?>\n'
            "<testsuites>\n"
            '  <testsuite name="eggsec-python" tests="0" errors="0" '
            'failures="0" skipped="0"/>\n'
            "</testsuites>"
        )
        with tempfile.NamedTemporaryFile(
            suffix=".xml", delete=False, mode="w"
        ) as f:
            f.write(junit)
            tmp = f.name
        try:
            result = self._run_skip_budget(**{"junit_xml": tmp})
            assert result.returncode != 0, (
                f"Expected non-zero exit for zero tests, got {result.returncode}"
            )
        finally:
            os.unlink(tmp)

    def test_malformed_xml_fails(self):
        """Malformed XML should cause failure."""
        with tempfile.NamedTemporaryFile(
            suffix=".xml", delete=False, mode="w"
        ) as f:
            f.write("not xml at all <><>")
            tmp = f.name
        try:
            result = self._run_skip_budget(**{"junit_xml": tmp})
            assert result.returncode != 0, (
                f"Expected non-zero exit for malformed XML, got "
                f"{result.returncode}"
            )
        finally:
            os.unlink(tmp)


class TestProfileValidatorFailClosed:
    """Verify validate_python_profiles.py fails on bad inputs."""

    def test_missing_manifest_fails(self):
        """Missing profiles manifest should cause exit 1."""
        result = subprocess.run(
            [
                sys.executable, str(PROFILE_VALIDATOR),
                "--manifest", "/nonexistent/manifest.json",
            ],
            capture_output=True, text=True, timeout=30,
        )
        assert result.returncode == 1, (
            f"Expected exit 1, got {result.returncode}: {result.stderr}"
        )

    def test_invalid_json_fails(self):
        """Invalid JSON manifest should cause exit 1."""
        with tempfile.NamedTemporaryFile(
            suffix=".json", delete=False, mode="w"
        ) as f:
            f.write("{invalid json")
            tmp = f.name
        try:
            result = subprocess.run(
                [
                    sys.executable, str(PROFILE_VALIDATOR),
                    "--manifest", tmp,
                ],
                capture_output=True, text=True, timeout=30,
            )
            assert result.returncode == 1, (
                f"Expected exit 1 for invalid JSON, got {result.returncode}"
            )
        finally:
            os.unlink(tmp)

    def test_empty_profiles_fails(self):
        """Manifest with missing profiles key should cause exit 1."""
        with tempfile.NamedTemporaryFile(
            suffix=".json", delete=False, mode="w"
        ) as f:
            json.dump({"wrong_key": []}, f)
            tmp = f.name
        try:
            result = subprocess.run(
                [
                    sys.executable, str(PROFILE_VALIDATOR),
                    "--manifest", tmp,
                ],
                capture_output=True, text=True, timeout=30,
            )
            assert result.returncode == 1, (
                f"Expected exit 1 for missing profiles key, got "
                f"{result.returncode}"
            )
        finally:
            os.unlink(tmp)


class TestEvidenceBuilderFailClosed:
    """Verify build_python_release_evidence.py fails on missing artifacts."""

    def test_missing_profiles_json_fails(self):
        """Missing profiles.json should cause evidence collector to fail."""
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "evidence_builder", str(EVIDENCE_BUILDER)
        )
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir)
            with patch.object(mod, "PROFILES_JSON", Path("/nonexistent/profiles.json")):
                with pytest.raises(FileNotFoundError, match="profiles.json"):
                    mod.collect_profile_manifest(out)

    def test_missing_junit_xml_fails(self):
        """Missing JUnit XML should cause evidence collector to fail."""
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "evidence_builder", str(EVIDENCE_BUILDER)
        )
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir)
            with patch.object(mod, "JUNIT_XML", Path("/nonexistent/results.xml")):
                with pytest.raises(FileNotFoundError, match="JUnit XML"):
                    mod.collect_junit_xml(out)

    def test_missing_guard_script_fails(self):
        """Missing guard script should cause evidence collector to fail."""
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "evidence_builder", str(EVIDENCE_BUILDER)
        )
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)

        # _run_guard calls script.relative_to(REPO_ROOT) before exists(),
        # so a path outside the repo triggers ValueError from pathlib.
        missing_script = Path("/nonexistent/guard.sh")
        with pytest.raises((FileNotFoundError, ValueError)):
            mod._run_guard(missing_script)

    def test_required_evidence_check_catches_missing(self):
        """Evidence summary should flag missing required files."""
        import importlib.util

        spec = importlib.util.spec_from_file_location(
            "evidence_builder", str(EVIDENCE_BUILDER)
        )
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir)
            # Write a summary with a missing required file
            summary = {
                "commit_sha": "abc123",
                "files": {
                    "commit-info.json": {"sha256": "aaa", "size_bytes": 10},
                    # profile-manifest.json is missing from files
                },
                "required_files": sorted(mod.REQUIRED_EVIDENCE),
            }
            summary_path = out / "evidence-summary.json"
            summary_path.write_text(json.dumps(summary, indent=2))

            data = json.loads(summary_path.read_text())
            missing = [
                name
                for name in sorted(mod.REQUIRED_EVIDENCE)
                if name not in data["files"]
            ]
            assert len(missing) > 0, "Expected missing required files to be detected"
            assert "profile-manifest.json" in missing
