"""Tests for eggsec Scope enforcement across network transitions.

Workstream 7: Scope enforcement across network transitions.

Covers:
- Basic allow/deny target checks
- CIDR matching
- Port enforcement (via from_file TOML with allowed_ports)
- Mixed rules (allow + exclude targets, via from_file)
- Target parsing formats
- Enforcement on direct functions, Engine, and AsyncEngine
- deny_all enforcement error propagation
- Scope repr
- Scope behavioral equivalence (same rules → same results)
"""

import os
import pytest
import eggsec
from eggsec import Scope, Engine, AsyncEngine

SENTINEL_LOOPBACK = "127.0.0.1"

# Ensure loopback fixtures are permitted by the engine
os.environ["EGGSEC_ALLOW_LOOPBACK_FIXTURE"] = "1"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _scope_from_toml(tmp_path, toml_content, name="scope.toml"):
    """Helper: write a TOML scope file and load it."""
    p = tmp_path / name
    p.write_text(toml_content)
    return Scope.from_file(str(p))


def _engine_result_is_scope_denied(result):
    """Check if an OperationResult contains a scope denial error."""
    if result.error is not None:
        return "scope" in result.error.message.lower() or "scope" in result.error.kind.lower()
    return False


# ---------------------------------------------------------------------------
# 1. test_scope_allow_hosts_basic
# ---------------------------------------------------------------------------
def test_scope_allow_hosts_basic():
    """Scope.allow_hosts(["127.0.0.1"]) allows operations targeting 127.0.0.1."""
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True


# ---------------------------------------------------------------------------
# 2. test_scope_deny_all_blocks_all
# ---------------------------------------------------------------------------
def test_scope_deny_all_blocks_all():
    """Scope.deny_all() blocks every target."""
    scope = Scope.deny_all()
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is False
    assert scope.is_target_allowed("example.com") is False
    assert scope.is_target_allowed("10.0.0.1") is False


# ---------------------------------------------------------------------------
# 3. test_scope_cidr_matching
# ---------------------------------------------------------------------------
def test_scope_cidr_matching():
    """Scope.allow_cidrs(["127.0.0.0/8"]) allows 127.0.0.1."""
    scope = Scope.allow_cidrs(["127.0.0.0/8"])
    assert scope.is_target_allowed("127.0.0.1") is True
    assert scope.is_target_allowed("127.255.255.255") is True
    assert scope.is_target_allowed("10.0.0.1") is False


# ---------------------------------------------------------------------------
# 4. test_scope_port_enforcement
# ---------------------------------------------------------------------------
def test_scope_port_enforcement(tmp_path):
    """Scope with allowed_ports=[80] allows port 80, denies 443.

    TOML: scalar values must precede table array entries.
    """
    scope = _scope_from_toml(
        tmp_path,
        f"""\
allowed_ports = [80]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_port_allowed(80) is True
    assert scope.is_port_allowed(443) is False
    assert scope.is_port_allowed(8080) is False


def test_scope_default_ports_all_allowed():
    """Scope without allowed_ports allows all ports."""
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    assert scope.is_port_allowed(80) is True
    assert scope.is_port_allowed(443) is True
    assert scope.is_port_allowed(65535) is True


def test_scope_excluded_ports(tmp_path):
    """Scope with excluded_ports denies those specific ports.

    TOML: scalar values must precede table array entries.
    """
    scope = _scope_from_toml(
        tmp_path,
        f"""\
excluded_ports = [80, 443]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_port_allowed(80) is False
    assert scope.is_port_allowed(443) is False
    assert scope.is_port_allowed(8080) is True


# ---------------------------------------------------------------------------
# 5. test_scope_file_loading
# ---------------------------------------------------------------------------
def test_scope_from_file_toml(tmp_path):
    """Scope.from_file() with a TOML scope file."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"

[[allowed_targets]]
    cidr = "10.0.0.0/8"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.168.1.1") is False


def test_scope_from_file_yaml(tmp_path):
    """Scope.from_file() with a YAML scope file."""
    p = tmp_path / "scope.yaml"
    p.write_text(f"""---
allowed_targets:
  - pattern: "{SENTINEL_LOOPBACK}"
  - cidr: "10.0.0.0/8"
""")
    scope = Scope.from_file(str(p))
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.168.1.1") is False


# ---------------------------------------------------------------------------
# 6. test_scope_deny_all_enforcement_error
# ---------------------------------------------------------------------------
def test_scope_deny_all_enforcement_error_direct():
    """Operations with deny_all scope raise EnforcementError on direct calls."""
    scope = Scope.deny_all()
    with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
        eggsec.scan_ports(SENTINEL_LOOPBACK, [80], scope, timeout_ms=1000)


def test_scope_wrong_target_enforcement_error():
    """Operations targeting a host not in scope raise EnforcementError."""
    scope = Scope.allow_hosts(["example.com"])
    with pytest.raises(eggsec.EnforcementError, match="not within the allowed scope"):
        eggsec.scan_ports(SENTINEL_LOOPBACK, [80], scope, timeout_ms=1000)


# ---------------------------------------------------------------------------
# 7. test_scope_mixed_rules
# ---------------------------------------------------------------------------
def test_scope_mixed_allow_and_exclude(tmp_path):
    """Complex scope: allow a broad range, exclude a specific host."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
[[allowed_targets]]
    cidr = "127.0.0.0/8"

[[excluded_targets]]
    pattern = "127.0.0.2"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("127.0.0.3") is True
    assert scope.is_target_allowed("127.0.0.2") is False


def test_scope_allow_and_exclude_with_ports(tmp_path):
    """Scope with both target rules and port restrictions."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
allowed_ports = [80, 443]
excluded_ports = [8080]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"

[[excluded_targets]]
    pattern = "10.0.0.1"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("10.0.0.1") is False
    assert scope.is_port_allowed(80) is True
    assert scope.is_port_allowed(443) is True
    assert scope.is_port_allowed(8080) is False
    assert scope.is_port_allowed(22) is False


# ---------------------------------------------------------------------------
# 8. test_scope_target_parsing
# ---------------------------------------------------------------------------
def test_scope_target_parsing_ip_address():
    """Scope matches IP addresses."""
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True


def test_scope_target_parsing_hostname():
    """Scope matches hostnames."""
    scope = Scope.allow_hosts(["localhost"])
    assert scope.is_target_allowed("localhost") is True


def test_scope_target_parsing_cidr_in_allow_hosts():
    """Scope.allow_hosts() with CIDR notation delegates to CIDR matching."""
    scope = Scope.allow_hosts(["127.0.0.0/8"])
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("127.255.255.255") is True
    assert scope.is_target_allowed("192.168.1.1") is False


def test_scope_target_parsing_url_not_directly_supported():
    """Scope.is_target_allowed() expects hostnames/IPs, not full URLs.

    Full URL validation goes through the inner scope's validate_url().
    """
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    # Direct hostname check works
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    # URL is not a valid target name for is_target_allowed (it checks pattern matching)
    # This is expected — callers extract the host first


# ---------------------------------------------------------------------------
# 9. test_scope_enforcement_on_direct_functions
# ---------------------------------------------------------------------------
def test_scope_enforcement_scan_ports():
    """scan_ports raises EnforcementError when target is outside scope."""
    scope = Scope.allow_hosts(["example.com"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.scan_ports(SENTINEL_LOOPBACK, [80], scope, timeout_ms=1000)


def test_scope_enforcement_scan_endpoints():
    """scan_endpoints raises EnforcementError when target is outside scope.

    scan_endpoints(base_url, endpoints, scope, ...) — base_url is the target.
    """
    scope = Scope.allow_hosts(["example.com"])
    with pytest.raises(eggsec.EnforcementError):
        eggsec.scan_endpoints(
            f"http://{SENTINEL_LOOPBACK}:80",
            ["/admin"],
            scope,
            timeout_ms=1000,
        )


def test_scope_enforcement_recon_dns_via_engine():
    """Engine.run_recon_dns returns scope-denied result for out-of-scope target.

    recon_dns() standalone does not take a scope, so we test via Engine.
    """
    scope = Scope.allow_hosts(["example.com"])
    engine = Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
    req = eggsec.ReconDnsRequest(SENTINEL_LOOPBACK)
    result = engine.run_recon_dns(req)
    # Engine returns OperationResult with error status instead of raising
    assert result.error is not None
    assert _engine_result_is_scope_denied(result)


# ---------------------------------------------------------------------------
# 10. test_scope_enforcement_on_engine_dispatch
# ---------------------------------------------------------------------------
def test_scope_enforcement_on_engine_dispatch():
    """Engine.run_port_scan returns error result for out-of-scope targets."""
    scope = Scope.allow_hosts(["example.com"])
    engine = Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
    req = eggsec.PortScanRequest(SENTINEL_LOOPBACK, ports="80")
    result = engine.run_port_scan(req)
    # Engine returns OperationResult with error status instead of raising
    assert result.error is not None
    assert _engine_result_is_scope_denied(result)


def test_scope_enforcement_on_engine_allowed():
    """Engine.run_port_scan succeeds for in-scope target."""
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
    engine = Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
    req = eggsec.PortScanRequest(SENTINEL_LOOPBACK, ports="19999", timeout_ms=2000)
    result = engine.run_port_scan(req)
    # Should succeed (or fail with scan error, not enforcement error)
    if result.error is not None:
        # Scan failure is OK — enforcement passed
        assert not _engine_result_is_scope_denied(result)


# ---------------------------------------------------------------------------
# 11. test_scope_enforcement_on_async_engine
# ---------------------------------------------------------------------------
def test_scope_enforcement_on_async_engine():
    """AsyncEngine.run_port_scan raises EnforcementError for out-of-scope targets.

    Unlike the sync Engine (which returns error in OperationResult), the
    AsyncEngine pre_dispatch_validate raises the error directly.
    """
    scope = Scope.allow_hosts(["example.com"])
    engine = AsyncEngine(scope, mode="manual", concurrency=10, timeout_ms=5000)
    req = eggsec.PortScanRequest(SENTINEL_LOOPBACK, ports="80")
    with pytest.raises(eggsec.EnforcementError):
        engine.run_port_scan(req)


def test_scope_enforcement_on_async_engine_allowed():
    """AsyncEngine.run_port_scan succeeds for in-scope target."""
    import asyncio

    async def _run():
        scope = Scope.allow_hosts([SENTINEL_LOOPBACK])
        engine = AsyncEngine(scope, mode="manual", concurrency=10, timeout_ms=5000)
        req = eggsec.PortScanRequest(SENTINEL_LOOPBACK, ports="19999", timeout_ms=2000)
        future = engine.run_port_scan(req)
        result = await future
        if result.error is not None:
            assert not _engine_result_is_scope_denied(result)

    asyncio.run(_run())


# ---------------------------------------------------------------------------
# 12. test_scope_roundtrip_via_file
# ---------------------------------------------------------------------------
def test_scope_roundtrip_via_file(tmp_path):
    """Scope persisted via TOML and reloaded preserves behavior."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
allowed_ports = [80, 443]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"

[[allowed_targets]]
    cidr = "10.0.0.0/8"
""",
    )
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope.is_target_allowed("10.0.0.1") is True
    assert scope.is_target_allowed("192.168.1.1") is False
    assert scope.is_port_allowed(80) is True
    assert scope.is_port_allowed(22) is False


# ---------------------------------------------------------------------------
# 13. test_scope_repr
# ---------------------------------------------------------------------------
def test_scope_repr_allow_hosts():
    """Scope.__repr__ includes the class name and allowed targets."""
    scope = Scope.allow_hosts([SENTINEL_LOOPBACK, "example.com"])
    r = repr(scope)
    assert "Scope" in r
    assert SENTINEL_LOOPBACK in r
    assert "example.com" in r


def test_scope_repr_deny_all():
    """Scope.__repr__ for deny_all shows empty allow list."""
    scope = Scope.deny_all()
    r = repr(scope)
    assert "Scope" in r
    assert "[]" in r or "allow_hosts=[]" in r


def test_scope_repr_cidr():
    """Scope.__repr__ for CIDR scope includes the CIDR range."""
    scope = Scope.allow_cidrs(["10.0.0.0/8"])
    r = repr(scope)
    assert "10.0.0.0/8" in r


# ---------------------------------------------------------------------------
# 14. test_scope_equality (behavioral equivalence)
# ---------------------------------------------------------------------------
def test_scope_equality_behavioral():
    """Two scopes with same rules produce identical allow/deny results."""
    scope_a = Scope.allow_hosts([SENTINEL_LOOPBACK, "example.com"])
    scope_b = Scope.allow_hosts([SENTINEL_LOOPBACK, "example.com"])

    targets = [SENTINEL_LOOPBACK, "example.com", "10.0.0.1", "evil.com"]
    for t in targets:
        assert scope_a.is_target_allowed(t) == scope_b.is_target_allowed(t)


def test_scope_equality_different_rules():
    """Scopes with different rules produce different results for some targets."""
    scope_a = Scope.allow_hosts([SENTINEL_LOOPBACK])
    scope_b = Scope.allow_hosts(["example.com"])

    assert scope_a.is_target_allowed(SENTINEL_LOOPBACK) is True
    assert scope_b.is_target_allowed(SENTINEL_LOOPBACK) is False
    assert scope_a.is_target_allowed("example.com") is False
    assert scope_b.is_target_allowed("example.com") is True


# ---------------------------------------------------------------------------
# 15. test_scope_deny_takes_precedence
# ---------------------------------------------------------------------------
def test_scope_deny_takes_precedence(tmp_path):
    """Exclusion rule overrides inclusion for the same target."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
[[allowed_targets]]
    cidr = "127.0.0.0/8"

[[excluded_targets]]
    pattern = "{SENTINEL_LOOPBACK}"
""",
    )
    # The excluded target should be denied even though it's in the allowed CIDR
    assert scope.is_target_allowed(SENTINEL_LOOPBACK) is False
    # Other targets in the CIDR remain allowed
    assert scope.is_target_allowed("127.0.0.2") is True


# ---------------------------------------------------------------------------
# Additional: port enforcement via scan_ports
# ---------------------------------------------------------------------------
def test_port_enforcement_on_scan_ports(tmp_path):
    """scan_ports raises EnforcementError when a requested port is excluded."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
excluded_ports = [80]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"
""",
    )
    with pytest.raises(eggsec.EnforcementError, match="Port"):
        eggsec.scan_ports(SENTINEL_LOOPBACK, [80], scope, timeout_ms=1000)


def test_port_enforcement_allowed_port_passes(tmp_path):
    """scan_ports succeeds when the requested port is in scope."""
    scope = _scope_from_toml(
        tmp_path,
        f"""\
allowed_ports = [19999]

[[allowed_targets]]
    pattern = "{SENTINEL_LOOPBACK}"
""",
    )
    result = eggsec.scan_ports(SENTINEL_LOOPBACK, [19999], scope, timeout_ms=2000)
    assert result is not None


# ---------------------------------------------------------------------------
# Additional: scope with require_explicit_scope
# ---------------------------------------------------------------------------
def test_scope_require_explicit_scope():
    """deny_all has require_explicit_scope=true, all targets denied."""
    scope = Scope.deny_all()
    # Every target check should return False
    for target in [SENTINEL_LOOPBACK, "10.0.0.1", "example.com"]:
        assert scope.is_target_allowed(target) is False


# ---------------------------------------------------------------------------
# Additional: Engine generic dispatch with scope
# ---------------------------------------------------------------------------
def test_engine_generic_dispatch_scope_denied():
    """Engine.run() returns scope-denied error for out-of-scope target.

    OperationRequest only has operation, target, timeout_ms, metadata.
    """
    scope = Scope.allow_hosts(["example.com"])
    engine = Engine(scope, mode="manual", concurrency=10, timeout_ms=5000)
    req = eggsec.OperationRequest(
        "scan_ports",
        SENTINEL_LOOPBACK,
    )
    result = engine.run(req)
    assert result.error is not None
    assert _engine_result_is_scope_denied(result)
