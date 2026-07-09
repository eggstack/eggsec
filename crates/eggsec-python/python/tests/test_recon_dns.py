"""Tests for DNS reconnaissance (network-required)."""
import pytest
import eggsec


@pytest.mark.network
def test_recon_dns():
    result = eggsec.recon_dns("example.com")
    assert result.domain == "example.com"
    assert len(result.a) > 0
    assert result.to_dict() is not None


@pytest.mark.network
def test_recon_dns_to_json():
    result = eggsec.recon_dns("example.com")
    j = result.to_json()
    import json
    parsed = json.loads(j)
    assert parsed["domain"] == "example.com"


@pytest.mark.network
def test_recon_dns_records():
    result = eggsec.recon_dns("example.com")
    # example.com should have A records
    assert len(result.a) > 0
    # Should have NS records
    assert len(result.ns) > 0


@pytest.mark.network
def test_client_recon_dns():
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope)
    result = client.recon_dns("example.com")
    assert result.domain == "example.com"
    assert len(result.a) > 0


@pytest.mark.network
def test_recon_dns_scope_enforcement():
    scope = eggsec.Scope.allow_hosts(["allowed.com"])
    client = eggsec.Client(scope)
    try:
        client.recon_dns("notallowed.com")
        assert False, "Should have raised EnforcementError"
    except eggsec.EnforcementError:
        pass
