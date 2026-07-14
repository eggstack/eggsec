"""HTTP and WebSocket correctness tests for the eggsec Python API.

Tests DTO construction, serialization, behavioral contracts with loopback
servers, scope enforcement, and feature-gated WebSocket types.
"""

from __future__ import annotations

import json

import eggsec
import pytest

from fixtures.stable_core import HOST, StableCoreFixtures

SENTINEL_LOOPBACK = "127.0.0.1"


@pytest.fixture(scope="module")
def stable_fixtures():
    with StableCoreFixtures() as fixtures:
        yield fixtures


# ============================================================================
# HTTP Tests (using StableCoreFixtures)
# ============================================================================


class TestHttpRequestConstruction:
    def test_http_request_construction(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:8080/",
            headers=[("User-Agent", "test")],
            body_text=None,
            timeout_ms=5000,
        )
        assert req.method == "GET"
        assert req.url == "http://127.0.0.1:8080/"
        assert ("User-Agent", "test") in req.headers
        assert req.body_text is None
        assert req.timeout_ms == 5000

    def test_http_request_with_body(self):
        req = eggsec.HttpRequestPy(
            method="POST",
            url="http://127.0.0.1:8080/api",
            body_text='{"key": "value"}',
        )
        assert req.method == "POST"
        assert req.body_text == '{"key": "value"}'


class TestHttpHeaders:
    def test_http_headers_ordered(self):
        headers = eggsec.HttpHeadersPy(
            entries=[("Accept", "text/html"), ("User-Agent", "test")]
        )
        d = headers.to_dict()
        entries = d["entries"]
        assert len(entries) == 2
        assert entries[0]["name"] == "Accept"
        assert entries[0]["value"] == "text/html"
        assert entries[1]["name"] == "User-Agent"
        assert entries[1]["value"] == "test"

    def test_http_headers_duplicate(self):
        headers = eggsec.HttpHeadersPy(
            entries=[("Set-Cookie", "a=1"), ("Set-Cookie", "b=2")]
        )
        cookies = headers.get_all("set-cookie")
        assert len(cookies) == 2
        assert "a=1" in cookies
        assert "b=2" in cookies

    def test_http_headers_from_dict(self):
        d = {"Accept": "text/html", "User-Agent": "test"}
        headers = eggsec.HttpHeadersPy(entries=list(d.items()))
        names = headers.names()
        assert names == ["Accept", "User-Agent"]


class TestHttpRequestSerialization:
    def test_http_request_to_dict_roundtrip(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:8080/",
            headers=[("User-Agent", "test")],
        )
        d = req.to_dict()
        req2 = eggsec.HttpRequestPy(
            method=d["method"],
            url=d["url"],
            headers=[tuple(e) for e in d["headers"]],
        )
        assert req2.method == req.method
        assert req2.url == req.url
        assert req2.headers == req.headers

    def test_http_request_to_json_roundtrip(self):
        req = eggsec.HttpRequestPy(
            method="POST",
            url="http://127.0.0.1:8080/api",
            body_text="data",
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["method"] == "POST"
        assert parsed["url"] == "http://127.0.0.1:8080/api"
        assert parsed["body_text"] == "data"


class TestRedactConfig:
    def test_http_redact_config(self):
        config = eggsec.RedactConfigPy(
            redact_headers=["Authorization", "Cookie"],
            redact_query_params=["token"],
            redact_body_fields=["password"],
        )
        assert "Authorization" in config.redact_headers
        assert "Cookie" in config.redact_headers
        assert config.redact_query_params == ["token"]
        assert config.redact_body_fields == ["password"]


class TestHttpClientConfigConstruction:
    def test_http_client_config_construction(self):
        config = eggsec.HttpClientConfigPy(
            base_url="http://127.0.0.1:8080",
            timeout_ms=5000,
            connect_timeout_ms=2000,
            max_redirects=5,
            verify_tls=False,
            user_agent="eggsec-test/1.0",
            cookie_store=False,
        )
        assert config.base_url == "http://127.0.0.1:8080"
        assert config.timeout_ms == 5000
        assert config.connect_timeout_ms == 2000
        assert config.max_redirects == 5
        assert config.verify_tls is False
        assert config.user_agent == "eggsec-test/1.0"
        assert config.cookie_store is False


class TestHttpClientScopeEnforcement:
    def test_http_client_scope_enforcement(self, stable_fixtures):
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(config=eggsec.HttpClientConfigPy())
        req = eggsec.HttpRequestPy(
            method="GET",
            url=f"http://{HOST}:{fixtures.http_port}/",
        )
        resp = client.request(req)
        assert resp.status_code == 200

    def test_engine_scope_enforcement(self, stable_fixtures):
        fixtures = stable_fixtures
        scope = eggsec.Scope.allow_hosts([HOST])
        engine = eggsec.Engine(scope)
        result = engine.run(
            eggsec.OperationRequest(
                "validate_waf",
                f"http://192.0.2.1:{fixtures.http_port}/waf-block",
                timeout_ms=500,
            )
        )
        assert result.is_failure()
        assert result.error is not None
        assert result.error.kind == "scope_denial"


class TestHttpClientContextManager:
    def test_http_client_context_manager(self, stable_fixtures):
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(config=eggsec.HttpClientConfigPy())
        with client as c:
            assert c is client
            req = eggsec.HttpRequestPy(
                method="GET",
                url=f"http://{HOST}:{fixtures.http_port}/",
            )
            assert req.method == "GET"


class TestHttpClientCloseIdempotent:
    def test_http_client_close_idempotent(self, stable_fixtures):
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(config=eggsec.HttpClientConfigPy())
        with client:
            pass
        client.close()
        client.close()


class TestHttpClientUseAfterClose:
    def test_http_client_use_after_close(self, stable_fixtures):
        fixtures = stable_fixtures
        client = eggsec.HttpClientPy(config=eggsec.HttpClientConfigPy())
        client.close()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=f"http://{HOST}:{fixtures.http_port}/",
        )
        with pytest.raises(Exception) as exc_info:
            client.request(req)
        assert isinstance(exc_info.value, (eggsec.EggsecError, eggsec.NetworkError, RuntimeError))


# ============================================================================
# WebSocket Tests (feature-gated)
# ============================================================================

_has_websocket = hasattr(eggsec, "WebSocketSessionConfigPy")


class TestWebSocketConfigConstruction:
    @pytest.mark.skipif(not _has_websocket, reason="websocket not compiled")
    def test_websocket_config_construction(self):
        config = eggsec.WebSocketSessionConfigPy(
            url="ws://127.0.0.1:8080/ws",
            origin="http://localhost",
            timeout_ms=5000,
            max_message_size=2048,
            ping_interval_ms=10000,
            close_timeout_ms=2000,
            verify_tls=False,
            subprotocols=["graphql-ws"],
        )
        assert config.url == "ws://127.0.0.1:8080/ws"
        assert config.origin == "http://localhost"
        assert config.timeout_ms == 5000
        assert config.max_message_size == 2048
        assert config.ping_interval_ms == 10000
        assert config.close_timeout_ms == 2000
        assert config.verify_tls is False
        assert config.subprotocols == ["graphql-ws"]


class TestWebSocketConfigSerialization:
    @pytest.mark.skipif(not _has_websocket, reason="websocket not compiled")
    def test_websocket_config_to_dict(self):
        config = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:8080/ws")
        d = config.to_dict()
        assert d["url"] == "ws://127.0.0.1:8080/ws"
        assert d["timeout_ms"] == 10000

    @pytest.mark.skipif(not _has_websocket, reason="websocket not compiled")
    def test_websocket_config_to_json(self):
        config = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:8080/ws")
        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://127.0.0.1:8080/ws"
        assert parsed["timeout_ms"] == 10000


class TestWebSocketScopeEnforcement:
    @pytest.mark.skipif(not _has_websocket, reason="websocket not compiled")
    def test_websocket_scope_enforcement(self):
        config = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:8080/ws")
        session = eggsec.WebSocketSessionPy(config=config)
        with pytest.raises(Exception) as exc_info:
            session.connect()
        assert isinstance(exc_info.value, (eggsec.EggsecError, eggsec.NetworkError))
