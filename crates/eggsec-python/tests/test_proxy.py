"""Tests for eggsec Python bindings - Interception Proxy types (Release 3)."""

import pytest
import importlib

pytestmark = [pytest.mark.timeout(60)]

# Feature-gate: skip entire module if web-proxy feature not compiled
_mod = importlib.import_module("eggsec")
_PROXY_AVAILABLE = getattr(_mod, "ProxyType", None) is not None

if not _PROXY_AVAILABLE:
    pytest.skip("web-proxy feature not compiled", allow_module_level=True)


def _import_or_skip(name, feature="web-proxy"):
    """Import a name from eggsec, skip test if unavailable (feature-gated)."""
    obj = getattr(_mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


# Eagerly import types for backward compatibility with existing test code
ProxyType = _mod.ProxyType
RotationStrategy = _mod.RotationStrategy
ProxyConfig = _mod.ProxyConfig
ProxyEntry = _mod.ProxyEntry
InterceptConfig = _mod.InterceptConfig
CapturedExchange = _mod.CapturedExchange
InterceptSessionResult = _mod.InterceptSessionResult
InterceptSessionState = _mod.InterceptSessionState
InterceptStats = _mod.InterceptStats
InterceptFilter = _mod.InterceptFilter
InterceptRule = _mod.InterceptRule
CertificateAuthorityConfig = _mod.CertificateAuthorityConfig
IssuedCertificate = _mod.IssuedCertificate
HarEntry = _mod.HarEntry
HarDocument = _mod.HarDocument


# ---------------------------------------------------------------------------
# ProxyType enum
# ---------------------------------------------------------------------------

class TestProxyType:
    def test_from_str_valid(self):
        assert ProxyType.from_str("socks4") == ProxyType.Socks4
        assert ProxyType.from_str("socks5") == ProxyType.Socks5
        assert ProxyType.from_str("http") == ProxyType.Http
        assert ProxyType.from_str("https") == ProxyType.Https
        assert ProxyType.from_str("tor") == ProxyType.Tor

    def test_from_str_case_insensitive(self):
        assert ProxyType.from_str("SOCKS4") == ProxyType.Socks4
        assert ProxyType.from_str("Socks5") == ProxyType.Socks5

    def test_from_str_invalid(self):
        with pytest.raises(ValueError, match="Invalid proxy type"):
            ProxyType.from_str("invalid")

    def test_repr(self):
        assert repr(ProxyType.Http) == "ProxyType.http"

    def test_str(self):
        assert str(ProxyType.Http) == "http"


# ---------------------------------------------------------------------------
# RotationStrategy enum
# ---------------------------------------------------------------------------

class TestRotationStrategy:
    def test_from_str_valid(self):
        assert RotationStrategy.from_str("round_robin") == RotationStrategy.RoundRobin
        assert RotationStrategy.from_str("random") == RotationStrategy.Random
        assert RotationStrategy.from_str("weighted") == RotationStrategy.Weighted
        assert RotationStrategy.from_str("least_used") == RotationStrategy.LeastUsed
        assert RotationStrategy.from_str("lowest_latency") == RotationStrategy.LowestLatency

    def test_from_str_invalid(self):
        with pytest.raises(ValueError, match="Invalid rotation strategy"):
            RotationStrategy.from_str("invalid")

    def test_repr(self):
        assert repr(RotationStrategy.RoundRobin) == "RotationStrategy.round_robin"


# ---------------------------------------------------------------------------
# ProxyConfig
# ---------------------------------------------------------------------------

class TestProxyConfig:
    def test_default(self):
        config = ProxyConfig()
        assert config.rotation_strategy == RotationStrategy.RoundRobin
        assert config.health_check_enabled is True
        assert config.health_check_interval_secs == 60
        assert config.health_check_timeout_ms == 5000
        assert config.max_failures_before_disable == 3
        assert config.chain_proxies is False
        assert config.max_chain_length == 3

    def test_custom(self):
        config = ProxyConfig(
            rotation_strategy=RotationStrategy.Random,
            health_check_enabled=False,
            max_failures_before_disable=5,
        )
        assert config.rotation_strategy == RotationStrategy.Random
        assert config.health_check_enabled is False
        assert config.max_failures_before_disable == 5

    def test_to_dict(self):
        config = ProxyConfig()
        d = config.to_dict()
        assert isinstance(d, dict)
        assert d["rotation_strategy"] == "round_robin"
        assert d["health_check_enabled"] is True

    def test_to_json(self):
        config = ProxyConfig()
        j = config.to_json()
        assert isinstance(j, str)
        assert "round_robin" in j

    def test_repr(self):
        config = ProxyConfig()
        r = repr(config)
        assert "ProxyConfig" in r


# ---------------------------------------------------------------------------
# ProxyEntry
# ---------------------------------------------------------------------------

class TestProxyEntry:
    def test_basic(self):
        entry = ProxyEntry(ProxyType.Http, "127.0.0.1", 8080)
        assert entry.proxy_type == ProxyType.Http
        assert entry.address == "127.0.0.1"
        assert entry.port == 8080
        assert entry.weight == 1
        assert entry.enabled is True
        assert entry.tags == []

    def test_with_auth(self):
        entry = ProxyEntry(
            ProxyType.Https, "proxy.example.com", 443,
            name="my-proxy", username="user", password="pass",
        )
        assert entry.username == "user"
        assert entry.password == "pass"
        assert entry.name == "my-proxy"

    def test_to_dict(self):
        entry = ProxyEntry(ProxyType.Http, "127.0.0.1", 8080)
        d = entry.to_dict()
        assert d["proxy_type"] == "http"
        assert d["address"] == "127.0.0.1"
        assert d["port"] == 8080

    def test_repr_masks_password(self):
        entry = ProxyEntry(
            ProxyType.Http, "127.0.0.1", 8080,
            username="admin", password="secret",
        )
        r = repr(entry)
        assert "secret" not in r
        assert "admin" in r


# ---------------------------------------------------------------------------
# InterceptConfig
# ---------------------------------------------------------------------------

class TestInterceptConfig:
    def test_default(self):
        config = InterceptConfig()
        assert config.listen_addr == "127.0.0.1"
        assert config.listen_port == 8080
        assert config.ssl_intercept is False
        assert config.verbose is False
        assert config.max_flows == 1000
        assert config.timeout_secs == 300
        assert config.modify_request is False
        assert config.modify_response is False

    def test_custom(self):
        config = InterceptConfig(
            listen_addr="0.0.0.0",
            listen_port=9090,
            ssl_intercept=True,
            timeout_secs=60,
        )
        assert config.listen_addr == "0.0.0.0"
        assert config.listen_port == 9090
        assert config.ssl_intercept is True
        assert config.timeout_secs == 60

    def test_to_dict(self):
        config = InterceptConfig()
        d = config.to_dict()
        assert d["listen_addr"] == "127.0.0.1"
        assert d["listen_port"] == 8080

    def test_to_json(self):
        config = InterceptConfig()
        j = config.to_json()
        assert "127.0.0.1" in j

    def test_repr(self):
        config = InterceptConfig()
        r = repr(config)
        assert "InterceptConfig" in r
        assert "8080" in r


# ---------------------------------------------------------------------------
# CapturedExchange (no Python constructor — result type from Rust)
# ---------------------------------------------------------------------------

class TestCapturedExchange:
    @pytest.mark.skip(reason="CapturedExchange has no Python constructor")
    def test_fields(self):
        pass

    @pytest.mark.skip(reason="CapturedExchange has no Python constructor")
    def test_to_dict(self):
        pass

    @pytest.mark.skip(reason="CapturedExchange has no Python constructor")
    def test_str(self):
        pass


# ---------------------------------------------------------------------------
# InterceptSessionResult (no Python constructor — result type from Rust)
# ---------------------------------------------------------------------------

class TestInterceptSessionResult:
    @pytest.mark.skip(reason="InterceptSessionResult has no Python constructor")
    def test_fields(self):
        pass

    @pytest.mark.skip(reason="InterceptSessionResult has no Python constructor")
    def test_to_dict(self):
        pass


# ---------------------------------------------------------------------------
# InterceptSessionState
# ---------------------------------------------------------------------------

class TestInterceptSessionState:
    def test_values(self):
        assert repr(InterceptSessionState.Created) == "InterceptSessionState.created"
        assert str(InterceptSessionState.Listening) == "listening"
        assert str(InterceptSessionState.Capturing) == "capturing"
        assert str(InterceptSessionState.Stopped) == "stopped"
        assert str(InterceptSessionState.Error) == "error"


# ---------------------------------------------------------------------------
# InterceptStats
# ---------------------------------------------------------------------------

class TestInterceptStats:
    def test_fields(self):
        stats = InterceptStats(
            connections_total=100,
            exchanges_captured=50,
            bytes_captured=102400,
            errors=2,
            uptime_secs=60,
        )
        assert stats.connections_total == 100
        assert stats.exchanges_captured == 50
        assert stats.bytes_captured == 102400
        assert stats.errors == 2
        assert stats.uptime_secs == 60

    def test_to_dict(self):
        stats = InterceptStats(
            connections_total=10, exchanges_captured=5,
            bytes_captured=1024, errors=0, uptime_secs=30,
        )
        d = stats.to_dict()
        assert d["connections_total"] == 10
        assert d["errors"] == 0

    def test_repr(self):
        stats = InterceptStats(
            connections_total=10, exchanges_captured=5,
            bytes_captured=1024, errors=0, uptime_secs=30,
        )
        r = repr(stats)
        assert "InterceptStats" in r
        assert "10" in r


# ---------------------------------------------------------------------------
# InterceptFilter
# ---------------------------------------------------------------------------

class TestInterceptFilter:
    def test_default(self):
        f = InterceptFilter()
        assert f.host_pattern is None
        assert f.path_pattern is None
        assert f.method_pattern is None
        assert f.status_pattern is None

    def test_custom(self):
        f = InterceptFilter(
            host_pattern="*.example.com",
            path_pattern="/api/*",
            method_pattern="POST",
        )
        assert f.host_pattern == "*.example.com"
        assert f.path_pattern == "/api/*"
        assert f.method_pattern == "POST"

    def test_to_dict(self):
        f = InterceptFilter(host_pattern="example.com")
        d = f.to_dict()
        assert d["host_pattern"] == "example.com"
        assert d["path_pattern"] is None

    def test_to_json(self):
        f = InterceptFilter(host_pattern="example.com")
        j = f.to_json()
        assert "example.com" in j

    def test_repr(self):
        f = InterceptFilter(host_pattern="example.com")
        r = repr(f)
        assert "InterceptFilter" in r


# ---------------------------------------------------------------------------
# InterceptRule
# ---------------------------------------------------------------------------

class TestInterceptRule:
    def test_basic(self):
        rule = InterceptRule(
            name="block-ads",
            host_pattern="*.ads.example.com",
            action="block",
        )
        assert rule.name == "block-ads"
        assert rule.host_pattern == "*.ads.example.com"
        assert rule.action == "block"
        assert rule.priority == 0
        assert rule.enabled is True

    def test_invalid_action(self):
        with pytest.raises(ValueError, match="Invalid action"):
            InterceptRule(
                name="test",
                host_pattern="example.com",
                action="invalid",
            )

    def test_valid_actions(self):
        for action in ["allow", "block", "intercept", "monitor", "modify"]:
            rule = InterceptRule(
                name="test", host_pattern="example.com", action=action,
            )
            assert rule.action == action

    def test_to_dict(self):
        rule = InterceptRule(
            name="test", host_pattern="example.com", action="allow",
        )
        d = rule.to_dict()
        assert d["name"] == "test"
        assert d["action"] == "allow"

    def test_repr(self):
        rule = InterceptRule(
            name="test", host_pattern="example.com", action="block",
        )
        r = repr(rule)
        assert "InterceptRule" in r
        assert "block" in r


# ---------------------------------------------------------------------------
# CertificateAuthorityConfig
# ---------------------------------------------------------------------------

class TestCertificateAuthorityConfig:
    def test_default(self):
        ca = CertificateAuthorityConfig()
        assert ca.ca_cert_path is None
        assert ca.ca_key_path is None
        assert ca.auto_generate is True
        assert ca.valid_days == 365

    def test_custom(self):
        ca = CertificateAuthorityConfig(
            ca_cert_path="/path/to/ca.pem",
            ca_key_path="/path/to/ca.key",
            auto_generate=False,
            valid_days=730,
        )
        assert ca.ca_cert_path == "/path/to/ca.pem"
        assert ca.ca_key_path == "/path/to/ca.key"
        assert ca.auto_generate is False
        assert ca.valid_days == 730

    def test_to_dict(self):
        ca = CertificateAuthorityConfig()
        d = ca.to_dict()
        assert d["auto_generate"] is True
        assert d["valid_days"] == 365

    def test_to_json(self):
        ca = CertificateAuthorityConfig()
        j = ca.to_json()
        assert "auto_generate" in j

    def test_repr(self):
        ca = CertificateAuthorityConfig()
        r = repr(ca)
        assert "CertificateAuthorityConfig" in r


# ---------------------------------------------------------------------------
# IssuedCertificate
# ---------------------------------------------------------------------------

class TestIssuedCertificate:
    def test_fields(self):
        cert = IssuedCertificate(
            hostname="example.com",
            serial="00:11:22:33",
            valid_from="2026-01-01T00:00:00Z",
            valid_until="2027-01-01T00:00:00Z",
        )
        assert cert.hostname == "example.com"
        assert cert.serial == "00:11:22:33"

    def test_to_dict(self):
        cert = IssuedCertificate(
            hostname="example.com", serial="AA:BB",
            valid_from="2026-01-01", valid_until="2027-01-01",
        )
        d = cert.to_dict()
        assert d["hostname"] == "example.com"

    def test_str(self):
        cert = IssuedCertificate(
            hostname="example.com", serial="AA:BB",
            valid_from="2026-01-01", valid_until="2027-01-01",
        )
        s = str(cert)
        assert "example.com" in s


# ---------------------------------------------------------------------------
# HarEntry
# ---------------------------------------------------------------------------

class TestHarEntry:
    def test_fields(self):
        entry = HarEntry(
            method="GET",
            url="https://example.com/api",
            status=200,
            time_ms=150.5,
            request_headers=[("Accept", "application/json")],
            response_headers=[("Content-Type", "application/json")],
            request_body=None,
            response_body='{"ok": true}',
            started_date_time="2026-01-01T00:00:00.000Z",
        )
        assert entry.method == "GET"
        assert entry.url == "https://example.com/api"
        assert entry.status == 200
        assert entry.time_ms == 150.5

    def test_to_dict(self):
        entry = HarEntry(
            method="POST", url="https://example.com", status=201,
            time_ms=100.0, request_headers=[], response_headers=[],
            started_date_time="2026-01-01T00:00:00Z",
        )
        d = entry.to_dict()
        assert d["method"] == "POST"
        assert d["status"] == 201

    def test_repr(self):
        entry = HarEntry(
            method="GET", url="https://example.com", status=200,
            time_ms=50.0, request_headers=[], response_headers=[],
            started_date_time="2026-01-01T00:00:00Z",
        )
        r = repr(entry)
        assert "HarEntry" in r
        assert "200" in r


# ---------------------------------------------------------------------------
# HarDocument
# ---------------------------------------------------------------------------

class TestHarDocument:
    def test_default(self):
        doc = HarDocument()
        assert doc.version == "1.2"
        assert doc.creator_name == "eggsec"
        assert doc.entry_count == 0
        assert doc.entries == []

    def test_with_entries(self):
        entry = HarEntry(
            method="GET", url="https://example.com", status=200,
            time_ms=10.0, request_headers=[], response_headers=[],
            started_date_time="2026-01-01T00:00:00Z",
        )
        doc = HarDocument(entries=[entry], creator_name="test")
        assert doc.entry_count == 1
        assert doc.creator_name == "test"

    def test_to_dict(self):
        doc = HarDocument()
        d = doc.to_dict()
        assert d["version"] == "1.2"
        assert isinstance(d["entries"], list)

    def test_to_json(self):
        doc = HarDocument()
        j = doc.to_json()
        assert "1.2" in j
        assert "eggsec" in j

    def test_repr(self):
        doc = HarDocument()
        r = repr(doc)
        assert "HarDocument" in r
        assert "1.2" in r

    def test_str(self):
        doc = HarDocument()
        s = str(doc)
        assert "HAR" in s
        assert "1.2" in s


# ---------------------------------------------------------------------------
# CapturedExchange - detailed field coverage
# ---------------------------------------------------------------------------

class TestCapturedExchangeDetailed:
    def test_all_fields(self):
        exchange = CapturedExchange(
            id=1,
            method="POST",
            uri="https://api.example.com/login",
            request_headers=[("Content-Type", "application/json"), ("Authorization", "Bearer tok")],
            request_body='{"user":"admin","pass":"test"}',
            response_status=200,
            response_headers=[("Content-Type", "application/json")],
            response_body='{"token":"abc123"}',
            timestamp_ms=1700000000000,
            latency_ms=42,
            request_modified=False,
            response_modified=True,
        )
        assert exchange.id == 1
        assert exchange.method == "POST"
        assert exchange.uri == "https://api.example.com/login"
        assert len(exchange.request_headers) == 2
        assert exchange.request_body == '{"user":"admin","pass":"test"}'
        assert exchange.response_status == 200
        assert len(exchange.response_headers) == 1
        assert exchange.response_body == '{"token":"abc123"}'
        assert exchange.timestamp_ms == 1700000000000
        assert exchange.latency_ms == 42
        assert exchange.request_modified is False
        assert exchange.response_modified is True

    def test_to_dict(self):
        exchange = CapturedExchange(
            id=5,
            method="GET",
            uri="https://example.com/health",
            request_headers=[],
            response_status=204,
            response_headers=[],
            timestamp_ms=1000,
        )
        d = exchange.to_dict()
        assert d["id"] == 5
        assert d["method"] == "GET"
        assert d["uri"] == "https://example.com/health"
        assert d["response_status"] == 204
        assert d["timestamp_ms"] == 1000
        assert d["request_modified"] is False
        assert d["response_modified"] is False

    def test_to_json(self):
        exchange = CapturedExchange(
            id=1,
            method="DELETE",
            uri="https://example.com/resource",
            request_headers=[],
            response_status=404,
            response_headers=[],
            timestamp_ms=2000,
        )
        j = exchange.to_json()
        assert isinstance(j, str)
        assert "DELETE" in j
        assert "404" in j

    def test_repr(self):
        exchange = CapturedExchange(
            id=1,
            method="PUT",
            uri="https://example.com/update",
            request_headers=[],
            response_status=201,
            response_headers=[],
            timestamp_ms=3000,
        )
        r = repr(exchange)
        assert "CapturedExchange" in r
        assert "PUT" in r
        assert "201" in r

    def test_str(self):
        exchange = CapturedExchange(
            id=1,
            method="GET",
            uri="https://example.com/data",
            request_headers=[],
            response_status=200,
            response_headers=[],
            timestamp_ms=4000,
        )
        s = str(exchange)
        assert "GET" in s
        assert "200" in s

    def test_no_response_status(self):
        exchange = CapturedExchange(
            id=1,
            method="GET",
            uri="https://example.com/timeout",
            request_headers=[],
            response_status=None,
            response_headers=[],
            timestamp_ms=5000,
        )
        assert exchange.response_status is None
        s = str(exchange)
        assert "?" in s


# ---------------------------------------------------------------------------
# InterceptSessionState - detailed state transitions
# ---------------------------------------------------------------------------

class TestInterceptSessionStateDetailed:
    def test_all_variants(self):
        states = [
            (InterceptSessionState.Created, "created"),
            (InterceptSessionState.Listening, "listening"),
            (InterceptSessionState.Capturing, "capturing"),
            (InterceptSessionState.Stopped, "stopped"),
            (InterceptSessionState.Error, "error"),
        ]
        for state, expected_str in states:
            assert str(state) == expected_str
            assert repr(state) == f"InterceptSessionState.{expected_str}"

    def test_equality(self):
        assert InterceptSessionState.Created == InterceptSessionState.Created
        assert InterceptSessionState.Created != InterceptSessionState.Listening
        assert InterceptSessionState.Capturing != InterceptSessionState.Stopped

    def test_hash(self):
        states = {
            InterceptSessionState.Created,
            InterceptSessionState.Listening,
            InterceptSessionState.Capturing,
            InterceptSessionState.Stopped,
            InterceptSessionState.Error,
        }
        assert len(states) == 5


# ---------------------------------------------------------------------------
# ProxyConfig - serialization roundtrip
# ---------------------------------------------------------------------------

class TestProxyConfigSerialization:
    def test_json_roundtrip(self):
        config = ProxyConfig(
            rotation_strategy=RotationStrategy.Weighted,
            health_check_enabled=False,
            health_check_interval_secs=120,
            health_check_timeout_ms=10000,
            max_failures_before_disable=5,
            chain_proxies=True,
            max_chain_length=5,
        )
        j = config.to_json()
        assert "weighted" in j
        assert "chain_proxies" in j

    def test_dict_roundtrip(self):
        config = ProxyConfig(
            rotation_strategy=RotationStrategy.LowestLatency,
            health_check_enabled=True,
            health_check_frequency_secs=30,
        )
        d = config.to_dict()
        assert d["rotation_strategy"] == "lowest_latency"
        assert d["health_check_enabled"] is True
        assert d["health_check_frequency_secs"] == 30
        assert d["chain_proxies"] is False
        assert d["max_chain_length"] == 3

    def test_all_fields_in_dict(self):
        config = ProxyConfig()
        d = config.to_dict()
        expected_keys = {
            "rotation_strategy", "health_check_enabled",
            "health_check_interval_secs", "health_check_timeout_ms",
            "test_url", "health_check_url", "health_check_frequency_secs",
            "max_failures_before_disable", "chain_proxies", "max_chain_length",
        }
        assert expected_keys == set(d.keys())

    def test_optional_urls(self):
        config = ProxyConfig(
            test_url="https://example.com/health",
            health_check_url="https://example.com/ping",
        )
        assert config.test_url == "https://example.com/health"
        assert config.health_check_url == "https://example.com/ping"
        d = config.to_dict()
        assert d["test_url"] == "https://example.com/health"
        assert d["health_check_url"] == "https://example.com/ping"


# ---------------------------------------------------------------------------
# ProxyRotationStrategies - all 5 strategies
# ---------------------------------------------------------------------------

class TestProxyRotationStrategies:
    def test_round_robin(self):
        s = RotationStrategy.from_str("round_robin")
        assert s == RotationStrategy.RoundRobin
        assert str(s) == "round_robin"
        assert repr(s) == "RotationStrategy.round_robin"

    def test_random(self):
        s = RotationStrategy.from_str("random")
        assert s == RotationStrategy.Random
        assert str(s) == "random"

    def test_weighted(self):
        s = RotationStrategy.from_str("weighted")
        assert s == RotationStrategy.Weighted
        assert str(s) == "weighted"

    def test_least_used(self):
        s = RotationStrategy.from_str("least_used")
        assert s == RotationStrategy.LeastUsed
        assert str(s) == "least_used"

    def test_lowest_latency(self):
        s = RotationStrategy.from_str("lowest_latency")
        assert s == RotationStrategy.LowestLatency
        assert str(s) == "lowest_latency"

    def test_from_str_roundtrip_all(self):
        strategies = ["round_robin", "random", "weighted", "least_used", "lowest_latency"]
        for name in strategies:
            s = RotationStrategy.from_str(name)
            assert str(s) == name

    def test_invalid_all(self):
        for name in ["", "unknown", "ROUND_ROBIN", "RoundRobin"]:
            with pytest.raises(ValueError, match="Invalid rotation strategy"):
                RotationStrategy.from_str(name)


# ---------------------------------------------------------------------------
# ProxyTypeAllVariants - all 5 proxy types
# ---------------------------------------------------------------------------

class TestProxyTypeAllVariants:
    def test_socks4(self):
        pt = ProxyType.from_str("socks4")
        assert pt == ProxyType.Socks4
        assert str(pt) == "socks4"
        assert repr(pt) == "ProxyType.socks4"

    def test_socks5(self):
        pt = ProxyType.from_str("socks5")
        assert pt == ProxyType.Socks5
        assert str(pt) == "socks5"

    def test_http(self):
        pt = ProxyType.from_str("http")
        assert pt == ProxyType.Http
        assert str(pt) == "http"

    def test_https(self):
        pt = ProxyType.from_str("https")
        assert pt == ProxyType.Https
        assert str(pt) == "https"

    def test_tor(self):
        pt = ProxyType.from_str("tor")
        assert pt == ProxyType.Tor
        assert str(pt) == "tor"

    def test_from_str_roundtrip_all(self):
        types = ["socks4", "socks5", "http", "https", "tor"]
        for name in types:
            pt = ProxyType.from_str(name)
            assert str(pt) == name

    def test_invalid_all(self):
        for name in ["", "unknown", "SOCKS5", "HTTP"]:
            with pytest.raises(ValueError, match="Invalid proxy type"):
                ProxyType.from_str(name)


# ---------------------------------------------------------------------------
# ProxyChaining - config with chain_proxies
# ---------------------------------------------------------------------------

class TestProxyChaining:
    def test_chain_proxies_enabled(self):
        config = ProxyConfig(chain_proxies=True, max_chain_length=5)
        assert config.chain_proxies is True
        assert config.max_chain_length == 5

    def test_chain_proxies_disabled(self):
        config = ProxyConfig(chain_proxies=False)
        assert config.chain_proxies is False
        assert config.max_chain_length == 3

    def test_max_chain_length_in_dict(self):
        config = ProxyConfig(chain_proxies=True, max_chain_length=10)
        d = config.to_dict()
        assert d["chain_proxies"] is True
        assert d["max_chain_length"] == 10

    def test_max_chain_length_in_json(self):
        config = ProxyConfig(chain_proxies=True, max_chain_length=7)
        j = config.to_json()
        assert "chain_proxies" in j

    def test_chained_proxy_entries(self):
        entry1 = ProxyEntry(ProxyType.Http, "proxy1.example.com", 8080, weight=2)
        entry2 = ProxyEntry(ProxyType.Socks5, "proxy2.example.com", 1080, weight=1)
        assert entry1.weight == 2
        assert entry2.weight == 1
        config = ProxyConfig(chain_proxies=True, max_chain_length=2)
        d = config.to_dict()
        assert d["max_chain_length"] == 2
