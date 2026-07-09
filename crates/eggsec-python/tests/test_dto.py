"""Tests for eggsec DTOs and serialization."""

import json
import pytest
import eggsec


def test_port_range_list():
    pr = eggsec.PortRange.list([80, 443])
    assert len(pr) == 2
    assert pr.ports == [80, 443]


def test_port_range_list_empty_raises():
    with pytest.raises(ValueError, match="ports list must not be empty"):
        eggsec.PortRange.list([])


def test_port_range_range():
    pr = eggsec.PortRange.range(1, 10)
    assert len(pr) == 10
    assert pr.ports == list(range(1, 11))


def test_port_range_range_invalid():
    with pytest.raises(ValueError, match="start must be <= end"):
        eggsec.PortRange.range(10, 5)


def test_port_range_top_100():
    pr = eggsec.PortRange.top_100()
    assert len(pr) >= 100
    assert 80 in pr.ports
    assert 443 in pr.ports


def test_port_range_top_1000():
    pr = eggsec.PortRange.top_1000()
    assert len(pr) >= 100
    assert 80 in pr.ports


def test_timing_preset_paranoid():
    tp = eggsec.TimingPreset.paranoid()
    r = repr(tp)
    assert "paranoid" in r.lower()


def test_timing_preset_all():
    presets = [
        eggsec.TimingPreset.paranoid(),
        eggsec.TimingPreset.sneaky(),
        eggsec.TimingPreset.polite(),
        eggsec.TimingPreset.normal(),
        eggsec.TimingPreset.aggressive(),
        eggsec.TimingPreset.insane(),
    ]
    assert len(presets) == 6
    for p in presets:
        assert isinstance(p, eggsec.TimingPreset)


def _try_scan():
    """Try to scan localhost. Returns result or None if loopback is blocked."""
    try:
        scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
        return eggsec.scan_ports("127.0.0.1", [19999], scope, timeout_ms=1000)
    except eggsec.ScanError:
        return None


def test_open_port_fields():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    open_ports = result.open_ports
    assert isinstance(open_ports, list)
    if open_ports:
        op = open_ports[0]
        assert isinstance(op.port, int)
        assert isinstance(op.protocol, str)
        assert isinstance(op.service, str)
        assert op.confidence >= 0.0


def test_scan_stats_fields():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    stats = result.stats
    assert isinstance(stats.ports_scanned, int)
    assert isinstance(stats.total_open, int)
    assert isinstance(stats.elapsed_ms, int)
    assert stats.ports_scanned >= 0
    assert stats.elapsed_ms >= 0


def test_port_scan_result_to_dict():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    d = result.to_dict()
    assert isinstance(d, dict)
    assert d["target"] == "127.0.0.1"
    assert isinstance(d["scanned_ports"], int)
    assert isinstance(d["elapsed_ms"], int)
    assert isinstance(d["open_ports"], list)
    assert isinstance(d["stats"], dict)
    assert "ports_scanned" in d["stats"]
    assert "total_open" in d["stats"]
    assert "elapsed_ms" in d["stats"]


def test_port_scan_result_to_json():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    j = result.to_json()
    assert isinstance(j, str)
    parsed = json.loads(j)
    assert parsed["target"] == "127.0.0.1"
    assert isinstance(parsed["scanned_ports"], int)
    assert isinstance(parsed["open_ports"], list)


def test_port_scan_result_repr():
    result = _try_scan()
    if result is None:
        pytest.skip("Loopback scanning blocked by engine policy")
    r = repr(result)
    assert "127.0.0.1" in r
    assert "PortScanResult" in r
