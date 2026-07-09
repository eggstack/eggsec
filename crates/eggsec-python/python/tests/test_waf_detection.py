"""Tests for WAF detection (network-required)."""
import pytest
import eggsec


@pytest.mark.network
def test_detect_waf():
    result = eggsec.detect_waf("https://example.com")
    assert result.url == "https://example.com"
    assert isinstance(result.detected, bool)
    assert isinstance(result.confidence, int)
    assert result.status_code > 0


@pytest.mark.network
def test_detect_waf_result_fields():
    result = eggsec.detect_waf("https://example.com")
    # These fields should always be present
    _ = result.matched_headers
    _ = result.matched_cookies
    _ = result.matched_patterns
    _ = result.server_header


@pytest.mark.network
def test_detect_waf_to_dict():
    result = eggsec.detect_waf("https://example.com")
    d = result.to_dict()
    assert "url" in d
    assert "detected" in d
    assert "confidence" in d


@pytest.mark.network
def test_detect_waf_to_json():
    result = eggsec.detect_waf("https://example.com")
    import json
    j = result.to_json()
    parsed = json.loads(j)
    assert "detected" in parsed


@pytest.mark.network
def test_detect_waf_repr():
    result = eggsec.detect_waf("https://example.com")
    r = repr(result)
    assert "WafDetectionResult" in r


@pytest.mark.network
def test_detect_waf_str():
    result = eggsec.detect_waf("https://example.com")
    s = str(result)
    assert isinstance(s, str)


@pytest.mark.network
def test_client_detect_waf():
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope)
    result = client.detect_waf("https://example.com")
    assert isinstance(result.detected, bool)
