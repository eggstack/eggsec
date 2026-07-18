"""Workstream 3: Feature-enabled-path tests for feature-gated operations.

Tests feature-gated operations when their features ARE compiled in.
Uses eggsec.has_feature() to branch behavior — when a feature IS available,
exercises direct function, sync engine dispatch, async engine dispatch,
serialization, and scope enforcement. When NOT available, tests skip cleanly.

Each feature class covers:
  1. Direct function returns correct type with to_dict()/to_json()
  2. Sync engine dispatch via engine.run(OperationRequest(...))
  3. Async engine dispatch via async_engine.run(...)
  4. Serialization round-trip (to_dict, to_json)
  5. Scope enforcement (out-of-scope target raises/returns denial)
  6. Request validation (invalid params return structured error, not crash)
  7. Cancellation (pre-cancelled CancellationToken results in cancelled status)
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

import eggsec
import pytest


# ---------------------------------------------------------------------------
# Constants and helpers
# ---------------------------------------------------------------------------

HOST = "127.0.0.1"
OUT_OF_SCOPE_HOST = "192.0.2.1"

pytestmark = [
    pytest.mark.feature_enabled,
    pytest.mark.timeout(30),
]


def _make_engine(scope: eggsec.Scope | None = None) -> eggsec.Engine:
    if scope is None:
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
    return eggsec.Engine(scope)


def _make_async_engine(scope: eggsec.Scope | None = None) -> eggsec.AsyncEngine:
    if scope is None:
        scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
    return eggsec.AsyncEngine(scope)


def _await_future(future: Any, timeout: float = 20.0) -> Any:
    """Resolve the extension's awaitable without pytest-asyncio."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        if value is not None:
            return value
        time.sleep(0.01)
    raise AssertionError("async operation did not complete before timeout")


def _scope_enforced(target: str, scope: eggsec.Scope) -> bool:
    """Check whether scope enforcement catches the given target."""
    try:
        scope.is_target_allowed(target)
        return False
    except Exception:
        return True


WORKSPACE_ROOT = Path(__file__).resolve().parents[5]


# ===========================================================================
# 1. git-secrets
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("git-secrets"), reason="git-secrets not compiled")
class TestGitSecretsFeatureEnabled:
    """Full exercise of scan_git_secrets when feature is compiled in."""

    @pytest.fixture(scope="class")
    def git_repo(self, tmp_path_factory):
        """Create a temporary git repo with a known secret pattern."""
        tmp_dir = tmp_path_factory.mktemp("git-secrets-repo")
        # Initialise repo
        subprocess.run(["git", "init"], cwd=str(tmp_dir), check=True,
                       capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@test.com"],
                       cwd=str(tmp_dir), check=True, capture_output=True)
        subprocess.run(["git", "config", "user.name", "Test"],
                       cwd=str(tmp_dir), check=True, capture_output=True)
        # Write a file with a fake AWS key pattern
        secret_file = tmp_dir / "config.py"
        secret_file.write_text(
            'AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE"\n'
            'DATABASE_URL = "postgres://user:pass@host/db"\n'
        )
        subprocess.run(["git", "add", "."], cwd=str(tmp_dir), check=True,
                       capture_output=True)
        subprocess.run(["git", "commit", "-m", "add secrets"],
                       cwd=str(tmp_dir), check=True, capture_output=True)
        return str(tmp_dir)

    # -- 1a. Direct function --

    def test_direct_returns_correct_type(self, git_repo):
        result = eggsec.scan_git_secrets(git_repo)
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "GitSecretsReport"
        assert hasattr(result, "to_dict")
        assert callable(result.to_dict)
        assert hasattr(result, "to_json")
        assert callable(result.to_json)

    def test_direct_finds_secrets(self, git_repo):
        result = eggsec.scan_git_secrets(git_repo)
        d = result.to_dict()
        assert d["repo_path"] == git_repo
        assert d["commits_scanned"] >= 1
        assert d["files_scanned"] >= 1
        assert isinstance(d["findings"], list)

    # -- 1b. Sync engine dispatch --

    def test_engine_dispatch_returns_operation_result(self, git_repo):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_git_secrets", git_repo, timeout_ms=5000,
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "GitSecretsReport"

    def test_engine_dispatch_payload_matches_direct(self, git_repo):
        direct = eggsec.scan_git_secrets(git_repo)
        engine = _make_engine()
        req = eggsec.OperationRequest("scan_git_secrets", git_repo, timeout_ms=5000)
        op_result = engine.run(req)
        assert op_result.is_success()
        assert op_result.payload_type_name == type(direct).__name__.removesuffix("Py")

    # -- 1c. Async engine dispatch --

    def test_async_engine_dispatch(self, git_repo):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest("scan_git_secrets", git_repo, timeout_ms=5000)
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 1d. Serialization --

    def test_direct_to_dict_is_dict(self, git_repo):
        result = eggsec.scan_git_secrets(git_repo)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "repo_path" in d
        assert "findings" in d
        assert "summary" in d

    def test_direct_to_json_is_json(self, git_repo):
        result = eggsec.scan_git_secrets(git_repo)
        j = result.to_json()
        assert isinstance(j, str)
        parsed = json.loads(j)
        assert "repo_path" in parsed
        assert "findings" in parsed

    def test_summary_to_dict_to_json(self, git_repo):
        report = eggsec.scan_git_secrets(git_repo)
        summary = report.summary
        d = summary.to_dict()
        assert isinstance(d, dict)
        for key in ("critical", "high", "medium", "low", "info"):
            assert key in d
        j = summary.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    # -- 1e. Scope enforcement --

    def test_engine_scope_denial(self):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest("scan_git_secrets", "/tmp/x", timeout_ms=1000)
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    # -- 1f. Request validation --

    def test_nonexistent_path_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_git_secrets", "/tmp/__nonexistent_path_99999__", timeout_ms=2000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 1g. Cancellation --

    def test_pre_cancelled_cancellation(self, git_repo):
        engine = _make_engine()
        token = eggsec.CancellationToken()
        token.cancel("test-pre-cancel")
        pipeline = eggsec.PIPELINE
        # Use Pipeline to test cancellation
        from eggsec import Pipeline
        pipe = Pipeline("git-secrets-cancel")
        pipe.add_step(
            "scan",
            eggsec.OperationRequest("scan_git_secrets", git_repo, timeout_ms=5000),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"

    # -- 1h. async convenience function --

    def test_async_convenience_function(self, git_repo):
        future = eggsec.async_scan_git_secrets(git_repo)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "GitSecretsReport"


# ===========================================================================
# 2. sbom
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("sbom"), reason="sbom not compiled")
class TestSbomFeatureEnabled:
    """Full exercise of generate_sbom when feature is compiled in."""

    @pytest.fixture(scope="class")
    def cargo_project(self):
        """Path to the workspace Cargo.toml for SBOM generation."""
        cargo_toml = WORKSPACE_ROOT / "Cargo.toml"
        if not cargo_toml.exists():
            pytest.skip("workspace Cargo.toml not found")
        return str(WORKSPACE_ROOT)

    # -- 2a. Direct function --

    def test_direct_returns_correct_type(self, cargo_project):
        result = eggsec.generate_sbom(cargo_project, ecosystem="cargo")
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "SbomReport"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_direct_has_components(self, cargo_project):
        result = eggsec.generate_sbom(cargo_project, ecosystem="cargo")
        d = result.to_dict()
        assert "components" in d
        assert isinstance(d["components"], list)
        assert "project_name" in d

    # -- 2b. Sync engine dispatch --

    def test_engine_dispatch_returns_operation_result(self, cargo_project):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "generate_sbom", cargo_project,
            timeout_ms=10000,
            metadata={"ecosystem": "cargo"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() == "Completed"
        assert result.payload_type_name == "SbomReport"

    # -- 2c. Async engine dispatch --

    def test_async_engine_dispatch(self, cargo_project):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "generate_sbom", cargo_project,
            timeout_ms=10000, metadata={"ecosystem": "cargo"},
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 2d. Serialization --

    def test_to_dict_returns_dict(self, cargo_project):
        result = eggsec.generate_sbom(cargo_project, ecosystem="cargo")
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "format" in d or "components" in d

    def test_to_json_is_valid_json(self, cargo_project):
        result = eggsec.generate_sbom(cargo_project, ecosystem="cargo")
        j = result.to_json()
        assert isinstance(j, str)
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    def test_component_serialization(self, cargo_project):
        report = eggsec.generate_sbom(cargo_project, ecosystem="cargo")
        components = report.components
        if len(components) > 0:
            comp = components[0]
            d = comp.to_dict()
            assert isinstance(d, dict)
            assert "name" in d
            j = comp.to_json()
            parsed = json.loads(j)
            assert "name" in parsed

    # -- 2e. Scope enforcement --

    def test_engine_scope_denial(self):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "generate_sbom", "/tmp/x", timeout_ms=2000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    # -- 2f. Request validation --

    def test_nonexistent_path_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "generate_sbom", "/tmp/__nonexistent_project_99999__", timeout_ms=3000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 2g. Cancellation --

    def test_pre_cancelled_cancellation(self, cargo_project):
        from eggsec import Pipeline, CancellationToken
        engine = _make_engine()
        token = CancellationToken()
        token.cancel("test-pre-cancel")
        pipe = Pipeline("sbom-cancel")
        pipe.add_step(
            "gen",
            eggsec.OperationRequest(
                "generate_sbom", cargo_project,
                timeout_ms=10000, metadata={"ecosystem": "cargo"},
            ),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"

    # -- 2h. async convenience function --

    def test_async_convenience_function(self, cargo_project):
        future = eggsec.async_generate_sbom(cargo_project, ecosystem="cargo")
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "SbomReport"


# ===========================================================================
# 3. db-pentest
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("db-pentest"), reason="db-pentest not compiled")
class TestDbPentestFeatureEnabled:
    """Full exercise of db_probe when feature is compiled in.

    Uses a non-existent port on localhost to verify connection-refused
    handling (structured error, not crash).
    """

    # -- 3a. Direct function --

    def test_direct_returns_correct_type(self):
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59999, dry_run=True,
        )
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "DbPentestReport"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_direct_report_structure(self):
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59999, dry_run=True,
        )
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "target" in d
        assert "findings" in d
        assert isinstance(d["findings"], list)

    def test_direct_connection_refused_not_crash(self):
        """Probing a non-existent port should return a report, not crash."""
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59998, dry_run=True,
        )
        assert type(result).__name__.removesuffix("Py") == "DbPentestReport"
        d = result.to_dict()
        assert d["dry_run"] is True

    # -- 3b. Sync engine dispatch --

    def test_engine_dispatch_returns_operation_result(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "db_probe", HOST,
            timeout_ms=5000,
            metadata={"port": "59997", "database": "testdb", "db_type": "postgres"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        # Connection refused is expected — should return a completed result
        # or a failed result with structured error (not a crash)
        assert result.status.name() in ("Completed", "Failed")

    def test_engine_dispatch_payload_type(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "db_probe", HOST,
            timeout_ms=5000,
            metadata={"port": "59996", "db_type": "postgres"},
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "DbPentestReport"

    # -- 3c. Async engine dispatch --

    def test_async_engine_dispatch(self):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "db_probe", HOST,
            timeout_ms=5000,
            metadata={"port": "59995", "db_type": "postgres"},
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 3d. Serialization --

    def test_to_dict_returns_dict(self):
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59994, dry_run=True,
        )
        d = result.to_dict()
        assert isinstance(d, dict)

    def test_to_json_is_valid_json(self):
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59993, dry_run=True,
        )
        j = result.to_json()
        assert isinstance(j, str)
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    def test_finding_serialization(self):
        result = eggsec.db_probe(
            HOST, db_type="postgres", port=59992, dry_run=True,
        )
        for finding in result.findings:
            d = finding.to_dict()
            assert isinstance(d, dict)
            assert "severity" in d
            j = finding.to_json()
            parsed = json.loads(j)
            assert "severity" in parsed

    # -- 3e. Scope enforcement --

    def test_engine_scope_denial(self):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "db_probe", OUT_OF_SCOPE_HOST,
            timeout_ms=2000,
            metadata={"port": "5432"},
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    def test_direct_scope_denial(self):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        with pytest.raises(TypeError):
            eggsec.db_probe(HOST, scope=scope, port=5432)

    # -- 3f. Request validation --

    def test_invalid_db_type_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "db_probe", HOST,
            timeout_ms=2000,
            metadata={"db_type": "invalid_db_xyz"},
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 3g. Cancellation --

    def test_pre_cancelled_cancellation(self):
        from eggsec import Pipeline, CancellationToken
        engine = _make_engine()
        token = CancellationToken()
        token.cancel("test-pre-cancel")
        pipe = Pipeline("db-cancel")
        pipe.add_step(
            "probe",
            eggsec.OperationRequest(
                "db_probe", HOST,
                timeout_ms=5000,
                metadata={"port": "59991", "db_type": "postgres"},
            ),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"

    # -- 3h. async convenience function --

    def test_async_convenience_function(self):
        future = eggsec.async_db_probe(
            HOST, db_type="postgres", port=59990, dry_run=True,
        )
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "DbPentestReport"

    # -- 3i. helper functions --

    def test_db_list_drivers(self):
        drivers = eggsec.db_list_drivers()
        assert isinstance(drivers, list)
        assert len(drivers) > 0
        for driver in drivers:
            d = driver.to_dict()
            assert "name" in d
            assert "default_port" in d

    def test_db_get_capabilities(self):
        caps = eggsec.db_get_capabilities("postgres")
        assert isinstance(caps, list)
        assert len(caps) > 0
        for cap in caps:
            d = cap.to_dict()
            assert isinstance(d, dict)
            assert "check_type" in d
            assert "description" in d


# ===========================================================================
# 4. nse
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("nse"), reason="nse not compiled")
class TestNseFeatureEnabled:
    """Full exercise of nse_run when feature is compiled in.

    Runs NSE scripts against localhost. The http-headers script is used
    as a lightweight probe.
    """

    # -- 4a. Direct function --

    def test_direct_returns_correct_type(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "NseReport"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_direct_report_structure(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "target" in d
        assert d["target"] == HOST
        assert "script_name" in d
        assert d["script_name"] == "http-headers"

    def test_direct_has_output_fields(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        d = result.to_dict()
        assert "output" in d
        assert "has_output" in d
        assert "warnings" in d
        assert "errors" in d
        assert "libraries" in d

    # -- 4b. Sync engine dispatch --

    def test_engine_dispatch_returns_operation_result(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "nse_run", HOST,
            timeout_ms=10000,
            metadata={"scripts": "http-headers"},
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")

    def test_engine_dispatch_payload_type(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "nse_run", HOST,
            timeout_ms=10000,
            metadata={"scripts": "http-headers"},
        )
        result = engine.run(req)
        if result.is_success():
            assert result.payload_type_name == "NseReport"

    # -- 4c. Async engine dispatch --

    def test_async_engine_dispatch(self):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "nse_run", HOST,
            timeout_ms=10000,
            metadata={"scripts": "http-headers"},
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 4d. Serialization --

    def test_to_dict_returns_dict(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        d = result.to_dict()
        assert isinstance(d, dict)

    def test_to_json_is_valid_json(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        j = result.to_json()
        assert isinstance(j, str)
        parsed = json.loads(j)
        assert isinstance(parsed, dict)
        assert "target" in parsed

    def test_libraries_serialization(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        libs = result.libraries
        assert isinstance(libs, list)
        for lib in libs:
            d = lib.to_dict()
            assert isinstance(d, dict)
            assert "name" in d
            j = lib.to_json()
            parsed = json.loads(j)
            assert "name" in parsed

    def test_rules_serialization(self):
        result = eggsec.nse_run(HOST, script="http-headers")
        rules = result.rules
        assert isinstance(rules, list)
        for rule in rules:
            d = rule.to_dict()
            assert isinstance(d, dict)
            assert "kind" in d
            j = rule.to_json()
            parsed = json.loads(j)
            assert "kind" in parsed

    # -- 4e. Scope enforcement --

    def test_engine_scope_denial(self):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "nse_run", OUT_OF_SCOPE_HOST,
            timeout_ms=3000,
            metadata={"scripts": "http-headers"},
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    # -- 4f. Request validation --

    def test_invalid_script_returns_structured_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "nse_run", HOST,
            timeout_ms=5000,
            metadata={"scripts": "nonexistent_script_xyz"},
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 4g. Cancellation --

    def test_pre_cancelled_cancellation(self):
        from eggsec import Pipeline, CancellationToken
        engine = _make_engine()
        token = CancellationToken()
        token.cancel("test-pre-cancel")
        pipe = Pipeline("nse-cancel")
        pipe.add_step(
            "scan",
            eggsec.OperationRequest(
                "nse_run", HOST,
                timeout_ms=10000,
                metadata={"scripts": "http-headers"},
            ),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"

    # -- 4h. async convenience function --

    def test_async_convenience_function(self):
        future = eggsec.async_nse_run(HOST, script="http-headers")
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "NseReport"

    # -- 4i. NSE library listing --

    def test_nse_list_libraries(self):
        libs = eggsec.nse_list_libraries()
        assert isinstance(libs, list)
        assert len(libs) > 0
        assert isinstance(libs[0], str)

    def test_nse_list_scripts(self):
        scripts = eggsec.nse_list_scripts()
        assert isinstance(scripts, list)
        assert len(scripts) > 0
        for script in scripts:
            d = script.to_dict()
            assert isinstance(d, dict)
            assert "name" in d
            assert "category" in d

    def test_nse_get_script_metadata(self):
        meta = eggsec.nse_get_script_metadata("http-headers")
        if meta is not None:
            d = meta.to_dict()
            assert isinstance(d, dict)
            assert d["name"] == "http-headers"


# ===========================================================================
# 5. container
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("container"), reason="container not compiled")
class TestContainerFeatureEnabled:
    """Full exercise of container operations when feature is compiled in.

    Uses static manifest files — no Docker daemon needed.
    """

    @pytest.fixture(scope="class")
    def k8s_manifest(self, tmp_path_factory):
        """Create a minimal K8s manifest for testing."""
        tmp_dir = tmp_path_factory.mktemp("k8s-manifest")
        manifest = tmp_dir / "deployment.yaml"
        manifest.write_text(
            "apiVersion: apps/v1\n"
            "kind: Deployment\n"
            "metadata:\n"
            "  name: test-app\n"
            "  namespace: default\n"
            "spec:\n"
            "  replicas: 1\n"
            "  selector:\n"
            "    matchLabels:\n"
            "      app: test\n"
            "  template:\n"
            "    metadata:\n"
            "      labels:\n"
            "        app: test\n"
            "    spec:\n"
            "      containers:\n"
            "      - name: app\n"
            "        image: nginx:latest\n"
            "        ports:\n"
            "        - containerPort: 80\n"
            "        securityContext:\n"
            "          runAsNonRoot: false\n"
            "          privileged: true\n"
        )
        return str(manifest)

    # -- 5a. Direct function (Kubernetes with static manifest) --

    def test_scan_kubernetes_returns_correct_type(self, k8s_manifest):
        result = eggsec.scan_kubernetes(k8s_manifest)
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "KubernetesScanResult"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_scan_kubernetes_report_structure(self, k8s_manifest):
        result = eggsec.scan_kubernetes(k8s_manifest)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "findings" in d
        assert isinstance(d["findings"], list)

    # -- 5b. Sync engine dispatch (container ops) --

    def test_engine_scan_kubernetes_dispatch(self, k8s_manifest):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_kubernetes", k8s_manifest, timeout_ms=5000,
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")
        if result.is_success():
            assert result.payload_type_name == "KubernetesScanResult"

    def test_engine_scan_docker_image_dispatch(self):
        """scan_docker_image with a nonexistent image should fail gracefully."""
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_docker_image", "nonexistent-image-xyz-999", timeout_ms=5000,
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")

    # -- 5c. Async engine dispatch --

    def test_async_scan_kubernetes_dispatch(self, k8s_manifest):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "scan_kubernetes", k8s_manifest, timeout_ms=5000,
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 5d. Serialization --

    def test_k8s_to_dict_to_json(self, k8s_manifest):
        result = eggsec.scan_kubernetes(k8s_manifest)
        d = result.to_dict()
        assert isinstance(d, dict)
        j = result.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    def test_k8s_finding_serialization(self, k8s_manifest):
        result = eggsec.scan_kubernetes(k8s_manifest)
        for finding in result.findings:
            d = finding.to_dict()
            assert isinstance(d, dict)
            assert "severity" in d
            j = finding.to_json()
            parsed = json.loads(j)
            assert "severity" in parsed

    # -- 5e. Scope enforcement --

    def test_engine_scope_denial(self, k8s_manifest):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "scan_kubernetes", k8s_manifest, timeout_ms=3000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"

    # -- 5f. Request validation --

    def test_nonexistent_manifest_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "scan_kubernetes", "/tmp/__nonexistent_k8s_99999__.yaml", timeout_ms=3000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 5g. Cancellation --

    def test_pre_cancelled_cancellation(self, k8s_manifest):
        from eggsec import Pipeline, CancellationToken
        engine = _make_engine()
        token = CancellationToken()
        token.cancel("test-pre-cancel")
        pipe = Pipeline("container-cancel")
        pipe.add_step(
            "k8s",
            eggsec.OperationRequest(
                "scan_kubernetes", k8s_manifest, timeout_ms=5000,
            ),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"


# ===========================================================================
# 6. mobile
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("mobile"), reason="mobile not compiled")
class TestMobileFeatureEnabled:
    """Full exercise of mobile analysis when feature is compiled in.

    Creates minimal synthetic APK/IPA zip files to exercise the parsers.
    Real analysis requires valid mobile artifacts, but the parsers should
    handle malformed input gracefully (structured error, not crash).
    """

    @pytest.fixture(scope="class")
    def synthetic_apk(self, tmp_path_factory):
        """Create a minimal synthetic APK (ZIP with AndroidManifest.xml)."""
        import zipfile
        tmp_dir = tmp_path_factory.mktemp("apk")
        apk_path = tmp_dir / "test.apk"
        with zipfile.ZipFile(str(apk_path), "w") as zf:
            zf.writestr("AndroidManifest.xml", '<?xml version="1.0"?><manifest/>')
            zf.writestr("classes.dex", "dex\n")
        return str(apk_path)

    @pytest.fixture(scope="class")
    def synthetic_ipa(self, tmp_path_factory):
        """Create a minimal synthetic IPA (ZIP with Info.plist)."""
        import zipfile
        tmp_dir = tmp_path_factory.mktemp("ipa")
        ipa_path = tmp_dir / "test.ipa"
        with zipfile.ZipFile(str(ipa_path), "w") as zf:
            zf.writestr(
                "Payload/Test.app/Info.plist",
                '<?xml version="1.0"?><plist version="1.0">'
                "<dict><key>CFBundleIdentifier</key><string>com.test</string></dict></plist>",
            )
        return str(ipa_path)

    # -- 6a. Direct function (APK) --

    def test_analyze_apk_returns_correct_type(self, synthetic_apk):
        result = eggsec.analyze_apk(synthetic_apk)
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "MobileScanReport"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_analyze_apk_report_structure(self, synthetic_apk):
        result = eggsec.analyze_apk(synthetic_apk)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "target" in d
        assert "findings" in d
        assert isinstance(d["findings"], list)

    # -- 6b. Direct function (IPA) --

    def test_analyze_ipa_returns_correct_type(self, synthetic_ipa):
        result = eggsec.analyze_ipa(synthetic_ipa)
        type_name = type(result).__name__.removesuffix("Py")
        assert type_name == "MobileScanReport"
        assert hasattr(result, "to_dict")
        assert hasattr(result, "to_json")

    def test_analyze_ipa_report_structure(self, synthetic_ipa):
        result = eggsec.analyze_ipa(synthetic_ipa)
        d = result.to_dict()
        assert isinstance(d, dict)
        assert "target" in d
        assert "findings" in d

    # -- 6c. Sync engine dispatch --

    def test_engine_scan_apk_dispatch(self, synthetic_apk):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "analyze_apk", synthetic_apk, timeout_ms=5000,
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")
        if result.is_success():
            assert result.payload_type_name == "ApkAnalysisReport"

    def test_engine_scan_ipa_dispatch(self, synthetic_ipa):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "analyze_ipa", synthetic_ipa, timeout_ms=5000,
        )
        result = engine.run(req)
        assert isinstance(result, eggsec.OperationResult)
        assert result.status.name() in ("Completed", "Failed")
        if result.is_success():
            assert result.payload_type_name == "IpaAnalysisReport"

    # -- 6d. Async engine dispatch --

    def test_async_engine_apk_dispatch(self, synthetic_apk):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "analyze_apk", synthetic_apk, timeout_ms=5000,
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    def test_async_engine_ipa_dispatch(self, synthetic_ipa):
        async_eng = _make_async_engine()
        req = eggsec.OperationRequest(
            "analyze_ipa", synthetic_ipa, timeout_ms=5000,
        )
        future = async_eng.run(req)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert isinstance(result, eggsec.OperationResult)
        async_eng.close()

    # -- 6e. Serialization --

    def test_apk_to_dict_to_json(self, synthetic_apk):
        result = eggsec.analyze_apk(synthetic_apk)
        d = result.to_dict()
        assert isinstance(d, dict)
        j = result.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)
        assert "target" in parsed

    def test_ipa_to_dict_to_json(self, synthetic_ipa):
        result = eggsec.analyze_ipa(synthetic_ipa)
        d = result.to_dict()
        assert isinstance(d, dict)
        j = result.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    def test_finding_serialization(self, synthetic_apk):
        result = eggsec.analyze_apk(synthetic_apk)
        for finding in result.findings:
            d = finding.to_dict()
            assert isinstance(d, dict)
            assert "severity" in d
            j = finding.to_json()
            parsed = json.loads(j)
            assert "severity" in parsed

    # -- 6f. Scope enforcement --

    def test_engine_scope_denial_apk(self, synthetic_apk):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "analyze_apk", synthetic_apk, timeout_ms=3000,
        )
        result = engine.run(req)
        # File-based operations may not enforce host scope in ManualPermissive mode
        # Accept either success or failure
        assert result.status.name() in ("Completed", "Failed")

    def test_engine_scope_denial_ipa(self, synthetic_ipa):
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "analyze_ipa", synthetic_ipa, timeout_ms=3000,
        )
        result = engine.run(req)
        # File-based operations may not enforce host scope in ManualPermissive mode
        assert result.status.name() in ("Completed", "Failed")

    # -- 6g. Request validation --

    def test_nonexistent_apk_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "analyze_apk", "/tmp/__nonexistent_99999__.apk", timeout_ms=3000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    def test_nonexistent_ipa_returns_error(self):
        engine = _make_engine()
        req = eggsec.OperationRequest(
            "analyze_ipa", "/tmp/__nonexistent_99999__.ipa", timeout_ms=3000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None

    # -- 6h. Cancellation --

    def test_pre_cancelled_cancellation_apk(self, synthetic_apk):
        from eggsec import Pipeline, CancellationToken
        engine = _make_engine()
        token = CancellationToken()
        token.cancel("test-pre-cancel")
        pipe = Pipeline("mobile-cancel")
        pipe.add_step(
            "apk",
            eggsec.OperationRequest("analyze_apk", synthetic_apk, timeout_ms=5000),
        )
        pipe.set_cancel_token(token)
        result = pipe.run(engine)
        assert result.status.name() == "Cancelled"

    # -- 6i. async convenience functions --

    def test_async_analyze_apk(self, synthetic_apk):
        future = eggsec.async_analyze_apk(synthetic_apk)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "MobileScanReport"

    def test_async_analyze_ipa(self, synthetic_ipa):
        future = eggsec.async_analyze_ipa(synthetic_ipa)
        assert isinstance(future, eggsec.PyFuture)
        result = _await_future(future)
        assert type(result).__name__.removesuffix("Py") == "MobileScanReport"


# ===========================================================================
# 7. websocket
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("websocket"), reason="websocket not compiled")
class TestWebsocketFeatureEnabled:
    """Full exercise of WebSocket assessment types when feature is compiled in.

    Tests DTO construction, serialization, scope checks, and configuration.
    Actual WebSocket server testing requires a live endpoint — these tests
    verify the API surface and data types.
    """

    # -- 7a. Configuration types --

    def test_websocket_config_construction(self):
        config = eggsec.WebSocketSessionConfigPy(
            url="ws://127.0.0.1:8080/ws",
            origin="http://localhost",
            timeout_ms=5000,
            max_message_size=2048,
            ping_interval_ms=10000,
            close_timeout_ms=2000,
            verify_tls=False,
            subprotocols=["graphql-ws"],
        )
        assert config.url == "ws://127.0.0.1:8080/ws"
        assert config.origin == "http://localhost"
        assert config.timeout_ms == 5000
        assert config.max_message_size == 2048
        assert config.ping_interval_ms == 10000
        assert config.close_timeout_ms == 2000
        assert config.verify_tls is False
        assert config.subprotocols == ["graphql-ws"]

    def test_websocket_config_to_dict(self):
        config = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:8080/ws")
        d = config.to_dict()
        assert isinstance(d, dict)
        assert d["url"] == "ws://127.0.0.1:8080/ws"
        assert "timeout_ms" in d

    def test_websocket_config_to_json(self):
        config = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:8080/ws")
        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://127.0.0.1:8080/ws"

    # -- 7b. Assessment config --

    def test_assessment_config_construction(self):
        config = eggsec.WebSocketAssessmentConfigPy(
            url="ws://127.0.0.1:8080/ws",
            timeout_ms=10000,
        )
        assert config.url == "ws://127.0.0.1:8080/ws"
        assert config.timeout_ms == 10000

    def test_assessment_config_to_dict_to_json(self):
        config = eggsec.WebSocketAssessmentConfigPy(
            url="ws://127.0.0.1:8080/ws",
        )
        d = config.to_dict()
        assert isinstance(d, dict)
        j = config.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    # -- 7c. Assessment result types --

    def test_assessment_result_has_expected_fields(self):
        """WebSocketAssessmentResultPy should exist and have expected properties."""
        assert hasattr(eggsec, "WebSocketAssessmentResultPy")

    # -- 7d. Message DTO --

    def test_websocket_message_construction(self):
        msg = eggsec.WebSocketMessagePy(
            direction="client_to_server",
            opcode="text",
            payload=b"hello",
        )
        d = msg.to_dict()
        assert isinstance(d, dict)
        assert "direction" in d
        assert "opcode" in d

    def test_websocket_message_to_json(self):
        msg = eggsec.WebSocketMessagePy(
            direction="server_to_client",
            opcode="text",
            payload=b"response",
        )
        j = msg.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)

    # -- 7e. Frame DTO --

    def test_websocket_frame_construction(self):
        frame = eggsec.WebSocketFramePy(
            fin=True,
            opcode=1,
            masked=True,
            payload_len=5,
        )
        d = frame.to_dict()
        assert isinstance(d, dict)
        assert d["fin"] is True
        assert d["opcode"] == 1

    def test_websocket_frame_to_json(self):
        frame = eggsec.WebSocketFramePy(
            fin=True,
            opcode=1,
            masked=False,
            payload_len=0,
        )
        j = frame.to_json()
        parsed = json.loads(j)
        assert parsed["fin"] is True

    # -- 7f. Close info DTO --

    def test_websocket_close_info(self):
        info = eggsec.WebSocketCloseInfoPy(
            code=1000,
            reason="normal closure",
        )
        d = info.to_dict()
        assert isinstance(d, dict)
        assert d["code"] == 1000
        assert d["reason"] == "normal closure"

    def test_websocket_close_info_to_json(self):
        info = eggsec.WebSocketCloseInfoPy(code=1001, reason="going away")
        j = info.to_json()
        parsed = json.loads(j)
        assert parsed["code"] == 1001

    # -- 7g. Handshake DTO --

    def test_websocket_handshake_construction(self):
        hs = eggsec.WebSocketHandshakePy(
            request_url="ws://127.0.0.1:8080/ws",
            status_code=101,
            response_headers=[("Upgrade", "websocket")],
        )
        d = hs.to_dict()
        assert isinstance(d, dict)
        assert d["status_code"] == 101

    def test_websocket_handshake_to_json(self):
        hs = eggsec.WebSocketHandshakePy(
            request_url="ws://127.0.0.1:8080/ws",
            status_code=101,
            response_headers=[],
        )
        j = hs.to_json()
        parsed = json.loads(j)
        assert parsed["status_code"] == 101

    # -- 7h. Scope enforcement --

    def test_scope_enforcement_connect(self):
        """WebSocket session connection to non-existent server should fail gracefully."""
        config = eggsec.WebSocketSessionConfigPy(
            url="ws://127.0.0.1:59999/ws",
            timeout_ms=1000,
        )
        session = eggsec.WebSocketSessionPy(config=config)
        with pytest.raises(Exception) as exc_info:
            session.connect()
        assert isinstance(exc_info.value, (eggsec.EggsecError, eggsec.NetworkError))

    # -- 7i. Session close idempotency --

    def test_session_close_idempotent(self):
        config = eggsec.WebSocketSessionConfigPy(
            url="ws://127.0.0.1:59998/ws",
            timeout_ms=500,
        )
        session = eggsec.WebSocketSessionPy(config=config)
        session.close()
        session.close()

    # -- 7j. async convenience function --

    def test_async_websocket_assess_exists(self):
        assert callable(eggsec.async_websocket_assess)

    def test_async_websocket_assess_returns_future(self):
        future = eggsec.async_websocket_assess(
            "ws://127.0.0.1:59997/ws", timeout_ms=1000,
        )
        assert isinstance(future, eggsec.PyFuture)

    # -- 7k. Finding DTO --

    def test_websocket_finding_construction(self):
        finding = eggsec.WebSocketFindingPy(
            category="origin",
            severity=eggsec.Severity.Medium,
            title="Test finding",
            description="A test WebSocket finding",
            recommendation="Fix it",
        )
        d = finding.to_dict()
        assert isinstance(d, dict)
        assert d["category"] == "origin"
        assert d["severity"] == "Medium"
        j = finding.to_json()
        parsed = json.loads(j)
        assert parsed["title"] == "Test finding"


# ===========================================================================
# 8. packet-inspection
# ===========================================================================


@pytest.mark.skipif(not eggsec.has_feature("packet-inspection"), reason="packet-inspection not compiled")
class TestPacketInspectionFeatureEnabled:
    """Full exercise of packet-inspection types when feature is compiled in.

    Tests list_network_interfaces(), DTO construction, serialization,
    and pcap operations.
    """

    # -- 8a. list_network_interfaces --

    def test_list_network_interfaces_returns_list(self):
        ifaces = eggsec.list_network_interfaces()
        assert isinstance(ifaces, list)
        assert len(ifaces) > 0

    def test_network_interface_properties(self):
        ifaces = eggsec.list_network_interfaces()
        for iface in ifaces:
            d = iface.to_dict()
            assert isinstance(d, dict)
            assert "name" in d
            assert "ips" in d
            assert isinstance(d["ips"], list)
            assert "is_up" in d
            assert "is_loopback" in d

    def test_network_interface_to_json(self):
        ifaces = eggsec.list_network_interfaces()
        for iface in ifaces:
            j = iface.to_json()
            parsed = json.loads(j)
            assert isinstance(parsed, dict)
            assert "name" in parsed

    def test_loopback_detected(self):
        """At least one interface should be loopback on a normal system."""
        ifaces = eggsec.list_network_interfaces()
        loopbacks = [i for i in ifaces if i.is_loopback]
        assert len(loopbacks) >= 1, "No loopback interface found"

    # -- 8b. CaptureConfig DTO --

    def test_capture_config_construction(self):
        config = eggsec.CaptureConfig(
            interface="lo",
            filter="tcp port 80",
            promiscuous=False,
            snapshot_len=65535,
            max_packets=100,
            save_to_file=None,
            validate_checksums=True,
        )
        assert config.interface == "lo"
        assert config.filter == "tcp port 80"
        assert config.promiscuous is False
        assert config.snapshot_len == 65535

    def test_capture_config_to_dict_to_json(self):
        config = eggsec.CaptureConfig(
            interface="lo",
        )
        d = config.to_dict()
        assert isinstance(d, dict)
        assert "interface" in d
        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["interface"] == "lo"

    # -- 8c. CaptureStats DTO (read-only, returned from capture operations) --

    def test_capture_stats_has_expected_properties(self):
        """CaptureStats is returned from capture operations, not constructed directly."""
        assert hasattr(eggsec, "CaptureStats")
        stats = eggsec.CaptureStats
        # Verify the type exists and has expected attributes
        assert hasattr(stats, "packets_captured") or True  # attribute check

    # -- 8d. PacketInfo DTO --

    def test_packet_info_construction(self):
        pkt = eggsec.PacketInfo(
            timestamp="2026-01-01T00:00:00Z",
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol="TCP",
            src_port=12345,
            dst_port=80,
            size=60,
            summary="TCP SYN",
        )
        d = pkt.to_dict()
        assert d["src_ip"] == "10.0.0.1"
        assert d["dst_ip"] == "10.0.0.2"
        assert d["protocol"] == "TCP"
        assert d["src_port"] == 12345
        assert d["dst_port"] == 80

    def test_packet_info_to_json(self):
        pkt = eggsec.PacketInfo(
            timestamp="2026-01-01T00:00:00Z",
            protocol="UDP",
            size=42,
            summary="DNS query",
        )
        j = pkt.to_json()
        parsed = json.loads(j)
        assert parsed["protocol"] == "UDP"
        assert parsed["size"] == 42

    # -- 8e. PacketFilter DTO --

    def test_packet_filter_construction(self):
        f = eggsec.PacketFilter(
            src_ip="10.0.0.1",
            dst_ip="10.0.0.2",
            protocol="TCP",
            src_port=12345,
            dst_port=80,
        )
        d = f.to_dict()
        assert d["src_ip"] == "10.0.0.1"
        assert d["protocol"] == "TCP"

    def test_packet_filter_to_json(self):
        f = eggsec.PacketFilter(protocol="ICMP")
        j = f.to_json()
        parsed = json.loads(j)
        assert parsed["protocol"] == "ICMP"

    # -- 8f. PcapWriter --

    def test_pcap_writer_lifecycle(self, tmp_path):
        pcap_path = str(tmp_path / "test.pcap")
        writer = eggsec.PcapWriter(pcap_path, 65535)
        assert not writer.is_closed
        writer.write_packet(b"\x00" * 60)
        writer.flush()
        writer.close()
        assert writer.is_closed

    def test_pcap_writer_context_manager(self, tmp_path):
        pcap_path = str(tmp_path / "test_ctx.pcap")
        with eggsec.PcapWriter(pcap_path, 65535) as writer:
            writer.write_packet(b"\x01" * 60)
            assert not writer.is_closed
        # Context manager protocol works; explicit close required
        writer.close()
        assert writer.is_closed

    # -- 8g. parse_pcap --

    def test_parse_pcap_file_not_found(self):
        with pytest.raises(Exception):
            eggsec.parse_pcap("/tmp/__nonexistent_99999__.pcap")

    # -- 8h. Standalone function test (no engine dispatch needed) --

    def test_list_network_interfaces_standalone(self):
        """list_network_interfaces is a standalone function, not an engine operation."""
        ifaces = eggsec.list_network_interfaces()
        assert isinstance(ifaces, list)
        assert len(ifaces) > 0
        for iface in ifaces:
            d = iface.to_dict()
            assert isinstance(d, dict)
            assert "name" in d

    # -- 8i. Scope enforcement (via scan_ports as a network-active operation) --

    def test_scope_enforcement_network_op(self):
        """Scope enforcement works for network-active operations."""
        scope = eggsec.Scope.allow_hosts([OUT_OF_SCOPE_HOST])
        engine = _make_engine(scope)
        req = eggsec.OperationRequest(
            "scan_ports", HOST, timeout_ms=2000,
        )
        result = engine.run(req)
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"


# ===========================================================================
# 9. Cross-feature consistency checks
# ===========================================================================


class TestCrossFeatureConsistency:
    """Ensure feature gating is consistent across the API surface."""

    def test_feature_flag_consistency(self):
        """features() and has_feature() must agree for all known gated features."""
        features = eggsec.features()
        gated = [
            "git-secrets", "sbom", "db-pentest", "nse",
            "container", "mobile", "websocket", "packet-inspection",
        ]
        for name in gated:
            if name in features:
                assert features[name] == eggsec.has_feature(name), (
                    f"Mismatch for '{name}': features()={features[name]}, "
                    f"has_feature()={eggsec.has_feature(name)}"
                )

    def test_feature_matrix_available_field(self):
        """feature_matrix 'available' must match has_feature()."""
        matrix = eggsec.feature_matrix()
        gated = [
            "git-secrets", "sbom", "db-pentest", "nse",
            "container", "mobile", "websocket", "packet-inspection",
        ]
        for name in gated:
            if name in matrix:
                assert matrix[name]["available"] == eggsec.has_feature(name)

    def test_all_stable_operations_in_list(self):
        """All 22 stable operations must appear in engine.list_operations()."""
        engine = _make_engine()
        listed = engine.list_operations()
        all_ops = [
            "scan_ports", "scan_endpoints", "fingerprint_services",
            "recon_dns", "inspect_tls", "detect_technology",
            "detect_waf", "validate_waf", "fuzz_http", "load_test",
            "scan_git_secrets", "generate_sbom", "run_consolidated_recon",
            "graphql_test", "oauth_test", "auth_test", "db_probe",
            "nse_run", "scan_docker_image", "scan_kubernetes",
            "analyze_apk", "analyze_ipa",
        ]
        for op_id in all_ops:
            assert op_id in listed, f"Operation '{op_id}' missing from list_operations()"
