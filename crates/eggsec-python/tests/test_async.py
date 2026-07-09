"""Tests for async operations and context managers."""

import pytest
import eggsec


def test_async_client_creation():
    """Test AsyncClient can be created."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    assert client.mode == "manual"


def test_async_client_invalid_mode():
    """Test AsyncClient rejects invalid mode."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with pytest.raises(ValueError, match="Invalid mode"):
        eggsec.AsyncClient(scope, mode="invalid")


def test_async_client_scope():
    """Test AsyncClient exposes scope."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    assert client.scope is not None


def test_async_client_repr():
    """Test AsyncClient repr."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="automation")
    assert "AsyncClient" in repr(client)
    assert "automation" in repr(client)


def test_client_context_manager():
    """Test Client supports context manager protocol."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    with eggsec.Client(scope, mode="manual") as client:
        assert client.mode == "manual"


def test_async_client_context_manager():
    """Test AsyncClient has context manager methods."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    assert hasattr(client, "__aenter__")
    assert hasattr(client, "__aexit__")


def test_client_close():
    """Test Client.close() exists and is callable."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope, mode="manual")
    client.close()  # Should not raise


def test_async_client_close():
    """Test AsyncClient.close() exists and is callable."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    client.close()  # Should not raise


def test_sync_scan_ports_exists():
    """Test that sync scan_ports still works."""
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    # We can't actually scan without a target, but we can test the function signature
    assert callable(eggsec.scan_ports)


def test_async_scan_ports_exists():
    """Test that async_scan_ports convenience function exists."""
    assert callable(eggsec.async_scan_ports)


def test_async_scan_ports_returns_future():
    """Test that async_scan_ports returns a PyFuture."""
    scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
    future = eggsec.async_scan_ports("127.0.0.1", [80], scope)
    assert isinstance(future, eggsec.PyFuture)


def test_async_scan_ports_denied_scope():
    """Test that async_scan_ports raises EnforcementError for out-of-scope target."""
    scope = eggsec.Scope.allow_hosts(["10.0.0.0/8"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.async_scan_ports("evil.com", [80], scope)


def test_async_validate_waf_denied_scope():
    """Test that async_validate_waf raises EnforcementError for out-of-scope target."""
    scope = eggsec.Scope.allow_hosts(["10.0.0.0/8"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.async_validate_waf("http://evil.com", scope)


def test_async_fuzz_http_denied_scope():
    """Test that async_fuzz_http raises EnforcementError for out-of-scope target."""
    scope = eggsec.Scope.allow_hosts(["10.0.0.0/8"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.async_fuzz_http("http://evil.com", scope, "xss")


def test_async_load_test_denied_scope():
    """Test that async_load_test_http raises EnforcementError for out-of-scope target."""
    scope = eggsec.Scope.allow_hosts(["10.0.0.0/8"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.async_load_test_http("http://evil.com", 10, 1, 5, scope)


def test_py_future_exists():
    """Test that PyFuture class exists."""
    assert hasattr(eggsec, "PyFuture")
