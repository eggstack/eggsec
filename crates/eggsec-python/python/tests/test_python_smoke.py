"""Smoke tests for the eggsec Python package.

Run standalone: python crates/eggsec-python/python/tests/test_python_smoke.py
Run with pytest: pytest crates/eggsec-python/python/tests/test_python_smoke.py
"""

import json


def test_import():
    import eggsec
    assert eggsec is not None


def test_version_is_string():
    import eggsec
    assert isinstance(eggsec.__version__, str)
    assert len(eggsec.__version__) > 0


def test_features_returns_dict():
    import eggsec
    result = eggsec.features()
    assert isinstance(result, dict)
    assert len(result) > 0


def test_has_feature_core():
    import eggsec
    assert eggsec.has_feature("core") is True


def test_build_info_returns_dict():
    import eggsec
    info = eggsec.build_info()
    assert isinstance(info, dict)
    assert "version" in info
    assert "package_name" in info
    assert isinstance(info["version"], str)


def test_scope_creation():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com", "10.0.0.0/8"])
    assert scope.is_target_allowed("example.com") is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("evil.com") is False


def test_scope_deny_all():
    import eggsec
    scope = eggsec.Scope.deny_all()
    assert scope.is_target_allowed("example.com") is False


def test_client_creation():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope)
    assert client is not None
    assert client.mode == "manual"


def test_client_with_mode():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope, mode="automation")
    assert client.mode == "automation"


def test_client_context_manager():
    import eggsec
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with eggsec.Client(scope) as client:
        assert client is not None


def test_exceptions_importable():
    import eggsec
    assert issubclass(eggsec.EggsecError, Exception)
    assert issubclass(eggsec.ConfigError, eggsec.EggsecError)
    assert issubclass(eggsec.ScopeError, eggsec.EggsecError)
    assert issubclass(eggsec.EnforcementError, eggsec.EggsecError)
    assert issubclass(eggsec.NetworkError, eggsec.EggsecError)
    assert issubclass(eggsec.ScanError, eggsec.EggsecError)
    assert issubclass(eggsec.TimeoutError, eggsec.EggsecError)
    assert issubclass(eggsec.FeatureUnavailableError, eggsec.EggsecError)
    assert issubclass(eggsec.SerializationError, eggsec.EggsecError)
    assert issubclass(eggsec.InternalError, eggsec.EggsecError)


def test_exception_raising():
    import eggsec
    try:
        raise eggsec.ConfigError("test error")
    except eggsec.EggsecError as e:
        assert "test error" in str(e)


def test_severity_enum():
    import eggsec
    assert str(eggsec.Severity.Critical) == "Critical"
    assert str(eggsec.Severity.High) == "High"
    assert str(eggsec.Severity.Medium) == "Medium"
    assert str(eggsec.Severity.Low) == "Low"
    assert str(eggsec.Severity.Info) == "Info"
    s = eggsec.Severity.from_str("high")
    assert s == eggsec.Severity.High


def test_finding_creation():
    import eggsec
    f = eggsec.Finding(
        id="test-1",
        title="Test finding",
        severity=eggsec.Severity.High,
        target="example.com",
        category="smoke-test",
        description="Smoke test finding",
    )
    assert f.id == "test-1"
    assert f.severity == eggsec.Severity.High


def test_report_serialization():
    import eggsec
    report = eggsec.Report(metadata={"scanner": "smoke"})
    finding = eggsec.Finding(
        id="r-1",
        title="Report test",
        severity=eggsec.Severity.Info,
        target="localhost",
        category="test",
        description="Test",
    )
    report.add_finding(finding)
    assert len(report) == 1

    j = report.to_json()
    parsed = json.loads(j)
    assert parsed["findings"][0]["id"] == "r-1"

    d = report.to_dict()
    assert "findings" in d

    rows = report.to_rows()
    assert len(rows) == 1


def test_evidence_creation():
    import eggsec
    ev = eggsec.Evidence(
        kind="header",
        value="Server: nginx",
        source="response",
        confidence=0.95,
    )
    ev_j = ev.to_json()
    assert "nginx" in ev_j


def test_finding_set():
    import eggsec
    fs = eggsec.FindingSet()
    f1 = eggsec.Finding(
        id="fs-1", title="F1", severity=eggsec.Severity.Medium,
        target="a", category="c", description="d",
    )
    f2 = eggsec.Finding(
        id="fs-2", title="F2", severity=eggsec.Severity.Low,
        target="b", category="c", description="d",
    )
    fs.add_finding(f1)
    fs.add_finding(f2)
    assert len(fs) == 2
    medium = fs.by_severity(eggsec.Severity.Medium)
    assert len(medium) == 1


if __name__ == "__main__":
    import pytest
    raise SystemExit(pytest.main([__file__, "-v"]))
