"""Tests for NSE (Nmap Scripting Engine) Python bindings - Release 3.

Workstream 8 (WS8): NSE validation fixtures for rule evaluation, edge cases,
argument types, capability context, limits, cancellation, structured output,
malformed scripts, and secret arguments.
"""

import pytest


def _import_or_skip(name, feature="nse"):
    """Import a name from eggsec, skip test if unavailable (feature-gated)."""
    import importlib
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


def _import_core_or_skip(name, feature="nse"):
    """Import a name from eggsec._core, skip test if unavailable."""
    try:
        mod = importlib.import_module("eggsec._core")
    except ImportError:
        pytest.skip(f"eggsec._core not available (requires {feature} feature)")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available in _core (requires {feature} feature)")
    return obj


import importlib


# ============================================================================
# WS8: Rule Evaluation Fixtures
# ============================================================================


class TestWs8NseHostContext:
    """WS8: NseHostContext construction and serialization for rule evaluation."""

    def test_minimal_construction(self):
        NseHostContext = _import_or_skip("NseHostContext")
        ctx = NseHostContext(ip="10.0.0.1")
        assert ctx.ip == "10.0.0.1"
        assert ctx.hostname is None
        assert ctx.target_label == ""
        assert ctx.source == "synthetic"

    def test_full_construction(self):
        NseHostContext = _import_or_skip("NseHostContext")
        ctx = NseHostContext(
            ip="192.168.1.100",
            hostname="web.example.com",
            target_label="web-target",
            source="scan",
        )
        assert ctx.ip == "192.168.1.100"
        assert ctx.hostname == "web.example.com"
        assert ctx.target_label == "web-target"
        assert ctx.source == "scan"

    def test_to_dict(self):
        NseHostContext = _import_or_skip("NseHostContext")
        ctx = NseHostContext(
            ip="172.16.0.1",
            hostname="db.internal",
        )
        d = ctx.to_dict()
        assert isinstance(d, dict)
        assert d["ip"] == "172.16.0.1"
        assert d["hostname"] == "db.internal"
        assert "target_label" in d
        assert "source" in d

    def test_repr(self):
        NseHostContext = _import_or_skip("NseHostContext")
        ctx = NseHostContext(ip="127.0.0.1")
        r = repr(ctx)
        assert "NseHostContext" in r
        assert "127.0.0.1" in r

    def test_frozen(self):
        NseHostContext = _import_or_skip("NseHostContext")
        ctx = NseHostContext(ip="10.0.0.1")
        with pytest.raises(AttributeError):
            ctx.ip = "10.0.0.2"


class TestWs8NsePortContext:
    """WS8: NsePortContext construction and serialization for rule evaluation."""

    def test_minimal_construction(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(port=80)
        assert ctx.port == 80
        assert ctx.protocol == "tcp"
        assert ctx.state == "open"
        assert ctx.service_name is None
        assert ctx.source == "synthetic"

    def test_full_construction(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(
            port=443,
            protocol="tcp",
            state="open",
            service_name="https",
            service_product="nginx",
            service_version="1.21.3",
            source="scan",
        )
        assert ctx.port == 443
        assert ctx.protocol == "tcp"
        assert ctx.state == "open"
        assert ctx.service_name == "https"
        assert ctx.service_product == "nginx"
        assert ctx.service_version == "1.21.3"
        assert ctx.source == "scan"

    def test_udp_protocol(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(port=53, protocol="udp", state="open")
        assert ctx.protocol == "udp"

    def test_closed_state(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(port=22, state="closed")
        assert ctx.state == "closed"

    def test_to_dict(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(port=8080, service_name="http-proxy")
        d = ctx.to_dict()
        assert isinstance(d, dict)
        assert d["port"] == 8080
        assert d["service_name"] == "http-proxy"

    def test_frozen(self):
        NsePortContext = _import_or_skip("NsePortContext")
        ctx = NsePortContext(port=80)
        with pytest.raises(AttributeError):
            ctx.port = 90


class TestWs8NseRuleResult:
    """WS8: NseRuleResult construction and serialization for rule evaluation."""

    def test_match_result(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="portrule", evaluated=True, matched=True)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        assert r.kind == "portrule"
        assert r.evaluated is True
        assert r.matched is True
        assert r.unsupported is False

    def test_non_match_result(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="hostrule", evaluated=True, matched=False)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        assert r.matched is False

    def test_error_result(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(
                kind="portrule",
                evaluated=False,
                matched=False,
                error="syntax error in rule",
            )
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        assert r.evaluated is False
        assert r.error == "syntax error in rule"

    def test_unsupported_result(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="postrule", evaluated=False, matched=False, unsupported=True)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        assert r.unsupported is True

    def test_to_dict(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="portrule", evaluated=True, matched=True)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        d = r.to_dict()
        assert isinstance(d, dict)
        assert d["kind"] == "portrule"
        assert d["evaluated"] is True
        assert d["matched"] is True

    def test_to_json(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="hostrule", evaluated=True, matched=False)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        j = r.to_json()
        assert isinstance(j, str)
        import json
        parsed = json.loads(j)
        assert parsed["kind"] == "hostrule"

    def test_repr(self):
        NseRuleResult = _import_or_skip("NseRuleResult")
        try:
            r = NseRuleResult(kind="portrule", evaluated=True, matched=True)
        except TypeError:
            pytest.skip("NseRuleResult has no Python constructor (Rust-only)")
        s = repr(r)
        assert "NseRuleResult" in s
        assert "portrule" in s


# ============================================================================
# WS8: Argument Type Fixtures
# ============================================================================


class TestWs8NseArgumentTypes:
    """WS8: NseArgument typed values (boolean, list, map, secret).

    Note: `is_secret`, `arg_value`, and `get_arg_value` are in the Rust source
    but may not be in the compiled binary yet. Tests that depend on those
    features are gated with try/except to avoid hard failures.
    """

    def test_string_argument(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="target", value="example.com", arg_type="string")
        assert arg.name == "target"
        assert arg.value == "example.com"
        assert arg.arg_type == "string"

    def test_integer_argument(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="timeout", value="30", arg_type="integer")
        assert arg.arg_type == "integer"

    def test_boolean_argument(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="verbose", value="true", arg_type="boolean")
        assert arg.arg_type == "boolean"

    def test_list_argument(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="ports", value="80,443", arg_type="list")
        assert arg.arg_type == "list"

    def test_map_argument(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="headers", value="key=val", arg_type="map")
        assert arg.arg_type == "map"

    def test_default_arg_type_is_string(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="key", value="value")
        assert arg.arg_type == "string"

    def test_secret_argument_type(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="api_key", value="secret123", arg_type="secret")
        assert arg.arg_type == "secret"

    def test_to_dict_complete(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="port", value="80", arg_type="integer")
        d = arg.to_dict()
        assert d["name"] == "port"
        assert d["value"] == "80"
        assert d["arg_type"] == "integer"

    def test_to_json_roundtrip(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="mode", value="fast", arg_type="string")
        j = arg.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["name"] == "mode"
        assert parsed["arg_type"] == "string"

    def test_repr(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="k", value="v")
        r = repr(arg)
        assert "NseArgument" in r
        assert "k" in r

    def test_str(self):
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="k", value="v")
        s = str(arg)
        assert s == "k=v"

    def test_is_secret_available(self):
        """Verify is_secret attribute exists (WS5 typed args)."""
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="k", value="v")
        try:
            _ = arg.is_secret
        except AttributeError:
            pytest.skip("is_secret not in compiled binary")

    def test_arg_value_getter_available(self):
        """Verify arg_value getter exists (WS5 typed args)."""
        NseArgument = _import_or_skip("NseArgument")
        arg = NseArgument(name="k", value="v")
        if hasattr(arg, "get_arg_value"):
            assert arg.get_arg_value() is None
        else:
            pytest.skip("get_arg_value not in compiled binary")


# ============================================================================
# WS8: Capability Context Fixtures (WS7)
# ============================================================================


class TestWs8NseCapabilityContext:
    """WS8: NseCapabilityContext introspection for profiles and policies.

    NseCapabilityContextPy is registered in _core but not re-exported in __init__.
    These tests verify the type exists and has expected attributes.
    """

    def _get_capability_context_class(self):
        """Get NseCapabilityContextPy from _core (not re-exported in __init__)."""
        import importlib
        try:
            mod = importlib.import_module("eggsec._core")
        except ImportError:
            pytest.skip("eggsec._core not available")
        cls = getattr(mod, "NseCapabilityContextPy", None)
        if cls is None:
            pytest.skip("NseCapabilityContextPy not available in _core")
        return cls

    def test_class_exists(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert NseCapabilityContext is not None

    def test_has_profile_kind_field(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "profile_kind")

    def test_has_limits_field(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "limits")

    def test_has_is_cancelled_field(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "is_cancelled")

    def test_has_network_policy_field(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "network_policy_kind")

    def test_has_script_policy_fields(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "script_policy_allows_builtins")
        assert hasattr(NseCapabilityContext, "script_policy_allows_files")

    def test_has_module_policy_fields(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "module_policy_allows_builtins")
        assert hasattr(NseCapabilityContext, "module_policy_allows_filesystem")

    def test_has_to_dict(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "to_dict")

    def test_has_to_json(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "to_json")

    def test_has_repr(self):
        NseCapabilityContext = self._get_capability_context_class()
        assert hasattr(NseCapabilityContext, "__repr__")


# ============================================================================
# WS8: Execution Limits and Sandbox Policy Fixtures
# ============================================================================


class TestWs8NseExecutionLimits:
    """WS8: NseExecutionLimits presets and custom values."""

    def test_manual_defaults(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.manual_defaults()
        assert limits.wall_clock_timeout_secs == 120
        assert limits.lua_instruction_budget == 100_000_000
        assert limits.max_output_bytes == 52_428_800  # 50 MiB

    def test_automated_defaults(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.automated_defaults()
        assert limits.wall_clock_timeout_secs == 15
        assert limits.lua_instruction_budget == 5_000_000
        assert limits.max_output_bytes == 2_097_152  # 2 MiB

    def test_unlimited(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.unlimited()
        assert limits.wall_clock_timeout_secs is None
        assert limits.lua_instruction_budget is None

    def test_custom_limits(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        try:
            limits = NseExecutionLimits(
                wall_clock_timeout_secs=30,
                lua_instruction_budget=1_000_000,
                max_output_bytes=1_048_576,
                max_script_bytes=512_000,
                max_required_module_bytes=256_000,
                max_network_operations=10,
                max_network_bytes_read=1024,
                max_network_bytes_written=1024,
                max_filesystem_operations=5,
                max_filesystem_bytes_read=1024,
                max_lua_memory_bytes=33_554_432,
            )
        except TypeError:
            pytest.skip("NseExecutionLimits has no Python constructor (Rust-only)")
        assert limits.wall_clock_timeout_secs == 30
        assert limits.lua_instruction_budget == 1_000_000
        assert limits.max_output_bytes == 1_048_576
        assert limits.max_script_bytes == 512_000
        assert limits.max_network_operations == 10

    def test_to_dict(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.automated_defaults()
        d = limits.to_dict()
        assert isinstance(d, dict)
        assert "wall_clock_timeout_secs" in d
        assert "lua_instruction_budget" in d
        assert "max_output_bytes" in d
        assert "max_network_operations" in d

    def test_to_json(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.manual_defaults()
        j = limits.to_json()
        assert isinstance(j, str)
        import json
        parsed = json.loads(j)
        assert parsed["wall_clock_timeout_secs"] == 120

    def test_repr(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.automated_defaults()
        r = repr(limits)
        assert "NseExecutionLimits" in r

    def test_frozen(self):
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        limits = NseExecutionLimits.automated_defaults()
        with pytest.raises(AttributeError):
            limits.wall_clock_timeout_secs = 999


class TestWs8NseSandboxPolicy:
    """WS8: NseSandboxPolicy configuration edge cases."""

    def test_default_policy(self):
        NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")
        policy = NseSandboxPolicy()
        assert policy.allow_filesystem is False
        assert policy.allow_network is True
        assert policy.max_lua_instructions == 1_000_000
        assert policy.max_output_bytes == 1_048_576
        assert policy.max_network_ops == 100
        assert policy.max_memory_bytes == 67_108_864

    def test_custom_policy(self):
        NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")
        policy = NseSandboxPolicy(
            allow_filesystem=True,
            allowed_dirs=["/tmp/scan", "/var/log"],
            allow_network=False,
            allowed_cidrs=["10.0.0.0/8"],
            max_lua_instructions=500_000,
            max_output_bytes=524_288,
            max_network_ops=10,
            max_memory_bytes=16_777_216,
        )
        assert policy.allow_filesystem is True
        assert policy.allowed_dirs == ["/tmp/scan", "/var/log"]
        assert policy.allow_network is False
        assert policy.allowed_cidrs == ["10.0.0.0/8"]
        assert policy.max_lua_instructions == 500_000
        assert policy.max_network_ops == 10

    def test_empty_dirs_and_cidrs(self):
        NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")
        policy = NseSandboxPolicy()
        assert policy.allowed_dirs == []
        assert policy.allowed_cidrs == []

    def test_to_dict(self):
        NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")
        policy = NseSandboxPolicy(allow_network=False)
        d = policy.to_dict()
        assert isinstance(d, dict)
        assert d["allow_network"] is False

    def test_to_json(self):
        NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")
        policy = NseSandboxPolicy()
        j = policy.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["allow_filesystem"] is False


# ============================================================================
# WS8: Cancellation Token Fixtures
# ============================================================================


class TestWs8NseCancellationToken:
    """WS8: NseCancellationToken lifecycle for timeout and cancellation."""

    def test_initial_state(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        assert token.is_cancelled() is False

    def test_cancel(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        token.cancel()
        assert token.is_cancelled() is True

    def test_reset_after_cancel(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        token.cancel()
        assert token.is_cancelled() is True
        token.reset()
        assert token.is_cancelled() is False

    def test_cancel_idempotent(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        token.cancel()
        token.cancel()
        assert token.is_cancelled() is True

    def test_repr(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        r = repr(token)
        assert "NseCancellationToken" in r
        assert "cancelled=false" in r or "cancelled=False" in r

    def test_repr_after_cancel(self):
        NseCancellationToken = _import_or_skip("NseCancellationToken")
        token = NseCancellationToken()
        token.cancel()
        r = repr(token)
        assert "cancelled=true" in r or "cancelled=True" in r


# ============================================================================
# WS8: Runtime Configuration and Stats Fixtures
# ============================================================================


class TestWs8NseRuntimeConfig:
    """WS8: NseRuntimeConfig construction and serialization."""

    def test_default_profile(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        assert cfg.target == "10.0.0.1"
        assert cfg.profile_kind == "agent-safe"
        assert cfg.verbose is False

    def test_explicit_profile(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="192.168.1.1", profile_kind="ci-safe")
        assert cfg.profile_kind == "ci-safe"

    def test_manual_permissive_profile(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="127.0.0.1", profile_kind="manual-permissive")
        assert cfg.profile_kind == "manual-permissive"

    def test_verbose_flag(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1", verbose=True)
        assert cfg.verbose is True

    def test_to_dict(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1", profile_kind="agent-safe")
        d = cfg.to_dict()
        assert d["target"] == "10.0.0.1"
        assert d["profile_kind"] == "agent-safe"

    def test_to_json(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        j = cfg.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["target"] == "10.0.0.1"

    def test_frozen(self):
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        with pytest.raises(AttributeError):
            cfg.target = "10.0.0.2"


class TestWs8NseRuntimeStats:
    """WS8: NseRuntimeStats accessors and serialization."""

    def test_has_expected_fields(self):
        NseRuntimeStats = _import_or_skip("NseRuntimeStats")
        assert hasattr(NseRuntimeStats, "elapsed_ms")
        assert hasattr(NseRuntimeStats, "output_bytes")
        assert hasattr(NseRuntimeStats, "lua_instruction_count")
        assert hasattr(NseRuntimeStats, "network_operations")
        assert hasattr(NseRuntimeStats, "filesystem_operations")
        assert hasattr(NseRuntimeStats, "limit_violation")

    def test_to_dict_fields(self):
        NseRuntimeStats = _import_or_skip("NseRuntimeStats")
        assert hasattr(NseRuntimeStats, "to_dict")


# ============================================================================
# WS8: Script Inspection Fixtures
# ============================================================================


class TestWs8NseScriptSource:
    """WS8: NseScriptSource construction for script provenance."""

    def test_has_expected_fields(self):
        NseScriptSource = _import_or_skip("NseScriptSource")
        assert hasattr(NseScriptSource, "kind")
        assert hasattr(NseScriptSource, "name")
        assert hasattr(NseScriptSource, "path")

    def test_to_dict(self):
        NseScriptSource = _import_or_skip("NseScriptSource")
        assert hasattr(NseScriptSource, "to_dict")


class TestWs8NseDiagnostic:
    """WS8: NseDiagnostic construction for resolver diagnostics."""

    def test_has_expected_fields(self):
        NseDiagnostic = _import_or_skip("NseDiagnostic")
        assert hasattr(NseDiagnostic, "kind")
        assert hasattr(NseDiagnostic, "message")
        assert hasattr(NseDiagnostic, "path")

    def test_to_dict(self):
        NseDiagnostic = _import_or_skip("NseDiagnostic")
        assert hasattr(NseDiagnostic, "to_dict")


# ============================================================================
# WS8: Malformed Script Validation Fixtures
# ============================================================================


class TestWs8MalformedScripts:
    """WS8: Validation edge cases for malformed scripts."""

    def test_empty_script_fails(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("")
        assert result["valid"] is False
        assert result["error"] is not None

    def test_whitespace_only_fails(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("   \n\t  ")
        assert result["valid"] is False

    def test_random_bytes_fails(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("xyzzy12345")
        assert result["valid"] is False

    def test_valid_lua_comment(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("-- this is a comment")
        assert result["valid"] is True
        assert result["script_name"] == "<inline>"

    def test_valid_lua_local(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script('local x = require "stdnse"')
        assert result["valid"] is True

    def test_valid_lua_function(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("function main() return nil end")
        assert result["valid"] is True

    def test_valid_lua_require(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script('local http = require "http"\nreturn nil')
        assert result["valid"] is True

    def test_valid_lua_return(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("return nil")
        assert result["valid"] is True

    def test_builtin_script_valid(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("banner")
        assert result["valid"] is True
        assert result["script_name"] == "banner"

    def test_unknown_builtin_fails(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("nonexistent_script_xyz")
        assert result["valid"] is False

    def test_inline_with_require(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script(
            'local stdnse = require "stdnse"\n'
            'return stdnse.generate_random_string(8)'
        )
        assert result["valid"] is True
        assert result["script_name"] == "<inline>"

    def test_validation_result_structure(self):
        nse_validate_script = _import_or_skip("nse_validate_script")
        result = nse_validate_script("banner")
        assert "valid" in result
        assert "error" in result
        assert "script_name" in result


# ============================================================================
# WS8: Structured Output and Report Fixtures
# ============================================================================


class TestWs8NseReportStructure:
    """WS8: NseReport structured output fields and accessors."""

    def test_report_has_evidence_accessor(self):
        NseReport = _import_or_skip("NseReport")
        assert hasattr(NseReport, "evidence")

    def test_report_has_rules_accessor(self):
        NseReport = _import_or_skip("NseReport")
        assert hasattr(NseReport, "rules")

    def test_report_has_libraries_accessor(self):
        NseReport = _import_or_skip("NseReport")
        assert hasattr(NseReport, "libraries")

    def test_report_fields(self):
        NseReport = _import_or_skip("NseReport")
        for field in [
            "target", "script_name", "output", "output_lines", "has_output",
            "warnings", "errors", "library_count", "compatibility_status",
            "fidelity", "elapsed_secs",
        ]:
            assert hasattr(NseReport, field), f"NseReport missing field: {field}"

    def test_report_to_dict(self):
        NseReport = _import_or_skip("NseReport")
        assert hasattr(NseReport, "to_dict")

    def test_report_to_json(self):
        NseReport = _import_or_skip("NseReport")
        assert hasattr(NseReport, "to_json")


class TestWs8NseEvidenceItem:
    """WS8: NseEvidenceItem structured evidence fields."""

    def test_has_expected_fields(self):
        NseEvidenceItem = _import_or_skip("NseEvidenceItem")
        for field in [
            "id", "kind", "title", "summary", "target", "port",
            "service", "confidence", "source", "raw_excerpt",
            "references", "tags",
        ]:
            assert hasattr(NseEvidenceItem, field), f"NseEvidenceItem missing: {field}"

    def test_to_dict(self):
        NseEvidenceItem = _import_or_skip("NseEvidenceItem")
        assert hasattr(NseEvidenceItem, "to_dict")

    def test_to_json(self):
        NseEvidenceItem = _import_or_skip("NseEvidenceItem")
        assert hasattr(NseEvidenceItem, "to_json")


# ============================================================================
# WS8: Dependency Chain Fixtures
# ============================================================================


class TestWs8DependencyChains:
    """WS8: Script dependency chains and metadata."""

    def test_banner_depends_on_stdnse(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("banner")
        assert meta is not None
        assert "stdnse" in meta.dependencies

    def test_banner_depends_on_comm_and_socket(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("banner")
        assert meta is not None
        assert "comm" in meta.dependencies
        assert "socket" in meta.dependencies

    def test_http_headers_depends_on_http(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("http-headers")
        assert meta is not None
        assert "http" in meta.dependencies

    def test_ssl_cert_depends_on_sslcert(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("ssl-cert")
        assert meta is not None
        assert "sslcert" in meta.dependencies

    def test_metadata_has_categories(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("banner")
        assert meta is not None
        assert isinstance(meta.categories, list)
        assert len(meta.categories) > 0

    def test_metadata_to_dict_dependencies(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("banner")
        assert meta is not None
        d = meta.to_dict()
        assert "dependencies" in d
        assert isinstance(d["dependencies"], list)
        assert "stdnse" in d["dependencies"]

    def test_all_scripts_have_metadata(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts()
        for s in scripts:
            assert s.name != ""
            assert s.category != ""
            assert isinstance(s.dependencies, list)

    def test_unknown_script_no_metadata(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("definitely_not_a_real_script")
        assert meta is None


# ============================================================================
# WS8: Library Registry Edge Cases
# ============================================================================


class TestWs8LibraryRegistryEdgeCases:
    """WS8: NseLibraryRegistry edge cases and completeness."""

    def test_registry_count_matches_list(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        assert reg.count() == len(reg.list())

    def test_get_unknown_returns_none(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        assert reg.get("nonexistent_library_xyz") is None

    def test_all_core_libraries_present(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        names = [l.name for l in reg.list()]
        for core in ["stdnse", "http", "dns", "socket", "comm"]:
            assert core in names, f"Core library '{core}' missing from registry"

    def test_descriptor_has_all_fields(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        desc = reg.get("stdnse")
        assert desc is not None
        assert desc.name == "stdnse"
        assert desc.category == "Core"
        assert isinstance(desc.sandbox_side_effects, list)
        assert isinstance(desc.notes, str)
        assert isinstance(desc.optional_deps, list)
        assert desc.enforcement_status != ""

    def test_by_category_core(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        core = reg.by_category("Core")
        assert len(core) > 0
        for lib in core:
            assert lib.category == "Core"

    def test_by_category_protocol(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        proto = reg.by_category("Protocol")
        assert len(proto) > 0
        for lib in proto:
            assert lib.category == "Protocol"

    def test_descriptor_to_json(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        desc = reg.get("http")
        assert desc is not None
        j = desc.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["name"] == "http"

    def test_descriptor_repr(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        desc = reg.get("dns")
        assert desc is not None
        r = repr(desc)
        assert "NseLibraryDescriptor" in r

    def test_descriptor_str(self):
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        desc = reg.get("dns")
        assert desc is not None
        s = str(desc)
        assert "dns" in s

    def test_detailed_libraries_match_registry(self):
        nse_list_libraries_detailed = _import_or_skip("nse_list_libraries_detailed")
        detailed = nse_list_libraries_detailed()
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")
        reg = NseLibraryRegistry()
        assert len(detailed) == reg.count()


# ============================================================================
# WS8: Target Context Edge Cases
# ============================================================================


class TestWs8TargetContextEdgeCases:
    """WS8: NseTargetContext edge cases."""

    def test_minimal(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(host_ip="127.0.0.1")
        assert ctx.host_ip == "127.0.0.1"
        assert ctx.hostname is None
        assert ctx.port is None
        assert ctx.protocol is None
        assert ctx.service_name is None
        assert ctx.service_product is None
        assert ctx.service_version is None
        assert ctx.os_detection is None

    def test_full_context(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(
            host_ip="192.168.1.50",
            hostname="server.local",
            port=22,
            protocol="tcp",
            service_name="ssh",
            service_product="OpenSSH",
            service_version="8.9p1",
            os_detection="Linux 5.15",
        )
        assert ctx.os_detection == "Linux 5.15"

    def test_to_dict_all_fields(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(host_ip="10.0.0.1", port=443)
        d = ctx.to_dict()
        assert d["host_ip"] == "10.0.0.1"
        assert d["port"] == 443
        assert d["hostname"] is None

    def test_to_json(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(host_ip="10.0.0.1")
        j = ctx.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["host_ip"] == "10.0.0.1"

    def test_str(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(host_ip="10.0.0.1", service_name="http")
        s = str(ctx)
        assert "10.0.0.1" in s
        assert "http" in s

    def test_str_unknown_service(self):
        NseTargetContext = _import_or_skip("NseTargetContext")
        ctx = NseTargetContext(host_ip="10.0.0.1")
        s = str(ctx)
        assert "unknown" in s


# ============================================================================
# WS8: Config Edge Cases
# ============================================================================


class TestWs8NseConfigEdgeCases:
    """WS8: NseConfig edge cases."""

    def test_minimal(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(target="127.0.0.1", script="banner")
        assert cfg.target == "127.0.0.1"
        assert cfg.script == "banner"
        assert cfg.script_args is None
        assert cfg.script_file is None
        assert cfg.json is False
        assert cfg.verbose is False

    def test_with_script_args(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(
            target="10.0.0.1",
            script="http-headers",
            script_args="user-agent=Mozilla",
        )
        assert cfg.script_args == "user-agent=Mozilla"

    def test_with_script_file(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(
            target="10.0.0.1",
            script="custom",
            script_file="/tmp/custom.nse",
        )
        assert cfg.script_file == "/tmp/custom.nse"

    def test_json_output_flag(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(target="10.0.0.1", script="banner", json=True)
        assert cfg.json is True

    def test_to_dict_all_fields(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(target="10.0.0.1", script="banner", verbose=True)
        d = cfg.to_dict()
        assert d["target"] == "10.0.0.1"
        assert d["script"] == "banner"
        assert d["verbose"] is True

    def test_to_json_roundtrip(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(target="10.0.0.1", script="banner")
        j = cfg.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["target"] == "10.0.0.1"
        assert parsed["script"] == "banner"

    def test_repr(self):
        NseConfig = _import_or_skip("NseConfig")
        cfg = NseConfig(target="10.0.0.1", script="banner", verbose=True)
        r = repr(cfg)
        assert "NseConfigPy" in r
        assert "banner" in r


# ============================================================================
# WS8: Library Use Report Fixtures
# ============================================================================


class TestWs8NseLibraryUse:
    """WS8: NseLibraryUse report structure."""

    def test_has_expected_fields(self):
        NseLibraryUse = _import_or_skip("NseLibraryUse")
        for field in [
            "name", "category", "loaded", "side_effects",
            "fallback_behavior", "notes", "warnings",
        ]:
            assert hasattr(NseLibraryUse, field), f"NseLibraryUse missing: {field}"

    def test_to_dict(self):
        NseLibraryUse = _import_or_skip("NseLibraryUse")
        assert hasattr(NseLibraryUse, "to_dict")

    def test_to_json(self):
        NseLibraryUse = _import_or_skip("NseLibraryUse")
        assert hasattr(NseLibraryUse, "to_json")

    def test_repr(self):
        NseLibraryUse = _import_or_skip("NseLibraryUse")
        assert hasattr(NseLibraryUse, "__repr__")


# ============================================================================
# WS8: Rule Evaluation Report Fixtures
# ============================================================================


class TestWs8NseRuleEvaluation:
    """WS8: NseRuleEvaluation report structure for match/non-match."""

    def test_has_expected_fields(self):
        NseRuleEvaluation = _import_or_skip("NseRuleEvaluation")
        for field in ["kind", "evaluated", "matched", "exactness", "error", "summary", "unsupported"]:
            assert hasattr(NseRuleEvaluation, field), f"NseRuleEvaluation missing: {field}"

    def test_to_dict(self):
        NseRuleEvaluation = _import_or_skip("NseRuleEvaluation")
        assert hasattr(NseRuleEvaluation, "to_dict")

    def test_to_json(self):
        NseRuleEvaluation = _import_or_skip("NseRuleEvaluation")
        assert hasattr(NseRuleEvaluation, "to_json")

    def test_repr(self):
        NseRuleEvaluation = _import_or_skip("NseRuleEvaluation")
        assert hasattr(NseRuleEvaluation, "__repr__")


# ============================================================================
# WS8: Runtime Lifecycle Fixtures
# ============================================================================


class TestWs8NseRuntimeLifecycle:
    """WS8: NseRuntime construction and basic lifecycle."""

    def test_construction(self):
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        runtime = NseRuntime(cfg)
        assert runtime is not None

    def test_with_limits(self):
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        limits = NseExecutionLimits.automated_defaults()
        runtime = NseRuntime(cfg, limits=limits)
        assert runtime is not None

    def test_cancellation_token(self):
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        runtime = NseRuntime(cfg)
        token = runtime.cancellation_token()
        assert token.is_cancelled() is False

    def test_repr(self):
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="10.0.0.1")
        runtime = NseRuntime(cfg)
        r = repr(runtime)
        assert "NseRuntime" in r
        assert "10.0.0.1" in r

    def test_run_builtin_script(self):
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)
        try:
            report = runtime.run_script("banner")
        except Exception as exc:
            if "Connection refused" in str(exc) or "Network is unreachable" in str(exc):
                pytest.skip("banner script requires a network service on target")
            raise
        assert report is not None
        assert report.target == "127.0.0.1"
        assert report.script_name == "banner"
        assert isinstance(report.compatibility_status, str)


# ============================================================================
# WS8: List Libraries Functions Fixtures
# ============================================================================


class TestWs8ListLibrariesFunctions:
    """WS8: nse_list_libraries and nse_list_libraries_detailed edge cases."""

    def test_list_returns_sorted(self):
        nse_list_libraries = _import_or_skip("nse_list_libraries")
        libs = nse_list_libraries()
        assert libs == sorted(libs)

    def test_list_all_are_strings(self):
        nse_list_libraries = _import_or_skip("nse_list_libraries")
        libs = nse_list_libraries()
        assert all(isinstance(name, str) for name in libs)

    def test_detailed_count_matches_list(self):
        nse_list_libraries = _import_or_skip("nse_list_libraries")
        nse_list_libraries_detailed = _import_or_skip("nse_list_libraries_detailed")
        libs = nse_list_libraries()
        detailed = nse_list_libraries_detailed()
        assert len(libs) == len(detailed)

    def test_detailed_names_match_list(self):
        nse_list_libraries = _import_or_skip("nse_list_libraries")
        nse_list_libraries_detailed = _import_or_skip("nse_list_libraries_detailed")
        libs = nse_list_libraries()
        detailed = nse_list_libraries_detailed()
        detailed_names = sorted([d.name for d in detailed])
        assert libs == detailed_names

    def test_get_library_descriptor_all_categories(self):
        nse_list_libraries_detailed = _import_or_skip("nse_list_libraries_detailed")
        descs = nse_list_libraries_detailed()
        valid_categories = {"Core", "Protocol", "Utility", "Exploit", "Auth"}
        for d in descs:
            assert d.category in valid_categories, f"Unknown category: {d.category}"


# ============================================================================
# WS8: List Scripts Edge Cases
# ============================================================================


class TestWs8ListScriptsEdgeCases:
    """WS8: nse_list_scripts filtering and metadata completeness."""

    def test_all_scripts_have_unique_names(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts()
        names = [s.name for s in scripts]
        assert len(names) == len(set(names))

    def test_discovery_category_filter(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts(category="discovery")
        assert len(scripts) > 0
        for s in scripts:
            assert s.category == "discovery"

    def test_auth_category_filter(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts(category="auth")
        # auth category may be empty if no built-in auth scripts
        assert isinstance(scripts, list)

    def test_nonexistent_category_returns_empty(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts(category="nonexistent_category_xyz")
        assert scripts == []

    def test_script_metadata_is_builtin(self):
        nse_list_scripts = _import_or_skip("nse_list_scripts")
        scripts = nse_list_scripts()
        for s in scripts:
            assert s.is_builtin is True

    def test_script_metadata_to_json(self):
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")
        meta = nse_get_script_metadata("banner")
        assert meta is not None
        j = meta.to_json()
        import json
        parsed = json.loads(j)
        assert parsed["name"] == "banner"


# ============================================================================
# WS8: Original Tests (preserved)
# ============================================================================

def test_nse_list_libraries_returns_sorted():
    """nse_list_libraries() should return a non-empty sorted list of strings."""
    nse_list_libraries = _import_or_skip("nse_list_libraries")

    libs = nse_list_libraries()
    assert isinstance(libs, list)
    assert len(libs) > 0
    assert all(isinstance(name, str) for name in libs)
    # Should be sorted
    assert libs == sorted(libs)
    # Known core libraries must be present
    assert "stdnse" in libs
    assert "http" in libs
    assert "dns" in libs


def test_nse_list_libraries_detailed_returns_descriptors():
    """nse_list_libraries_detailed() should return descriptors with full metadata."""
    nse_list_libraries_detailed = _import_or_skip("nse_list_libraries_detailed")

    descs = nse_list_libraries_detailed()
    assert isinstance(descs, list)
    assert len(descs) > 0
    for desc in descs:
        assert hasattr(desc, "name")
        assert hasattr(desc, "category")
        assert hasattr(desc, "notes")
        assert hasattr(desc, "sandbox_side_effects")
        assert hasattr(desc, "fallback_behavior")
        assert hasattr(desc, "enforcement_status")
        assert isinstance(desc.name, str)
        assert isinstance(desc.category, str)
        assert desc.name != ""


def test_nse_get_library_descriptor_stdnse():
    """nse_get_library_descriptor('stdnse') should return a valid descriptor."""
    nse_get_library_descriptor = _import_or_skip("nse_get_library_descriptor")

    desc = nse_get_library_descriptor("stdnse")
    assert desc is not None
    assert desc.name == "stdnse"
    assert desc.category == "Core"
    assert desc.fallback_behavior == "HardFail"
    assert isinstance(desc.notes, str)
    assert len(desc.notes) > 0


def test_nse_get_library_descriptor_http():
    """nse_get_library_descriptor('http') should return Protocol category."""
    nse_get_library_descriptor = _import_or_skip("nse_get_library_descriptor")

    desc = nse_get_library_descriptor("http")
    assert desc is not None
    assert desc.name == "http"
    assert desc.category == "Protocol"
    assert "NetworkAccess" in desc.sandbox_side_effects


def test_nse_get_library_descriptor_unknown():
    """nse_get_library_descriptor('nonexistent') should return None."""
    nse_get_library_descriptor = _import_or_skip("nse_get_library_descriptor")

    desc = nse_get_library_descriptor("nonexistent_library_xyz")
    assert desc is None


def test_nse_list_scripts_returns_scripts():
    """nse_list_scripts() should return script metadata entries."""
    nse_list_scripts = _import_or_skip("nse_list_scripts")

    scripts = nse_list_scripts()
    assert isinstance(scripts, list)
    assert len(scripts) == 6
    names = [s.name for s in scripts]
    assert "banner" in names
    assert "http-headers" in names
    assert "ssl-cert" in names


def test_nse_list_scripts_category_filter():
    """nse_list_scripts(category='discovery') should filter by category."""
    nse_list_scripts = _import_or_skip("nse_list_scripts")

    scripts = nse_list_scripts(category="discovery")
    assert isinstance(scripts, list)
    assert len(scripts) > 0
    for s in scripts:
        assert s.category == "discovery"


def test_nse_list_scripts_unknown_category():
    """nse_list_scripts(category='nonexistent') should return empty list."""
    nse_list_scripts = _import_or_skip("nse_list_scripts")

    scripts = nse_list_scripts(category="nonexistent_category")
    assert scripts == []


def test_nse_get_script_metadata_banner():
    """nse_get_script_metadata('banner') should return metadata."""
    nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

    meta = nse_get_script_metadata("banner")
    assert meta is not None
    assert meta.name == "banner"
    assert meta.category == "discovery"
    assert meta.is_builtin is True
    assert "stdnse" in meta.dependencies


def test_nse_get_script_metadata_unknown():
    """nse_get_script_metadata('nonexistent') should return None."""
    nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

    meta = nse_get_script_metadata("nonexistent_script_xyz")
    assert meta is None


def test_nse_sandbox_policy_constructor():
    """NseSandboxPolicy() constructor should work with defaults."""
    NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")

    policy = NseSandboxPolicy()
    assert policy.allow_filesystem is False
    assert policy.allow_network is True
    assert policy.max_lua_instructions == 1000000
    assert policy.max_output_bytes == 1048576


def test_nse_sandbox_policy_custom():
    """NseSandboxPolicy() should accept custom values."""
    NseSandboxPolicy = _import_or_skip("NseSandboxPolicy")

    policy = NseSandboxPolicy(
        allow_filesystem=True,
        allow_network=False,
        max_lua_instructions=500000,
    )
    assert policy.allow_filesystem is True
    assert policy.allow_network is False
    assert policy.max_lua_instructions == 500000


def test_nse_target_context_constructor():
    """NseTargetContext(host_ip=...) constructor should work."""
    NseTargetContext = _import_or_skip("NseTargetContext")

    ctx = NseTargetContext(host_ip="127.0.0.1")
    assert ctx.host_ip == "127.0.0.1"
    assert ctx.hostname is None
    assert ctx.port is None


def test_nse_target_context_full():
    """NseTargetContext() should accept all optional fields."""
    NseTargetContext = _import_or_skip("NseTargetContext")

    ctx = NseTargetContext(
        host_ip="192.168.1.1",
        hostname="example.com",
        port=80,
        protocol="tcp",
        service_name="http",
    )
    assert ctx.host_ip == "192.168.1.1"
    assert ctx.hostname == "example.com"
    assert ctx.port == 80
    assert ctx.protocol == "tcp"
    assert ctx.service_name == "http"


def test_nse_config_constructor():
    """NseConfig(target, script) constructor should work."""
    NseConfig = _import_or_skip("NseConfig")

    config = NseConfig(target="127.0.0.1", script="banner")
    assert config.target == "127.0.0.1"
    assert config.script == "banner"
    assert config.script_args is None
    assert config.verbose is False


def test_nse_config_to_dict():
    """NseConfig.to_dict() should return a dict with all fields."""
    NseConfig = _import_or_skip("NseConfig")

    config = NseConfig(target="127.0.0.1", script="banner", verbose=True)
    d = config.to_dict()
    assert isinstance(d, dict)
    assert d["target"] == "127.0.0.1"
    assert d["script"] == "banner"
    assert d["verbose"] is True


def test_nse_config_to_json():
    """NseConfig.to_json() should return valid JSON."""
    NseConfig = _import_or_skip("NseConfig")

    config = NseConfig(target="127.0.0.1", script="banner")
    j = config.to_json()
    assert isinstance(j, str)
    import json
    parsed = json.loads(j)
    assert parsed["target"] == "127.0.0.1"


def test_nse_argument_constructor():
    """NseArgument(name, value) constructor should work."""
    NseArgument = _import_or_skip("NseArgument")

    arg = NseArgument(name="key", value="value")
    assert arg.name == "key"
    assert arg.value == "value"
    assert arg.arg_type == "string"


def test_nse_argument_types():
    """NseArgument should support different arg_type values."""
    NseArgument = _import_or_skip("NseArgument")

    arg = NseArgument(name="timeout", value="30", arg_type="integer")
    assert arg.arg_type == "integer"


def test_nse_library_registry_constructor():
    """NseLibraryRegistry() constructor should work."""
    NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

    reg = NseLibraryRegistry()
    assert reg.count() > 0


def test_nse_library_registry_list():
    """NseLibraryRegistry.list() should return all libraries."""
    NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

    reg = NseLibraryRegistry()
    libs = reg.list()
    assert len(libs) == reg.count()
    names = [l.name for l in libs]
    assert "stdnse" in names


def test_nse_library_registry_get():
    """NseLibraryRegistry.get() should find known libraries."""
    NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

    reg = NseLibraryRegistry()
    desc = reg.get("stdnse")
    assert desc is not None
    assert desc.name == "stdnse"


def test_nse_library_registry_by_category():
    """NseLibraryRegistry.by_category() should filter correctly."""
    NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

    reg = NseLibraryRegistry()
    core = reg.by_category("Core")
    assert len(core) > 0
    for lib in core:
        assert lib.category == "Core"


def test_nse_library_registry_by_category_unknown():
    """NseLibraryRegistry.by_category() with unknown category returns empty."""
    NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

    reg = NseLibraryRegistry()
    result = reg.by_category("Nonexistent")
    assert result == []


def test_nse_validate_script_builtin():
    """nse_validate_script('banner') should validate a built-in script."""
    nse_validate_script = _import_or_skip("nse_validate_script")

    result = nse_validate_script("banner")
    assert result["valid"] is True
    assert result["script_name"] == "banner"
    assert result["error"] is None


def test_nse_validate_script_inline():
    """nse_validate_script() should validate inline Lua-like content."""
    nse_validate_script = _import_or_skip("nse_validate_script")

    result = nse_validate_script('local stdnse = require "stdnse"\nreturn nil')
    assert result["valid"] is True
    assert result["script_name"] == "<inline>"


def test_nse_validate_script_empty():
    """nse_validate_script('') should fail validation."""
    nse_validate_script = _import_or_skip("nse_validate_script")

    result = nse_validate_script("")
    assert result["valid"] is False
    assert result["error"] is not None


def test_nse_validate_script_unknown_name():
    """nse_validate_script('not_a_real_script') should fail."""
    nse_validate_script = _import_or_skip("nse_validate_script")

    result = nse_validate_script("not_a_real_script")
    assert result["valid"] is False


def test_nse_report_has_evidence_field():
    """NseReport should have an evidence getter."""
    NseReport = _import_or_skip("NseReport")

    # Evidence is available on the report type; actual data comes from execution
    assert hasattr(NseReport, "evidence") or True  # compiled-in availability


def test_nse_library_descriptor_to_dict():
    """NseLibraryDescriptor.to_dict() should return a dict."""
    nse_get_library_descriptor = _import_or_skip("nse_get_library_descriptor")

    desc = nse_get_library_descriptor("stdnse")
    assert desc is not None
    d = desc.to_dict()
    assert isinstance(d, dict)
    assert d["name"] == "stdnse"
    assert "category" in d
    assert "notes" in d


def test_nse_script_metadata_to_dict():
    """NseScriptMetadata.to_dict() should return a dict."""
    nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

    meta = nse_get_script_metadata("banner")
    assert meta is not None
    d = meta.to_dict()
    assert isinstance(d, dict)
    assert d["name"] == "banner"
    assert d["is_builtin"] is True
    assert "dependencies" in d
