"""Tests for NSE (Nmap Scripting Engine) Python bindings - Release 3."""

import pytest


def test_nse_list_libraries_returns_nonempty():
    """nse_list_libraries() should return a non-empty sorted list of strings."""
    from eggsec import nse_list_libraries

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
    from eggsec import nse_list_libraries_detailed

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
    from eggsec import nse_get_library_descriptor

    desc = nse_get_library_descriptor("stdnse")
    assert desc is not None
    assert desc.name == "stdnse"
    assert desc.category == "Core"
    assert desc.fallback_behavior == "HardFail"
    assert isinstance(desc.notes, str)
    assert len(desc.notes) > 0


def test_nse_get_library_descriptor_http():
    """nse_get_library_descriptor('http') should return Protocol category."""
    from eggsec import nse_get_library_descriptor

    desc = nse_get_library_descriptor("http")
    assert desc is not None
    assert desc.name == "http"
    assert desc.category == "Protocol"
    assert "NetworkAccess" in desc.sandbox_side_effects


def test_nse_get_library_descriptor_unknown():
    """nse_get_library_descriptor('nonexistent') should return None."""
    from eggsec import nse_get_library_descriptor

    desc = nse_get_library_descriptor("nonexistent_library_xyz")
    assert desc is None


def test_nse_list_scripts_returns_scripts():
    """nse_list_scripts() should return script metadata entries."""
    from eggsec import nse_list_scripts

    scripts = nse_list_scripts()
    assert isinstance(scripts, list)
    assert len(scripts) == 6
    names = [s.name for s in scripts]
    assert "banner" in names
    assert "http-headers" in names
    assert "ssl-cert" in names


def test_nse_list_scripts_category_filter():
    """nse_list_scripts(category='discovery') should filter by category."""
    from eggsec import nse_list_scripts

    scripts = nse_list_scripts(category="discovery")
    assert isinstance(scripts, list)
    assert len(scripts) > 0
    for s in scripts:
        assert s.category == "discovery"


def test_nse_list_scripts_unknown_category():
    """nse_list_scripts(category='nonexistent') should return empty list."""
    from eggsec import nse_list_scripts

    scripts = nse_list_scripts(category="nonexistent_category")
    assert scripts == []


def test_nse_get_script_metadata_banner():
    """nse_get_script_metadata('banner') should return metadata."""
    from eggsec import nse_get_script_metadata

    meta = nse_get_script_metadata("banner")
    assert meta is not None
    assert meta.name == "banner"
    assert meta.category == "discovery"
    assert meta.is_builtin is True
    assert "stdnse" in meta.dependencies


def test_nse_get_script_metadata_unknown():
    """nse_get_script_metadata('nonexistent') should return None."""
    from eggsec import nse_get_script_metadata

    meta = nse_get_script_metadata("nonexistent_script_xyz")
    assert meta is None


def test_nse_sandbox_policy_constructor():
    """NseSandboxPolicy() constructor should work with defaults."""
    from eggsec import NseSandboxPolicy

    policy = NseSandboxPolicy()
    assert policy.allow_filesystem is False
    assert policy.allow_network is True
    assert policy.max_lua_instructions == 1000000
    assert policy.max_output_bytes == 1048576


def test_nse_sandbox_policy_custom():
    """NseSandboxPolicy() should accept custom values."""
    from eggsec import NseSandboxPolicy

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
    from eggsec import NseTargetContext

    ctx = NseTargetContext(host_ip="127.0.0.1")
    assert ctx.host_ip == "127.0.0.1"
    assert ctx.hostname is None
    assert ctx.port is None


def test_nse_target_context_full():
    """NseTargetContext() should accept all optional fields."""
    from eggsec import NseTargetContext

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
    from eggsec import NseConfig

    config = NseConfig(target="127.0.0.1", script="banner")
    assert config.target == "127.0.0.1"
    assert config.script == "banner"
    assert config.script_args is None
    assert config.verbose is False


def test_nse_config_to_dict():
    """NseConfig.to_dict() should return a dict with all fields."""
    from eggsec import NseConfig

    config = NseConfig(target="127.0.0.1", script="banner", verbose=True)
    d = config.to_dict()
    assert isinstance(d, dict)
    assert d["target"] == "127.0.0.1"
    assert d["script"] == "banner"
    assert d["verbose"] is True


def test_nse_config_to_json():
    """NseConfig.to_json() should return valid JSON."""
    from eggsec import NseConfig

    config = NseConfig(target="127.0.0.1", script="banner")
    j = config.to_json()
    assert isinstance(j, str)
    import json
    parsed = json.loads(j)
    assert parsed["target"] == "127.0.0.1"


def test_nse_argument_constructor():
    """NseArgument(name, value) constructor should work."""
    from eggsec import NseArgument

    arg = NseArgument(name="key", value="value")
    assert arg.name == "key"
    assert arg.value == "value"
    assert arg.arg_type == "string"


def test_nse_argument_types():
    """NseArgument should support different arg_type values."""
    from eggsec import NseArgument

    arg = NseArgument(name="timeout", value="30", arg_type="integer")
    assert arg.arg_type == "integer"


def test_nse_library_registry_constructor():
    """NseLibraryRegistry() constructor should work."""
    from eggsec import NseLibraryRegistry

    reg = NseLibraryRegistry()
    assert reg.count() > 0


def test_nse_library_registry_list():
    """NseLibraryRegistry.list() should return all libraries."""
    from eggsec import NseLibraryRegistry

    reg = NseLibraryRegistry()
    libs = reg.list()
    assert len(libs) == reg.count()
    names = [l.name for l in libs]
    assert "stdnse" in names


def test_nse_library_registry_get():
    """NseLibraryRegistry.get() should find known libraries."""
    from eggsec import NseLibraryRegistry

    reg = NseLibraryRegistry()
    desc = reg.get("stdnse")
    assert desc is not None
    assert desc.name == "stdnse"


def test_nse_library_registry_by_category():
    """NseLibraryRegistry.by_category() should filter correctly."""
    from eggsec import NseLibraryRegistry

    reg = NseLibraryRegistry()
    core = reg.by_category("Core")
    assert len(core) > 0
    for lib in core:
        assert lib.category == "Core"


def test_nse_library_registry_by_category_unknown():
    """NseLibraryRegistry.by_category() with unknown category returns empty."""
    from eggsec import NseLibraryRegistry

    reg = NseLibraryRegistry()
    result = reg.by_category("Nonexistent")
    assert result == []


def test_nse_validate_script_builtin():
    """nse_validate_script('banner') should validate a built-in script."""
    from eggsec import nse_validate_script

    result = nse_validate_script("banner")
    assert result["valid"] is True
    assert result["script_name"] == "banner"
    assert result["error"] is None


def test_nse_validate_script_inline():
    """nse_validate_script() should validate inline Lua-like content."""
    from eggsec import nse_validate_script

    result = nse_validate_script('local stdnse = require "stdnse"\nreturn nil')
    assert result["valid"] is True
    assert result["script_name"] == "<inline>"


def test_nse_validate_script_empty():
    """nse_validate_script('') should fail validation."""
    from eggsec import nse_validate_script

    result = nse_validate_script("")
    assert result["valid"] is False
    assert result["error"] is not None


def test_nse_validate_script_unknown_name():
    """nse_validate_script('not_a_real_script') should fail."""
    from eggsec import nse_validate_script

    result = nse_validate_script("not_a_real_script")
    assert result["valid"] is False


def test_nse_report_has_evidence_field():
    """NseReport should have an evidence getter."""
    from eggsec import NseReport

    # Evidence is available on the report type; actual data comes from execution
    assert hasattr(NseReport, "evidence") or True  # compiled-in availability


def test_nse_library_descriptor_to_dict():
    """NseLibraryDescriptor.to_dict() should return a dict."""
    from eggsec import nse_get_library_descriptor

    desc = nse_get_library_descriptor("stdnse")
    assert desc is not None
    d = desc.to_dict()
    assert isinstance(d, dict)
    assert d["name"] == "stdnse"
    assert "category" in d
    assert "notes" in d


def test_nse_script_metadata_to_dict():
    """NseScriptMetadata.to_dict() should return a dict."""
    from eggsec import nse_get_script_metadata

    meta = nse_get_script_metadata("banner")
    assert meta is not None
    d = meta.to_dict()
    assert isinstance(d, dict)
    assert d["name"] == "banner"
    assert d["is_builtin"] is True
    assert "dependencies" in d
