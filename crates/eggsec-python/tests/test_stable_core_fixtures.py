"""Hermetic release coverage for all ten stable-core operations."""

from __future__ import annotations

import time
from typing import Any, Callable

import eggsec
import pytest

from fixtures.stable_core import HOST, StableCoreFixtures


@pytest.fixture(scope="module")
def stable_fixtures():
    with StableCoreFixtures() as fixtures:
        yield fixtures


def _await_future(future: Any, timeout: float = 30.0) -> Any:
    """Resolve the extension's awaitable without pytest-asyncio."""

    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = next(future)
        except StopIteration as done:
            return done.value
        if value is not None:
            return value
        time.sleep(0.01)
    raise AssertionError("async fixture operation did not complete before timeout")


def _normalized(value: Any) -> Any:
    if isinstance(value, dict):
        ignored = {
            "elapsed_ms",
            "duration_ms",
            "response_time_ms",
            "total_duration_ms",
            "requests_per_second",
            "latency_min_ms",
            "latency_max_ms",
            "latency_mean_ms",
            "latency_p50_ms",
            "latency_p90_ms",
            "latency_p95_ms",
            "latency_p99_ms",
        }
        return {key: _normalized(item) for key, item in value.items() if key not in ignored}
    if isinstance(value, list):
        return [_normalized(item) for item in value]
    return value


def _assert_serializable_and_target(result: Any, expected_type: str, target_markers: tuple[str, ...]) -> dict:
    assert type(result).__name__.removesuffix("Py") == expected_type.removesuffix("Py")
    data = result.to_dict()
    encoded = result.to_json()
    assert isinstance(data, dict)
    assert encoded.startswith("{")
    assert any(marker in encoded for marker in target_markers)
    return data


def test_all_stable_operations_have_structured_policy_denials():
    operations = [
        "scan_ports",
        "scan_endpoints",
        "fingerprint_services",
        "recon_dns",
        "inspect_tls",
        "detect_technology",
        "detect_waf",
        "validate_waf",
        "fuzz_http",
        "load_test",
    ]
    for operation in operations:
        engine = eggsec.Engine(eggsec.Scope.allow_hosts([HOST]))
        result = engine.run(eggsec.OperationRequest(operation, "192.0.2.1", timeout_ms=25))
        assert result.is_failure(), operation
        assert result.error is not None, operation
        assert result.error.kind == "scope_denial", (operation, result.error.to_dict())
        events = engine.audit_events()
        assert len(events) == 1
        assert events[0].operation_id == operation
        assert events[0].allowed is False
        assert events[0].redacted is True


def test_stable_core_operations_use_only_local_fixtures(stable_fixtures):
    fixtures = stable_fixtures
    scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
    http_url = fixtures.http_url

    sync_calls: list[tuple[str, Callable[[], Any], str, tuple[str, ...]]] = [
        (
            "scan_ports",
            lambda: eggsec.scan_ports(HOST, [fixtures.tcp_port, fixtures.closed_port], scope, timeout_ms=1000),
            "PortScanResult",
            (HOST,),
        ),
        (
            "scan_endpoints",
            lambda: eggsec.scan_endpoints(
                http_url,
                ["/", "/admin", "/missing", "/redirect-local"],
                scope,
                timeout_ms=2000,
                include_404=True,
            ),
            "EndpointScanResult",
            (http_url,),
        ),
        (
            "fingerprint_services",
            lambda: eggsec.fingerprint_services(HOST, [fixtures.tcp_port], scope, timeout_ms=1000),
            "FingerprintScanResult",
            (HOST,),
        ),
        ("recon_dns", lambda: eggsec.recon_dns("localhost"), "DnsRecordSet", ("localhost",)),
        (
            "inspect_tls",
            lambda: eggsec.inspect_tls(HOST, port=fixtures.tls_port),
            "TlsInspectionResult",
            (HOST,),
        ),
        (
            "detect_technology",
            lambda: eggsec.detect_technology(http_url),
            "TechDetectionResult",
            (http_url,),
        ),
        (
            "detect_waf",
            lambda: eggsec.detect_waf(f"{http_url}/waf-block"),
            "WafDetectionResult",
            (http_url,),
        ),
        (
            "validate_waf",
            lambda: eggsec.validate_waf(f"{http_url}/waf-block", scope, test_type="headers"),
            "WafScanResult",
            (http_url,),
        ),
        (
            "fuzz_http",
            lambda: eggsec.fuzz_http(f"{http_url}/fuzz/{{FUZZ}}", scope, "xss", concurrency=1, timeout=2),
            "FuzzSession",
            (http_url,),
        ),
        (
            "load_test",
            lambda: eggsec.load_test_http(f"{http_url}/load", 4, 1, 5, scope),
            "LoadTestResult",
            (http_url,),
        ),
    ]

    for operation, call, expected_type, target_markers in sync_calls:
        result = call()
        data = _assert_serializable_and_target(result, expected_type, target_markers)
        assert data, operation

    assert any(request["path"] == "/admin" for request in fixtures.http_requests)
    assert any(request["path"] == "/waf-block" for request in fixtures.http_requests)
    assert any(request["path"] == "/load" for request in fixtures.http_requests)


def test_stable_core_sync_async_normalized_equivalence(stable_fixtures):
    fixtures = stable_fixtures
    scope = eggsec.Scope.allow_hosts([HOST, "localhost"])
    http_url = fixtures.http_url
    operations = [
        (
            "scan_ports",
            lambda: eggsec.scan_ports(HOST, [fixtures.tcp_port], scope, timeout_ms=1000),
            lambda: _await_future(eggsec.async_scan_ports(HOST, [fixtures.tcp_port], scope, timeout_ms=1000)),
        ),
        (
            "scan_endpoints",
            lambda: eggsec.scan_endpoints(http_url, ["/", "/admin"], scope, timeout_ms=2000),
            lambda: _await_future(eggsec.async_scan_endpoints(http_url, ["/", "/admin"], scope, timeout_ms=2000)),
        ),
        (
            "fingerprint_services",
            lambda: eggsec.fingerprint_services(HOST, [fixtures.tcp_port], scope, timeout_ms=1000),
            lambda: _await_future(eggsec.async_fingerprint_services(HOST, [fixtures.tcp_port], scope, timeout_ms=1000)),
        ),
        (
            "recon_dns",
            lambda: eggsec.recon_dns("localhost"),
            lambda: _await_future(eggsec.async_recon_dns("localhost")),
        ),
        (
            "inspect_tls",
            lambda: eggsec.inspect_tls(HOST, port=fixtures.tls_port),
            lambda: _await_future(eggsec.async_inspect_tls(HOST, port=fixtures.tls_port)),
        ),
        (
            "detect_technology",
            lambda: eggsec.detect_technology(http_url),
            lambda: _await_future(eggsec.async_detect_technology(http_url)),
        ),
        (
            "detect_waf",
            lambda: eggsec.detect_waf(f"{http_url}/waf-clean"),
            lambda: _await_future(eggsec.async_detect_waf(f"{http_url}/waf-clean")),
        ),
        (
            "validate_waf",
            lambda: eggsec.validate_waf(f"{http_url}/waf-block", scope, test_type="headers"),
            lambda: _await_future(
                eggsec.async_validate_waf(f"{http_url}/waf-block", scope, test_type="headers")
            ),
        ),
        (
            "fuzz_http",
            lambda: eggsec.fuzz_http(
                f"{http_url}/fuzz/{{FUZZ}}", scope, "xss", concurrency=1, timeout=2
            ),
            lambda: _await_future(
                eggsec.async_fuzz_http(
                    f"{http_url}/fuzz/{{FUZZ}}", scope, "xss", concurrency=1, timeout=2
                )
            ),
        ),
        (
            "load_test",
            lambda: eggsec.load_test_http(f"{http_url}/load", 4, 1, 5, scope),
            lambda: _await_future(
                eggsec.async_load_test_http(f"{http_url}/load", 4, 1, 5, scope)
            ),
        ),
    ]
    for operation, sync_call, async_call in operations:
        sync_result = sync_call()
        async_result = async_call()
        assert type(sync_result) is type(async_result), operation
        assert _normalized(sync_result.to_dict()) == _normalized(async_result.to_dict()), operation
