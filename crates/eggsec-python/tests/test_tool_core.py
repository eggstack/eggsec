"""Release 5 Phase A: tool-core bindings tests.

Covers enums, structs, credential redaction, ToolRequest, ToolDescriptor,
ToolRegistry, SchemaGenerator, operation_as_tool, ValidationReport, and
round-trip serialization for all eggsec-tool-core Python wrappers.
"""

import json
import re
import uuid

import pytest


# ---------------------------------------------------------------------------
# 1. Enum construction and properties
# ---------------------------------------------------------------------------

class TestTargetType:
    def test_static_constructors(self):
        from eggsec import ToolTargetType
        assert ToolTargetType.url().value == "url"
        assert ToolTargetType.domain().value == "domain"
        assert ToolTargetType.ip().value == "ip"
        assert ToolTargetType.cidr().value == "cidr"
        assert ToolTargetType.file().value == "file"

    def test_from_str_round_trip(self):
        from eggsec import ToolTargetType
        for name in ("url", "domain", "ip", "cidr", "file"):
            obj = ToolTargetType.from_str(name)
            assert obj.value == name

    def test_from_str_case_insensitive(self):
        from eggsec import ToolTargetType
        obj = ToolTargetType.from_str("URL")
        assert obj.value == "url"

    def test_from_str_invalid(self):
        from eggsec import ToolTargetType
        with pytest.raises(ValueError, match="Invalid target type"):
            ToolTargetType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolTargetType
        t = ToolTargetType.url()
        assert "TargetTypePy" in repr(t)
        assert str(t) == "url"

    def test_equality(self):
        from eggsec import ToolTargetType
        assert ToolTargetType.url() == ToolTargetType.url()
        assert ToolTargetType.url() != ToolTargetType.ip()

    def test_json_round_trip(self):
        from eggsec import ToolTargetType
        j = ToolTargetType.url().to_json()
        assert json.loads(j) == "url"

    def test_hash(self):
        from eggsec import ToolTargetType
        assert hash(ToolTargetType.url()) == hash(ToolTargetType.url())
        assert hash(ToolTargetType.url()) != hash(ToolTargetType.ip())


class TestAuthType:
    def test_static_constructors(self):
        from eggsec import ToolAuthType
        assert ToolAuthType.none().value == "none"
        assert ToolAuthType.basic().value == "basic"
        assert ToolAuthType.bearer().value == "bearer"
        assert ToolAuthType.api_key().value == "api_key"
        assert ToolAuthType.oauth2().value == "oauth2"

    def test_from_str_round_trip(self):
        from eggsec import ToolAuthType
        for name in ("none", "basic", "bearer", "api_key", "oauth2"):
            obj = ToolAuthType.from_str(name)
            assert obj is not None

    def test_from_str_aliases(self):
        from eggsec import ToolAuthType
        assert ToolAuthType.from_str("apikey").value == "api_key"
        assert ToolAuthType.from_str("api-key").value == "api_key"
        assert ToolAuthType.from_str("oauth").value == "oauth2"

    def test_from_str_invalid(self):
        from eggsec import ToolAuthType
        with pytest.raises(ValueError, match="Invalid auth type"):
            ToolAuthType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolAuthType
        t = ToolAuthType.bearer()
        assert "AuthTypePy" in repr(t)
        assert str(t) == "bearer"

    def test_equality(self):
        from eggsec import ToolAuthType
        assert ToolAuthType.basic() == ToolAuthType.basic()
        assert ToolAuthType.basic() != ToolAuthType.bearer()


class TestResponseType:
    def test_static_constructors(self):
        from eggsec import ToolResponseType
        assert ToolResponseType.success().value == "success"
        assert ToolResponseType.partial_success().value == "partial_success"
        assert ToolResponseType.failed().value == "failed"
        assert ToolResponseType.timeout().value == "timeout"
        assert ToolResponseType.scope_violation().value == "scope_violation"
        assert ToolResponseType.cancelled().value == "cancelled"

    def test_from_str_round_trip(self):
        from eggsec import ToolResponseType
        for name in ("success", "partial_success", "failed", "timeout",
                      "scope_violation", "cancelled"):
            obj = ToolResponseType.from_str(name)
            assert obj is not None

    def test_from_str_canceled_alias(self):
        from eggsec import ToolResponseType
        assert ToolResponseType.from_str("canceled").value == "cancelled"

    def test_from_str_invalid(self):
        from eggsec import ToolResponseType
        with pytest.raises(ValueError, match="Invalid response status"):
            ToolResponseType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolResponseType
        t = ToolResponseType.success()
        assert "ResponseTypePy" in repr(t)
        assert str(t) == "success"


class TestToolFindingType:
    def test_static_constructors(self):
        from eggsec import ToolFindingType
        assert ToolFindingType.vulnerability().value == "vulnerability"
        assert ToolFindingType.information().value == "information"
        assert ToolFindingType.weakness().value == "weakness"
        assert ToolFindingType.configuration().value == "configuration"
        assert ToolFindingType.misconfiguration().value == "misconfiguration"
        assert ToolFindingType.sensitive_data().value == "sensitive_data"
        assert ToolFindingType.banner().value == "banner"
        assert ToolFindingType.technology().value == "technology"
        assert ToolFindingType.service().value == "service"
        assert ToolFindingType.endpoint().value == "endpoint"
        assert ToolFindingType.subdomain().value == "subdomain"
        assert ToolFindingType.open_port().value == "open_port"

    def test_from_str_aliases(self):
        from eggsec import ToolFindingType
        assert ToolFindingType.from_str("vuln").value == "vulnerability"
        assert ToolFindingType.from_str("info").value == "information"
        assert ToolFindingType.from_str("config").value == "configuration"
        assert ToolFindingType.from_str("tech").value == "technology"
        assert ToolFindingType.from_str("sub").value == "subdomain"
        assert ToolFindingType.from_str("openport").value == "open_port"

    def test_from_str_invalid(self):
        from eggsec import ToolFindingType
        with pytest.raises(ValueError, match="Invalid finding type"):
            ToolFindingType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolFindingType
        t = ToolFindingType.vulnerability()
        assert "FindingTypePy" in repr(t)
        assert str(t) == "vulnerability"


class TestToolSeverity:
    def test_static_constructors(self):
        from eggsec import ToolSeverity
        assert ToolSeverity.critical().value == "critical"
        assert ToolSeverity.high().value == "high"
        assert ToolSeverity.medium().value == "medium"
        assert ToolSeverity.low().value == "low"
        assert ToolSeverity.info().value == "info"
        assert ToolSeverity.none().value == "none"

    def test_from_str_aliases(self):
        from eggsec import ToolSeverity
        assert ToolSeverity.from_str("moderate").value == "medium"
        assert ToolSeverity.from_str("informational").value == "info"

    def test_from_str_invalid(self):
        from eggsec import ToolSeverity
        with pytest.raises(ValueError, match="Invalid severity"):
            ToolSeverity.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolSeverity
        t = ToolSeverity.high()
        assert "SeverityPy" in repr(t)
        assert str(t) == "high"

    def test_json(self):
        from eggsec import ToolSeverity
        j = ToolSeverity.critical().to_json()
        assert json.loads(j) == "critical"


class TestToolErrorType:
    def test_static_constructors(self):
        from eggsec import ToolErrorType
        assert ToolErrorType.validation().value == "validation"
        assert ToolErrorType.authentication().value == "authentication"
        assert ToolErrorType.authorization().value == "authorization"
        assert ToolErrorType.rate_limit().value == "rate_limit"
        assert ToolErrorType.network().value == "network"
        assert ToolErrorType.timeout().value == "timeout"
        assert ToolErrorType.scope_violation().value == "scope_violation"
        assert ToolErrorType.not_found().value == "not_found"
        assert ToolErrorType.configuration().value == "configuration"
        assert ToolErrorType.internal().value == "internal"
        assert ToolErrorType.tool_not_found().value == "tool_not_found"

    def test_from_str_aliases(self):
        from eggsec import ToolErrorType
        assert ToolErrorType.from_str("auth").value == "authentication"
        assert ToolErrorType.from_str("ratelimit").value == "rate_limit"
        assert ToolErrorType.from_str("config").value == "configuration"

    def test_from_str_invalid(self):
        from eggsec import ToolErrorType
        with pytest.raises(ValueError, match="Invalid error type"):
            ToolErrorType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolErrorType
        t = ToolErrorType.network()
        assert "ToolErrorTypePy" in repr(t)
        assert str(t) == "network"


class TestToolPortState:
    def test_static_constructors(self):
        from eggsec import ToolPortState
        assert ToolPortState.open().value == "open"
        assert ToolPortState.closed().value == "closed"
        assert ToolPortState.filtered().value == "filtered"

    def test_from_str_round_trip(self):
        from eggsec import ToolPortState
        for name in ("open", "closed", "filtered"):
            obj = ToolPortState.from_str(name)
            assert obj.value == name

    def test_from_str_invalid(self):
        from eggsec import ToolPortState
        with pytest.raises(ValueError, match="Invalid port state"):
            ToolPortState.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolPortState
        t = ToolPortState.open()
        assert "PortStatePy" in repr(t)
        assert str(t) == "open"


class TestToolStreamEventType:
    def test_static_constructors(self):
        from eggsec import ToolStreamEventType
        assert ToolStreamEventType.progress().value == "progress"
        assert ToolStreamEventType.finding().value == "finding"
        assert ToolStreamEventType.result().value == "result"
        assert ToolStreamEventType.error().value == "error"

    def test_from_str_round_trip(self):
        from eggsec import ToolStreamEventType
        for name in ("progress", "finding", "result", "error"):
            obj = ToolStreamEventType.from_str(name)
            assert obj is not None

    def test_from_str_invalid(self):
        from eggsec import ToolStreamEventType
        with pytest.raises(ValueError, match="Invalid stream event type"):
            ToolStreamEventType.from_str("nope")

    def test_repr_and_str(self):
        from eggsec import ToolStreamEventType
        t = ToolStreamEventType.progress()
        assert "StreamEventTypePy" in repr(t)
        assert str(t) == "progress"


# ---------------------------------------------------------------------------
# 2. Struct construction and properties
# ---------------------------------------------------------------------------

class TestToolTarget:
    def test_static_constructors(self):
        from eggsec import ToolTarget
        t = ToolTarget.url("https://example.com")
        assert t.value == "https://example.com"
        assert t.target_type.value == "url"

    def test_domain(self):
        from eggsec import ToolTarget
        t = ToolTarget.domain("example.com")
        assert t.value == "example.com"
        assert t.target_type.value == "domain"

    def test_ip(self):
        from eggsec import ToolTarget
        t = ToolTarget.ip("10.0.0.1")
        assert t.value == "10.0.0.1"
        assert t.target_type.value == "ip"

    def test_cidr(self):
        from eggsec import ToolTarget
        t = ToolTarget.cidr("192.168.0.0/16")
        assert t.value == "192.168.0.0/16"
        assert t.target_type.value == "cidr"

    def test_file(self):
        from eggsec import ToolTarget
        t = ToolTarget.file("/tmp/test.apk")
        assert t.value == "/tmp/test.apk"
        assert t.target_type.value == "file"

    def test_with_scope(self):
        from eggsec import ToolTarget, ToolScope
        scope = ToolScope.allow_all()
        t = ToolTarget.with_scope(ToolTarget.url("example.com"), scope)
        assert t.scope is not None
        assert t.scope.allow_subdomains is True

    def test_scope_none_by_default(self):
        from eggsec import ToolTarget
        t = ToolTarget.url("example.com")
        assert t.scope is None

    def test_to_dict(self):
        from eggsec import ToolTarget
        d = ToolTarget.url("https://example.com").to_dict()
        assert isinstance(d, dict)
        assert d["target_type"] == "url"
        assert d["value"] == "https://example.com"

    def test_to_json(self):
        from eggsec import ToolTarget
        j = ToolTarget.url("https://example.com").to_json()
        parsed = json.loads(j)
        assert "value" in parsed

    def test_repr_and_str(self):
        from eggsec import ToolTarget
        t = ToolTarget.url("https://example.com")
        assert "TargetPy" in repr(t)
        assert "example.com" in str(t)


class TestToolScope:
    def test_allow_all(self):
        from eggsec import ToolScope
        s = ToolScope.allow_all()
        assert s.allow_subdomains is True
        assert "*" in s.allowed_patterns

    def test_deny_all(self):
        from eggsec import ToolScope
        s = ToolScope.deny_all()
        assert s.allowed_patterns == []
        assert "*" in s.excluded_patterns

    def test_custom(self):
        from eggsec import ToolScope
        s = ToolScope.new(
            allowed_patterns=["example.com", "*.test.com"],
            excluded_patterns=["admin.example.com"],
            allowed_ips=["10.0.0.1"],
            allow_subdomains=False,
        )
        assert len(s.allowed_patterns) == 2
        assert len(s.excluded_patterns) == 1
        assert len(s.allowed_ips) == 1
        assert s.allow_subdomains is False

    def test_is_allowed(self):
        from eggsec import ToolScope
        s = ToolScope.new(allowed_patterns=["example.com"])
        assert s.is_allowed("example.com") is True
        assert s.is_allowed("other.com") is False

    def test_to_dict(self):
        from eggsec import ToolScope
        d = ToolScope.allow_all().to_dict()
        assert isinstance(d, dict)
        assert "allowed_patterns" in d

    def test_to_json(self):
        from eggsec import ToolScope
        j = ToolScope.allow_all().to_json()
        parsed = json.loads(j)
        assert "allowed_patterns" in parsed

    def test_repr_and_str(self):
        from eggsec import ToolScope
        s = ToolScope.allow_all()
        assert "ScopeToolPy" in repr(s)
        assert "Scope" in str(s)


class TestToolRequestOptions:
    def test_defaults(self):
        from eggsec import ToolRequestOptions
        o = ToolRequestOptions.new()
        assert o.timeout_ms is None
        assert o.concurrency is None
        assert o.stealth is False
        assert o.follow_redirects is True
        assert o.verify_ssl is True

    def test_custom(self):
        from eggsec import ToolRequestOptions
        o = ToolRequestOptions.new(
            timeout_ms=5000,
            concurrency=10,
            rate_limit=1.5,
            proxy="http://proxy:8080",
            stealth=True,
            follow_redirects=False,
            verify_ssl=False,
        )
        assert o.timeout_ms == 5000
        assert o.concurrency == 10
        assert o.rate_limit == 1.5
        assert o.proxy == "http://proxy:8080"
        assert o.stealth is True
        assert o.follow_redirects is False
        assert o.verify_ssl is False

    def test_to_dict(self):
        from eggsec import ToolRequestOptions
        d = ToolRequestOptions.new(timeout_ms=3000).to_dict()
        assert isinstance(d, dict)
        assert d["timeout_ms"] == 3000

    def test_to_json(self):
        from eggsec import ToolRequestOptions
        j = ToolRequestOptions.new().to_json()
        parsed = json.loads(j)
        assert "timeout_ms" in parsed

    def test_repr_and_str(self):
        from eggsec import ToolRequestOptions
        o = ToolRequestOptions.new(timeout_ms=1000, stealth=True)
        assert "RequestOptionsPy" in repr(o)
        s = str(o)
        assert "1000" in s


class TestToolAuthConfig:
    def test_basic(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.basic("admin", "secret123")
        assert c.auth_type.value == "basic"

    def test_bearer(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.bearer("tok_abc")
        assert c.auth_type.value == "bearer"

    def test_api_key(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.api_key("key123", "X-API-Key")
        assert c.auth_type.value == "api_key"

    def test_to_dict_redacts(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.basic("user", "pass")
        d = c.to_dict()
        assert isinstance(d, dict)
        creds = d["credentials"]
        for v in creds.values():
            assert v == "[REDACTED]"

    def test_to_json_redacts(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.bearer("secret_token")
        j = c.to_json()
        parsed = json.loads(j)
        assert "secret_token" not in json.dumps(parsed)


# ---------------------------------------------------------------------------
# 3. AuthConfig credential redaction
# ---------------------------------------------------------------------------

class TestAuthConfigRedaction:
    def test_basic_repr_no_credentials(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.basic("admin", "hunter2")
        r = repr(c)
        assert "admin" not in r
        assert "hunter2" not in r
        assert "[REDACTED]" in r

    def test_basic_str_no_credentials(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.basic("user", "pass")
        s = str(c)
        assert "user" not in s
        assert "pass" not in s
        assert "[REDACTED]" in s

    def test_bearer_repr_no_token(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.bearer("token123")
        r = repr(c)
        assert "token123" not in r

    def test_bearer_str_no_token(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.bearer("super_secret_token_value")
        s = str(c)
        assert "super_secret_token_value" not in s

    def test_api_key_repr_no_key(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.api_key("sk_live_abc123", "Authorization")
        r = repr(c)
        assert "sk_live_abc123" not in r

    def test_api_key_str_no_key(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.api_key("sk_live_abc123", "Authorization")
        s = str(c)
        assert "sk_live_abc123" not in s


# ---------------------------------------------------------------------------
# 4. ToolRequest creation
# ---------------------------------------------------------------------------

class TestToolRequest:
    def test_create(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        assert req.tool == "scan_ports"
        assert req.target.value == "example.com"
        assert req.target.target_type.value == "url"

    def test_id_is_uuid(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        parsed = uuid.UUID(req.id)
        assert str(parsed) == req.id

    def test_params_default_empty(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        params = req.params
        assert isinstance(params, dict)
        assert len(params) == 0

    def test_params_custom(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new(
            "scan_ports",
            ToolTarget.url("example.com"),
            params={"ports": "80,443"},
        )
        params = req.params
        assert params["ports"] == "80,443"

    def test_options(self):
        from eggsec import ToolRequest, ToolTarget, ToolRequestOptions
        opts = ToolRequestOptions.new(timeout_ms=5000, stealth=True)
        req = ToolRequest.new(
            "scan_ports", ToolTarget.url("example.com"), options=opts
        )
        assert req.options.timeout_ms == 5000
        assert req.options.stealth is True

    def test_has_cancellation(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        assert req.has_cancellation is False

    def test_to_dict(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        d = req.to_dict()
        assert isinstance(d, dict)
        assert d["id"] == req.id
        assert d["tool"] == "scan_ports"
        assert "target_type" in d
        assert "target_value" in d
        assert "params" in d
        assert "options" in d

    def test_to_json(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        j = req.to_json()
        parsed = json.loads(j)
        assert "id" in parsed
        assert parsed["tool"] == "scan_ports"

    def test_repr_and_str(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("scan_ports", ToolTarget.url("example.com"))
        assert "ToolRequestPy" in repr(req)
        s = str(req)
        assert "scan_ports" in s
        assert "example.com" in s

    def test_different_tools(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new("recon_dns", ToolTarget.domain("example.com"))
        assert req.tool == "recon_dns"
        assert req.target.value == "example.com"
        assert req.target.target_type.value == "domain"


# ---------------------------------------------------------------------------
# 5. ToolResponse
# ---------------------------------------------------------------------------

class TestToolResponse:
    """ToolResponse is only constructible via internal engine dispatch.
    These tests exercise the API surface on instances obtained from
    the engine or verify the type exists and has expected attributes."""

    def test_type_exists(self):
        from eggsec import ToolResponse
        assert ToolResponse is not None

    def test_has_is_success(self):
        from eggsec import ToolResponse
        assert hasattr(ToolResponse, "is_success") or callable(
            getattr(ToolResponse, "is_success", None)
        )


# ---------------------------------------------------------------------------
# 6. ToolDescriptor and ToolRegistry
# ---------------------------------------------------------------------------

class TestToolRegistry:
    def test_list_nonempty(self):
        from eggsec import ToolRegistry
        tools = ToolRegistry.list()
        assert len(tools) > 0

    def test_count_matches_list(self):
        from eggsec import ToolRegistry
        tools = ToolRegistry.list()
        assert ToolRegistry.count() == len(tools)

    def test_count_is_22(self):
        from eggsec import ToolRegistry
        assert ToolRegistry.count() == 22

    def test_get_scan_ports(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        assert d is not None
        assert d.tool_id == "scan_ports"
        assert len(d.operation_id) > 0

    def test_get_by_operation_id(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan-ports")
        assert d is not None
        assert d.tool_id == "scan_ports"
        assert d.operation_id == "scan-ports"

    def test_get_nonexistent(self):
        from eggsec import ToolRegistry
        assert ToolRegistry.get("nonexistent_tool") is None

    def test_descriptor_fields(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        assert len(d.title) > 0
        assert len(d.description) > 0
        assert len(d.category) > 0
        assert len(d.version) > 0
        assert len(d.risk) > 0
        assert len(d.maturity) > 0

    def test_all_descriptors_valid(self):
        from eggsec import ToolRegistry
        for d in ToolRegistry.list():
            assert len(d.title) > 0, f"{d.tool_id} has empty title"
            assert len(d.description) > 0, f"{d.tool_id} has empty description"
            assert len(d.category) > 0, f"{d.tool_id} has empty category"

    def test_schema_scan_ports(self):
        from eggsec import ToolRegistry
        s = ToolRegistry.schema("scan_ports")
        assert s is not None
        parsed = json.loads(s)
        assert "$schema" in parsed
        assert "properties" in parsed

    def test_schema_nonexistent(self):
        from eggsec import ToolRegistry
        assert ToolRegistry.schema("nonexistent_tool") is None

    def test_operations_for_feature(self):
        from eggsec import ToolRegistry
        db_ops = ToolRegistry.operations_for_feature("db-pentest")
        assert len(db_ops) >= 1
        for op in db_ops:
            assert op.feature_required == "db-pentest"

    def test_operations_for_category(self):
        from eggsec import ToolRegistry
        scanning = ToolRegistry.operations_for_category("scanning")
        assert len(scanning) >= 1
        for op in scanning:
            assert op.category == "scanning"


class TestToolDescriptor:
    def test_to_dict(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        dd = d.to_dict()
        assert isinstance(dd, dict)
        expected_keys = {
            "tool_id", "operation_id", "title", "description", "version",
            "category", "risk", "feature_required", "maturity",
            "confirmation_required", "target_policy", "input_schema",
            "output_schema", "supports_streaming", "supports_cancellation",
            "supports_timeout", "local_available", "daemon_available",
            "intended_uses", "supported_surfaces",
        }
        assert expected_keys.issubset(set(dd.keys()))

    def test_to_json(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        j = d.to_json()
        parsed = json.loads(j)
        assert parsed["tool_id"] == "scan_ports"
        assert "operation_id" in parsed

    def test_repr_and_str(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        assert "ToolDescriptor" in repr(d)
        s = str(d)
        assert "scan_ports" in s

    def test_category_values(self):
        from eggsec import ToolRegistry
        valid_categories = {
            "scanning", "recon", "fingerprinting", "waf", "fuzzing",
            "load_testing", "assessment", "mobile", "container",
            "database", "nse", "other",
        }
        for d in ToolRegistry.list():
            assert d.category in valid_categories, (
                f"{d.tool_id} has unexpected category: {d.category}"
            )

    def test_maturity_is_stable(self):
        from eggsec import ToolRegistry
        for d in ToolRegistry.list():
            assert d.maturity == "stable", (
                f"{d.tool_id} maturity is {d.maturity}, expected stable"
            )

    def test_confirmation_required_flags(self):
        from eggsec import ToolRegistry
        confirming = [d for d in ToolRegistry.list() if d.confirmation_required]
        assert len(confirming) >= 1
        for d in ToolRegistry.list():
            if not d.confirmation_required:
                assert d.confirmation_message is None


# ---------------------------------------------------------------------------
# 7. SchemaGenerator
# ---------------------------------------------------------------------------

class TestSchemaGenerator:
    def test_generate_input_schema(self):
        from eggsec import SchemaGenerator
        s = SchemaGenerator.generate_input_schema("scan_ports")
        assert s is not None
        parsed = json.loads(s)
        assert "$schema" in parsed
        assert "type" in parsed
        assert parsed["type"] == "object"
        assert "properties" in parsed

    def test_scan_ports_schema_has_target(self):
        from eggsec import SchemaGenerator
        s = SchemaGenerator.generate_input_schema("scan_ports")
        parsed = json.loads(s)
        assert "target" in parsed["properties"]

    def test_all_tools_have_schemas(self):
        from eggsec import SchemaGenerator, ToolRegistry
        for d in ToolRegistry.list():
            s = SchemaGenerator.generate_input_schema(d.tool_id)
            assert s is not None, f"{d.tool_id} has no input schema"
            parsed = json.loads(s)
            assert "$schema" in parsed, f"{d.tool_id} schema missing $schema"

    def test_nonexistent_returns_none(self):
        from eggsec import SchemaGenerator
        assert SchemaGenerator.generate_input_schema("nonexistent") is None

    def test_generate_output_schema(self):
        from eggsec import SchemaGenerator
        s = SchemaGenerator.generate_output_schema("scan_ports")
        assert s is not None
        parsed = json.loads(s)
        assert "$schema" in parsed
        assert "properties" in parsed


# ---------------------------------------------------------------------------
# 8. operation_as_tool
# ---------------------------------------------------------------------------

class TestOperationAsTool:
    def test_valid_operation(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        assert view is not None
        assert view.descriptor.tool_id == "scan_ports"
        assert len(view.descriptor.operation_id) > 0

    def test_nonexistent_returns_none(self):
        from eggsec import operation_as_tool
        assert operation_as_tool("nonexistent_operation") is None

    def test_invoke_description(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        desc = view.invoke_description()
        assert len(desc) > 0
        assert "scan_ports" in desc or "Port Scan" in desc

    def test_request_type_name(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        assert len(view.request_type_name) > 0

    def test_result_type_name(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        assert len(view.result_type_name) > 0

    def test_example_request(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        assert view.example_request is not None
        parsed = json.loads(view.example_request)
        assert "target" in parsed

    def test_to_dict(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        d = view.to_dict()
        assert isinstance(d, dict)
        assert "descriptor" in d
        assert "request_type_name" in d
        assert "result_type_name" in d

    def test_to_json(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        j = view.to_json()
        parsed = json.loads(j)
        assert "request_type_name" in parsed

    def test_repr(self):
        from eggsec import operation_as_tool
        view = operation_as_tool("scan_ports")
        assert "OperationToolView" in repr(view)


# ---------------------------------------------------------------------------
# 9. ValidationReport
# ---------------------------------------------------------------------------

class TestValidationReport:
    def test_valid_payload(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        assert report.valid is True
        assert len(report.errors) == 0

    def test_missing_required_field(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {})
        assert report.valid is False
        assert len(report.errors) > 0
        assert any("target" in e for e in report.errors)

    def test_bool_truth(self):
        from eggsec import ToolRegistry
        valid_report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        invalid_report = ToolRegistry.validate("scan_ports", {})
        assert bool(valid_report) is True
        assert bool(invalid_report) is False

    def test_errors_is_list(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {})
        assert isinstance(report.errors, list)

    def test_warnings_is_list(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        assert isinstance(report.warnings, list)

    def test_unexpected_field_warning(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate(
            "scan_ports", {"target": "example.com", "unknown_field": "value"}
        )
        assert any("unknown_field" in w for w in report.warnings)

    def test_to_dict(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        d = report.to_dict()
        assert isinstance(d, dict)
        assert "valid" in d
        assert "errors" in d
        assert "warnings" in d

    def test_to_json(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        j = report.to_json()
        parsed = json.loads(j)
        assert "valid" in parsed
        assert parsed["valid"] is True

    def test_repr(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {})
        r = repr(report)
        assert "ValidationReport" in r
        assert "valid=false" in r


# ---------------------------------------------------------------------------
# 10. Round-trip serialization
# ---------------------------------------------------------------------------

class TestRoundTripSerialization:
    def test_tool_request_round_trip(self):
        from eggsec import ToolRequest, ToolTarget
        req = ToolRequest.new(
            "scan_ports",
            ToolTarget.url("example.com"),
            params={"ports": "80,443"},
        )
        d = req.to_dict()
        j = req.to_json()
        parsed = json.loads(j)
        assert parsed["tool"] == "scan_ports"
        assert d["tool"] == "scan_ports"
        assert d["id"] == req.id
        assert d["target_value"] == "example.com"

    def test_target_round_trip(self):
        from eggsec import ToolTarget
        t = ToolTarget.url("https://example.com")
        d = t.to_dict()
        j = t.to_json()
        parsed_json = json.loads(j)
        assert d["value"] == "https://example.com"
        assert "value" in parsed_json

    def test_scope_round_trip(self):
        from eggsec import ToolScope
        s = ToolScope.new(
            allowed_patterns=["example.com"],
            allow_subdomains=False,
        )
        d = s.to_dict()
        j = s.to_json()
        parsed = json.loads(j)
        assert "example.com" in parsed["allowed_patterns"]
        assert d["allow_subdomains"] is False

    def test_request_options_round_trip(self):
        from eggsec import ToolRequestOptions
        o = ToolRequestOptions.new(timeout_ms=3000, stealth=True)
        d = o.to_dict()
        j = o.to_json()
        parsed = json.loads(j)
        assert parsed["timeout_ms"] == 3000
        assert parsed["stealth"] is True
        assert d["timeout_ms"] == 3000

    def test_auth_config_round_trip(self):
        from eggsec import ToolAuthConfig
        c = ToolAuthConfig.basic("user", "pass")
        j = c.to_json()
        parsed = json.loads(j)
        assert "basic" in parsed["auth_type"].lower()
        # to_json returns credential keys only, not values
        creds = parsed["credentials"]
        assert isinstance(creds, list)
        assert len(creds) > 0

    def test_tool_descriptor_round_trip(self):
        from eggsec import ToolRegistry
        d = ToolRegistry.get("scan_ports")
        dd = d.to_dict()
        j = d.to_json()
        parsed = json.loads(j)
        assert parsed["tool_id"] == "scan_ports"
        assert dd["tool_id"] == "scan_ports"

    def test_validation_report_round_trip(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "example.com"})
        d = report.to_dict()
        j = report.to_json()
        parsed = json.loads(j)
        assert parsed["valid"] is True
        assert d["valid"] is True

    def test_schema_json_valid(self):
        from eggsec import SchemaGenerator
        s = SchemaGenerator.generate_input_schema("scan_ports")
        parsed = json.loads(s)
        assert parsed["$schema"] == "https://json-schema.org/draft/2020-12/schema"
        assert parsed["type"] == "object"
        assert isinstance(parsed["properties"], dict)


# ---------------------------------------------------------------------------
# Additional: enum cross-type coverage
# ---------------------------------------------------------------------------

class TestEnumHashability:
    """All enum types should be hashable and usable in sets/dicts."""

    def test_target_type_in_set(self):
        from eggsec import ToolTargetType
        s = {ToolTargetType.url(), ToolTargetType.ip(), ToolTargetType.url()}
        assert len(s) == 2

    def test_auth_type_in_dict(self):
        from eggsec import ToolAuthType
        d = {ToolAuthType.basic(): "basic", ToolAuthType.bearer(): "bearer"}
        assert d[ToolAuthType.basic()] == "basic"

    def test_severity_in_set(self):
        from eggsec import ToolSeverity
        s = {ToolSeverity.high(), ToolSeverity.low(), ToolSeverity.high()}
        assert len(s) == 2

    def test_error_type_in_set(self):
        from eggsec import ToolErrorType
        s = {ToolErrorType.timeout(), ToolErrorType.network()}
        assert len(s) == 2


class TestPortData:
    def test_create(self):
        from eggsec import ToolPortData, ToolPortState
        p = ToolPortData(port=80, protocol="tcp", state=ToolPortState.open())
        assert p.port == 80
        assert p.protocol == "tcp"
        assert p.state.value == "open"

    def test_optional_fields(self):
        from eggsec import ToolPortData, ToolPortState
        p = ToolPortData(
            port=443,
            protocol="tcp",
            state=ToolPortState.open(),
            service="https",
            version="1.1",
            banner="nginx",
        )
        assert p.service == "https"
        assert p.version == "1.1"
        assert p.banner == "nginx"

    def test_to_dict(self):
        from eggsec import ToolPortData, ToolPortState
        p = ToolPortData(port=80, protocol="tcp", state=ToolPortState.open())
        d = p.to_dict()
        assert d["port"] == 80
        assert d["state"] == "open"

    def test_to_json(self):
        from eggsec import ToolPortData, ToolPortState
        p = ToolPortData(port=80, protocol="tcp", state=ToolPortState.open())
        j = p.to_json()
        parsed = json.loads(j)
        assert parsed["port"] == 80

    def test_repr_and_str(self):
        from eggsec import ToolPortData, ToolPortState
        p = ToolPortData(port=80, protocol="tcp", state=ToolPortState.open())
        assert "PortDataPy" in repr(p)
        assert "80" in str(p)


class TestEndpointData:
    def test_create(self):
        from eggsec import ToolEndpointData
        e = ToolEndpointData(url="https://example.com/api")
        assert e.url == "https://example.com/api"

    def test_with_status(self):
        from eggsec import ToolEndpointData
        e = ToolEndpointData(
            url="https://example.com/api",
            status_code=200,
            content_length=1024,
            content_type="application/json",
        )
        assert e.status_code == 200
        assert e.content_length == 1024

    def test_to_dict(self):
        from eggsec import ToolEndpointData
        e = ToolEndpointData(url="https://example.com", status_code=404)
        d = e.to_dict()
        assert d["url"] == "https://example.com"
        assert d["status_code"] == 404


class TestTechnologyData:
    def test_create(self):
        from eggsec import ToolTechnologyData
        t = ToolTechnologyData(name="nginx", category="web-server", confidence=0.95)
        assert t.name == "nginx"
        assert t.category == "web-server"
        assert abs(t.confidence - 0.95) < 0.01

    def test_optional_fields(self):
        from eggsec import ToolTechnologyData
        t = ToolTechnologyData(
            name="nginx",
            category="web-server",
            version="1.21.0",
            website="https://nginx.org",
            cpe="cpe:2.3:a:nginx:nginx:1.21.0",
        )
        assert t.version == "1.21.0"
        assert t.website == "https://nginx.org"
        assert t.cpe is not None

    def test_to_dict(self):
        from eggsec import ToolTechnologyData
        t = ToolTechnologyData(name="php", category="language", confidence=0.8)
        d = t.to_dict()
        assert d["name"] == "php"
        assert d["category"] == "language"


class TestRateLimitConfig:
    def test_standard(self):
        from eggsec import ToolRateLimitConfig
        c = ToolRateLimitConfig.standard()
        assert c.requests_per_minute > 0
        assert c.concurrent_scans > 0

    def test_relaxed(self):
        from eggsec import ToolRateLimitConfig
        c = ToolRateLimitConfig.relaxed()
        assert c.requests_per_minute >= ToolRateLimitConfig.standard().requests_per_minute

    def test_strict(self):
        from eggsec import ToolRateLimitConfig
        c = ToolRateLimitConfig.strict()
        assert c.requests_per_minute <= ToolRateLimitConfig.standard().requests_per_minute

    def test_custom(self):
        from eggsec import ToolRateLimitConfig
        c = ToolRateLimitConfig(
            requests_per_minute=120,
            concurrent_scans=10,
            burst_size=20,
            enable_ip_based_limiting=True,
        )
        assert c.requests_per_minute == 120
        assert c.concurrent_scans == 10
        assert c.enable_ip_based_limiting is True

    def test_to_dict(self):
        from eggsec import ToolRateLimitConfig
        d = ToolRateLimitConfig.standard().to_dict()
        assert isinstance(d, dict)
        assert "requests_per_minute" in d


class TestToolError:
    def test_create(self):
        from eggsec import ToolError
        e = ToolError(code="NET_TIMEOUT", message="Connection timed out")
        assert e.code == "NET_TIMEOUT"
        assert e.message == "Connection timed out"
        assert e.recoverable is False

    def test_with_type(self):
        from eggsec import ToolError, ToolErrorType
        e = ToolError(
            code="AUTH_FAIL",
            message="Invalid credentials",
            error_type=ToolErrorType.authentication(),
            recoverable=True,
            retry_after_ms=5000,
        )
        assert e.error_type.value == "authentication"
        assert e.recoverable is True
        assert e.retry_after_ms == 5000

    def test_to_dict(self):
        from eggsec import ToolError
        e = ToolError(code="E001", message="test error")
        d = e.to_dict()
        assert d["code"] == "E001"
        assert d["message"] == "test error"

    def test_repr_and_str(self):
        from eggsec import ToolError
        e = ToolError(code="E001", message="test error")
        assert "ToolErrorPy" in repr(e)
        assert "E001" in str(e)
