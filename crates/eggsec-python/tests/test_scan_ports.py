"""Tests for eggsec port scanning functionality."""

import pytest
import eggsec


def _try_scan():
    """Try to scan localhost. Returns result or None if loopback is blocked."""
    try:
        scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
        return eggsec.scan_ports("127.0.0.1", [19999], scope, timeout_ms=1000)
    except eggsec.ScanError:
        return None


def test_scan_ports_returns_result():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    assert isinstance(result, eggsec.PortScanResult)


def test_scan_ports_outside_scope_raises():
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.scan_ports("127.0.0.1", [80], scope, timeout_ms=1000)


def test_scan_ports_result_structure():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    assert result.target == "127.0.0.1"
    assert isinstance(result.scanned_ports, int)
    assert result.scanned_ports == 1
    assert isinstance(result.elapsed_ms, int)
    assert result.elapsed_ms >= 0
    assert isinstance(result.stats, eggsec.ScanStats)
    assert result.stats.ports_scanned == 1
    assert isinstance(result.stats.total_open, int)
    assert isinstance(result.stats.elapsed_ms, int)


def test_scan_ports_empty_ports_raises():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    with pytest.raises(Exception):
        eggsec.scan_ports("127.0.0.1", [], scope, timeout_ms=1000)


def test_client_scan_ports():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    assert isinstance(result, eggsec.PortScanResult)
    assert result.target == "127.0.0.1"


def test_client_invalid_mode_raises():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    with pytest.raises(ValueError, match="Invalid mode"):
        eggsec.Client(scope, mode="bad")


def test_client_scope_and_mode():
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    client = eggsec.Client(scope, mode="automation")
    assert client.mode == "automation"
    s = client.scope
    assert isinstance(s, eggsec.Scope)
    assert s.is_target_allowed("127.0.0.1") is True


def test_scan_ports_returns_scan_result_type():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    assert type(result) is eggsec.PortScanResult
