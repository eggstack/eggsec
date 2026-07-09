"""Tests for to_rows() output on various result types."""
import pytest
import eggsec


@pytest.mark.network
def test_port_scan_result_to_rows():
    scope = eggsec.Scope.allow_hosts(["google.com"])
    result = eggsec.scan_ports(
        target="google.com",
        ports=[80, 443],
        scope=scope,
    )
    rows = result.to_rows()
    assert isinstance(rows, list)
    if rows:
        row = rows[0]
        assert "target" in row
        assert "port" in row
        assert "service" in row
        assert "protocol" in row


def test_endpoint_scan_result_to_rows():
    scope = eggsec.Scope.allow_hosts(["httpbin.org"])
    result = eggsec.scan_endpoints(
        base_url="https://httpbin.org",
        endpoints=["/get", "/status/200"],
        scope=scope,
        timeout_ms=10000,
    )
    rows = result.to_rows()
    assert isinstance(rows, list)
    if rows:
        row = rows[0]
        assert "url" in row
        assert "path" in row
        assert "status_code" in row


@pytest.mark.network
def test_fingerprint_scan_result_to_rows():
    scope = eggsec.Scope.allow_hosts(["google.com"])
    result = eggsec.fingerprint_services(
        target="google.com",
        ports=[80, 443],
        scope=scope,
    )
    rows = result.to_rows()
    assert isinstance(rows, list)
    if rows:
        row = rows[0]
        assert "target" in row
        assert "port" in row
        assert "service" in row


def test_finding_set_to_rows():
    fs = eggsec.FindingSet()
    fs.add_finding(eggsec.Finding(
        id="t1", title="T1", severity=eggsec.Severity.High,
        target="x", category="c", description="d",
    ))
    fs.add_finding(eggsec.Finding(
        id="t2", title="T2", severity=eggsec.Severity.Low,
        target="y", category="c", description="d",
    ))
    rows = fs.to_rows()
    assert len(rows) == 2
    assert rows[0]["id"] == "t1"
    assert rows[1]["id"] == "t2"


def test_finding_set_to_dicts():
    fs = eggsec.FindingSet()
    fs.add_finding(eggsec.Finding(
        id="d1", title="D1", severity=eggsec.Severity.Medium,
        target="z", category="c", description="desc",
    ))
    dicts = fs.to_dicts()
    assert len(dicts) == 1
    assert dicts[0]["id"] == "d1"
    assert dicts[0]["severity"] == "Medium"
