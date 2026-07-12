"""Tests for eggsec Scope class."""

import pytest
import eggsec


def test_scope_allow_hosts():
    scope = eggsec.Scope.allow_hosts(["example.com", "10.0.0.1"])
    assert scope.is_target_allowed("example.com") is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.0.2.1") is False


def test_scope_allow_cidrs():
    scope = eggsec.Scope.allow_cidrs(["127.0.0.0/8", "10.0.0.0/8"])
    assert scope.is_target_allowed("127.0.0.1") is True
    assert scope.is_target_allowed("127.255.255.255") is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.168.1.1") is False


def test_scope_deny_all():
    scope = eggsec.Scope.deny_all()
    assert scope.is_target_allowed("example.com") is False
    assert scope.is_target_allowed("127.0.0.1") is False


def test_scope_empty_hosts_raises():
    with pytest.raises(ValueError, match="hosts list must not be empty"):
        eggsec.Scope.allow_hosts([])


def test_scope_empty_cidrs_raises():
    with pytest.raises(ValueError, match="cidrs list must not be empty"):
        eggsec.Scope.allow_cidrs([])


def test_scope_is_port_allowed():
    scope = eggsec.Scope.deny_all()
    assert scope.is_port_allowed(80) is True
    assert scope.is_port_allowed(443) is True
    assert scope.is_port_allowed(1) is True
    assert scope.is_port_allowed(65535) is True


def test_scope_repr():
    scope = eggsec.Scope.allow_hosts(["example.com", "10.0.0.1"])
    r = repr(scope)
    assert "Scope" in r
    assert "example.com" in r
    assert "10.0.0.1" in r


def test_scope_target_outside_scope_raises():
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
        eggsec.scan_ports("evil.com", [80, 443], scope, timeout_ms=1000)


def test_scope_from_file(tmp_path):
    scope_file = tmp_path / "scope.toml"
    scope_file.write_text("""
[[allowed_targets]]
    pattern = "127.0.0.1"

[[allowed_targets]]
cidr = "10.0.0.0/8"
""")
    scope = eggsec.Scope.from_file(str(scope_file))
    assert scope.is_target_allowed("127.0.0.1") is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.0.2.1") is False


def test_scope_from_file_invalid():
    with pytest.raises(eggsec.ScopeError):
        eggsec.Scope.from_file("/nonexistent/path/scope.toml")
