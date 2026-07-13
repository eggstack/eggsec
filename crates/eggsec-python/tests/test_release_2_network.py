"""Unit tests for Release 2 network programmability types.

Tests type construction, property access, default values, serialization,
repr/str output, context manager protocol, and error handling.
"""

import json
import pytest

import eggsec


# ============================================================================
# Network Types
# ============================================================================


class TestTarget:
    def test_target_construction(self):
        t = eggsec.TargetPy(host="example.com")
        assert t.host == "example.com"
        assert t.port is None
        assert t.scheme is None
        assert t.url_path is None

    def test_target_construction_full(self):
        t = eggsec.TargetPy(host="example.com", port=8443, scheme="https", url_path="/api/v1")
        assert t.host == "example.com"
        assert t.port == 8443
        assert t.scheme == "https"
        assert t.url_path == "/api/v1"

    def test_target_normalized(self):
        t = eggsec.TargetPy(host="example.com", port=443, scheme="https")
        assert t.normalized() == "https://example.com"

    def test_target_normalized_default_port_omitted(self):
        t = eggsec.TargetPy(host="example.com", port=80, scheme="http")
        assert t.normalized() == "http://example.com"

    def test_target_normalized_non_default_port(self):
        t = eggsec.TargetPy(host="example.com", port=8080, scheme="http")
        assert t.normalized() == "http://example.com:8080"

    def test_target_normalized_no_scheme(self):
        t = eggsec.TargetPy(host="example.com", port=22)
        assert t.normalized() == "example.com:22"

    def test_target_normalized_with_path(self):
        t = eggsec.TargetPy(host="example.com", scheme="https", url_path="api/data")
        assert t.normalized() == "https://example.com/api/data"

    def test_target_normalized_path_leading_slash(self):
        t = eggsec.TargetPy(host="example.com", url_path="/api")
        assert t.normalized() == "example.com/api"

    def test_target_is_ip(self):
        t = eggsec.TargetPy(host="192.168.1.1")
        assert t.is_ip() is True
        assert t.is_hostname() is False

    def test_target_is_hostname(self):
        t = eggsec.TargetPy(host="example.com")
        assert t.is_ip() is False
        assert t.is_hostname() is True

    def test_target_is_ip_v6(self):
        t = eggsec.TargetPy(host="::1")
        assert t.is_ip() is True

    def test_target_to_dict(self):
        t = eggsec.TargetPy(host="example.com", port=443)
        d = t.to_dict()
        assert d["host"] == "example.com"
        assert d["port"] == 443
        assert d["scheme"] is None
        assert d["url_path"] is None

    def test_target_to_json(self):
        t = eggsec.TargetPy(host="example.com", port=443, scheme="https")
        j = t.to_json()
        parsed = json.loads(j)
        assert parsed["host"] == "example.com"
        assert parsed["port"] == 443
        assert parsed["scheme"] == "https"

    def test_target_repr(self):
        t = eggsec.TargetPy(host="example.com", port=443, scheme="https")
        r = repr(t)
        assert "example.com" in r
        assert "443" in r
        assert "https" in r

    def test_target_str(self):
        t = eggsec.TargetPy(host="example.com", port=443, scheme="https")
        s = str(t)
        assert s == "https://example.com"


class TestConnectionConfig:
    def test_connection_config_defaults(self):
        c = eggsec.ConnectionConfigPy()
        assert c.connect_timeout_ms == 5000
        assert c.read_timeout_ms == 30000
        assert c.write_timeout_ms == 30000
        assert c.handshake_timeout_ms == 10000
        assert c.idle_timeout_ms == 60000
        assert c.max_retries == 0
        assert c.retry_delay_ms == 1000

    def test_connection_config_custom(self):
        c = eggsec.ConnectionConfigPy(
            connect_timeout_ms=1000,
            read_timeout_ms=5000,
            write_timeout_ms=5000,
            handshake_timeout_ms=2000,
            idle_timeout_ms=10000,
            max_retries=3,
            retry_delay_ms=500,
        )
        assert c.connect_timeout_ms == 1000
        assert c.max_retries == 3
        assert c.retry_delay_ms == 500

    def test_connection_config_to_dict(self):
        c = eggsec.ConnectionConfigPy(connect_timeout_ms=1000)
        d = c.to_dict()
        assert d["connect_timeout_ms"] == 1000
        assert d["max_retries"] == 0

    def test_connection_config_to_json(self):
        c = eggsec.ConnectionConfigPy()
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["connect_timeout_ms"] == 5000

    def test_connection_config_repr(self):
        c = eggsec.ConnectionConfigPy()
        r = repr(c)
        assert "connect" in r
        assert "5000" in r

    def test_connection_config_str(self):
        c = eggsec.ConnectionConfigPy()
        s = str(c)
        assert "connect=5000ms" in s


class TestTimeoutConfig:
    def test_timeout_config_defaults(self):
        t = eggsec.TimeoutConfigPy()
        assert t.connect_ms == 5000
        assert t.read_ms == 30000
        assert t.write_ms == 30000
        assert t.handshake_ms == 10000
        assert t.operation_ms == 60000
        assert t.idle_ms == 60000

    def test_timeout_config_custom(self):
        t = eggsec.TimeoutConfigPy(connect_ms=1000, operation_ms=120000)
        assert t.connect_ms == 1000
        assert t.operation_ms == 120000

    def test_timeout_config_to_dict(self):
        t = eggsec.TimeoutConfigPy()
        d = t.to_dict()
        assert d["connect_ms"] == 5000
        assert d["operation_ms"] == 60000

    def test_timeout_config_to_json(self):
        t = eggsec.TimeoutConfigPy()
        j = t.to_json()
        parsed = json.loads(j)
        assert parsed["connect_ms"] == 5000

    def test_timeout_config_repr(self):
        t = eggsec.TimeoutConfigPy()
        r = repr(t)
        assert "TimeoutConfigPy" in r

    def test_timeout_config_str(self):
        t = eggsec.TimeoutConfigPy()
        s = str(t)
        assert "connect=5000ms" in s


class TestRetryPolicy:
    def test_retry_policy_defaults(self):
        r = eggsec.RetryPolicyPy()
        assert r.max_retries == 0
        assert r.delay_ms == 1000
        assert r.backoff_multiplier == 1.0
        assert r.max_delay_ms == 30000
        assert r.retryable_errors == []

    def test_retry_policy_custom(self):
        r = eggsec.RetryPolicyPy(
            max_retries=5,
            delay_ms=2000,
            backoff_multiplier=2.0,
            max_delay_ms=60000,
            retryable_errors=["ConnectionRefused", "Timeout"],
        )
        assert r.max_retries == 5
        assert r.delay_ms == 2000
        assert r.backoff_multiplier == 2.0
        assert r.retryable_errors == ["ConnectionRefused", "Timeout"]

    def test_retry_policy_to_dict(self):
        r = eggsec.RetryPolicyPy(max_retries=3)
        d = r.to_dict()
        assert d["max_retries"] == 3
        assert d["retryable_errors"] == []

    def test_retry_policy_to_json(self):
        r = eggsec.RetryPolicyPy(max_retries=2)
        j = r.to_json()
        parsed = json.loads(j)
        assert parsed["max_retries"] == 2

    def test_retry_policy_repr(self):
        r = eggsec.RetryPolicyPy(max_retries=3)
        rep = repr(r)
        assert "max_retries=3" in rep

    def test_retry_policy_str(self):
        r = eggsec.RetryPolicyPy(max_retries=3, delay_ms=500)
        s = str(r)
        assert "retries=3" in s
        assert "delay=500ms" in s


class TestSocketEndpoint:
    def test_socket_endpoint_construction(self):
        ep = eggsec.SocketEndpointPy(
            address="127.0.0.1", port=8080, address_family="ipv4", is_loopback=True
        )
        assert ep.address == "127.0.0.1"
        assert ep.port == 8080
        assert ep.address_family == "ipv4"
        assert ep.is_loopback is True

    def test_socket_endpoint_ipv6(self):
        ep = eggsec.SocketEndpointPy(
            address="::1", port=443, address_family="ipv6", is_loopback=True
        )
        s = str(ep)
        assert "[::1]:443" == s

    def test_socket_endpoint_ipv4_str(self):
        ep = eggsec.SocketEndpointPy(
            address="10.0.0.1", port=22, address_family="ipv4", is_loopback=False
        )
        s = str(ep)
        assert s == "10.0.0.1:22"

    def test_socket_endpoint_to_dict(self):
        ep = eggsec.SocketEndpointPy(
            address="127.0.0.1", port=80, address_family="ipv4", is_loopback=True
        )
        d = ep.to_dict()
        assert d["address"] == "127.0.0.1"
        assert d["port"] == 80

    def test_socket_endpoint_to_json(self):
        ep = eggsec.SocketEndpointPy(
            address="127.0.0.1", port=80, address_family="ipv4", is_loopback=True
        )
        j = ep.to_json()
        parsed = json.loads(j)
        assert parsed["address"] == "127.0.0.1"

    def test_socket_endpoint_repr(self):
        ep = eggsec.SocketEndpointPy(
            address="127.0.0.1", port=80, address_family="ipv4", is_loopback=True
        )
        r = repr(ep)
        assert "127.0.0.1" in r
        assert "80" in r


class TestConnectionTiming:
    def test_connection_timing_defaults(self):
        t = eggsec.ConnectionTimingPy()
        assert t.dns_resolution_ms is None
        assert t.tcp_connect_ms is None
        assert t.tls_handshake_ms is None
        assert t.first_byte_ms is None
        assert t.total_ms == 0.0
        assert t.connection_reused is False

    def test_connection_timing_custom(self):
        t = eggsec.ConnectionTimingPy(
            dns_resolution_ms=10.5,
            tcp_connect_ms=20.3,
            tls_handshake_ms=30.1,
            first_byte_ms=40.0,
            total_ms=100.9,
            connection_reused=True,
        )
        assert t.dns_resolution_ms == 10.5
        assert t.tcp_connect_ms == 20.3
        assert t.total_ms == 100.9
        assert t.connection_reused is True

    def test_connection_timing_to_dict(self):
        t = eggsec.ConnectionTimingPy(total_ms=50.0)
        d = t.to_dict()
        assert d["total_ms"] == 50.0
        assert d["connection_reused"] is False

    def test_connection_timing_to_json(self):
        t = eggsec.ConnectionTimingPy(total_ms=50.0)
        j = t.to_json()
        parsed = json.loads(j)
        assert parsed["total_ms"] == 50.0

    def test_connection_timing_repr(self):
        t = eggsec.ConnectionTimingPy(total_ms=50.0, connection_reused=True)
        r = repr(t)
        assert "50.0" in r
        assert "reused=True" in r

    def test_connection_timing_str(self):
        t = eggsec.ConnectionTimingPy(total_ms=50.0, dns_resolution_ms=5.0)
        s = str(t)
        assert "total=50.0ms" in s
        assert "dns=5.0ms" in s


class TestNetworkTranscript:
    def test_network_transcript_construction(self):
        nt = eggsec.NetworkTranscriptPy()
        assert len(nt) == 0
        assert nt.total_bytes == 0
        assert nt.truncated is False

    def test_network_transcript_add_entry(self):
        entry = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=1000.0,
            data_type="data",
            size=100,
        )
        nt = eggsec.NetworkTranscriptPy()
        nt2 = nt.add_entry(entry)
        assert len(nt2) == 1
        assert nt2.total_bytes == 100

    def test_network_transcript_iterate(self):
        e1 = eggsec.TranscriptEntryPy(
            sequence=1, direction="sent", timestamp_ms=1.0, data_type="data", size=10
        )
        e2 = eggsec.TranscriptEntryPy(
            sequence=2, direction="received", timestamp_ms=2.0, data_type="data", size=20
        )
        nt = eggsec.NetworkTranscriptPy().add_entry(e1).add_entry(e2)
        entries = nt.entries
        assert len(entries) == 2
        assert entries[0].direction == "sent"
        assert entries[1].direction == "received"

    def test_network_transcript_getitem(self):
        e1 = eggsec.TranscriptEntryPy(
            sequence=1, direction="sent", timestamp_ms=1.0, data_type="data", size=10
        )
        nt = eggsec.NetworkTranscriptPy().add_entry(e1)
        assert nt[0].direction == "sent"

    def test_network_transcript_getitem_out_of_range(self):
        nt = eggsec.NetworkTranscriptPy()
        with pytest.raises(IndexError):
            nt[0]

    def test_network_transcript_to_dict(self):
        e1 = eggsec.TranscriptEntryPy(
            sequence=1, direction="sent", timestamp_ms=1.0, data_type="data", size=10
        )
        nt = eggsec.NetworkTranscriptPy().add_entry(e1)
        d = nt.to_dict()
        assert len(d["entries"]) == 1
        assert d["total_bytes"] == 10

    def test_network_transcript_to_json(self):
        nt = eggsec.NetworkTranscriptPy()
        j = nt.to_json()
        parsed = json.loads(j)
        assert parsed["entries"] == []

    def test_network_transcript_repr(self):
        nt = eggsec.NetworkTranscriptPy()
        r = repr(nt)
        assert "NetworkTranscriptPy" in r

    def test_network_transcript_str(self):
        nt = eggsec.NetworkTranscriptPy()
        s = str(nt)
        assert "entries" in s


# ============================================================================
# Transport Types
# ============================================================================


class TestTcpConfig:
    def test_tcp_config_construction(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        assert c.host == "10.0.0.1"
        assert c.port == 22
        assert c.connect_timeout_ms == 5000
        assert c.read_timeout_ms == 30000
        assert c.write_timeout_ms == 30000
        assert c.idle_timeout_ms == 60000
        assert c.nodelay is True

    def test_tcp_config_custom(self):
        c = eggsec.TcpConfigPy(
            host="10.0.0.1",
            port=22,
            connect_timeout_ms=2000,
            nodelay=False,
        )
        assert c.connect_timeout_ms == 2000
        assert c.nodelay is False

    def test_tcp_config_to_dict(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        d = c.to_dict()
        assert d["host"] == "10.0.0.1"
        assert d["port"] == 22
        assert d["nodelay"] is True

    def test_tcp_config_to_json(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["host"] == "10.0.0.1"

    def test_tcp_config_repr(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        r = repr(c)
        assert "10.0.0.1" in r
        assert "22" in r

    def test_tcp_config_str(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=22)
        s = str(c)
        assert "tcp://10.0.0.1:22" in s


class TestTcpSession:
    def test_tcp_session_context_manager(self):
        c = eggsec.TcpConfigPy(host="127.0.0.1", port=1)
        session = eggsec.TcpSessionPy(config=c)
        with session as s:
            assert s is session
            assert s.is_closed is False
        assert session.is_closed is True

    def test_tcp_session_repr_not_connected(self):
        c = eggsec.TcpConfigPy(host="127.0.0.1", port=1)
        session = eggsec.TcpSessionPy(config=c)
        r = repr(session)
        assert "127.0.0.1" in r
        assert "closed=False" in r

    def test_tcp_session_config_property(self):
        c = eggsec.TcpConfigPy(host="10.0.0.1", port=443)
        session = eggsec.TcpSessionPy(config=c)
        assert session.config.host == "10.0.0.1"
        assert session.config.port == 443


class TestUdpConfig:
    def test_udp_config_construction(self):
        c = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        assert c.host == "10.0.0.1"
        assert c.port == 53
        assert c.timeout_ms == 5000
        assert c.max_datagram_size == 65535
        assert c.bind_address is None

    def test_udp_config_custom(self):
        c = eggsec.UdpConfigPy(
            host="10.0.0.1", port=53, timeout_ms=2000, bind_address="0.0.0.0:12345"
        )
        assert c.timeout_ms == 2000
        assert c.bind_address == "0.0.0.0:12345"

    def test_udp_config_to_dict(self):
        c = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        d = c.to_dict()
        assert d["host"] == "10.0.0.1"
        assert d["port"] == 53

    def test_udp_config_to_json(self):
        c = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["host"] == "10.0.0.1"

    def test_udp_config_repr(self):
        c = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        r = repr(c)
        assert "10.0.0.1" in r
        assert "53" in r

    def test_udp_config_str(self):
        c = eggsec.UdpConfigPy(host="10.0.0.1", port=53)
        s = str(c)
        assert "udp://10.0.0.1:53" in s


# ============================================================================
# Probe Types
# ============================================================================


class TestDnsQueryConfig:
    def test_dns_query_config_construction(self):
        c = eggsec.DnsQueryConfigPy(domain="example.com")
        assert c.domain == "example.com"
        assert c.record_types == ["A", "AAAA"]
        assert c.resolver is None
        assert c.timeout_ms == 5000
        assert c.tcp_fallback is True
        assert c.max_retries == 2

    def test_dns_query_config_custom(self):
        c = eggsec.DnsQueryConfigPy(
            domain="example.com",
            record_types=["MX", "TXT"],
            resolver="8.8.8.8",
            timeout_ms=3000,
        )
        assert c.record_types == ["MX", "TXT"]
        assert c.resolver == "8.8.8.8"
        assert c.timeout_ms == 3000

    def test_dns_query_config_to_dict(self):
        c = eggsec.DnsQueryConfigPy(domain="example.com")
        d = c.to_dict()
        assert d["domain"] == "example.com"
        assert d["record_types"] == ["A", "AAAA"]

    def test_dns_query_config_to_json(self):
        c = eggsec.DnsQueryConfigPy(domain="example.com")
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["domain"] == "example.com"

    def test_dns_query_config_repr(self):
        c = eggsec.DnsQueryConfigPy(domain="example.com")
        r = repr(c)
        assert "example.com" in r

    def test_dns_query_config_str(self):
        c = eggsec.DnsQueryConfigPy(domain="example.com")
        s = str(c)
        assert "example.com" in s


class TestTlsProbeConfig:
    def test_tls_probe_config_defaults(self):
        c = eggsec.TlsProbeConfigPy(host="example.com")
        assert c.host == "example.com"
        assert c.port == 443
        assert c.sni is None
        assert c.alpn == ["http/1.1", "h2"]
        assert c.min_version is None
        assert c.max_version is None
        assert c.verify_certificate is True
        assert c.timeout_ms == 10000
        assert c.include_chain is True

    def test_tls_probe_config_custom(self):
        c = eggsec.TlsProbeConfigPy(
            host="example.com",
            port=8443,
            sni="custom.example.com",
            verify_certificate=False,
        )
        assert c.port == 8443
        assert c.sni == "custom.example.com"
        assert c.verify_certificate is False

    def test_tls_probe_config_to_dict(self):
        c = eggsec.TlsProbeConfigPy(host="example.com")
        d = c.to_dict()
        assert d["host"] == "example.com"
        assert d["port"] == 443

    def test_tls_probe_config_to_json(self):
        c = eggsec.TlsProbeConfigPy(host="example.com")
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["host"] == "example.com"

    def test_tls_probe_config_repr(self):
        c = eggsec.TlsProbeConfigPy(host="example.com")
        r = repr(c)
        assert "example.com" in r

    def test_tls_probe_config_str(self):
        c = eggsec.TlsProbeConfigPy(host="example.com")
        s = str(c)
        assert "TLS probe" in s


class TestHttpProbeConfig:
    def test_http_probe_config_defaults(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com")
        assert c.url == "https://example.com"
        assert c.method == "GET"
        assert c.headers == []
        assert c.body is None
        assert c.follow_redirects is True
        assert c.max_redirects == 10
        assert c.verify_tls is True
        assert c.timeout_ms == 10000
        assert c.user_agent is None

    def test_http_probe_config_method_uppercased(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com", method="post")
        assert c.method == "POST"

    def test_http_probe_config_to_dict(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com")
        d = c.to_dict()
        assert d["url"] == "https://example.com"
        assert d["method"] == "GET"

    def test_http_probe_config_to_json(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com")
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "https://example.com"

    def test_http_probe_config_repr(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com")
        r = repr(c)
        assert "https://example.com" in r

    def test_http_probe_config_str(self):
        c = eggsec.HttpProbeConfigPy(url="https://example.com")
        s = str(c)
        assert "HTTP GET" in s


# ============================================================================
# HTTP Client Types
# ============================================================================


class TestHttpRequest:
    def test_http_request_construction(self):
        r = eggsec.HttpRequestPy(method="GET", url="https://example.com/api")
        assert r.method == "GET"
        assert r.url == "https://example.com/api"
        assert r.follow_redirects is True
        assert r.max_redirects == 10
        assert r.verify_tls is True
        assert r.timeout_ms == 30000
        assert r.connect_timeout_ms == 5000
        assert r.user_agent is None
        assert r.proxy_url is None

    def test_http_request_with_body(self):
        r = eggsec.HttpRequestPy(
            method="POST",
            url="https://example.com/api",
            body_text='{"key": "value"}',
        )
        assert r.body_text == '{"key": "value"}'

    def test_http_request_to_dict(self):
        r = eggsec.HttpRequestPy(method="GET", url="https://example.com")
        d = r.to_dict()
        assert d["method"] == "GET"
        assert d["url"] == "https://example.com"

    def test_http_request_to_json(self):
        r = eggsec.HttpRequestPy(method="GET", url="https://example.com")
        j = r.to_json()
        parsed = json.loads(j)
        assert parsed["method"] == "GET"

    def test_http_request_repr(self):
        r = eggsec.HttpRequestPy(method="GET", url="https://example.com")
        rep = repr(r)
        assert "GET" in rep
        assert "example.com" in rep

    def test_http_request_str(self):
        r = eggsec.HttpRequestPy(method="GET", url="https://example.com")
        s = str(r)
        assert s == "GET https://example.com"


class TestHttpHeaders:
    def test_http_headers_construction(self):
        h = eggsec.HttpHeadersPy()
        assert len(h) == 0
        assert bool(h) is False

    def test_http_headers_with_entries(self):
        h = eggsec.HttpHeadersPy(entries=[("Content-Type", "application/json"), ("Accept", "*/*")])
        assert len(h) == 2
        assert bool(h) is True

    def test_http_headers_get(self):
        h = eggsec.HttpHeadersPy(entries=[("Content-Type", "application/json")])
        assert h.get("content-type") == "application/json"
        assert h.get("CONTENT-TYPE") == "application/json"
        assert h.get("missing") is None

    def test_http_headers_get_all(self):
        h = eggsec.HttpHeadersPy(
            entries=[("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"), ("Content-Type", "text/html")]
        )
        cookies = h.get_all("set-cookie")
        assert len(cookies) == 2
        assert "a=1" in cookies
        assert "b=2" in cookies

    def test_http_headers_contains(self):
        h = eggsec.HttpHeadersPy(entries=[("X-Custom", "value")])
        assert h.contains("x-custom") is True
        assert h.contains("X-CUSTOM") is True
        assert h.contains("missing") is False

    def test_http_headers_names(self):
        h = eggsec.HttpHeadersPy(entries=[("A", "1"), ("B", "2"), ("a", "3")])
        names = h.names()
        assert names == ["A", "B"]

    def test_http_headers_to_dict(self):
        h = eggsec.HttpHeadersPy(entries=[("Key", "Value")])
        d = h.to_dict()
        assert d["len"] == 1

    def test_http_headers_to_json(self):
        h = eggsec.HttpHeadersPy(entries=[("Key", "Value")])
        j = h.to_json()
        parsed = json.loads(j)
        assert "entries" in parsed

    def test_http_headers_repr(self):
        h = eggsec.HttpHeadersPy(entries=[("Key", "Value")])
        r = repr(h)
        assert "HttpHeadersPy" in r

    def test_http_headers_str(self):
        h = eggsec.HttpHeadersPy(entries=[("Key", "Value")])
        s = str(h)
        assert "Key: Value" in s


class TestHttpClientConfig:
    def test_http_client_config_defaults(self):
        c = eggsec.HttpClientConfigPy()
        assert c.base_url is None
        assert c.timeout_ms == 30000
        assert c.connect_timeout_ms == 5000
        assert c.max_redirects == 10
        assert c.verify_tls is True
        assert c.proxy_url is None
        assert c.user_agent is None
        assert c.cookie_store is True
        assert c.pool_idle_timeout_ms == 90000
        assert c.pool_max_idle_per_host == 10

    def test_http_client_config_custom(self):
        c = eggsec.HttpClientConfigPy(
            base_url="https://api.example.com",
            timeout_ms=5000,
            user_agent="test/1.0",
        )
        assert c.base_url == "https://api.example.com"
        assert c.timeout_ms == 5000
        assert c.user_agent == "test/1.0"

    def test_http_client_config_to_dict(self):
        c = eggsec.HttpClientConfigPy()
        d = c.to_dict()
        assert d["timeout_ms"] == 30000
        assert d["verify_tls"] is True

    def test_http_client_config_to_json(self):
        c = eggsec.HttpClientConfigPy()
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["timeout_ms"] == 30000

    def test_http_client_config_repr(self):
        c = eggsec.HttpClientConfigPy()
        r = repr(c)
        assert "HttpClientConfigPy" in r

    def test_http_client_config_str(self):
        c = eggsec.HttpClientConfigPy()
        s = str(c)
        assert "timeout=30000ms" in s


class TestRedactConfig:
    def test_redact_config_defaults(self):
        r = eggsec.RedactConfigPy()
        assert "Authorization" in r.redact_headers
        assert "Cookie" in r.redact_headers
        assert "Proxy-Authorization" in r.redact_headers
        assert "X-API-Key" in r.redact_headers
        assert r.redact_query_params == []
        assert r.redact_body_fields == []

    def test_redact_config_custom(self):
        r = eggsec.RedactConfigPy(
            redact_headers=["X-Auth-Token"],
            redact_query_params=["token"],
            redact_body_fields=["password"],
        )
        assert r.redact_headers == ["X-Auth-Token"]
        assert r.redact_query_params == ["token"]
        assert r.redact_body_fields == ["password"]

    def test_redact_config_to_dict(self):
        r = eggsec.RedactConfigPy()
        d = r.to_dict()
        assert len(d["redact_headers"]) == 4

    def test_redact_config_to_json(self):
        r = eggsec.RedactConfigPy()
        j = r.to_json()
        parsed = json.loads(j)
        assert "Authorization" in parsed["redact_headers"]

    def test_redact_config_repr(self):
        r = eggsec.RedactConfigPy()
        rep = repr(r)
        assert "RedactConfigPy" in rep

    def test_redact_config_str(self):
        r = eggsec.RedactConfigPy()
        s = str(r)
        assert "Authorization" in s


# ============================================================================
# WebSocket Types (feature-gated)
# ============================================================================

_has_websocket = hasattr(eggsec, "WebSocketSessionConfigPy")


class TestWebSocketSessionConfig:
    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_construction(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        assert c.url == "ws://localhost:8080"
        assert c.origin is None
        assert c.timeout_ms == 10000
        assert c.max_message_size == 1048576
        assert c.ping_interval_ms == 30000
        assert c.close_timeout_ms == 5000
        assert c.verify_tls is True
        assert c.subprotocols == []

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_custom(self):
        c = eggsec.WebSocketSessionConfigPy(
            url="wss://echo.websocket.org",
            origin="https://example.com",
            subprotocols=["graphql-ws"],
            timeout_ms=5000,
            verify_tls=False,
        )
        assert c.url == "wss://echo.websocket.org"
        assert c.origin == "https://example.com"
        assert c.subprotocols == ["graphql-ws"]
        assert c.timeout_ms == 5000
        assert c.verify_tls is False

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_empty_url_raises(self):
        with pytest.raises(ValueError, match="url must not be empty"):
            eggsec.WebSocketSessionConfigPy(url="")

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_invalid_scheme_raises(self):
        with pytest.raises(ValueError, match="url must start with ws:// or wss://"):
            eggsec.WebSocketSessionConfigPy(url="http://example.com")

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_to_dict(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        d = c.to_dict()
        assert d["url"] == "ws://localhost:8080"
        assert d["timeout_ms"] == 10000

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_to_json(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        j = c.to_json()
        parsed = json.loads(j)
        assert parsed["url"] == "ws://localhost:8080"

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_repr(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        r = repr(c)
        assert "ws://localhost:8080" in r

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_config_str(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://localhost:8080")
        s = str(c)
        assert "ws://localhost:8080" in s


class TestWebSocketMessage:
    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_text(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello world", is_text=True, is_binary=False,
            text_content="hello world", size=11,
        )
        assert msg.is_text is True
        assert msg.is_binary is False
        assert msg.text_content == "hello world"
        assert msg.size == 11
        assert msg.to_text() == "hello world"
        assert msg.to_bytes() == b"hello world"

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_binary(self):
        data = bytes([0x00, 0xFF, 0x42])
        msg = eggsec.WebSocketMessagePy(
            data=data, is_text=False, is_binary=True,
            text_content=None, size=3,
        )
        assert msg.is_text is False
        assert msg.is_binary is True
        assert msg.text_content is None
        assert msg.to_bytes() == data

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_to_dict(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"test", is_text=True, is_binary=False,
            text_content="test", size=4,
        )
        d = msg.to_dict()
        assert d["is_text"] is True
        assert d["size"] == 4

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_to_json(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"test", is_text=True, is_binary=False,
            text_content="test", size=4,
        )
        j = msg.to_json()
        parsed = json.loads(j)
        assert parsed["is_text"] is True

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_repr(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"test", is_text=True, is_binary=False,
            text_content="test", size=4,
        )
        r = repr(msg)
        assert "text" in r
        assert "4" in r

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_str_text(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"hello", is_text=True, is_binary=False,
            text_content="hello", size=5,
        )
        s = str(msg)
        assert s == "hello"

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_message_str_binary(self):
        msg = eggsec.WebSocketMessagePy(
            data=b"\x00\x01", is_text=False, is_binary=True,
            text_content=None, size=2,
        )
        s = str(msg)
        assert "binary" in s


class TestWebSocketCloseInfo:
    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_close_info_construction(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1000, reason="normal close", was_clean=True)
        assert ci.code == 1000
        assert ci.reason == "normal close"
        assert ci.was_clean is True

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_close_info_to_dict(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1001, reason="going away", was_clean=False)
        d = ci.to_dict()
        assert d["code"] == 1001
        assert d["was_clean"] is False

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_close_info_to_json(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1000, reason="ok", was_clean=True)
        j = ci.to_json()
        parsed = json.loads(j)
        assert parsed["code"] == 1000

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_close_info_repr(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1000, reason="ok", was_clean=True)
        r = repr(ci)
        assert "1000" in r
        assert "was_clean=True" in r

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_close_info_str(self):
        ci = eggsec.WebSocketCloseInfoPy(code=1000, reason="ok", was_clean=True)
        s = str(ci)
        assert "1000" in s
        assert "clean" in s


class TestWebSocketSessionContextManager:
    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_context_manager_enter_exit(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        session = eggsec.WebSocketSessionPy(config=c)
        with session as s:
            assert s is session
            assert s.is_closed is False
        assert session.is_closed is True

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_repr(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        session = eggsec.WebSocketSessionPy(config=c)
        r = repr(session)
        assert "ws://127.0.0.1:1" in r
        assert "closed=False" in r

    @pytest.mark.skipif(not _has_websocket, reason="websocket feature not enabled")
    def test_websocket_session_url_property(self):
        c = eggsec.WebSocketSessionConfigPy(url="ws://127.0.0.1:1")
        session = eggsec.WebSocketSessionPy(config=c)
        assert session.url == "ws://127.0.0.1:1"


# ============================================================================
# API Surface
# ============================================================================


class TestApiSurface:
    def test_api_surface_includes_release_2(self):
        surface = eggsec.api_surface()
        # Network types
        assert "TargetPy" in surface
        assert "ConnectionConfigPy" in surface
        assert "TimeoutConfigPy" in surface
        assert "RetryPolicyPy" in surface
        assert "SocketEndpointPy" in surface
        assert "ConnectionTimingPy" in surface
        # Transport types
        assert "TcpConfigPy" in surface
        assert "TcpSessionPy" in surface
        assert "UdpConfigPy" in surface
        assert "UdpSocketPy" in surface
        # Probe types
        assert "DnsQueryConfigPy" in surface
        assert "TlsProbeConfigPy" in surface
        assert "HttpProbeConfigPy" in surface
        # HTTP client types
        assert "HttpRequestPy" in surface
        assert "HttpResponsePy" in surface
        assert "HttpClientConfigPy" in surface
        assert "HttpHeadersPy" in surface
        assert "RedactConfigPy" in surface
        # Probe functions
        assert "dns_query" in surface
        assert "tls_probe" in surface
        assert "http_probe" in surface
        # Verify stability is provisional for Release 2 types
        assert surface["TargetPy"]["stability"] == "provisional"
        assert surface["TcpConfigPy"]["stability"] == "provisional"
        assert surface["dns_query"]["stability"] == "provisional"

    def test_api_surface_websocket_types(self):
        surface = eggsec.api_surface()
        if _has_websocket:
            assert "WebSocketSessionConfigPy" in surface
            assert "WebSocketMessagePy" in surface
            assert "WebSocketCloseInfoPy" in surface
            assert "WebSocketSessionPy" in surface
