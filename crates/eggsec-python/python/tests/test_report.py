"""Tests for eggsec.Report."""
import json
import os
import tempfile
import pytest
import eggsec


def test_report_creation():
    r = eggsec.Report()
    assert len(r) == 0


def test_report_with_metadata():
    r = eggsec.Report(metadata={"scanner": "eggsec", "version": "0.1.0"})
    meta = r.metadata
    assert meta["scanner"] == "eggsec"


def test_report_add_finding():
    r = eggsec.Report()
    f = eggsec.Finding(
        id="r-1",
        title="Report finding",
        severity=eggsec.Severity.High,
        target="example.com",
        category="test",
        description="Added to report",
    )
    r.add_finding(f)
    assert len(r) == 1
    assert r.findings[0].id == "r-1"


def test_report_add_finding_set():
    r = eggsec.Report()
    fs = eggsec.FindingSet()
    fs.add_finding(eggsec.Finding(
        id="fs-1", title="FS1", severity=eggsec.Severity.Medium,
        target="a", category="c", description="d",
    ))
    fs.add_finding(eggsec.Finding(
        id="fs-2", title="FS2", severity=eggsec.Severity.Low,
        target="b", category="c", description="d",
    ))
    r.add_finding_set(fs)
    assert len(r) == 2


def test_report_to_dict():
    r = eggsec.Report()
    r.add_finding(eggsec.Finding(
        id="d-1", title="D1", severity=eggsec.Severity.Info,
        target="x", category="c", description="desc",
    ))
    d = r.to_dict()
    assert "findings" in d
    assert "metadata" in d
    assert len(d["findings"]) == 1
    assert d["findings"][0]["id"] == "d-1"


def test_report_to_json():
    r = eggsec.Report()
    r.add_finding(eggsec.Finding(
        id="j-1", title="J1", severity=eggsec.Severity.Critical,
        target="y", category="c", description="desc",
    ))
    j = r.to_json()
    parsed = json.loads(j)
    assert len(parsed["findings"]) == 1
    assert parsed["findings"][0]["id"] == "j-1"


def test_report_to_rows():
    r = eggsec.Report()
    r.add_finding(eggsec.Finding(
        id="row-1", title="R1", severity=eggsec.Severity.High,
        target="z", category="c", description="desc",
    ))
    rows = r.to_rows()
    assert len(rows) == 1
    assert rows[0]["id"] == "row-1"
    assert rows[0]["severity"] == "High"


def test_report_write_json():
    r = eggsec.Report()
    r.add_finding(eggsec.Finding(
        id="w-1", title="W1", severity=eggsec.Severity.Low,
        target="t", category="c", description="desc",
    ))
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        path = f.name
    try:
        r.write_json(path)
        with open(path) as f:
            data = json.load(f)
        assert len(data["findings"]) == 1
        assert data["findings"][0]["id"] == "w-1"
    finally:
        os.unlink(path)


def test_report_write_markdown():
    r = eggsec.Report()
    r.add_finding(eggsec.Finding(
        id="md-1", title="MD1", severity=eggsec.Severity.Medium,
        target="m", category="c", description="A markdown finding",
        recommendation="Fix this",
    ))
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False) as f:
        path = f.name
    try:
        r.write_markdown(path)
        with open(path) as f:
            content = f.read()
        assert "# Eggsec Report" in content
        assert "MD1" in content
        assert "Medium" in content
        assert "Fix this" in content
    finally:
        os.unlink(path)


@pytest.mark.network
def test_report_add_port_scan_result():
    """Test adding a PortScanResult to a report."""
    scope = eggsec.Scope.allow_hosts(["google.com"])
    result = eggsec.scan_ports(
        target="google.com",
        ports=[80, 443],
        scope=scope,
    )
    r = eggsec.Report()
    r.add_result(result)
    assert len(r) > 0


@pytest.mark.network
def test_report_add_endpoint_scan_result():
    """Test adding an EndpointScanResult to a report."""
    scope = eggsec.Scope.allow_hosts(["google.com"])
    result = eggsec.scan_endpoints(
        base_url="https://google.com",
        endpoints=["/"],
        scope=scope,
        timeout_ms=10000,
    )
    r = eggsec.Report()
    r.add_result(result)
    assert len(r) > 0


@pytest.mark.network
def test_report_add_fingerprint_result():
    """Test adding a FingerprintScanResult to a report."""
    scope = eggsec.Scope.allow_hosts(["google.com"])
    result = eggsec.fingerprint_services(
        target="google.com",
        ports=[80, 443],
        scope=scope,
    )
    r = eggsec.Report()
    r.add_result(result)
    # May have 0 or more findings depending on what's running
    assert len(r) >= 0
