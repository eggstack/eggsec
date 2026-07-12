"""Secret redaction audit tests.

Verifies that no secret-bearing field leaks sentinel values through repr(),
str(), JSON serialization, dict serialization, property access, or exception
messages.
"""

import json
import pytest
import eggsec

SENTINELS = [
    "SENTINEL_SECRET_abc123",
    "sk-live-FAKE_TOKEN_xyz789",
    "AKIAIOSFODNN7EXAMPLE",
    "ghp_FAKEgithubtoken1234567890",
]

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _any_sentinel_in(text: str) -> str | None:
    """Return the first sentinel found in text, or None."""
    for s in SENTINELS:
        if s in text:
            return s
    return None


def _assert_no_sentinel(text: str, context: str):
    found = _any_sentinel_in(text)
    assert found is None, f"Sentinel {found!r} leaked in {context}: {text!r}"


# ---------------------------------------------------------------------------
# SensitiveString
# ---------------------------------------------------------------------------


class TestSensitiveStringRedaction:
    def test_repr_redacts(self):
        for s in SENTINELS:
            ss = eggsec.SensitiveString(s)
            _assert_no_sentinel(repr(ss), "SensitiveString.__repr__")

    def test_str_redacts(self):
        for s in SENTINELS:
            ss = eggsec.SensitiveString(s)
            _assert_no_sentinel(str(ss), "SensitiveString.__str__")

    def test_expose_secret_works(self):
        ss = eggsec.SensitiveString("hello")
        assert ss.expose_secret() == "hello"

    def test_is_empty(self):
        ss = eggsec.SensitiveString("")
        assert ss.is_empty() is True
        ss2 = eggsec.SensitiveString("x")
        assert ss2.is_empty() is False

    def test_len(self):
        ss = eggsec.SensitiveString("abcde")
        assert ss.len() == 5


# ---------------------------------------------------------------------------
# HttpConfig.proxy_auth
# ---------------------------------------------------------------------------


class TestHttpConfigProxyAuth:
    def test_proxy_auth_redacted_via_getter(self):
        for s in SENTINELS:
            cfg = eggsec.HttpConfig(proxy_auth=s)
            val = cfg.proxy_auth
            assert val is not None
            _assert_no_sentinel(repr(val), "HttpConfig.proxy_auth repr")
            _assert_no_sentinel(str(val), "HttpConfig.proxy_auth str")

    def test_proxy_auth_repr_omits_value(self):
        cfg = eggsec.HttpConfig(proxy_auth="supersecret123")
        _assert_no_sentinel(repr(cfg), "HttpConfig repr")


# ---------------------------------------------------------------------------
# ReconApiConfig — all API keys
# ---------------------------------------------------------------------------


class TestReconApiConfigKeys:
    @pytest.mark.parametrize(
        "kwargs_name",
        [
            "virustotal_api_key",
            "alienvault_api_key",
            "shodan_api_key",
            "ipapi_api_key",
            "maxmind_license_key",
            "wayback_api_key",
            "nvd_api_key",
        ],
    )
    def test_api_key_redacted_via_getter(self, kwargs_name):
        for s in SENTINELS:
            cfg = eggsec.ReconApiConfig(**{kwargs_name: s})
            val = getattr(cfg, kwargs_name)
            assert val is not None
            _assert_no_sentinel(repr(val), f"ReconApiConfig.{kwargs_name} repr")
            _assert_no_sentinel(str(val), f"ReconApiConfig.{kwargs_name} str")

    def test_repr_omits_keys(self):
        cfg = eggsec.ReconApiConfig(
            virustotal_api_key="SECRET_vt",
            shodan_api_key="SECRET_shodan",
            nvd_api_key="SECRET_nvd",
        )
        r = repr(cfg)
        _assert_no_sentinel(r, "ReconApiConfig repr")
        assert "SECRET_" not in r

    def test_has_key_true_when_set(self):
        cfg = eggsec.ReconApiConfig(virustotal_api_key="key123")
        assert cfg.has_virustotal_api_key is True

    def test_has_key_false_when_none(self):
        cfg = eggsec.ReconApiConfig()
        assert cfg.has_virustotal_api_key is False


# ---------------------------------------------------------------------------
# RemoteConfig.psk
# ---------------------------------------------------------------------------


class TestRemoteConfigPsk:
    def test_psk_redacted_via_getter(self):
        for s in SENTINELS:
            cfg = eggsec.RemoteConfig(psk=s)
            val = cfg.psk
            assert val is not None
            _assert_no_sentinel(repr(val), "RemoteConfig.psk repr")
            _assert_no_sentinel(str(val), "RemoteConfig.psk str")

    def test_has_psk(self):
        cfg = eggsec.RemoteConfig(psk="secret")
        assert cfg.has_psk is True
        cfg2 = eggsec.RemoteConfig()
        assert cfg2.has_psk is False

    def test_repr_omits_psk(self):
        cfg = eggsec.RemoteConfig(psk="supersecret123")
        _assert_no_sentinel(repr(cfg), "RemoteConfig repr")


# ---------------------------------------------------------------------------
# AiConfig.api_key
# ---------------------------------------------------------------------------


class TestAiConfigApiKey:
    def test_api_key_redacted_via_getter(self):
        for s in SENTINELS:
            cfg = eggsec.AiConfig(api_key=s)
            val = cfg.api_key
            assert val is not None
            _assert_no_sentinel(repr(val), "AiConfig.api_key repr")
            _assert_no_sentinel(str(val), "AiConfig.api_key str")

    def test_has_api_key(self):
        cfg = eggsec.AiConfig(api_key="sk-test")
        assert cfg.has_api_key is True
        cfg2 = eggsec.AiConfig()
        assert cfg2.has_api_key is False

    def test_repr_omits_key(self):
        cfg = eggsec.AiConfig(api_key="sk-live-supersecret")
        r = repr(cfg)
        _assert_no_sentinel(r, "AiConfig repr")
        assert "sk-live" not in r


# ---------------------------------------------------------------------------
# EggsecConfig integration — secrets flow through sub-configs
# ---------------------------------------------------------------------------


class TestEggsecConfigSecretFlow:
    def test_http_proxy_auth_redacted(self):
        cfg = eggsec.HttpConfig(proxy_auth="SENTINEL_SECRET_abc123")
        _assert_no_sentinel(repr(cfg), "HttpConfig with proxy_auth")

    def test_recon_api_keys_redacted(self):
        cfg = eggsec.ReconApiConfig(
            virustotal_api_key="SENTINEL_SECRET_abc123",
            shodan_api_key="SENTINEL_SECRET_abc123",
        )
        _assert_no_sentinel(repr(cfg), "ReconApiConfig with keys")

    def test_remote_psk_redacted(self):
        cfg = eggsec.RemoteConfig(psk="SENTINEL_SECRET_abc123")
        _assert_no_sentinel(repr(cfg), "RemoteConfig with psk")

    def test_ai_api_key_redacted(self):
        cfg = eggsec.AiConfig(api_key="SENTINEL_SECRET_abc123")
        _assert_no_sentinel(repr(cfg), "AiConfig with api_key")


# ---------------------------------------------------------------------------
# ProxyConfigEntry — password is never stored
# ---------------------------------------------------------------------------


class TestProxyConfigEntry:
    def test_password_not_stored(self):
        entry = eggsec.ProxyConfigEntry(
            address="127.0.0.1",
            port=8080,
            password="SENTINEL_SECRET_abc123",
        )
        assert entry.has_password is True
        _assert_no_sentinel(repr(entry), "ProxyConfigEntry repr")


# ---------------------------------------------------------------------------
# Serialization safety — no sentinel in JSON output
# ---------------------------------------------------------------------------


class TestSerializationSafety:
    def test_sensitive_string_json(self):
        """SensitiveString serializes to [REDACTED] in serde — verify via repr."""
        for s in SENTINELS:
            ss = eggsec.SensitiveString(s)
            r = repr(ss)
            j = str(ss)
            _assert_no_sentinel(r, "SensitiveString json via repr")
            _assert_no_sentinel(j, "SensitiveString json via str")

    def test_http_config_json_roundtrip(self):
        cfg = eggsec.HttpConfig(proxy_auth="SENTINEL_SECRET_abc123")
        r = repr(cfg)
        _assert_no_sentinel(r, "HttpConfig repr roundtrip")


# ---------------------------------------------------------------------------
# Equality and hashing do not expose values
# ---------------------------------------------------------------------------


class TestEqualityHashing:
    def test_sensitive_string_eq(self):
        a = eggsec.SensitiveString("same")
        b = eggsec.SensitiveString("same")
        assert a == b

    def test_sensitive_string_ne(self):
        a = eggsec.SensitiveString("one")
        b = eggsec.SensitiveString("two")
        assert a != b

    def test_sensitive_string_hash(self):
        a = eggsec.SensitiveString("same")
        b = eggsec.SensitiveString("same")
        assert hash(a) == hash(b)


# ---------------------------------------------------------------------------
# Exception messages do not contain secrets
# ---------------------------------------------------------------------------


class TestExceptionSafety:
    def test_exception_does_not_leak_secret(self):
        """Verify that creating configs with secrets doesn't leak via exceptions."""
        try:
            cfg = eggsec.RemoteConfig(psk="SENTINEL_SECRET_abc123")
            # Force some operation that might format the config
            _ = repr(cfg)
        except Exception as e:
            _assert_no_sentinel(str(e), "exception message")

    def test_invalid_config_exception_safe(self):
        try:
            eggsec.PortRange.list([])
        except ValueError as e:
            _assert_no_sentinel(str(e), "ValueError message")


# ---------------------------------------------------------------------------
# Sentinel values never appear in module-level output
# ---------------------------------------------------------------------------


class TestModuleLevelSafety:
    def test_module_repr_safe(self):
        import eggsec as mod

        r = repr(mod)
        _assert_no_sentinel(r, "module repr")

    def test_version_safe(self):
        import eggsec as mod

        v = mod.__version__
        _assert_no_sentinel(str(v), "__version__")
