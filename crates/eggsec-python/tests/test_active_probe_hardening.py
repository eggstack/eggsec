"""Active probe hardening tests.

Covers DNS, TLS, HTTP, and UDP probe configuration types, network config types,
target construction, network transcript construction, serialization roundtrips,
scope enforcement on probe dispatch, and repr/str output.
"""

import json
import os
import pytest

import eggsec

SENTINEL_LOOPBACK = "127.0.0.1"
LOOPBACK_ALLOWED = os.environ.get("EGGSEC_ALLOW_LOOPBACK_FIXTURE", "0") == "1"


# ============================================================================
# 1-5: Probe config construction
# ============================================================================


class TestProbeConfigConstruction:
    def test_dns_query_config_construction(self):
        cfg = eggsec.DnsQueryConfigPy(
            domain="example.com",
            record_types=["A", "AAAA", "MX"],
            resolver="8.8.8.8",
            timeout_ms=8000,
            tcp_fallback=False,
            max_retries=5,
        )
        assert cfg.domain == "example.com"
        assert cfg.record_types == ["A", "AAAA", "MX"]
        assert cfg.resolver == "8.8.8.8"
        assert cfg.timeout_ms == 8000
        assert cfg.tcp_fallback is False
        assert cfg.max_retries == 5

    def test_dns_query_config_defaults(self):
        cfg = eggsec.DnsQueryConfigPy(domain="test.local")
        assert cfg.domain == "test.local"
        assert cfg.record_types == ["A", "AAAA"]
        assert cfg.resolver is None
        assert cfg.timeout_ms == 5000
        assert cfg.tcp_fallback is True
        assert cfg.max_retries == 2

    def test_tls_probe_config_construction(self):
        cfg = eggsec.TlsProbeConfigPy(
            host="example.com",
            port=8443,
            sni="custom.example.com",
            alpn=["h2", "http/1.1"],
            min_version="TLSv1.2",
            max_version="TLSv1.3",
            verify_certificate=False,
            timeout_ms=15000,
            include_chain=False,
        )
        assert cfg.host == "example.com"
        assert cfg.port == 8443
        assert cfg.sni == "custom.example.com"
        assert cfg.alpn == ["h2", "http/1.1"]
        assert cfg.min_version == "TLSv1.2"
        assert cfg.max_version == "TLSv1.3"
        assert cfg.verify_certificate is False
        assert cfg.timeout_ms == 15000
        assert cfg.include_chain is False

    def test_tls_probe_config_defaults(self):
        cfg = eggsec.TlsProbeConfigPy(host="secure.example.com")
        assert cfg.host == "secure.example.com"
        assert cfg.port == 443
        assert cfg.sni is None
        assert cfg.verify_certificate is True
        assert cfg.timeout_ms == 10000
        assert cfg.include_chain is True

    def test_http_probe_config_construction(self):
        cfg = eggsec.HttpProbeConfigPy(
            url="https://example.com/api/v1",
            method="POST",
            headers=[("Authorization", "Bearer tok"), ("X-Custom", "val")],
            body='{"key":"value"}',
            follow_redirects=False,
            max_redirects=3,
            verify_tls=False,
            timeout_ms=20000,
            user_agent="test-agent/1.0",
        )
        assert cfg.url == "https://example.com/api/v1"
        assert cfg.method == "POST"
        assert cfg.headers == [("Authorization", "Bearer tok"), ("X-Custom", "val")]
        assert cfg.body == '{"key":"value"}'
        assert cfg.follow_redirects is False
        assert cfg.max_redirects == 3
        assert cfg.verify_tls is False
        assert cfg.timeout_ms == 20000
        assert cfg.user_agent == "test-agent/1.0"

    def test_http_probe_config_defaults(self):
        cfg = eggsec.HttpProbeConfigPy(url="https://example.com")
        assert cfg.url == "https://example.com"
        assert cfg.method == "GET"
        assert cfg.headers == []
        assert cfg.body is None
        assert cfg.follow_redirects is True
        assert cfg.max_redirects == 10
        assert cfg.verify_tls is True
        assert cfg.timeout_ms == 10000
        assert cfg.user_agent is None

    def test_udp_reachability_construction(self):
        cfg = eggsec.UdpReachabilityConfigPy(
            host="10.0.0.1",
            port=5353,
            attempts=5,
            timeout_ms=3000,
            payload=b"\x00\x01\x02",
        )
        assert cfg.host == "10.0.0.1"
        assert cfg.port == 5353
        assert cfg.attempts == 5
        assert cfg.timeout_ms == 3000
        assert list(cfg.payload) == [0, 1, 2]

    def test_udp_probe_config_construction(self):
        cfg = eggsec.UdpProbeConfigPy(
            host="10.0.0.1",
            port=12345,
            payload=b"\xde\xad",
            timeout_ms=7000,
            max_response_size=32768,
            retries=4,
        )
        assert cfg.host == "10.0.0.1"
        assert cfg.port == 12345
        assert list(cfg.payload) == [222, 173]
        assert cfg.timeout_ms == 7000
        assert cfg.max_response_size == 32768
        assert cfg.retries == 4

    def test_udp_probe_config_defaults(self):
        cfg = eggsec.UdpProbeConfigPy(host="10.0.0.1", port=53)
        assert cfg.host == "10.0.0.1"
        assert cfg.port == 53
        assert cfg.timeout_ms == 5000
        assert cfg.max_response_size == 65535
        assert cfg.retries == 2


# ============================================================================
# 6-7: Serialization roundtrips
# ============================================================================


class TestProbeConfigSerialization:
    def test_probe_config_to_dict_roundtrip(self):
        configs = [
            eggsec.DnsQueryConfigPy(domain="a.com", record_types=["MX"]),
            eggsec.TlsProbeConfigPy(host="b.com", port=993),
            eggsec.HttpProbeConfigPy(url="https://c.com/path", method="HEAD"),
            eggsec.UdpProbeConfigPy(host="d.com", port=53),
            eggsec.UdpReachabilityConfigPy(host="e.com", port=1234),
        ]
        for cfg in configs:
            d = cfg.to_dict()
            assert isinstance(d, dict)
            assert len(d) > 0

    def test_probe_config_to_json_roundtrip(self):
        configs = [
            eggsec.DnsQueryConfigPy(domain="a.com"),
            eggsec.TlsProbeConfigPy(host="b.com"),
            eggsec.HttpProbeConfigPy(url="https://c.com"),
            eggsec.UdpProbeConfigPy(host="d.com", port=53),
            eggsec.UdpReachabilityConfigPy(host="e.com", port=1234),
        ]
        for cfg in configs:
            j = cfg.to_json()
            parsed = json.loads(j)
            assert isinstance(parsed, dict)
            assert len(parsed) > 0


# ============================================================================
# 8: Repr
# ============================================================================


class TestProbeConfigRepr:
    def test_probe_config_repr(self):
        configs = [
            ("DnsQueryConfigPy", eggsec.DnsQueryConfigPy(domain="test.com")),
            ("TlsProbeConfigPy", eggsec.TlsProbeConfigPy(host="test.com")),
            ("HttpProbeConfigPy", eggsec.HttpProbeConfigPy(url="https://test.com")),
            ("UdpProbeConfigPy", eggsec.UdpProbeConfigPy(host="test.com", port=53)),
            ("UdpReachabilityConfigPy", eggsec.UdpReachabilityConfigPy(host="test.com", port=53)),
        ]
        for name, cfg in configs:
            r = repr(cfg)
            assert isinstance(r, str)
            assert len(r) > 0


# ============================================================================
# 9: Scope enforcement on probe dispatch
# ============================================================================


class TestProbeScopeEnforcement:
    def test_probe_scope_enforcement(self):
        """Engine with deny_all scope blocks probe operations (returns failed result)."""
        scope = eggsec.Scope.deny_all()
        engine = eggsec.Engine(scope, timeout_ms=2000)
        req = eggsec.OperationRequest("scan_ports", SENTINEL_LOOPBACK, timeout_ms=2000)
        result = engine.run(req)
        assert result.is_failure() is True
        assert result.error is not None
        assert "scope" in str(result.error).lower() or "enforcement" in str(result.error).lower()
        engine.close()


# ============================================================================
# 10-13: Network config types
# ============================================================================


class TestNetworkConfigTypes:
    def test_probe_timeout_config(self):
        cfg = eggsec.TimeoutConfigPy(
            connect_ms=3000,
            read_ms=15000,
            write_ms=15000,
            handshake_ms=5000,
            operation_ms=30000,
            idle_ms=30000,
        )
        assert cfg.connect_ms == 3000
        assert cfg.read_ms == 15000
        assert cfg.write_ms == 15000
        assert cfg.handshake_ms == 5000
        assert cfg.operation_ms == 30000
        assert cfg.idle_ms == 30000

    def test_probe_timeout_config_defaults(self):
        cfg = eggsec.TimeoutConfigPy()
        assert cfg.connect_ms == 5000
        assert cfg.read_ms == 30000
        assert cfg.write_ms == 30000
        assert cfg.handshake_ms == 10000
        assert cfg.operation_ms == 60000
        assert cfg.idle_ms == 60000

    def test_probe_retry_policy(self):
        cfg = eggsec.RetryPolicyPy(
            max_retries=5,
            delay_ms=2000,
            backoff_multiplier=2.0,
            max_delay_ms=60000,
            retryable_errors=["timeout", "connection_refused"],
        )
        assert cfg.max_retries == 5
        assert cfg.delay_ms == 2000
        assert cfg.backoff_multiplier == 2.0
        assert cfg.max_delay_ms == 60000
        assert cfg.retryable_errors == ["timeout", "connection_refused"]

    def test_probe_retry_policy_defaults(self):
        cfg = eggsec.RetryPolicyPy()
        assert cfg.max_retries == 0
        assert cfg.delay_ms == 1000
        assert cfg.backoff_multiplier == 1.0
        assert cfg.max_delay_ms == 30000
        assert cfg.retryable_errors == []

    def test_probe_connection_config(self):
        cfg = eggsec.ConnectionConfigPy(
            connect_timeout_ms=3000,
            read_timeout_ms=15000,
            write_timeout_ms=15000,
            handshake_timeout_ms=5000,
            idle_timeout_ms=30000,
            max_retries=3,
            retry_delay_ms=500,
        )
        assert cfg.connect_timeout_ms == 3000
        assert cfg.read_timeout_ms == 15000
        assert cfg.write_timeout_ms == 15000
        assert cfg.handshake_timeout_ms == 5000
        assert cfg.idle_timeout_ms == 30000
        assert cfg.max_retries == 3
        assert cfg.retry_delay_ms == 500

    def test_probe_connection_config_defaults(self):
        cfg = eggsec.ConnectionConfigPy()
        assert cfg.connect_timeout_ms == 5000
        assert cfg.read_timeout_ms == 30000
        assert cfg.write_timeout_ms == 30000
        assert cfg.handshake_timeout_ms == 10000
        assert cfg.idle_timeout_ms == 60000
        assert cfg.max_retries == 0
        assert cfg.retry_delay_ms == 1000

    def test_probe_target_construction(self):
        t = eggsec.TargetPy(host="192.168.1.1", port=8080, scheme="http", url_path="/api")
        assert t.host == "192.168.1.1"
        assert t.port == 8080
        assert t.scheme == "http"
        assert t.url_path == "/api"

    def test_probe_target_types(self):
        ip_target = eggsec.TargetPy(host="10.0.0.1")
        assert ip_target.is_ip() is True
        assert ip_target.is_hostname() is False

        hostname_target = eggsec.TargetPy(host="example.com")
        assert hostname_target.is_ip() is False
        assert hostname_target.is_hostname() is True

        url_target = eggsec.TargetPy(host="example.com", port=443, scheme="https", url_path="/v1/data")
        assert url_target.is_hostname() is True
        normalized = url_target.normalized()
        assert "example.com" in normalized
        assert "https" in normalized


# ============================================================================
# 15-16: Network transcript construction and redaction
# ============================================================================


class TestNetworkTranscript:
    def test_network_transcript_construction(self):
        entry1 = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=1000.0,
            data_type="request",
            size=128,
            summary="GET / HTTP/1.1",
            redacted=False,
        )
        entry2 = eggsec.TranscriptEntryPy(
            sequence=2,
            direction="received",
            timestamp_ms=1050.0,
            data_type="response",
            size=2048,
            summary="200 OK",
            redacted=False,
        )
        t = eggsec.NetworkTranscriptPy(entries=[entry1, entry2], total_bytes=2176, truncated=False)
        assert len(t) == 2
        assert t.total_bytes == 2176
        assert t.truncated is False
        assert t[0].direction == "sent"
        assert t[1].direction == "received"

    def test_network_transcript_add_entry(self):
        t = eggsec.NetworkTranscriptPy()
        entry = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=0.0,
            data_type="data",
            size=64,
        )
        t2 = t.add_entry(entry)
        assert len(t2) == 1
        assert t2.total_bytes == 64

    def test_network_transcript_summary(self):
        entry = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=0.0,
            data_type="request",
            size=100,
        )
        t = eggsec.NetworkTranscriptPy(entries=[entry], total_bytes=100)
        s = t.summary()
        assert "1" in s
        assert "sent" in s

    def test_network_transcript_redaction(self):
        entry_redacted = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=0.0,
            data_type="auth",
            size=32,
            summary="[REDACTED]",
            redacted=True,
        )
        entry_clear = eggsec.TranscriptEntryPy(
            sequence=2,
            direction="received",
            timestamp_ms=1.0,
            data_type="response",
            size=64,
            summary="200 OK",
            redacted=False,
        )
        t = eggsec.NetworkTranscriptPy(entries=[entry_redacted, entry_clear], total_bytes=96)
        assert t[0].redacted is True
        assert t[0].summary == "[REDACTED]"
        assert t[1].redacted is False
        assert t[1].summary == "200 OK"

    def test_network_transcript_to_dict(self):
        entry = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=0.0,
            data_type="data",
            size=10,
        )
        t = eggsec.NetworkTranscriptPy(entries=[entry], total_bytes=10)
        d = t.to_dict()
        assert isinstance(d, dict)
        assert "entries" in d
        assert "total_bytes" in d
        assert "truncated" in d

    def test_network_transcript_to_json(self):
        entry = eggsec.TranscriptEntryPy(
            sequence=1,
            direction="sent",
            timestamp_ms=0.0,
            data_type="data",
            size=10,
        )
        t = eggsec.NetworkTranscriptPy(entries=[entry], total_bytes=10)
        j = t.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)
