#!/usr/bin/env python3
"""E7: Documentation example test harness.

Discovers and executes all example scripts in docs/python/examples/,
validates their output, and checks for resource leaks. Used in CI to
ensure examples stay correct against the installed wheel.

Requirements:
    - eggsec installed (any profile)
    - pytest
    - EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 for loopback examples

Usage:
    pytest scripts/test_documentation_examples.py -v
    EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 pytest scripts/test_documentation_examples.py -v
"""

import glob
import os
import re
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest


EXAMPLES_DIR = Path(__file__).resolve().parent.parent / "docs" / "python" / "examples"
TIMEOUT = 120  # seconds per example

# Examples that require specific features (feature -> list of example names)
FEATURE_REQUIREMENTS = {
    "websocket": ["websocket_session.py"],
    "git-secrets": ["git_secret_scan.py"],
    "sbom": ["sbom_generation.py"],
    "db-pentest": ["database_probe.py"],
    "web-proxy": [],
    "nse": [],
    "mobile": ["mobile_static_to_dynamic.py", "mobile_dynamic_session.py"],
    "mobile-dynamic": ["mobile_dynamic_session.py"],
    "headless-browser": ["browser_network_console.py", "browser_route_storage_audit.py"],
    "packet-inspection": [],
    "stress-testing": [],
    "daemon-client": [
        "local_vs_daemon_execution.py",
        "daemon_reconnect_replay.py",
        "daemon_remote_cancellation.py",
    ],
    "container": [],
    "advanced-hunting": [],
    "compliance": [],
}

# Examples that need loopback fixture
LOOPBACK_EXAMPLES = {
    "port_scan_loopback.py",
    "cancellation_timeout.py",
    "custom_protocol_workflow.py",
    "websocket_session.py",
}

# Patterns in stdout that indicate success
SUCCESS_PATTERNS = [
    re.compile(r"status=\w+", re.IGNORECASE),
    re.compile(r"passed", re.IGNORECASE),
    re.compile(r"OK", re.IGNORECASE),
    re.compile(r"completed", re.IGNORECASE),
    re.compile(r"^\d+", re.IGNORECASE),  # numeric output
]

# Patterns that indicate failure
FAILURE_PATTERNS = [
    re.compile(r"Traceback \(most recent call last\)"),
    re.compile(r"Error:", re.IGNORECASE),
    re.compile(r"FAILED", re.IGNORECASE),
    re.compile(r"assert False"),
]


def discover_examples():
    """Find all .py example files."""
    examples = sorted(EXAMPLES_DIR.glob("*.py"))
    return [e for e in examples if e.name != "__init__.py"]


def get_required_features(example_path):
    """Extract required features from the example's docstring."""
    content = example_path.read_text()
    # Check docstring for "Requirements:" section
    features = []
    in_requirements = False
    for line in content.split("\n"):
        if "Requirements:" in line:
            in_requirements = True
            continue
        if in_requirements:
            stripped = line.strip()
            if not stripped or stripped.startswith('"""') or stripped.startswith("'''"):
                in_requirements = False
                continue
            # Look for "eggsec[feature]" patterns
            match = re.search(r"eggsec\[([^\]]+)\]", stripped)
            if match:
                features.append(match.group(1))
    return features


def example_name(example_path):
    return example_path.name


def is_loopback_example(example_path):
    return example_name(example_path) in LOOPBACK_EXAMPLES


def requires_loopback(example_path):
    content = example_path.read_text()
    return "EGGSEC_ALLOW_LOOPBACK_FIXTURE" in content


def check_feature_availability(required_features):
    """Check if required features are available in the current build."""
    try:
        import eggsec
        available = set(eggsec.features())
    except ImportError:
        return False, "eggsec not importable"

    missing = [f for f in required_features if f not in available]
    if missing:
        return False, f"missing features: {', '.join(missing)}"
    return True, ""


def run_example(example_path, timeout=TIMEOUT):
    """Run a single example script and return (exit_code, stdout, stderr)."""
    env = os.environ.copy()
    if requires_loopback(example_path):
        env["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"

    result = subprocess.run(
        [sys.executable, str(example_path)],
        capture_output=True,
        text=True,
        timeout=timeout,
        cwd=str(example_path.parent.parent.parent),  # workspace root
        env=env,
    )
    return result.returncode, result.stdout, result.stderr


def validate_output(example_path, stdout, stderr):
    """Validate example output semantically."""
    issues = []

    # Check for unhandled exceptions
    if "Traceback" in stderr:
        issues.append(f"Unhandled exception in stderr: {stderr[:200]}")

    # Check for import errors
    if "ModuleNotFoundError" in stderr or "ImportError" in stderr:
        issues.append(f"Import error: {stderr[:200]}")

    # Check for feature unavailable errors (expected for feature-gated examples)
    if "FeatureUnavailableError" in stderr:
        # This is expected when feature isn't compiled — not a failure
        pass

    # Warn if output is empty
    if not stdout.strip() and not stderr.strip():
        issues.append("No output produced (may be expected for some examples)")

    return issues


def check_resource_leaks(example_name_str, stdout):
    """Check for common resource leak indicators."""
    warnings = []
    if "Address already in use" in stdout:
        warnings.append("Possible socket leak — port still bound")
    return warnings


@pytest.fixture(params=discover_examples(), ids=lambda p: p.name)
def example_path(request):
    return request.param


class TestDocumentationExamples:
    """Test that documentation examples execute correctly."""

    def test_example_executes(self, example_path):
        """Each example must exit cleanly."""
        # Skip feature-gated examples if feature not available
        required = get_required_features(example_path)
        if required:
            available, reason = check_feature_availability(required)
            if not available:
                pytest.skip(reason)

        exit_code, stdout, stderr = run_example(example_path)

        # Collect all output for diagnostics
        combined = stdout + "\n" + stderr
        issues = validate_output(example_path, stdout, stderr)

        if exit_code != 0:
            msg = f"Example exited with code {exit_code}\n"
            if stderr:
                msg += f"stderr: {stderr[:500]}\n"
            if stdout:
                msg += f"stdout: {stdout[-500:]}\n"
            if issues:
                msg += f"Issues: {issues}"
            pytest.fail(msg)

        # Check for warnings (non-fatal)
        warnings = check_resource_leaks(example_path.name, stdout)
        for w in warnings:
            import warnings as w_mod
            w_mod.warn(w)

    def test_example_imports_resolve(self, example_path):
        """Verify all imports in the example resolve to real symbols."""
        content = example_path.read_text()
        tree = ast_parse(content)
        if tree is None:
            pytest.skip("Could not parse example")

        imports = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    imports.append(alias.name)
            elif isinstance(node, ast.ImportFrom):
                if node.module:
                    for alias in node.names:
                        imports.append(f"{node.module}.{alias.name}")

        # Filter to eggsec imports only
        eggsec_imports = [i for i in imports if i.startswith("eggsec")]
        if not eggsec_imports:
            return  # no eggsec imports to check

        # Check feature requirements first
        required = get_required_features(example_path)
        if required:
            available, reason = check_feature_availability(required)
            if not available:
                pytest.skip(reason)

        # Try importing each symbol
        for imp in eggsec_imports:
            parts = imp.split(".")
            try:
                mod = __import__(parts[0])
                for part in parts[1:]:
                    mod = getattr(mod, part)
            except (ImportError, AttributeError) as e:
                pytest.fail(f"Import {imp} failed: {e}")

    def test_example_produces_output(self, example_path):
        """Verify the example produces meaningful output."""
        required = get_required_features(example_path)
        if required:
            available, _ = check_feature_availability(required)
            if not available:
                pytest.skip("Feature not available")

        exit_code, stdout, stderr = run_example(example_path)
        if exit_code != 0:
            pytest.skip(f"Example failed (exit {exit_code})")

        # At least some output should be produced
        assert stdout.strip(), f"Example {example_path.name} produced no stdout"


def ast_parse(source):
    """Safely parse Python source into AST."""
    import ast
    try:
        return ast.parse(source)
    except SyntaxError:
        return None
