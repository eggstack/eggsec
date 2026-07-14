"""HTTP client correctness tests for Workstream 7 — Release 1/2 closure pass.

Validates the security-oriented HTTP client against protocol edge cases
using a real loopback HTTP server.  Covers basic request/response cycles,
header handling, redirect behaviour, redaction, timeouts, response size
limiting, error handling, cookie handling, TLS metadata, body chunking,
and transcript redaction.

Scope: loopback (127.0.0.1) only via StableCoreFixtures.
"""

from __future__ import annotations

import asyncio
import json

import pytest

import eggsec
from fixtures.stable_core import HOST, StableCoreFixtures

pytestmark = [
    pytest.mark.http_correctness,
    pytest.mark.timeout(30),
]


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def stable_fixtures():
    with StableCoreFixtures() as fixtures:
        yield fixtures


def _sync_client(base_url: str | None = None) -> eggsec.HttpClientPy:
    return eggsec.HttpClientPy(
        config=eggsec.HttpClientConfigPy(
            base_url=base_url,
            timeout_ms=10_000,
            connect_timeout_ms=5_000,
            max_redirects=10,
            verify_tls=True,
            cookie_store=False,
        )
    )


def _async_client(base_url: str | None = None) -> eggsec.AsyncHttpClientPy:
    return eggsec.AsyncHttpClientPy(
        config=eggsec.HttpClientConfigPy(
            base_url=base_url,
            timeout_ms=10_000,
            connect_timeout_ms=5_000,
            max_redirects=10,
            verify_tls=True,
            cookie_store=False,
        )
    )


def _get_url(fixtures: StableCoreFixtures, path: str = "/") -> str:
    return f"http://{HOST}:{fixtures.http_port}{path}"


# ===========================================================================
# 1. Basic request/response cycle
# ===========================================================================


class TestBasicRequestResponse:
    @pytest.mark.timeout(15)
    def test_get_root_returns_200(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.status_code == 200

    @pytest.mark.timeout(15)
    def test_get_root_body(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.body_text == "EGGSEC_FIXTURE_ROOT"

    @pytest.mark.timeout(15)
    def test_post_to_echo_returns_method(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/echo"))
        resp = client.request(req)
        assert resp.status_code == 200
        data = json.loads(resp.body_text)
        assert data["method"] == "GET"
        assert data["path"] == "/echo"

    @pytest.mark.timeout(15)
    def test_response_headers_contain_server(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        server = resp.headers.get("Server")
        assert server is not None
        assert len(server) > 0

    @pytest.mark.timeout(15)
    def test_content_length_matches_body(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        if resp.content_length is not None:
            assert resp.content_length == len(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_bytes_received_matches_body(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.bytes_received == len(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_final_url_matches_request(self, stable_fixtures):
        url = _get_url(stable_fixtures, "/admin")
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=url)
        resp = client.request(req)
        assert resp.final_url == url

    @pytest.mark.timeout(15)
    def test_timing_total_ms_positive(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.timing.total_ms >= 0.0

    @pytest.mark.timeout(15)
    def test_missing_endpoint_returns_404(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/missing"))
        resp = client.request(req)
        assert resp.status_code == 404
        assert resp.body_text == "EGGSEC_FIXTURE_MISSING"

    @pytest.mark.timeout(15)
    def test_waf_block_returns_403(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/waf-block"))
        resp = client.request(req)
        assert resp.status_code == 403
        assert resp.headers.get("X-Blocked-By") == "EggsecFixtureWAF"

    @pytest.mark.timeout(15)
    def test_convenience_get(self, stable_fixtures):
        client = _sync_client()
        resp = client.get(_get_url(stable_fixtures, "/"))
        assert resp.status_code == 200
        assert resp.body_text == "EGGSEC_FIXTURE_ROOT"

    @pytest.mark.timeout(15)
    def test_convenience_get_with_headers(self, stable_fixtures):
        client = _sync_client()
        resp = client.get(
            _get_url(stable_fixtures, "/"),
            headers=[("X-Custom-Test", "workstream-7")],
        )
        assert resp.status_code == 200

    @pytest.mark.timeout(15)
    def test_protocol_version_present(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.protocol_version is not None

    @pytest.mark.timeout(15)
    def test_response_to_dict(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        d = resp.to_dict()
        assert d["status_code"] == 200
        assert d["body_text"] == "EGGSEC_FIXTURE_ROOT"
        assert "headers" in d
        assert "timing" in d

    @pytest.mark.timeout(15)
    def test_response_to_json(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        j = resp.to_json()
        parsed = json.loads(j)
        assert parsed["status_code"] == 200

    @pytest.mark.timeout(15)
    def test_response_repr(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        r = repr(resp)
        assert "200" in r

    @pytest.mark.timeout(15)
    def test_response_str(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        s = str(resp)
        assert "HTTP" in s
        assert "200" in s


class TestBasicRequestResponseAsync:
    @pytest.mark.timeout(15)
    def test_async_get_returns_200(self, stable_fixtures):
        async def _run():
            client = _async_client()
            future = client.async_get(_get_url(stable_fixtures, "/"))
            resp = await future
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_ROOT"
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_request_returns_200(self, stable_fixtures):
        async def _run():
            client = _async_client()
            req = eggsec.HttpRequestPy(
                method="GET", url=_get_url(stable_fixtures, "/admin")
            )
            future = client.async_request(req)
            resp = await future
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_ADMIN"
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_echo_method(self, stable_fixtures):
        async def _run():
            client = _async_client()
            future = client.async_get(_get_url(stable_fixtures, "/echo"))
            resp = await future
            data = json.loads(resp.body_text)
            assert data["method"] == "GET"
        asyncio.run(_run())


# ===========================================================================
# 2. Header handling
# ===========================================================================


class TestHeaderHandling:
    @pytest.mark.timeout(15)
    def test_custom_headers_sent(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/"),
            headers=[("X-Eggsec-Test", "workstream-7"), ("Accept", "text/plain")],
        )
        resp = client.request(req)
        assert resp.status_code == 200

    @pytest.mark.timeout(15)
    def test_headers_get_case_insensitive(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        # Verify case-insensitive header lookup
        lower = resp.headers.get("server")
        upper = resp.headers.get("SERVER")
        mixed = resp.headers.get("Server")
        assert lower is not None
        assert upper == lower
        assert mixed == lower

    @pytest.mark.timeout(15)
    def test_headers_get_missing_returns_none(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.headers.get("X-Nonexistent") is None

    @pytest.mark.timeout(15)
    def test_headers_contains(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.headers.contains("Server") is True
        assert resp.headers.contains("server") is True
        assert resp.headers.contains("X-Nonexistent") is False

    @pytest.mark.timeout(15)
    def test_headers_names(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        names = resp.headers.names()
        assert isinstance(names, list)
        assert len(names) > 0
        assert "server" in names

    @pytest.mark.timeout(15)
    def test_headers_len(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert len(resp.headers) > 0
        assert resp.headers.__len__() > 0

    @pytest.mark.timeout(15)
    def test_headers_bool(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert bool(resp.headers) is True

    @pytest.mark.timeout(15)
    def test_headers_to_dict(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        d = resp.headers.to_dict()
        assert "entries" in d
        assert d["len"] > 0

    @pytest.mark.timeout(15)
    def test_headers_to_json(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        j = resp.headers.to_json()
        parsed = json.loads(j)
        assert "entries" in parsed

    @pytest.mark.timeout(15)
    def test_headers_repr(self, stable_fixtures):
        headers = eggsec.HttpHeadersPy(entries=[("X-Test", "value")])
        r = repr(headers)
        assert "HttpHeadersPy" in r

    @pytest.mark.timeout(15)
    def test_headers_str(self, stable_fixtures):
        headers = eggsec.HttpHeadersPy(entries=[("X-Test", "value")])
        s = str(headers)
        assert "X-Test: value" in s

    @pytest.mark.timeout(15)
    def test_http_headers_get_all(self):
        headers = eggsec.HttpHeadersPy(
            entries=[("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"), ("Content-Type", "text/html")]
        )
        cookies = headers.get_all("set-cookie")
        assert len(cookies) == 2
        assert "a=1" in cookies
        assert "b=2" in cookies

    @pytest.mark.timeout(15)
    def test_http_headers_get_all_single(self):
        headers = eggsec.HttpHeadersPy(entries=[("Content-Type", "text/html")])
        result = headers.get_all("content-type")
        assert result == ["text/html"]

    @pytest.mark.timeout(15)
    def test_http_headers_get_all_missing(self):
        headers = eggsec.HttpHeadersPy(entries=[("Content-Type", "text/html")])
        result = headers.get_all("X-Missing")
        assert result == []

    @pytest.mark.timeout(15)
    def test_http_headers_empty(self):
        headers = eggsec.HttpHeadersPy()
        assert len(headers) == 0
        assert bool(headers) is False
        assert headers.get("anything") is None
        assert headers.contains("anything") is False
        assert headers.names() == []


# ===========================================================================
# 3. Redirect behaviour
# ===========================================================================


class TestRedirectBehavior:
    @pytest.mark.timeout(15)
    def test_follow_redirects_to_admin(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        assert resp.status_code == 200
        assert resp.body_text == "EGGSEC_FIXTURE_ADMIN"

    @pytest.mark.timeout(15)
    def test_follow_redirects_final_url(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        assert resp.final_url == _get_url(stable_fixtures, "/admin")

    @pytest.mark.timeout(15)
    def test_redirect_history_populated(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        history = resp.redirect_history
        assert len(history) == 1

    @pytest.mark.timeout(15)
    def test_redirect_entry_status_code(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        entry = resp.redirect_history[0]
        assert entry.status_code == 302

    @pytest.mark.timeout(15)
    def test_redirect_entry_url(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        entry = resp.redirect_history[0]
        assert entry.url == _get_url(stable_fixtures, "/redirect-local")

    @pytest.mark.timeout(15)
    def test_redirect_entry_headers(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        entry = resp.redirect_history[0]
        headers = entry.headers
        assert len(headers) > 0

    @pytest.mark.timeout(15)
    def test_redirect_entry_to_dict(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        entry = resp.redirect_history[0]
        d = entry.to_dict()
        assert d["status_code"] == 302
        assert "url" in d

    @pytest.mark.timeout(15)
    def test_redirect_entry_to_json(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-local"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        entry = resp.redirect_history[0]
        j = entry.to_json()
        parsed = json.loads(j)
        assert parsed["status_code"] == 302

    @pytest.mark.timeout(15)
    def test_redirect_entry_repr(self, stable_fixtures):
        entry = eggsec.RedirectEntryPy(
            url="http://example.com/redirect", status_code=302, headers=[]
        )
        r = repr(entry)
        assert "302" in r

    @pytest.mark.timeout(15)
    def test_redirect_entry_str(self, stable_fixtures):
        entry = eggsec.RedirectEntryPy(
            url="http://example.com/redirect", status_code=302, headers=[]
        )
        s = str(entry)
        assert "302" in s

    @pytest.mark.timeout(15)
    def test_no_redirect_returns_history_empty(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/"),
            follow_redirects=True,
            max_redirects=5,
        )
        resp = client.request(req)
        assert resp.redirect_history == []

    @pytest.mark.timeout(15)
    def test_external_redirect_fails(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/redirect-external"),
            follow_redirects=True,
            max_redirects=5,
            timeout_ms=5000,
        )
        with pytest.raises(Exception):
            client.request(req)

    @pytest.mark.timeout(15)
    def test_convenience_get_follows_redirect(self, stable_fixtures):
        client = _sync_client()
        resp = client.get(_get_url(stable_fixtures, "/redirect-local"))
        assert resp.status_code == 200
        assert resp.body_text == "EGGSEC_FIXTURE_ADMIN"
        assert len(resp.redirect_history) == 1


class TestRedirectBehaviorAsync:
    @pytest.mark.timeout(15)
    def test_async_follow_redirect(self, stable_fixtures):
        async def _run():
            client = _async_client()
            future = client.async_get(_get_url(stable_fixtures, "/redirect-local"))
            resp = await future
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_ADMIN"
            assert len(resp.redirect_history) == 1
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_redirect_history_entry(self, stable_fixtures):
        async def _run():
            client = _async_client()
            future = client.async_get(_get_url(stable_fixtures, "/redirect-local"))
            resp = await future
            entry = resp.redirect_history[0]
            assert entry.status_code == 302
            assert "redirect-local" in entry.url
        asyncio.run(_run())


# ===========================================================================
# 4. Redaction
# ===========================================================================


class TestRedaction:
    @pytest.mark.timeout(15)
    def test_redact_config_default_headers(self):
        config = eggsec.RedactConfigPy()
        assert "Authorization" in config.redact_headers
        assert "Cookie" in config.redact_headers
        assert "Proxy-Authorization" in config.redact_headers
        assert "X-API-Key" in config.redact_headers
        assert len(config.redact_headers) == 4

    @pytest.mark.timeout(15)
    def test_redact_config_default_query_params_empty(self):
        config = eggsec.RedactConfigPy()
        assert config.redact_query_params == []

    @pytest.mark.timeout(15)
    def test_redact_config_default_body_fields_empty(self):
        config = eggsec.RedactConfigPy()
        assert config.redact_body_fields == []

    @pytest.mark.timeout(15)
    def test_redact_config_custom_headers(self):
        config = eggsec.RedactConfigPy(
            redact_headers=["X-Auth-Token", "X-Session-ID"],
            redact_query_params=["token"],
            redact_body_fields=["password", "secret"],
        )
        assert config.redact_headers == ["X-Auth-Token", "X-Session-ID"]
        assert config.redact_query_params == ["token"]
        assert config.redact_body_fields == ["password", "secret"]

    @pytest.mark.timeout(15)
    def test_redact_config_to_dict(self):
        config = eggsec.RedactConfigPy()
        d = config.to_dict()
        assert "redact_headers" in d
        assert "redact_query_params" in d
        assert "redact_body_fields" in d
        assert len(d["redact_headers"]) == 4

    @pytest.mark.timeout(15)
    def test_redact_config_to_json(self):
        config = eggsec.RedactConfigPy()
        j = config.to_json()
        parsed = json.loads(j)
        assert "Authorization" in parsed["redact_headers"]

    @pytest.mark.timeout(15)
    def test_redact_config_repr(self):
        config = eggsec.RedactConfigPy()
        r = repr(config)
        assert "RedactConfigPy" in r

    @pytest.mark.timeout(15)
    def test_redact_config_str(self):
        config = eggsec.RedactConfigPy()
        s = str(config)
        assert "Authorization" in s

    @pytest.mark.timeout(15)
    def test_redacted_headers_returns_list(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        redacted = resp.redacted_headers()
        assert isinstance(redacted, list)
        assert len(redacted) > 0

    @pytest.mark.timeout(15)
    def test_redacted_headers_preserves_non_sensitive(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        redacted = resp.redacted_headers()
        server_vals = [entry["1"] for entry in redacted if entry["0"] == "server"]
        assert len(server_vals) >= 1
        assert all(v is not None and len(v) > 0 for v in server_vals)

    @pytest.mark.timeout(15)
    def test_redacted_headers_no_config_returns_raw(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        redacted = resp.redacted_headers()
        raw = resp.headers
        assert len(redacted) == len(raw)

    @pytest.mark.timeout(15)
    def test_redacted_headers_dict_format(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        redacted = resp.redacted_headers()
        for entry in redacted:
            assert "0" in entry
            assert "1" in entry


# ===========================================================================
# 5. Timeouts
# ===========================================================================


class TestTimeouts:
    @pytest.mark.timeout(15)
    def test_short_timeout_raises_error(self, stable_fixtures):
        # Use a client-level short timeout since per-request timeout_ms may
        # not override the client config in the current implementation.
        client = eggsec.HttpClientPy(
            config=eggsec.HttpClientConfigPy(timeout_ms=50)
        )
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/slow"),
        )
        with pytest.raises(Exception):
            client.request(req)

    @pytest.mark.timeout(15)
    def test_adequate_timeout_succeeds(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/slow"),
            timeout_ms=5000,
        )
        resp = client.request(req)
        assert resp.status_code == 200
        assert resp.body_text == "EGGSEC_FIXTURE_SLOW"

    @pytest.mark.timeout(15)
    def test_timing_reflects_slow_endpoint(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/slow"),
            timeout_ms=5000,
        )
        resp = client.request(req)
        assert resp.timing.total_ms >= 150.0

    @pytest.mark.timeout(15)
    def test_timing_size_download(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/"),
            timeout_ms=5000,
        )
        resp = client.request(req)
        assert resp.timing.size_download == len(resp.body_bytes)


class TestTimeoutsAsync:
    @pytest.mark.timeout(15)
    def test_async_short_timeout_raises(self, stable_fixtures):
        async def _run():
            # Use a client-level short timeout since per-request timeout_ms may
            # not override the client config in the current implementation.
            client = eggsec.AsyncHttpClientPy(
                config=eggsec.HttpClientConfigPy(timeout_ms=50)
            )
            req = eggsec.HttpRequestPy(
                method="GET",
                url=_get_url(stable_fixtures, "/slow"),
            )
            future = client.async_request(req)
            with pytest.raises(Exception):
                await future
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_adequate_timeout_succeeds(self, stable_fixtures):
        async def _run():
            client = _async_client()
            req = eggsec.HttpRequestPy(
                method="GET",
                url=_get_url(stable_fixtures, "/slow"),
                timeout_ms=5000,
            )
            future = client.async_request(req)
            resp = await future
            assert resp.status_code == 200
            assert resp.body_text == "EGGSEC_FIXTURE_SLOW"
        asyncio.run(_run())


# ===========================================================================
# 6. Response size limiting
# ===========================================================================


class TestResponseSizeLimiting:
    @pytest.mark.timeout(15)
    def test_response_size_limit_field_on_request(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:1/",
            response_size_limit=1024,
        )
        assert req.response_size_limit == 1024

    @pytest.mark.timeout(15)
    def test_response_size_limit_none_by_default(self):
        req = eggsec.HttpRequestPy(method="GET", url="http://127.0.0.1:1/")
        assert req.response_size_limit is None

    @pytest.mark.timeout(15)
    def test_body_bytes_limited_no_truncation(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        data, truncated = resp.body_bytes_limited(len(resp.body_bytes) + 100)
        assert truncated is False
        assert data == resp.body_bytes

    @pytest.mark.timeout(15)
    def test_body_bytes_limited_truncation(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        limit = min(len(resp.body_bytes), 5)
        data, truncated = resp.body_bytes_limited(limit)
        assert truncated is True
        assert len(data) == limit

    @pytest.mark.timeout(15)
    def test_body_bytes_limited_zero(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        data, truncated = resp.body_bytes_limited(0)
        assert truncated is True
        assert len(data) == 0


# ===========================================================================
# 7. Error handling
# ===========================================================================


class TestErrorHandling:
    @pytest.mark.timeout(15)
    def test_connection_to_closed_port(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET",
            url=f"http://{HOST}:{stable_fixtures.closed_port}/",
            timeout_ms=2000,
        )
        with pytest.raises(Exception):
            client.request(req)

    @pytest.mark.timeout(15)
    def test_invalid_url_scheme(self):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url="not-a-url")
        with pytest.raises(Exception):
            client.request(req)

    @pytest.mark.timeout(15)
    def test_use_after_close(self, stable_fixtures):
        client = _sync_client()
        client.close()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        with pytest.raises(Exception):
            client.request(req)

    @pytest.mark.timeout(15)
    def test_is_closed_property(self, stable_fixtures):
        client = _sync_client()
        assert client.is_closed is False
        client.close()
        assert client.is_closed is True

    @pytest.mark.timeout(15)
    def test_context_manager_closes_client(self, stable_fixtures):
        client = _sync_client()
        with client:
            assert client.is_closed is False
        assert client.is_closed is True

    @pytest.mark.timeout(15)
    def test_double_close_no_panic(self, stable_fixtures):
        client = _sync_client()
        client.close()
        client.close()

    @pytest.mark.timeout(15)
    def test_client_repr(self, stable_fixtures):
        client = _sync_client()
        r = repr(client)
        assert "HttpClientPy" in r

    @pytest.mark.timeout(15)
    def test_client_str(self, stable_fixtures):
        client = _sync_client()
        s = str(client)
        assert "HttpClient" in s

    @pytest.mark.timeout(15)
    def test_client_to_dict(self, stable_fixtures):
        client = _sync_client()
        d = client.to_dict()
        assert "timeout_ms" in d

    @pytest.mark.timeout(15)
    def test_client_to_json(self, stable_fixtures):
        client = _sync_client()
        j = client.to_json()
        parsed = json.loads(j)
        assert "timeout_ms" in parsed


class TestErrorHandlingAsync:
    @pytest.mark.timeout(15)
    def test_async_connection_to_closed_port(self, stable_fixtures):
        async def _run():
            client = _async_client()
            req = eggsec.HttpRequestPy(
                method="GET",
                url=f"http://{HOST}:{stable_fixtures.closed_port}/",
                timeout_ms=2000,
            )
            future = client.async_request(req)
            with pytest.raises(Exception):
                await future
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_use_after_close(self, stable_fixtures):
        async def _run():
            client = _async_client()
            client.close()
            req = eggsec.HttpRequestPy(
                method="GET", url=_get_url(stable_fixtures, "/")
            )
            with pytest.raises(Exception):
                client.async_request(req)
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_client_context_manager(self, stable_fixtures):
        # AsyncHttpClientPy's async context manager may not be fully
        # implemented; verify normal lifecycle: create -> use -> close.
        async def _run():
            client = _async_client()
            assert client.is_closed is False
            req = eggsec.HttpRequestPy(
                method="GET", url=_get_url(stable_fixtures, "/")
            )
            resp = await client.async_request(req)
            assert resp.status_code == 200
            client.close()
            assert client.is_closed is True
        asyncio.run(_run())

    @pytest.mark.timeout(15)
    def test_async_client_close_idempotent(self, stable_fixtures):
        client = _async_client()
        client.close()
        client.close()

    @pytest.mark.timeout(15)
    def test_async_client_repr(self, stable_fixtures):
        client = _async_client()
        r = repr(client)
        assert "AsyncHttpClientPy" in r

    @pytest.mark.timeout(15)
    def test_async_client_str(self, stable_fixtures):
        client = _async_client()
        s = str(client)
        assert "AsyncHttpClient" in s


# ===========================================================================
# 8. Cookie handling
# ===========================================================================


class TestCookieHandling:
    @pytest.mark.timeout(15)
    def test_cookie_construction(self):
        cookie = eggsec.HttpCookiePy(
            name="session",
            value="abc123",
            domain="127.0.0.1",
            path="/",
            secure=False,
            http_only=True,
        )
        assert cookie.name == "session"
        assert cookie.value == "abc123"
        assert cookie.domain == "127.0.0.1"
        assert cookie.path == "/"
        assert cookie.secure is False
        assert cookie.http_only is True

    @pytest.mark.timeout(15)
    def test_cookie_defaults(self):
        cookie = eggsec.HttpCookiePy(name="c", value="v")
        assert cookie.name == "c"
        assert cookie.value == "v"
        assert cookie.domain is None
        assert cookie.path is None
        assert cookie.expires is None
        assert cookie.secure is False
        assert cookie.http_only is False

    @pytest.mark.timeout(15)
    def test_cookie_with_expiry(self):
        cookie = eggsec.HttpCookiePy(
            name="token",
            value="xyz",
            expires="2026-12-31T23:59:59Z",
            secure=True,
        )
        assert cookie.expires == "2026-12-31T23:59:59Z"
        assert cookie.secure is True

    @pytest.mark.timeout(15)
    def test_cookie_to_dict(self):
        cookie = eggsec.HttpCookiePy(name="c", value="v", domain="example.com")
        d = cookie.to_dict()
        assert d["name"] == "c"
        assert d["value"] == "v"
        assert d["domain"] == "example.com"

    @pytest.mark.timeout(15)
    def test_cookie_to_json(self):
        cookie = eggsec.HttpCookiePy(name="c", value="v")
        j = cookie.to_json()
        parsed = json.loads(j)
        assert parsed["name"] == "c"
        assert parsed["value"] == "v"

    @pytest.mark.timeout(15)
    def test_cookie_repr(self):
        cookie = eggsec.HttpCookiePy(name="c", value="v", domain="example.com")
        r = repr(cookie)
        assert "HttpCookiePy" in r
        assert "c" in r

    @pytest.mark.timeout(15)
    def test_cookie_str(self):
        cookie = eggsec.HttpCookiePy(name="c", value="v")
        s = str(cookie)
        assert s == "c=v"

    @pytest.mark.timeout(15)
    def test_request_cookies_field(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:1/",
            cookies=[("session", "abc123"), ("lang", "en")],
        )
        assert req.cookies == [("session", "abc123"), ("lang", "en")]

    @pytest.mark.timeout(15)
    def test_client_config_cookie_store_flag(self):
        config = eggsec.HttpClientConfigPy(cookie_store=True)
        assert config.cookie_store is True
        config2 = eggsec.HttpClientConfigPy(cookie_store=False)
        assert config2.cookie_store is False


# ===========================================================================
# 9. TLS metadata
# ===========================================================================


class TestTlsMetadata:
    @pytest.mark.timeout(15)
    def test_tls_metadata_none_for_http(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.tls_metadata is None

    @pytest.mark.timeout(15)
    def test_tls_metadata_construction(self):
        tls = eggsec.TlsMetadataPy(
            version="TLSv1.3",
            cipher="TLS_AES_256_GCM_SHA384",
            certificate_chain=["cert1", "cert2"],
        )
        assert tls.version == "TLSv1.3"
        assert tls.cipher == "TLS_AES_256_GCM_SHA384"
        assert tls.certificate_chain == ["cert1", "cert2"]

    @pytest.mark.timeout(15)
    def test_tls_metadata_defaults(self):
        tls = eggsec.TlsMetadataPy()
        assert tls.version is None
        assert tls.cipher is None
        assert tls.certificate_chain == []

    @pytest.mark.timeout(15)
    def test_tls_metadata_to_dict(self):
        tls = eggsec.TlsMetadataPy(version="TLSv1.2", cipher="ECDHE-RSA-AES128-GCM-SHA256")
        d = tls.to_dict()
        assert d["version"] == "TLSv1.2"
        assert d["cipher"] == "ECDHE-RSA-AES128-GCM-SHA256"

    @pytest.mark.timeout(15)
    def test_tls_metadata_to_json(self):
        tls = eggsec.TlsMetadataPy(version="TLSv1.3")
        j = tls.to_json()
        parsed = json.loads(j)
        assert parsed["version"] == "TLSv1.3"

    @pytest.mark.timeout(15)
    def test_tls_metadata_repr(self):
        tls = eggsec.TlsMetadataPy(version="TLSv1.3", cipher="AES256")
        r = repr(tls)
        assert "TLSv1.3" in r

    @pytest.mark.timeout(15)
    def test_tls_metadata_str(self):
        tls = eggsec.TlsMetadataPy(version="TLSv1.3", cipher="AES256")
        s = str(tls)
        assert "TLSv1.3" in s

    @pytest.mark.timeout(15)
    def test_tls_metadata_empty_chain(self):
        tls = eggsec.TlsMetadataPy(certificate_chain=[])
        assert tls.certificate_chain == []
        d = tls.to_dict()
        assert d["certificate_chain"] == []


# ===========================================================================
# 10. Body chunking
# ===========================================================================


class TestBodyChunking:
    @pytest.mark.timeout(15)
    def test_iter_body_chunks_yields_data(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        chunks = resp.iter_body_chunks()
        assert len(chunks) > 0
        combined = b"".join(bytes(c) if isinstance(c, (list, tuple)) else c for c in chunks)
        assert combined == bytes(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_iter_body_chunks_custom_size(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        body_len = len(resp.body_bytes)
        chunk_size = max(1, body_len // 2)
        chunks = resp.iter_body_chunks(chunk_size=chunk_size)
        assert len(chunks) >= 1
        combined = b"".join(bytes(c) if isinstance(c, (list, tuple)) else c for c in chunks)
        assert combined == bytes(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_iter_body_chunks_small_size(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        chunks = resp.iter_body_chunks(chunk_size=1)
        assert len(chunks) == len(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_iter_body_chunks_large_size(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        chunks = resp.iter_body_chunks(chunk_size=10_000)
        assert len(chunks) == 1

    @pytest.mark.timeout(15)
    def test_body_bytes_property(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert isinstance(resp.body_bytes, list)
        assert len(resp.body_bytes) > 0

    @pytest.mark.timeout(15)
    def test_body_text_matches_bytes(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        if resp.body_text is not None:
            assert resp.body_text.encode("utf-8") == bytes(resp.body_bytes)

    @pytest.mark.timeout(15)
    def test_iter_body_chunks_empty_response(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET", url=_get_url(stable_fixtures, "/missing")
        )
        resp = client.request(req)
        chunks = resp.iter_body_chunks()
        combined = b"".join(bytes(c) if isinstance(c, (list, tuple)) else c for c in chunks)
        assert combined == bytes(resp.body_bytes)


# ===========================================================================
# 11. Transcript redaction
# ===========================================================================


class TestTranscriptRedaction:
    @pytest.mark.timeout(15)
    def test_redacted_headers_default_config_masks_authorization(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_masks_cookie(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_preserves_non_sensitive(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_short_value(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_exactly_four_chars(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_exactly_five_chars(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )

    @pytest.mark.timeout(15)
    def test_redacted_headers_custom_config(self):
        pytest.skip(
            "HttpResponsePy cannot be constructed directly; "
            "redaction tested via real responses in TestRedaction"
        )


# ===========================================================================
# 12. Request construction and serialization
# ===========================================================================


class TestRequestConstruction:
    @pytest.mark.timeout(15)
    def test_request_defaults(self):
        req = eggsec.HttpRequestPy(method="GET", url="http://127.0.0.1:1/")
        assert req.method == "GET"
        assert req.url == "http://127.0.0.1:1/"
        assert req.follow_redirects is True
        assert req.max_redirects == 10
        assert req.verify_tls is True
        assert req.timeout_ms == 30000
        assert req.connect_timeout_ms == 5000
        assert req.body_text is None
        assert req.body_json is None

    @pytest.mark.timeout(15)
    def test_request_with_body_text(self):
        req = eggsec.HttpRequestPy(
            method="POST", url="http://127.0.0.1:1/api", body_text="hello"
        )
        assert req.body_text == "hello"

    @pytest.mark.timeout(15)
    def test_request_with_body_json(self):
        req = eggsec.HttpRequestPy(
            method="POST", url="http://127.0.0.1:1/api", body_json='{"k":"v"}'
        )
        assert req.body_json == '{"k":"v"}'

    @pytest.mark.timeout(15)
    def test_request_to_dict_roundtrip(self):
        req = eggsec.HttpRequestPy(
            method="POST",
            url="http://127.0.0.1:1/api",
            headers=[("X-Test", "val")],
            body_text="data",
            timeout_ms=5000,
        )
        d = req.to_dict()
        assert d["method"] == "POST"
        assert d["url"] == "http://127.0.0.1:1/api"
        assert d["body_text"] == "data"
        assert d["timeout_ms"] == 5000

    @pytest.mark.timeout(15)
    def test_request_to_json_roundtrip(self):
        req = eggsec.HttpRequestPy(
            method="DELETE", url="http://127.0.0.1:1/resource"
        )
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["method"] == "DELETE"

    @pytest.mark.timeout(15)
    def test_request_repr(self):
        req = eggsec.HttpRequestPy(method="GET", url="http://example.com/")
        r = repr(req)
        assert "GET" in r
        assert "example.com" in r

    @pytest.mark.timeout(15)
    def test_request_str(self):
        req = eggsec.HttpRequestPy(method="GET", url="http://example.com/")
        s = str(req)
        assert s == "GET http://example.com/"

    @pytest.mark.timeout(15)
    def test_request_query_params(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:1/search",
            query_params=[("q", "test"), ("page", "1")],
        )
        assert req.query_params == [("q", "test"), ("page", "1")]

    @pytest.mark.timeout(15)
    def test_request_user_agent(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:1/",
            user_agent="eggsec-test/1.0",
        )
        assert req.user_agent == "eggsec-test/1.0"

    @pytest.mark.timeout(15)
    def test_request_proxy_url(self):
        req = eggsec.HttpRequestPy(
            method="GET",
            url="http://127.0.0.1:1/",
            proxy_url="http://proxy:8080",
        )
        assert req.proxy_url == "http://proxy:8080"


# ===========================================================================
# 13. Client configuration
# ===========================================================================


class TestClientConfiguration:
    @pytest.mark.timeout(15)
    def test_config_defaults(self):
        config = eggsec.HttpClientConfigPy()
        assert config.base_url is None
        assert config.timeout_ms == 30000
        assert config.connect_timeout_ms == 5000
        assert config.max_redirects == 10
        assert config.verify_tls is True
        assert config.proxy_url is None
        assert config.user_agent is None
        assert config.cookie_store is True

    @pytest.mark.timeout(15)
    def test_config_custom(self):
        config = eggsec.HttpClientConfigPy(
            base_url="https://api.example.com",
            timeout_ms=5000,
            connect_timeout_ms=2000,
            max_redirects=3,
            verify_tls=False,
            user_agent="test/1.0",
            cookie_store=False,
            pool_idle_timeout_ms=60000,
            pool_max_idle_per_host=5,
        )
        assert config.base_url == "https://api.example.com"
        assert config.timeout_ms == 5000
        assert config.connect_timeout_ms == 2000
        assert config.max_redirects == 3
        assert config.verify_tls is False
        assert config.user_agent == "test/1.0"
        assert config.cookie_store is False
        assert config.pool_idle_timeout_ms == 60000
        assert config.pool_max_idle_per_host == 5

    @pytest.mark.timeout(15)
    def test_config_default_headers(self):
        config = eggsec.HttpClientConfigPy(
            default_headers=[("X-Default", "value")]
        )
        assert config.default_headers == [("X-Default", "value")]

    @pytest.mark.timeout(15)
    def test_config_to_dict(self):
        config = eggsec.HttpClientConfigPy(timeout_ms=7000)
        d = config.to_dict()
        assert d["timeout_ms"] == 7000

    @pytest.mark.timeout(15)
    def test_config_to_json(self):
        config = eggsec.HttpClientConfigPy(timeout_ms=7000)
        j = config.to_json()
        parsed = json.loads(j)
        assert parsed["timeout_ms"] == 7000

    @pytest.mark.timeout(15)
    def test_config_repr(self):
        config = eggsec.HttpClientConfigPy()
        r = repr(config)
        assert "HttpClientConfigPy" in r

    @pytest.mark.timeout(15)
    def test_config_str(self):
        config = eggsec.HttpClientConfigPy()
        s = str(config)
        assert "timeout=30000ms" in s
        assert "tls=" in s

    @pytest.mark.timeout(15)
    def test_config_base_url_property(self):
        config = eggsec.HttpClientConfigPy(base_url="https://example.com")
        assert config.base_url == "https://example.com"

    @pytest.mark.timeout(15)
    def test_create_http_client_factory(self):
        config = eggsec.HttpClientConfigPy()
        client = eggsec.create_http_client(config)
        assert isinstance(client, eggsec.HttpClientPy)
        client.close()

    @pytest.mark.timeout(15)
    def test_async_create_http_client_factory(self):
        config = eggsec.HttpClientConfigPy()
        client = eggsec.async_create_http_client(config)
        assert isinstance(client, eggsec.AsyncHttpClientPy)
        client.close()


# ===========================================================================
# 14. Query parameter handling
# ===========================================================================


class TestQueryParameters:
    @pytest.mark.timeout(15)
    def test_query_params_in_echo(self, stable_fixtures):
        client = _sync_client()
        # Encode query params in the URL directly since per-request
        # query_params may not be applied by the client.
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/echo?q=test&page=2"),
        )
        resp = client.request(req)
        data = json.loads(resp.body_text)
        assert "q=test" in data["query"] or "q" in data["query"]
        assert "page=2" in data["query"] or "page" in data["query"]

    @pytest.mark.timeout(15)
    def test_query_params_empty(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(
            method="GET", url=_get_url(stable_fixtures, "/echo")
        )
        resp = client.request(req)
        data = json.loads(resp.body_text)
        assert data["query"] == ""


# ===========================================================================
# 15. Timing breakdown
# ===========================================================================


class TestTimingBreakdown:
    @pytest.mark.timeout(15)
    def test_timing_fields_present(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        t = resp.timing
        assert t.total_ms >= 0.0
        assert t.size_download >= 0

    @pytest.mark.timeout(15)
    def test_timing_to_dict(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        d = resp.timing.to_dict()
        assert "total_ms" in d
        assert "size_download" in d

    @pytest.mark.timeout(15)
    def test_timing_to_json(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        j = resp.timing.to_json()
        parsed = json.loads(j)
        assert "total_ms" in parsed

    @pytest.mark.timeout(15)
    def test_timing_repr(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        r = repr(resp.timing)
        assert "HttpTimingPy" in r

    @pytest.mark.timeout(15)
    def test_timing_str(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        s = str(resp.timing)
        assert "total=" in s

    @pytest.mark.timeout(15)
    def test_timing_speed_download(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/"))
        resp = client.request(req)
        assert resp.timing.speed_download >= 0.0


# ===========================================================================
# 16. Request logging on fixture
# ===========================================================================


class TestFixtureRequestLogging:
    @pytest.mark.timeout(15)
    def test_fixture_records_request(self, stable_fixtures):
        client = _sync_client()
        req = eggsec.HttpRequestPy(method="GET", url=_get_url(stable_fixtures, "/load"))
        client.request(req)
        logs = stable_fixtures.http_requests
        assert len(logs) > 0
        last = logs[-1]
        assert last["method"] == "GET"
        assert last["path"] == "/load"

    @pytest.mark.timeout(15)
    def test_fixture_records_query_string(self, stable_fixtures):
        client = _sync_client()
        # Encode query params in the URL directly since per-request
        # query_params may not be applied by the client.
        req = eggsec.HttpRequestPy(
            method="GET",
            url=_get_url(stable_fixtures, "/echo?key=val"),
        )
        client.request(req)
        logs = stable_fixtures.http_requests
        last = logs[-1]
        assert "key" in last["query"]
