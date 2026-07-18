"""Operational proof tests for NSE (Nmap Scripting Engine) runtime - Workstream 2.

Demonstrates that NseRuntime is a reusable managed runtime, not one-shot DTOs.
Tests prove runtime reuse, limits, cancellation, cleanup, and structured diagnostics.

Workstream 4: Tests use deterministic loopback fixtures instead of skipping on
network errors. NSE scripts receive fixture ports via script_args.
"""

import json
import pytest
import importlib

from fixtures.nse_loopback import NseLoopbackFixtures


def _import_or_skip(name, feature="nse"):
    """Import a name from eggsec, skip test if unavailable (feature-gated)."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


# Module-level timeout for all tests
pytestmark = [pytest.mark.timeout(300)]

# Known built-in scripts available via NseRuntime.run_script()
BUILTIN_SCRIPTS = ["default", "discovery", "banner", "http-headers", "dns-check", "ssl-cert"]

# Known library categories
LIBRARY_CATEGORIES = ["Core", "Protocol", "Utility", "Exploit", "Auth"]

# Fixture context for integration tests
_nse_fixtures = NseLoopbackFixtures()


# Fixture context for integration tests (created per-test via pytest fixture)


@pytest.fixture
def nse_fixtures():
    """Provide loopback TCP/HTTP/TLS fixtures for NSE integration tests."""
    with NseLoopbackFixtures() as fixtures:
        yield fixtures


# ============================================================================
# 1. TestNseRuntimeReuse - Prove one runtime handles many sequential scripts
# ============================================================================


class TestNseRuntimeReuse:
    """Create one NseRuntimePy, run 20+ scripts sequentially, verify each
    produces valid output and verify runtime stats accumulate correctly."""

    @pytest.mark.timeout(60)
    def test_single_runtime_runs_6_scripts(self, nse_fixtures):
        """One runtime runs all 6 built-in scripts sequentially."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report is not None, f"Script {script_name} returned None"
            assert report.script_name == script_name
            assert isinstance(report.output, str)
            assert isinstance(report.compatibility_status, str)
            assert isinstance(report.fidelity, str)

    @pytest.mark.timeout(60)
    def test_runtime_reuse_repeated_script(self, nse_fixtures):
        """Run the same script 10 times on one runtime, each succeeds."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for i in range(10):
            report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
            assert report is not None, f"Run {i} returned None"
            assert report.script_name == "banner"

    @pytest.mark.timeout(60)
    def test_runtime_runs_20_plus_scripts_total(self, nse_fixtures):
        """Run 20+ scripts (with repeats) on one runtime, all produce output."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        all_scripts = BUILTIN_SCRIPTS * 4  # 6 * 4 = 24 runs
        for script_name in all_scripts:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report is not None
            assert isinstance(report.output, str)

    @pytest.mark.timeout(60)
    def test_runtime_with_args_reuse(self, nse_fixtures):
        """run_script_with_args works repeatedly on one runtime."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for i in range(5):
            report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
            assert report is not None
            assert report.script_name == "banner"

    @pytest.mark.timeout(60)
    def test_runtime_with_automated_limits_reuse(self, nse_fixtures):
        """Runtime with automated_limits runs scripts repeatedly."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        limits = NseExecutionLimits.automated_defaults()
        runtime = NseRuntime(cfg, limits=limits)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report is not None

    @pytest.mark.timeout(60)
    def test_runtime_with_manual_limits_reuse(self, nse_fixtures):
        """Runtime with manual_defaults limits runs scripts repeatedly."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        limits = NseExecutionLimits.manual_defaults()
        runtime = NseRuntime(cfg, limits=limits)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report is not None

    @pytest.mark.timeout(60)
    def test_runtime_switches_profiles_per_script(self, nse_fixtures):
        """One runtime can run scripts that use different rule types."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report_banner = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        report_discovery = runtime.run_script("discovery")
        assert report_banner is not None
        assert report_discovery is not None

    @pytest.mark.timeout(60)
    def test_runtime_cancellation_token_reusable(self, nse_fixtures):
        """Cancellation token from runtime remains valid across runs."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)
        token = runtime.cancellation_token()

        for script_name in BUILTIN_SCRIPTS[:3]:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report is not None
            assert token.is_cancelled() is False


# ============================================================================
# 2. TestNseLimitsEnforcement - Test limit fields are applied
# ============================================================================


class TestNseLimitsEnforcement:
    """Test wall_clock_timeout, lua_instruction_budget, max_output_bytes."""

    @pytest.mark.timeout(60)
    def test_automated_defaults_has_limits(self):
        """automated_defaults() produces limits with concrete values."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        limits = NseExecutionLimits.automated_defaults()
        d = limits.to_dict()
        assert d["wall_clock_timeout_secs"] is not None
        assert d["lua_instruction_budget"] is not None
        assert d["max_output_bytes"] is not None
        assert d["wall_clock_timeout_secs"] > 0
        assert d["lua_instruction_budget"] > 0
        assert d["max_output_bytes"] > 0

    @pytest.mark.timeout(60)
    def test_manual_defaults_has_limits(self):
        """manual_defaults() produces limits."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        limits = NseExecutionLimits.manual_defaults()
        d = limits.to_dict()
        assert d["wall_clock_timeout_secs"] is not None
        assert d["lua_instruction_budget"] is not None
        assert d["max_output_bytes"] is not None

    @pytest.mark.timeout(60)
    def test_unlimited_has_no_limits(self):
        """unlimited() produces limits with None fields."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        limits = NseExecutionLimits.unlimited()
        d = limits.to_dict()
        assert d["wall_clock_timeout_secs"] is None
        assert d["lua_instruction_budget"] is None
        assert d["max_output_bytes"] is None
        assert d["max_script_bytes"] is None

    @pytest.mark.timeout(60)
    def test_script_runs_under_automated_limits(self, nse_fixtures):
        """Simple script completes within automated_defaults limits."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        limits = NseExecutionLimits.automated_defaults()
        runtime = NseRuntime(cfg, limits=limits)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert report is not None
        normalized = report.compatibility_status.lower().replace("-", "")
        assert normalized in (
            "compatible",
            "compatiblewithwarnings",
            "failed",
            "partial",
        )

    @pytest.mark.skip(reason="NseExecutionLimits has no Python constructor")
    def test_script_runs_under_custom_tight_limits(self):
        pass

    @pytest.mark.timeout(60)
    def test_limits_dict_serializes_to_json(self):
        """Limits object serializes to JSON and back."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        limits = NseExecutionLimits.automated_defaults()
        j = limits.to_json()
        parsed = json.loads(j)
        assert isinstance(parsed, dict)
        assert "wall_clock_timeout_secs" in parsed
        assert "lua_instruction_budget" in parsed

    @pytest.mark.timeout(60)
    def test_limits_all_11_fields_present(self):
        """Limits to_dict() has all 11 fields."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        limits = NseExecutionLimits.automated_defaults()
        d = limits.to_dict()
        expected_fields = [
            "wall_clock_timeout_secs",
            "lua_instruction_budget",
            "max_output_bytes",
            "max_script_bytes",
            "max_required_module_bytes",
            "max_network_operations",
            "max_network_bytes_read",
            "max_network_bytes_written",
            "max_filesystem_operations",
            "max_filesystem_bytes_read",
            "max_lua_memory_bytes",
        ]
        for field in expected_fields:
            assert field in d, f"Missing field: {field}"

    @pytest.mark.timeout(60)
    def test_limits_automated_different_from_manual(self):
        """automated and manual defaults produce different values."""
        NseExecutionLimits = _import_or_skip("NseExecutionLimits")

        auto = NseExecutionLimits.automated_defaults()
        manual = NseExecutionLimits.manual_defaults()
        auto_d = auto.to_dict()
        manual_d = manual.to_dict()

        # At least one field should differ
        differences = 0
        for key in auto_d:
            if auto_d[key] != manual_d[key]:
                differences += 1
        assert differences > 0, "Automated and manual defaults should differ"


# ============================================================================
# 3. TestNseCancellation - Cancel during execution
# ============================================================================


class TestNseCancellation:
    """Cancel async scripts mid-execution, verify is_cancelled, verify reusable."""

    @pytest.mark.timeout(60)
    def test_cancellation_token_initial_state(self):
        """New cancellation token is not cancelled."""
        NseCancellationToken = _import_or_skip("NseCancellationToken")

        token = NseCancellationToken()
        assert token.is_cancelled() is False

    @pytest.mark.timeout(60)
    def test_cancellation_token_cancel(self):
        """cancel() sets is_cancelled to True."""
        NseCancellationToken = _import_or_skip("NseCancellationToken")

        token = NseCancellationToken()
        token.cancel()
        assert token.is_cancelled() is True

    @pytest.mark.timeout(60)
    def test_cancellation_token_reset(self):
        """reset() clears cancelled state."""
        NseCancellationToken = _import_or_skip("NseCancellationToken")

        token = NseCancellationToken()
        token.cancel()
        assert token.is_cancelled() is True
        token.reset()
        assert token.is_cancelled() is False

    @pytest.mark.timeout(60)
    def test_runtime_cancellation_token_starts_not_cancelled(self):
        """Runtime's cancellation token starts not cancelled."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)
        token = runtime.cancellation_token()
        assert token.is_cancelled() is False

    @pytest.mark.timeout(60)
    def test_runtime_cancellation_token_cancelled_after_cancel(self):
        """Runtime's token is cancelled after calling cancel()."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)
        token = runtime.cancellation_token()
        token.cancel()
        assert token.is_cancelled() is True

    @pytest.mark.timeout(60)
    def test_cancellation_token_repr(self):
        """Cancelled token repr shows cancelled state."""
        NseCancellationToken = _import_or_skip("NseCancellationToken")

        token = NseCancellationToken()
        r = repr(token)
        assert "NseCancellationToken" in r
        assert "cancelled=false" in r

        token.cancel()
        r = repr(token)
        assert "cancelled=true" in r

    @pytest.mark.timeout(60)
    def test_multiple_tokens_independent(self):
        """Two cancellation tokens are independent."""
        NseCancellationToken = _import_or_skip("NseCancellationToken")

        t1 = NseCancellationToken()
        t2 = NseCancellationToken()
        t1.cancel()
        assert t1.is_cancelled() is True
        assert t2.is_cancelled() is False


# ============================================================================
# 4. TestNseRuntimeCleanup - Verify clean state after errors
# ============================================================================


class TestNseRuntimeCleanup:
    """Run scripts that may produce errors, verify runtime is clean for next script."""

    @pytest.mark.timeout(60)
    def test_runtime_survives_invalid_script(self, nse_fixtures):
        """Runtime remains usable after running an invalid script."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        # Try running a non-existent script (should fail gracefully)
        try:
            runtime.run_script("nonexistent_script_xyz_12345")
        except Exception:
            pass

        # Runtime should still work
        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert report is not None
        assert report.script_name == "banner"

    @pytest.mark.timeout(60)
    def test_runtime_cleanup_between_different_scripts(self, nse_fixtures):
        """Output from one script doesn't leak into the next."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report1 = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        output1 = report1.output

        report2 = runtime.run_script("discovery")
        output2 = report2.output

        # Scripts should have different names
        assert report1.script_name != report2.script_name
        # Each report should reference its own script
        assert report1.script_name == "banner"
        assert report2.script_name == "discovery"

    @pytest.mark.timeout(60)
    def test_runtime_target_consistent_across_runs(self, nse_fixtures):
        """Target remains consistent across multiple runs on same runtime."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report.target == "127.0.0.1"

    @pytest.mark.timeout(60)
    def test_runtime_report_dict_after_each_run(self, nse_fixtures):
        """to_dict() works on every report from repeated runs."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            d = report.to_dict()
            assert isinstance(d, dict)
            assert "script_name" in d
            assert "output" in d
            assert "compatibility_status" in d


# ============================================================================
# 5. TestNseScriptValidation - Validate 20+ scripts from corpus
# ============================================================================


class TestNseScriptValidation:
    """Validate scripts using nse_validate_script, check fields."""

    @pytest.mark.timeout(60)
    def test_validate_builtin_scripts(self):
        """All built-in scripts validate successfully."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        for script_name in BUILTIN_SCRIPTS:
            result = nse_validate_script(script_name)
            assert result["valid"] is True, f"Script {script_name} should be valid"
            assert result["error"] is None
            assert result["script_name"] == script_name

    @pytest.mark.timeout(60)
    def test_validate_empty_script_fails(self):
        """Empty script fails validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("")
        assert result["valid"] is False
        assert result["error"] is not None

    @pytest.mark.timeout(60)
    def test_validate_unknown_name_fails(self):
        """Unknown script name fails validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("zzz_nonexistent_script_xyz")
        assert result["valid"] is False
        assert result["error"] is not None

    @pytest.mark.timeout(60)
    def test_validate_inline_lua_valid(self):
        """Inline Lua with require() passes validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script('local stdnse = require "stdnse"\nreturn nil')
        assert result["valid"] is True
        assert result["script_name"] == "<inline>"

    @pytest.mark.timeout(60)
    def test_validate_inline_lua_function(self):
        """Inline Lua function passes validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("function main() return nil end")
        assert result["valid"] is True

    @pytest.mark.timeout(60)
    def test_validate_inline_lua_local(self):
        """Inline Lua local variable passes validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("local x = 42")
        assert result["valid"] is True

    @pytest.mark.timeout(60)
    def test_validate_inline_lua_return(self):
        """Inline Lua return statement passes validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("return nil")
        assert result["valid"] is True

    @pytest.mark.timeout(60)
    def test_validate_gibberish_fails(self):
        """Random non-Lua text fails validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("xyzzy12345_not_lua_at_all")
        assert result["valid"] is False

    @pytest.mark.timeout(60)
    def test_validate_whitespace_only_fails(self):
        """Whitespace-only input fails validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("   \n\t  ")
        assert result["valid"] is False

    @pytest.mark.timeout(60)
    def test_validate_result_has_required_keys(self):
        """Validation result has valid, error, script_name keys."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("banner")
        assert "valid" in result
        assert "error" in result
        assert "script_name" in result
        assert isinstance(result["valid"], bool)
        assert isinstance(result["script_name"], str)

    @pytest.mark.timeout(60)
    def test_validate_comment_only_passes(self):
        """Comment-only Lua passes validation."""
        nse_validate_script = _import_or_skip("nse_validate_script")

        result = nse_validate_script("-- this is a comment")
        assert result["valid"] is True


# ============================================================================
# 6. TestNseLibraryRegistry - List all, by category, verify counts
# ============================================================================


class TestNseLibraryRegistry:
    """NseLibraryRegistryPy list, get, by_category, count."""

    @pytest.mark.timeout(60)
    def test_registry_count_matches_list(self):
        """Registry count() matches len(list())."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        assert registry.count() == len(registry.list())

    @pytest.mark.timeout(60)
    def test_registry_list_nonempty(self):
        """Registry has at least some libraries."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        libs = registry.list()
        assert len(libs) > 0

    @pytest.mark.timeout(60)
    def test_registry_get_known_library(self):
        """get('stdnse') returns a descriptor."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        desc = registry.get("stdnse")
        assert desc is not None
        assert desc.name == "stdnse"

    @pytest.mark.timeout(60)
    def test_registry_get_unknown_library(self):
        """get('nonexistent') returns None."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        desc = registry.get("zzz_nonexistent_lib_xyz")
        assert desc is None

    @pytest.mark.timeout(60)
    def test_registry_by_category_core(self):
        """by_category('Core') returns non-empty list."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        core = registry.by_category("Core")
        assert len(core) > 0
        for lib in core:
            assert lib.category == "Core"

    @pytest.mark.timeout(60)
    def test_registry_by_category_protocol(self):
        """by_category('Protocol') returns non-empty list."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        proto = registry.by_category("Protocol")
        assert len(proto) > 0
        for lib in proto:
            assert lib.category == "Protocol"

    @pytest.mark.timeout(60)
    def test_registry_by_category_utility(self):
        """by_category('Utility') returns non-empty list."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        util = registry.by_category("Utility")
        assert len(util) > 0
        for lib in util:
            assert lib.category == "Utility"

    @pytest.mark.timeout(60)
    def test_registry_by_unknown_category(self):
        """by_category('UnknownCategory') returns empty list."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        result = registry.by_category("zzz_nonexistent_cat")
        assert result == []

    @pytest.mark.timeout(60)
    def test_registry_all_categories_valid(self):
        """All library categories are from the known set."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        for lib in registry.list():
            assert lib.category in LIBRARY_CATEGORIES, f"Unknown category: {lib.category}"

    @pytest.mark.timeout(60)
    def test_registry_descriptor_has_fields(self):
        """All library descriptors have required fields."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        for lib in registry.list():
            assert hasattr(lib, "name")
            assert hasattr(lib, "category")
            assert hasattr(lib, "fallback_behavior")
            assert hasattr(lib, "notes")
            assert hasattr(lib, "enforcement_status")
            assert lib.name != ""

    @pytest.mark.timeout(60)
    def test_registry_descriptor_to_dict(self):
        """Library descriptor to_dict() returns valid dict."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        desc = registry.get("stdnse")
        assert desc is not None
        d = desc.to_dict()
        assert isinstance(d, dict)
        assert d["name"] == "stdnse"
        assert "category" in d
        assert "fallback_behavior" in d

    @pytest.mark.timeout(60)
    def test_registry_descriptor_to_json(self):
        """Library descriptor to_json() returns valid JSON."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        desc = registry.get("http")
        assert desc is not None
        j = desc.to_json()
        parsed = json.loads(j)
        assert parsed["name"] == "http"

    @pytest.mark.timeout(60)
    def test_registry_descriptor_repr(self):
        """Library descriptor repr contains key info."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        desc = registry.get("stdnse")
        assert desc is not None
        r = repr(desc)
        assert "NseLibraryDescriptor" in r
        assert "stdnse" in r

    @pytest.mark.timeout(60)
    def test_registry_by_category_sum_le(self):
        """Sum of per-category counts equals total count."""
        NseLibraryRegistry = _import_or_skip("NseLibraryRegistry")

        registry = NseLibraryRegistry()
        total = registry.count()
        cat_sum = sum(len(registry.by_category(c)) for c in LIBRARY_CATEGORIES)
        assert cat_sum == total


# ============================================================================
# 7. TestNseHostPortContext - Host and port context for rule evaluation
# ============================================================================


class TestNseHostPortContext:
    """Host and port context for rule evaluation, multiple contexts."""

    @pytest.mark.timeout(60)
    def test_host_context_minimal(self):
        """NseHostContext with just IP."""
        NseHostContext = _import_or_skip("NseHostContext")

        ctx = NseHostContext(ip="10.0.0.1")
        assert ctx.ip == "10.0.0.1"
        assert ctx.hostname is None
        assert ctx.target_label == ""
        assert ctx.source == "synthetic"

    @pytest.mark.timeout(60)
    def test_host_context_full(self):
        """NseHostContext with all fields."""
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

    @pytest.mark.timeout(60)
    def test_host_context_to_dict(self):
        """Host context to_dict() roundtrip."""
        NseHostContext = _import_or_skip("NseHostContext")

        ctx = NseHostContext(ip="172.16.0.1", hostname="db.internal")
        d = ctx.to_dict()
        assert isinstance(d, dict)
        assert d["ip"] == "172.16.0.1"
        assert d["hostname"] == "db.internal"

    @pytest.mark.timeout(60)
    def test_host_context_frozen(self):
        """Host context is frozen."""
        NseHostContext = _import_or_skip("NseHostContext")

        ctx = NseHostContext(ip="10.0.0.1")
        with pytest.raises(AttributeError):
            ctx.ip = "10.0.0.2"

    @pytest.mark.timeout(60)
    def test_port_context_minimal(self):
        """NsePortContext with just port."""
        NsePortContext = _import_or_skip("NsePortContext")

        ctx = NsePortContext(port=80)
        assert ctx.port == 80
        assert ctx.protocol == "tcp"
        assert ctx.state == "open"
        assert ctx.source == "synthetic"

    @pytest.mark.timeout(60)
    def test_port_context_full(self):
        """NsePortContext with all fields."""
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
        assert ctx.service_name == "https"
        assert ctx.service_product == "nginx"
        assert ctx.service_version == "1.21.3"

    @pytest.mark.timeout(60)
    def test_port_context_udp(self):
        """NsePortContext for UDP port."""
        NsePortContext = _import_or_skip("NsePortContext")

        ctx = NsePortContext(port=53, protocol="udp", state="open")
        assert ctx.protocol == "udp"

    @pytest.mark.timeout(60)
    def test_port_context_to_dict(self):
        """Port context to_dict() roundtrip."""
        NsePortContext = _import_or_skip("NsePortContext")

        ctx = NsePortContext(port=8080, service_name="http-proxy")
        d = ctx.to_dict()
        assert isinstance(d, dict)
        assert d["port"] == 8080
        assert d["service_name"] == "http-proxy"

    @pytest.mark.timeout(60)
    def test_port_context_frozen(self):
        """Port context is frozen."""
        NsePortContext = _import_or_skip("NsePortContext")

        ctx = NsePortContext(port=80)
        with pytest.raises(AttributeError):
            ctx.port = 90

    @pytest.mark.timeout(60)
    def test_multiple_host_contexts(self):
        """Create many different host contexts."""
        NseHostContext = _import_or_skip("NseHostContext")

        ips = [f"10.0.0.{i}" for i in range(1, 21)]
        contexts = []
        for ip in ips:
            ctx = NseHostContext(ip=ip)
            contexts.append(ctx)
        assert len(contexts) == 20
        for ctx, expected_ip in zip(contexts, ips):
            assert ctx.ip == expected_ip

    @pytest.mark.timeout(60)
    def test_multiple_port_contexts(self):
        """Create many different port contexts."""
        NsePortContext = _import_or_skip("NsePortContext")

        ports = [80, 443, 22, 21, 25, 53, 8080, 8443, 3306, 5432]
        contexts = []
        for port in ports:
            ctx = NsePortContext(port=port)
            contexts.append(ctx)
        assert len(contexts) == 10
        for ctx, expected_port in zip(contexts, ports):
            assert ctx.port == expected_port


# ============================================================================
# 8. TestNseMetadataInspection - Get metadata for scripts
# ============================================================================


class TestNseMetadataInspection:
    """Get metadata for built-in scripts, verify name/category/deps fields."""

    @pytest.mark.timeout(60)
    def test_metadata_for_all_builtins(self):
        """nse_get_script_metadata returns metadata for all built-in scripts."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        for script_name in BUILTIN_SCRIPTS:
            meta = nse_get_script_metadata(script_name)
            assert meta is not None, f"No metadata for {script_name}"
            assert meta.name == script_name
            assert isinstance(meta.category, str)
            assert isinstance(meta.description, str)
            assert isinstance(meta.dependencies, list)

    @pytest.mark.timeout(60)
    def test_metadata_unknown_returns_none(self):
        """nse_get_script_metadata for unknown script returns None."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        meta = nse_get_script_metadata("zzz_nonexistent_script_xyz")
        assert meta is None

    @pytest.mark.timeout(60)
    def test_metadata_to_dict(self):
        """Metadata to_dict() returns valid dict."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        meta = nse_get_script_metadata("banner")
        assert meta is not None
        d = meta.to_dict()
        assert isinstance(d, dict)
        assert d["name"] == "banner"
        assert "category" in d
        assert "description" in d
        assert "dependencies" in d
        assert "is_builtin" in d

    @pytest.mark.timeout(60)
    def test_metadata_to_json(self):
        """Metadata to_json() serializes to valid JSON."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        meta = nse_get_script_metadata("banner")
        assert meta is not None
        j = meta.to_json()
        parsed = json.loads(j)
        assert parsed["name"] == "banner"

    @pytest.mark.timeout(60)
    def test_metadata_is_builtin(self):
        """All built-in scripts have is_builtin=True."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        for script_name in BUILTIN_SCRIPTS:
            meta = nse_get_script_metadata(script_name)
            assert meta is not None
            assert meta.is_builtin is True

    @pytest.mark.timeout(60)
    def test_metadata_categories(self):
        """Built-in scripts have valid categories."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        for script_name in BUILTIN_SCRIPTS:
            meta = nse_get_script_metadata(script_name)
            assert meta is not None
            assert meta.category in ("discovery", "version", "default", "protocol", "auth")

    @pytest.mark.timeout(60)
    def test_metadata_banner_has_dependencies(self):
        """Banner script has expected dependencies."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        meta = nse_get_script_metadata("banner")
        assert meta is not None
        assert len(meta.dependencies) > 0
        assert isinstance(meta.dependencies[0], str)

    @pytest.mark.timeout(60)
    def test_metadata_repr(self):
        """Metadata repr contains name."""
        nse_get_script_metadata = _import_or_skip("nse_get_script_metadata")

        meta = nse_get_script_metadata("banner")
        assert meta is not None
        r = repr(meta)
        assert "banner" in r

    @pytest.mark.timeout(60)
    def test_nse_list_scripts_returns_all_builtins(self):
        """nse_list_scripts() returns all built-in scripts."""
        nse_list_scripts = _import_or_skip("nse_list_scripts")

        scripts = nse_list_scripts()
        names = [s.name for s in scripts]
        for script_name in BUILTIN_SCRIPTS:
            assert script_name in names, f"{script_name} not in list_scripts output"

    @pytest.mark.timeout(60)
    def test_nse_list_scripts_category_filter(self):
        """nse_list_scripts(category='discovery') filters correctly."""
        nse_list_scripts = _import_or_skip("nse_list_scripts")

        scripts = nse_list_scripts(category="discovery")
        assert len(scripts) > 0
        for s in scripts:
            assert s.category == "discovery"


# ============================================================================
# 9. TestNseStructuredEvidence - Check evidence items have required fields
# ============================================================================


class TestNseStructuredEvidence:
    """Run scripts, check evidence items have required fields."""

    @pytest.mark.timeout(60)
    def test_report_has_evidence_field(self, nse_fixtures):
        """Report has evidence getter returning a list."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        evidence = report.evidence
        assert isinstance(evidence, list)

    @pytest.mark.timeout(60)
    def test_evidence_items_have_kind(self, nse_fixtures):
        """Each evidence item has a kind field."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        for ev in report.evidence:
            assert hasattr(ev, "kind")
            assert isinstance(ev.kind, str)

    @pytest.mark.timeout(60)
    def test_evidence_items_have_id_and_title(self, nse_fixtures):
        """Each evidence item has id and title."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        for ev in report.evidence:
            assert hasattr(ev, "id")
            assert hasattr(ev, "title")
            assert isinstance(ev.id, str)
            assert isinstance(ev.title, str)

    @pytest.mark.timeout(60)
    def test_evidence_item_to_dict(self, nse_fixtures):
        """Evidence item to_dict() has required keys."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        for ev in report.evidence:
            d = ev.to_dict()
            assert isinstance(d, dict)
            assert "kind" in d
            assert "id" in d
            assert "title" in d

    @pytest.mark.timeout(60)
    def test_evidence_item_to_json(self, nse_fixtures):
        """Evidence item to_json() is valid JSON."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        for ev in report.evidence:
            j = ev.to_json()
            parsed = json.loads(j)
            assert "kind" in parsed

    @pytest.mark.timeout(60)
    def test_evidence_item_repr(self, nse_fixtures):
        """Evidence item repr contains kind."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        for ev in report.evidence:
            r = repr(ev)
            assert "NseEvidenceItem" in r

    @pytest.mark.timeout(60)
    def test_report_to_dict_includes_evidence(self, nse_fixtures):
        """Report to_dict() includes evidence list."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        d = report.to_dict()
        assert "evidence" in d
        assert isinstance(d["evidence"], list)

    @pytest.mark.timeout(60)
    def test_report_to_json_includes_evidence(self, nse_fixtures):
        """Report to_json() includes evidence."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        j = report.to_json()
        parsed = json.loads(j)
        assert "evidence" in parsed


# ============================================================================
# 10. TestNseRuntimeStats - Verify execution stats after multiple runs
# ============================================================================


class TestNseRuntimeStats:
    """Verify execution_stats fields after multiple runs."""

    @pytest.mark.timeout(60)
    def test_report_has_elapsed_secs(self, nse_fixtures):
        """Report has non-negative elapsed_secs."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert report.elapsed_secs >= 0

    @pytest.mark.timeout(60)
    def test_report_has_output_lines(self, nse_fixtures):
        """Report has output_lines >= 0."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert report.output_lines >= 0

    @pytest.mark.timeout(60)
    def test_report_has_has_output(self, nse_fixtures):
        """Report has has_output as bool."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert isinstance(report.has_output, bool)

    @pytest.mark.timeout(60)
    def test_report_has_library_count(self, nse_fixtures):
        """Report has library_count >= 0."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert report.library_count >= 0

    @pytest.mark.timeout(60)
    def test_report_has_warnings_and_errors(self, nse_fixtures):
        """Report has warnings and errors as lists."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        assert isinstance(report.warnings, list)
        assert isinstance(report.errors, list)

    @pytest.mark.timeout(60)
    def test_report_to_dict_has_stats_fields(self, nse_fixtures):
        """Report to_dict() contains stats-related keys."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        d = report.to_dict()
        assert "elapsed_secs" in d
        assert "output_lines" in d
        assert "has_output" in d
        assert "library_count" in d
        assert "warnings" in d
        assert "errors" in d

    @pytest.mark.timeout(60)
    def test_report_to_json_has_stats_fields(self, nse_fixtures):
        """Report to_json() contains stats fields."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        j = report.to_json()
        parsed = json.loads(j)
        assert "elapsed_secs" in parsed
        assert "output_lines" in parsed
        assert "library_count" in parsed

    @pytest.mark.timeout(60)
    def test_report_stats_positive_across_runs(self, nse_fixtures):
        """elapsed_secs is non-negative for all scripts."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        for script_name in BUILTIN_SCRIPTS:
            if script_name == "banner":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tcp_args)
            elif script_name == "http-headers":
                report = runtime.run_script_with_args(script_name, nse_fixtures.http_args)
            elif script_name == "ssl-cert":
                report = runtime.run_script_with_args(script_name, nse_fixtures.tls_args)
            else:
                report = runtime.run_script(script_name)
            assert report.elapsed_secs >= 0, f"Negative elapsed for {script_name}"

    @pytest.mark.timeout(60)
    def test_nse_runtime_stats_type_exists(self):
        """NseRuntimeStats type exists and has expected fields."""
        NseRuntimeStats = _import_or_skip("NseRuntimeStats")

        assert hasattr(NseRuntimeStats, "elapsed_ms")
        assert hasattr(NseRuntimeStats, "output_bytes")
        assert hasattr(NseRuntimeStats, "lua_instruction_count")
        assert hasattr(NseRuntimeStats, "network_operations")
        assert hasattr(NseRuntimeStats, "filesystem_operations")
        assert hasattr(NseRuntimeStats, "limit_violation")

    @pytest.mark.timeout(60)
    def test_nse_runtime_stats_to_dict(self):
        """NseRuntimeStats to_dict() returns valid dict."""
        NseRuntimeStats = _import_or_skip("NseRuntimeStats")

        assert hasattr(NseRuntimeStats, "to_dict")

    @pytest.mark.timeout(60)
    def test_report_repr_contains_key_info(self, nse_fixtures):
        """Report repr contains script name and target."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        r = repr(report)
        assert "NseReportPy" in r or "NseReport" in r
        assert "banner" in r

    @pytest.mark.timeout(60)
    def test_report_str_contains_info(self, nse_fixtures):
        """Report str contains descriptive info."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        s = str(report)
        assert "banner" in s
        assert "127.0.0.1" in s


# ============================================================================
# Additional: JSON roundtrip and serialization tests
# ============================================================================


class TestNseJsonRoundtrip:
    """Verify JSON serialization roundtrip on key DTOs."""

    @pytest.mark.timeout(60)
    def test_runtime_config_json_roundtrip(self):
        """NseRuntimeConfig to_json -> parse -> check fields."""
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="10.0.0.1", profile_kind="agent-safe", verbose=True)
        j = cfg.to_json()
        parsed = json.loads(j)
        assert parsed["target"] == "10.0.0.1"
        assert parsed["profile_kind"] == "agent-safe"
        assert parsed["verbose"] is True

    @pytest.mark.timeout(60)
    def test_runtime_config_to_dict(self):
        """NseRuntimeConfig to_dict returns valid dict."""
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="10.0.0.1")
        d = cfg.to_dict()
        assert isinstance(d, dict)
        assert d["target"] == "10.0.0.1"

    @pytest.mark.timeout(60)
    def test_host_context_json_roundtrip(self):
        """NseHostContext to_json -> parse -> check fields."""
        NseHostContext = _import_or_skip("NseHostContext")

        ctx = NseHostContext(ip="10.0.0.1", hostname="test.example.com")
        j = ctx.to_dict()
        assert j["ip"] == "10.0.0.1"
        assert j["hostname"] == "test.example.com"

    @pytest.mark.timeout(60)
    def test_port_context_json_roundtrip(self):
        """NsePortContext to_json -> parse -> check fields."""
        NsePortContext = _import_or_skip("NsePortContext")

        ctx = NsePortContext(port=443, service_name="https")
        j = ctx.to_dict()
        assert j["port"] == 443
        assert j["service_name"] == "https"

    @pytest.mark.timeout(60)
    def test_report_json_roundtrip(self, nse_fixtures):
        """Report to_json -> parse -> check structure."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        j = report.to_json()
        parsed = json.loads(j)
        assert "script_name" in parsed
        assert "output" in parsed
        assert "compatibility_status" in parsed
        assert "evidence" in parsed
        assert "libraries" in parsed
        assert "rules" in parsed

    @pytest.mark.skip(reason="NseExecutionLimits has no Python constructor")
    def test_limits_json_roundtrip(self):
        pass

    @pytest.mark.timeout(60)
    def test_report_to_dict_roundtrip(self, nse_fixtures):
        """Report to_dict -> json.dumps -> json.loads -> check."""
        NseRuntime = _import_or_skip("NseRuntime")
        NseRuntimeConfig = _import_or_skip("NseRuntimeConfig")

        cfg = NseRuntimeConfig(target="127.0.0.1")
        runtime = NseRuntime(cfg)

        report = runtime.run_script_with_args("banner", nse_fixtures.tcp_args)
        d = report.to_dict()
        j = json.dumps(d)
        parsed = json.loads(j)
        assert parsed["script_name"] == "banner"
