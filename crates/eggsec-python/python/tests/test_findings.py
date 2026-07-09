"""Tests for eggsec.Finding and eggsec.FindingSet."""
import json
import eggsec


def test_finding_creation():
    f = eggsec.Finding(
        id="test-1",
        title="Test Finding",
        severity=eggsec.Severity.High,
        target="example.com",
        category="test",
        description="A test finding",
    )
    assert f.id == "test-1"
    assert f.title == "Test Finding"
    assert f.severity == eggsec.Severity.High
    assert f.target == "example.com"
    assert f.category == "test"
    assert f.description == "A test finding"
    assert f.recommendation is None


def test_finding_with_evidence():
    e = eggsec.Evidence(
        kind="header",
        value="X-Frame-Options: DENY",
        source="response",
        confidence=0.95,
    )
    f = eggsec.Finding(
        id="test-2",
        title="Header found",
        severity=eggsec.Severity.Medium,
        target="example.com",
        category="header-check",
        description="Security header present",
        evidence=[e],
    )
    assert len(f.evidence) == 1
    assert f.evidence[0].kind == "header"


def test_finding_to_dict():
    f = eggsec.Finding(
        id="test-3",
        title="Dict test",
        severity=eggsec.Severity.Low,
        target="10.0.0.1",
        category="info",
        description="Low priority",
    )
    d = f.to_dict()
    assert d["id"] == "test-3"
    assert d["severity"] == "Low"
    assert isinstance(d, dict)


def test_finding_to_json():
    f = eggsec.Finding(
        id="test-4",
        title="JSON test",
        severity=eggsec.Severity.Info,
        target="10.0.0.2",
        category="info",
        description="Info finding",
    )
    j = f.to_json()
    parsed = json.loads(j)
    assert parsed["id"] == "test-4"
    assert parsed["severity"] == "Info"


def test_finding_to_row():
    f = eggsec.Finding(
        id="test-5",
        title="Row test",
        severity=eggsec.Severity.Critical,
        target="10.0.0.3",
        category="vuln",
        description="Critical vuln",
    )
    row = f.to_row()
    assert row["id"] == "test-5"
    assert row["severity"] == "Critical"


def test_finding_set():
    fs = eggsec.FindingSet()
    assert len(fs) == 0

    f1 = eggsec.Finding(
        id="a", title="A", severity=eggsec.Severity.High,
        target="x", category="c", description="d",
    )
    f2 = eggsec.Finding(
        id="b", title="B", severity=eggsec.Severity.Low,
        target="y", category="c", description="d",
    )
    fs.add_finding(f1)
    fs.add_finding(f2)
    assert len(fs) == 2

    highs = fs.by_severity(eggsec.Severity.High)
    assert len(highs) == 1
    assert highs[0].id == "a"


def test_severity_from_str():
    assert eggsec.Severity.from_str("critical") == eggsec.Severity.Critical
    assert eggsec.Severity.from_str("HIGH") == eggsec.Severity.High
    assert eggsec.Severity.from_str("medium") == eggsec.Severity.Medium
    assert eggsec.Severity.from_str("low") == eggsec.Severity.Low
    assert eggsec.Severity.from_str("info") == eggsec.Severity.Info
    assert eggsec.Severity.from_str("informational") == eggsec.Severity.Info

    try:
        eggsec.Severity.from_str("invalid")
        assert False, "Should have raised ValueError"
    except ValueError:
        pass


def test_severity_repr():
    assert repr(eggsec.Severity.High) == "Severity.High"
    assert str(eggsec.Severity.Medium) == "Medium"
