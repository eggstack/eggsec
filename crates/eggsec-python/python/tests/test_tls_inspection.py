"""Tests for TLS inspection (network-required)."""
import pytest
import eggsec


@pytest.mark.network
def test_inspect_tls():
    result = eggsec.inspect_tls("example.com")
    assert result.target == "example.com"
    assert result.has_ssl is True
    assert result.certificate is not None
    assert result.certificate.subject is not None
    assert result.certificate.issuer is not None


@pytest.mark.network
def test_inspect_tls_certificate_details():
    result = eggsec.inspect_tls("example.com")
    cert = result.certificate
    assert cert is not None
    assert cert.is_expired is False
    assert cert.valid_from is not None
    assert cert.valid_until is not None
    assert len(cert.subject_alternative_names) > 0


@pytest.mark.network
def test_inspect_tls_versions():
    result = eggsec.inspect_tls("example.com")
    assert len(result.supported_versions) > 0


@pytest.mark.network
def test_inspect_tls_to_dict():
    result = eggsec.inspect_tls("example.com")
    d = result.to_dict()
    assert d["has_ssl"] is True
    assert "certificate" in d


@pytest.mark.network
def test_inspect_tls_to_json():
    result = eggsec.inspect_tls("example.com")
    import json
    j = result.to_json()
    parsed = json.loads(j)
    assert parsed["has_ssl"] is True


@pytest.mark.network
def test_client_inspect_tls():
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope)
    result = client.inspect_tls("example.com")
    assert result.has_ssl is True
