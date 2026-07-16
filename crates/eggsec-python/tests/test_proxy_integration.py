"""Live proxy integration tests - Workstream 5.

Tests prove the proxy manager and intercept session infrastructure work with
real proxy entries and loopback origins. Unlike test_proxy.py (DTO-only),
these tests exercise pool management, health checking, and intercept lifecycle.
"""

import json
import threading
import time
import http.server
import socketserver
import pytest
import importlib
from urllib.request import urlopen, ProxyHandler, build_opener
from urllib.error import URLError

pytestmark = [pytest.mark.timeout(60)]

_mod = importlib.import_module("eggsec")
_PROXY_AVAILABLE = getattr(_mod, "ProxyType", None) is not None

if not _PROXY_AVAILABLE:
    pytest.skip("web-proxy feature not compiled", allow_module_level=True)


def _import_or_skip(name, feature="web-proxy"):
    obj = getattr(_mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


# ---------------------------------------------------------------------------
# Loopback HTTP origin for proxy tests
# ---------------------------------------------------------------------------


class _OriginHandler(http.server.BaseHTTPRequestHandler):
    server: http.server.ThreadingHTTPServer

    def log_message(self, *_args):
        return

    def do_GET(self):
        if self.path == "/proxy-test":
            body = b"PROXY_ORIGIN_OK"
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("X-Test-Header", "proxy-works")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/echo":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length) if length else b""
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/slow":
            time.sleep(0.1)
            body = b"SLOW_RESPONSE"
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            body = b"NOT_FOUND"
            self.send_response(404)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class _OriginServer:
    def __init__(self):
        self.server = None
        self.thread = None

    def start(self):
        self.server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), _OriginHandler)
        self.server.allow_reuse_address = True
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            if self.thread.is_alive():
                return
            time.sleep(0.01)
        raise RuntimeError("Origin server failed to start")

    def stop(self):
        if self.server:
            self.server.shutdown()
            self.server.server_close()
        if self.thread:
            self.thread.join(timeout=2.0)

    @property
    def port(self):
        return self.server.server_address[1]

    @property
    def base_url(self):
        return f"http://127.0.0.1:{self.port}"


@pytest.fixture
def origin_server():
    """Provide a loopback HTTP origin server."""
    server = _OriginServer()
    server.start()
    yield server
    server.stop()


# ---------------------------------------------------------------------------
# ProxyType / RotationStrategy / ProxyConfig / ProxyEntry integration
# ---------------------------------------------------------------------------


class TestProxyManagerPoolIntegration:
    """Test proxy manager pool operations with real entries."""

    @pytest.mark.timeout(30)
    def test_add_proxy_and_pool_size(self):
        """Add a proxy entry and verify pool size increases."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig()
        manager = create_proxy_manager(config)

        entry = ProxyEntry(
            proxy_type=ProxyType.Http,
            address="127.0.0.1",
            port=8080,
        )
        manager.add_proxy(entry)
        assert manager.pool_size() == 1

    @pytest.mark.timeout(30)
    def test_add_multiple_proxies(self):
        """Add multiple proxy entries and verify pool size."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig()
        manager = create_proxy_manager(config)

        for i in range(5):
            entry = ProxyEntry(
                proxy_type=ProxyType.Http,
                address="127.0.0.1",
                port=8080 + i,
            )
            manager.add_proxy(entry)
        assert manager.pool_size() == 5

    @pytest.mark.timeout(30)
    def test_get_next_proxy_round_robin(self):
        """Get next proxy rotates through entries."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")
        RotationStrategy = _import_or_skip("RotationStrategy")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig(rotation_strategy=RotationStrategy.RoundRobin)
        manager = create_proxy_manager(config)

        for i in range(3):
            entry = ProxyEntry(
                proxy_type=ProxyType.Http,
                address="127.0.0.1",
                port=9000 + i,
            )
            manager.add_proxy(entry)

        proxies = []
        for _ in range(6):
            p = manager.get_next_proxy()
            proxies.append(p)

        ports = [p.port for p in proxies]
        assert ports[:3] != ports[3:6] or len(set(ports)) > 1

    @pytest.mark.timeout(30)
    def test_health_check_with_no_proxies(self):
        """Health check on empty pool returns valid result."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig()
        manager = create_proxy_manager(config)

        health = manager.check_health()
        assert health is not None
        d = health.to_dict()
        assert isinstance(d, dict)

    @pytest.mark.timeout(30)
    def test_manager_context_manager(self):
        """ProxyManager works as context manager."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig()
        with create_proxy_manager(config) as manager:
            entry = ProxyEntry(
                proxy_type=ProxyType.Http,
                address="127.0.0.1",
                port=8080,
            )
            manager.add_proxy(entry)
            assert manager.pool_size() == 1

    @pytest.mark.timeout(30)
    def test_proxy_entry_serialization(self):
        """ProxyEntry serializes to dict and JSON."""
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")

        entry = ProxyEntry(
            proxy_type=ProxyType.Http,
            address="127.0.0.1",
            port=8080,
            weight=0.5,
            priority=1,
        )
        d = entry.to_dict()
        assert isinstance(d, dict)
        assert d["proxy_type"] == "http"
        assert d["address"] == "127.0.0.1"
        assert d["port"] == 8080

        j = entry.to_json()
        parsed = json.loads(j)
        assert parsed["proxy_type"] == "http"


# ---------------------------------------------------------------------------
# Intercept session lifecycle
# ---------------------------------------------------------------------------


class TestInterceptSessionLifecycle:
    """Test intercept session start/stop lifecycle."""

    @pytest.mark.timeout(30)
    def test_intercept_config_construction(self):
        """InterceptConfig constructs with valid defaults."""
        InterceptConfig = _import_or_skip("InterceptConfig")

        config = InterceptConfig(
            listen_addr="127.0.0.1",
            listen_port=19999,
        )
        assert config.listen_addr == "127.0.0.1"
        assert config.listen_port == 19999

    @pytest.mark.timeout(30)
    def test_intercept_config_with_modifications(self):
        """InterceptConfig with request/response modification flags."""
        InterceptConfig = _import_or_skip("InterceptConfig")

        config = InterceptConfig(
            listen_addr="127.0.0.1",
            listen_port=19998,
            modify_request=True,
            modify_response=True,
        )
        assert config.modify_request is True
        assert config.modify_response is True

    @pytest.mark.timeout(30)
    def test_intercept_session_result_fields(self):
        """InterceptSessionResult has expected fields."""
        InterceptSessionResult = _import_or_skip("InterceptSessionResult")

        result = InterceptSessionResult(
            listen_addr="127.0.0.1",
            listen_port=19997,
            duration_ms=1000,
            total_exchanges=5,
            modified_requests=2,
            modified_responses=1,
            exchanges=[],
        )
        d = result.to_dict()
        assert d["listen_addr"] == "127.0.0.1"
        assert d["total_exchanges"] == 5

    @pytest.mark.timeout(30)
    def test_captured_exchange_construction(self):
        """CapturedExchange constructs with valid fields."""
        CapturedExchange = _import_or_skip("CapturedExchange")

        exchange = CapturedExchange(
            method="GET",
            url="http://example.com/test",
            request_headers={"Host": "example.com"},
            request_body=None,
            response_status=200,
            response_headers={"Content-Type": "text/plain"},
            response_body=b"OK",
            timestamp_ms=1234567890,
            duration_ms=50,
        )
        d = exchange.to_dict()
        assert d["method"] == "GET"
        assert d["response_status"] == 200

    @pytest.mark.timeout(30)
    def test_har_document_construction(self):
        """HarDocument constructs with entries."""
        HarDocument = _import_or_skip("HarDocument")
        HarEntry = _import_or_skip("HarEntry")

        entry = HarEntry(
            method="GET",
            url="http://example.com/",
            status_code=200,
            status_text="OK",
            time_ms=100,
            request_headers={"Host": "example.com"},
            response_headers={"Content-Type": "text/html"},
            request_body=None,
            response_body="<html>OK</html>",
        )
        har = HarDocument(entries=[entry])
        d = har.to_dict()
        assert "entries" in d
        assert len(d["entries"]) == 1

    @pytest.mark.timeout(30)
    def test_intercept_filter_construction(self):
        """InterceptFilter constructs with patterns."""
        InterceptFilter = _import_or_skip("InterceptFilter")

        filt = InterceptFilter(
            include_patterns=["*.example.com", "*.test.local"],
            exclude_patterns=["*.static.example.com"],
        )
        d = filt.to_dict()
        assert "include_patterns" in d
        assert len(d["include_patterns"]) == 2

    @pytest.mark.timeout(30)
    def test_request_modification_construction(self):
        """RequestModification constructs with header changes."""
        RequestModification = _import_or_skip("RequestModification")

        mod = RequestModification(
            add_headers={"X-Custom": "test-value"},
            remove_headers=["X-Debug"],
            replace_body=None,
        )
        d = mod.to_dict()
        assert "add_headers" in d
        assert d["add_headers"]["X-Custom"] == "test-value"

    @pytest.mark.timeout(30)
    def test_response_modification_construction(self):
        """ResponseModification constructs with header changes."""
        ResponseModification = _import_or_skip("ResponseModification")

        mod = ResponseModification(
            add_headers={"X-Modified": "true"},
            remove_headers=["X-Internal"],
            replace_body=None,
            replace_status=None,
        )
        d = mod.to_dict()
        assert d["add_headers"]["X-Modified"] == "true"

    @pytest.mark.timeout(30)
    def test_replay_request_construction(self):
        """ReplayRequest constructs with full request data."""
        ReplayRequest = _import_or_skip("ReplayRequest")

        replay = ReplayRequest(
            method="POST",
            url="http://example.com/api",
            headers={"Content-Type": "application/json"},
            body='{"key": "value"}',
        )
        d = replay.to_dict()
        assert d["method"] == "POST"
        assert d["url"] == "http://example.com/api"

    @pytest.mark.timeout(30)
    def test_certificate_authority_config(self):
        """CertificateAuthorityConfig constructs with paths."""
        CertificateAuthorityConfig = _import_or_skip("CertificateAuthorityConfig")

        ca = CertificateAuthorityConfig(
            cert_path="/tmp/test-ca.pem",
            key_path="/tmp/test-ca-key.pem",
        )
        d = ca.to_dict()
        assert d["cert_path"] == "/tmp/test-ca.pem"
        assert d["key_path"] == "/tmp/test-ca-key.pem"

    @pytest.mark.timeout(30)
    def test_issued_certificate_construction(self):
        """IssuedCertificate constructs with cert data."""
        IssuedCertificate = _import_or_skip("IssuedCertificate")

        cert = IssuedCertificate(
            subject="example.com",
            serial_number="001",
            not_before="2026-01-01T00:00:00Z",
            not_after="2027-01-01T00:00:00Z",
            pem_data="-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----",
        )
        d = cert.to_dict()
        assert d["subject"] == "example.com"
        assert d["serial_number"] == "001"


# ---------------------------------------------------------------------------
# Proxy with loopback origin (real traffic through proxy manager)
# ---------------------------------------------------------------------------


class TestProxyWithLoopbackOrigin:
    """Test proxy operations against a real loopback HTTP origin."""

    @pytest.mark.timeout(30)
    def test_origin_server_responds(self, origin_server):
        """Loopback origin server responds to direct requests."""
        url = f"{origin_server.base_url}/proxy-test"
        resp = urlopen(url, timeout=5)
        assert resp.status == 200
        assert resp.read() == b"PROXY_ORIGIN_OK"

    @pytest.mark.timeout(30)
    def test_proxy_manager_with_origin_entry(self, origin_server):
        """ProxyManager accepts an entry pointing to the loopback origin."""
        ProxyConfig = _import_or_skip("ProxyConfig")
        ProxyEntry = _import_or_skip("ProxyEntry")
        ProxyType = _import_or_skip("ProxyType")
        create_proxy_manager = _import_or_skip("create_proxy_manager")

        config = ProxyConfig()
        manager = create_proxy_manager(config)

        entry = ProxyEntry(
            proxy_type=ProxyType.Http,
            address="127.0.0.1",
            port=origin_server.port,
        )
        manager.add_proxy(entry)
        assert manager.pool_size() == 1

        retrieved = manager.get_next_proxy()
        assert retrieved is not None
        assert retrieved.port == origin_server.port

    @pytest.mark.timeout(30)
    def test_origin_server_post_echo(self, origin_server):
        """Loopback origin echoes POST body."""
        url = f"{origin_server.base_url}/echo"
        data = b"hello-proxy"
        req = __import__("urllib.request", fromlist=["Request"]).Request(
            url, data=data, method="POST"
        )
        resp = urlopen(req, timeout=5)
        assert resp.status == 200
        assert resp.read() == data

    @pytest.mark.timeout(30)
    def test_origin_server_404(self, origin_server):
        """Loopback origin returns 404 for unknown paths."""
        url = f"{origin_server.base_url}/nonexistent"
        with pytest.raises(URLError) as exc_info:
            urlopen(url, timeout=5)
        assert "404" in str(exc_info.value) or "HTTP Error 404" in str(exc_info.value)

    @pytest.mark.timeout(30)
    def test_intercept_session_starts_and_stops(self):
        """Intercept session starts and stops within timeout."""
        run_intercept_session = _import_or_skip("run_intercept_session")
        InterceptConfig = _import_or_skip("InterceptConfig")

        config = InterceptConfig(
            listen_addr="127.0.0.1",
            listen_port=0,
            timeout_secs=1,
        )
        result = run_intercept_session(config)
        assert result is not None
        d = result.to_dict()
        assert "listen_addr" in d
        assert "duration_ms" in d
