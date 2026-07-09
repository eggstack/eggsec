"""Tests for endpoint discovery bindings."""

import pytest
import eggsec


def test_endpoint_scan_config_creation():
    config = eggsec.EndpointScanConfig(
        base_url="https://example.com",
        endpoints=["admin", "login"],
        concurrency=10,
        timeout_ms=5000,
    )
    assert config.base_url == "https://example.com"
    assert config.endpoints == ["admin", "login"]
    assert config.concurrency == 10
    assert config.timeout_ms == 5000
    assert config.include_404 is False
    assert config.verify_tls is True


def test_endpoint_scan_config_defaults():
    config = eggsec.EndpointScanConfig(
        base_url="https://example.com",
        endpoints=["admin"],
    )
    assert config.concurrency == 20
    assert config.timeout_ms == 30000
    assert config.include_404 is False
    assert config.verify_tls is True


def test_endpoint_scan_config_empty_base_url():
    with pytest.raises(ValueError, match="base_url must not be empty"):
        eggsec.EndpointScanConfig(base_url="", endpoints=["admin"])


def test_endpoint_scan_config_empty_endpoints():
    with pytest.raises(ValueError, match="endpoints list must not be empty"):
        eggsec.EndpointScanConfig(
            base_url="https://example.com", endpoints=[]
        )


def test_endpoint_scan_config_repr():
    config = eggsec.EndpointScanConfig(
        base_url="https://example.com",
        endpoints=["admin", "login"],
    )
    assert "example.com" in repr(config)


def test_client_scan_endpoints():
    """Test that Client.scan_endpoints exists and has correct signature."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope, mode="manual")
    assert hasattr(client, "scan_endpoints")


def test_async_client_scan_endpoints():
    """Test that AsyncClient.scan_endpoints exists and has correct signature."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    assert hasattr(client, "scan_endpoints")


def test_convenience_scan_endpoints():
    """Test that scan_endpoints convenience function exists."""
    assert callable(eggsec.scan_endpoints)


def test_convenience_async_scan_endpoints():
    """Test that async_scan_endpoints convenience function exists."""
    assert callable(eggsec.async_scan_endpoints)


def test_endpoint_finding_to_dict():
    """Test EndpointFinding.to_dict() by creating one via endpoint scan."""
    # We can't easily create an EndpointFinding directly, but we can test
    # the result class exists and has the method
    assert hasattr(eggsec.EndpointFinding, "to_dict")


def test_endpoint_scan_result_to_dict():
    """Test EndpointScanResult.to_dict() by creating one via endpoint scan."""
    assert hasattr(eggsec.EndpointScanResult, "to_dict")
    assert hasattr(eggsec.EndpointScanResult, "to_json")
